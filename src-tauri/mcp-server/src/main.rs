use std::{env, io, sync::Arc};

#[cfg(target_os = "linux")]
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, Command},
    thread,
    time::Duration,
};
#[cfg(target_os = "linux")]
use std::{os::unix::process::CommandExt, process::Stdio};

use mesh2motion_mcp::{AppPhase, AppSnapshotSource, IpcClient, Mesh2MotionMcp};
use rmcp::{transport::stdio, ServiceExt};

const AUTOSTART_ENV: &str = "M2M_MCP_AUTOSTART_APP";
#[cfg(target_os = "linux")]
const APP_PATH_ENV: &str = "M2M_MCP_APP_PATH";
const IPC_ENDPOINT_ENV: &str = "M2M_MCP_IPC_ENDPOINT";
#[cfg(target_os = "linux")]
const SIDECAR_PATH_ENV: &str = "M2M_MCP_SIDECAR_PATH";
#[cfg(target_os = "linux")]
const APP_ENABLE_ENV: &str = "M2M_MCP_ENABLE";
#[cfg(target_os = "linux")]
const DEFAULT_APP_PATH: &str = "/usr/lib/mesh2motion/mesh2motion";
#[cfg(target_os = "linux")]
const STARTUP_TIMEOUT: Duration = Duration::from_secs(25);
#[cfg(target_os = "linux")]
const STARTUP_POLL_INTERVAL: Duration = Duration::from_millis(250);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Autostart is opt-in through the local MCP client's environment. The
    // binary path is validated and executed directly; no shell or arguments
    // from MCP requests are accepted.
    if autostart_requested(env::var_os(AUTOSTART_ENV).as_deref()) {
        start_application_if_needed().await?;
    }

    // MCP protocol bytes use stdout exclusively; diagnostics stay on stderr.
    let mcp = Mesh2MotionMcp::new(Arc::new(IpcClient::from_environment()));
    let cleanup = mcp.clone();
    let service = mcp.serve(stdio()).await?;
    let wait_result = service.waiting().await;
    cleanup.dispose_observation_captures().await;
    cleanup.cancel_authoring_session().await;
    wait_result?;
    Ok(())
}

fn autostart_requested(value: Option<&std::ffi::OsStr>) -> bool {
    value.is_some_and(|value| value == "1")
}

async fn start_application_if_needed() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(target_os = "linux"))]
    return Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "M2M_MCP_AUTOSTART_APP is currently supported on Linux only",
    )
    .into());

    #[cfg(target_os = "linux")]
    {
        let endpoint = env::var(IPC_ENDPOINT_ENV).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "M2M_MCP_IPC_ENDPOINT is required when application autostart is enabled",
            )
        })?;
        if endpoint.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "M2M_MCP_IPC_ENDPOINT cannot be empty when application autostart is enabled",
            )
            .into());
        }

        let client = IpcClient::with_endpoint(endpoint);
        if client
            .snapshot()
            .await
            .is_ok_and(|snapshot| app_phase_ready(&snapshot.application.phase))
        {
            eprintln!("M2M_MCP_AUTOSTART result=already_ready");
            return Ok(());
        }

        let app_path = configured_app_path()?;
        if application_process_running(&app_path) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Mesh2Motion is running but its MCP host is unavailable; enable MCP in the app",
            )
            .into());
        }

        let sidecar_path = env::current_exe()?;
        let mut command = Command::new(&app_path);
        command
            .env(APP_ENABLE_ENV, "1")
            .env(SIDECAR_PATH_ENV, sidecar_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        // The MCP transport often uses pipes; the GUI must survive
        // sidecar/client shutdown and must not inherit the protocol streams.
        command.process_group(0);
        let child = command.spawn()?;
        let child_pid = child.id();
        reap_child(child);
        eprintln!("M2M_MCP_AUTOSTART result=spawned pid={child_pid}");

        let deadline = tokio::time::Instant::now() + STARTUP_TIMEOUT;
        while tokio::time::Instant::now() < deadline {
            if client
                .snapshot()
                .await
                .is_ok_and(|snapshot| app_phase_ready(&snapshot.application.phase))
            {
                eprintln!("M2M_MCP_AUTOSTART result=ready pid={child_pid}");
                return Ok(());
            }
            tokio::time::sleep(STARTUP_POLL_INTERVAL).await;
        }

        Err(io::Error::new(
        io::ErrorKind::TimedOut,
        format!(
            "Mesh2Motion process {child_pid} did not publish a fresh MCP snapshot within {} seconds",
            STARTUP_TIMEOUT.as_secs()
        ),
    )
    .into())
    }
}

fn app_phase_ready(phase: &AppPhase) -> bool {
    matches!(phase, AppPhase::Ready | AppPhase::Degraded)
}

#[cfg(target_os = "linux")]
fn configured_app_path() -> Result<PathBuf, io::Error> {
    let configured = env::var_os(APP_PATH_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_APP_PATH));
    if !configured.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "M2M_MCP_APP_PATH must be an absolute executable path",
        ));
    }
    validate_app_path(configured)
}

#[cfg(target_os = "linux")]
fn validate_app_path(configured: PathBuf) -> Result<PathBuf, io::Error> {
    if !configured.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "M2M_MCP_APP_PATH must be an absolute executable path",
        ));
    }
    let path = fs::canonicalize(configured)?;
    let metadata = fs::metadata(&path)?;
    if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "configured Mesh2Motion executable is not an executable file",
        ));
    }
    Ok(path)
}

#[cfg(target_os = "linux")]
fn application_process_running(app_path: &Path) -> bool {
    let Ok(expected) = fs::canonicalize(app_path) else {
        return false;
    };
    let Ok(processes) = fs::read_dir("/proc") else {
        return false;
    };
    processes.flatten().any(|process| {
        if !process
            .file_name()
            .to_string_lossy()
            .bytes()
            .all(|b| b.is_ascii_digit())
        {
            return false;
        }
        let executable = process.path().join("exe");
        fs::read_link(executable)
            .ok()
            .and_then(|path| fs::canonicalize(path).ok())
            .is_some_and(|path| path == expected)
    })
}

#[cfg(target_os = "linux")]
fn reap_child(mut child: Child) {
    let _ = thread::Builder::new()
        .name("m2m-mcp-app-reaper".into())
        .spawn(move || match child.wait() {
            Ok(status) => eprintln!("M2M_MCP_AUTOSTART app_exit status={status}"),
            Err(error) => eprintln!("M2M_MCP_AUTOSTART app_wait_error={error}"),
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    #[test]
    fn autostart_requires_exact_explicit_opt_in() {
        assert!(!autostart_requested(None));
        assert!(!autostart_requested(Some(OsStr::new("0"))));
        assert!(!autostart_requested(Some(OsStr::new("true"))));
        assert!(autostart_requested(Some(OsStr::new("1"))));
    }

    #[test]
    fn app_startup_waits_until_read_tools_are_usable() {
        assert!(app_phase_ready(&AppPhase::Ready));
        assert!(app_phase_ready(&AppPhase::Degraded));
        assert!(!app_phase_ready(&AppPhase::Loading));
        assert!(!app_phase_ready(&AppPhase::Uninitialized));
        assert!(!app_phase_ready(&AppPhase::Fatal));
        assert!(!app_phase_ready(&AppPhase::Closed));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn configured_application_path_must_be_absolute_and_executable() {
        let executable = env::current_exe().unwrap();
        let validated = validate_app_path(executable.clone()).unwrap();
        assert!(validated.is_absolute());
        assert!(fs::metadata(validated).unwrap().is_file());
        assert_eq!(
            validate_app_path(PathBuf::from("relative/mesh2motion"))
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
    }
}
