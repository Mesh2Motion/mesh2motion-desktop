#[cfg(feature = "mcp-host")]
use mesh2motion_mcp::AppSnapshotSource;
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;
#[cfg(feature = "mcp-host")]
use std::sync::Mutex;
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Manager, State};

fn preview_asset_relative_path(asset_key: &str) -> Result<PathBuf, String> {
    let mut segments = asset_key.split('/');
    let folder = segments.next().unwrap_or_default();
    let filename = segments.next().unwrap_or_default();
    if segments.next().is_some()
        || !matches!(
            folder,
            "human" | "fox" | "bird" | "dragon" | "kaiju" | "spider" | "snake" | "shark" | "horse"
        )
        || !(filename.starts_with("dark_") || filename.starts_with("light_"))
        || !filename.ends_with(".mp4")
    {
        return Err("INVALID_PREVIEW_ASSET".to_owned());
    }

    let animation = filename
        .strip_prefix("dark_")
        .or_else(|| filename.strip_prefix("light_"))
        .and_then(|name| name.strip_suffix(".mp4"))
        .unwrap_or_default();
    if animation.is_empty()
        || !animation.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, ' ' | '_' | '-')
        })
    {
        return Err("INVALID_PREVIEW_ASSET".to_owned());
    }

    Ok(PathBuf::from(folder).join(filename))
}

/// Resolve only bundled preview keys into the read-only asset protocol scope.
/// The frontend never supplies or receives an arbitrary user filesystem path.
#[tauri::command]
fn animation_preview_asset_path(app: AppHandle, asset_key: String) -> Result<String, String> {
    let relative_path = preview_asset_relative_path(&asset_key)?;
    let resource_root = app
        .path()
        .resolve("animpreviews", BaseDirectory::Resource)
        .map_err(|_| "PREVIEW_ASSETS_UNAVAILABLE".to_owned())?;
    let canonical_root = resource_root
        .canonicalize()
        .map_err(|_| "PREVIEW_ASSETS_UNAVAILABLE".to_owned())?;
    let canonical_file = canonical_root
        .join(relative_path)
        .canonicalize()
        .map_err(|_| "PREVIEW_NOT_FOUND".to_owned())?;

    if !canonical_file.starts_with(&canonical_root) || !canonical_file.is_file() {
        return Err("INVALID_PREVIEW_ASSET".to_owned());
    }

    Ok(canonical_file.to_string_lossy().into_owned())
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// Runtime-visible host truth. This contains counters and snapshot metadata
/// only; it never returns a model path or snapshot payload.
#[derive(Clone, Debug, Default, Serialize)]
struct McpHostStatus {
    supported: bool,
    enabled: bool,
    endpoint: Option<String>,
    client_count: Option<u64>,
    commands: Vec<String>,
    sidecar_path: Option<String>,
    tools: Vec<String>,
    socket_available: bool,
    snapshot_available: bool,
    publish_attempts: u64,
    publish_accepted: u64,
    publish_rejected: u64,
    last_accepted_sequence: Option<u64>,
    last_accepted_at: Option<String>,
    snapshot_age_ms: Option<u64>,
    last_rejection_code: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct McpConnectionTest {
    supported: bool,
    enabled: bool,
    socket_available: bool,
    connected: bool,
    snapshot_available: bool,
    snapshot_fresh: bool,
    snapshot_age_ms: Option<u64>,
    error_code: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct McpTestConnection {
    ok: bool,
    message: String,
    latency_ms: Option<u64>,
}

#[derive(Clone, Debug, Default, Serialize)]
struct McpHostPaths {
    sidecar_path: Option<String>,
    socket_path: Option<String>,
}

#[cfg(feature = "mcp-host")]
struct McpHostState {
    store: mesh2motion_mcp::SnapshotStore,
    host: Mutex<Option<mesh2motion_mcp::McpHost>>,
}

#[cfg(not(feature = "mcp-host"))]
#[derive(Default)]
struct McpHostState;

#[cfg(feature = "mcp-host")]
impl McpHostState {
    fn new() -> Self {
        let store = mesh2motion_mcp::SnapshotStore::default();
        let host = if std::env::var("M2M_MCP_ENABLE").as_deref() == Ok("1") {
            match mesh2motion_mcp::McpHost::start(store.clone()) {
                Ok(host) => {
                    eprintln!(
                        "M2M_HOST enabled=true socket_bound=true endpoint={}",
                        host.endpoint().display()
                    );
                    Some(host)
                }
                Err(error) => {
                    eprintln!("M2M_HOST enabled=false start_error={error:?}");
                    None
                }
            }
        } else {
            None
        };
        Self {
            store,
            host: Mutex::new(host),
        }
    }

    fn status(&self) -> Result<McpHostStatus, String> {
        let diagnostics = self.store.diagnostics();
        let host = self
            .host
            .lock()
            .map_err(|_| "MCP_HOST_STATE_LOCKED".to_owned())?;
        let enabled = host.is_some();
        let socket_available = host
            .as_ref()
            .is_some_and(mesh2motion_mcp::McpHost::socket_available);
        let endpoint = host
            .as_ref()
            .map(|host| host.endpoint().to_string_lossy().into_owned());
        Ok(McpHostStatus {
            supported: true,
            enabled,
            endpoint,
            client_count: None,
            commands: vec![
                "mcp_host_enable".to_owned(),
                "mcp_host_disable".to_owned(),
                "mcp_connection_test".to_owned(),
                "mcp_set_enabled".to_owned(),
                "mcp_test_connection".to_owned(),
                "mcp_host_paths".to_owned(),
            ],
            sidecar_path: configured_sidecar_path(),
            tools: vec![
                "application_get_status".to_owned(),
                "renderer_get_state".to_owned(),
                "rigs_list".to_owned(),
                "animations_list".to_owned(),
                "scene_inspect".to_owned(),
            ],
            socket_available,
            snapshot_available: diagnostics.snapshot_available,
            publish_attempts: diagnostics.publish_attempts,
            publish_accepted: diagnostics.publish_accepted,
            publish_rejected: diagnostics.publish_rejected,
            last_accepted_sequence: diagnostics.last_accepted_sequence,
            last_accepted_at: diagnostics.last_accepted_at,
            snapshot_age_ms: diagnostics.snapshot_age_ms,
            last_rejection_code: diagnostics
                .last_rejection
                .map(store_error_code)
                .map(str::to_owned),
        })
    }

    fn enable(&self) -> Result<(), String> {
        let mut host = self
            .host
            .lock()
            .map_err(|_| "MCP_HOST_STATE_LOCKED".to_owned())?;
        if host.is_some() {
            return Ok(());
        }
        let started = mesh2motion_mcp::McpHost::start(self.store.clone())
            .map_err(|error| host_error_code(&error).to_owned())?;
        *host = Some(started);
        let endpoint = host
            .as_ref()
            .map(|host| host.endpoint().display().to_string())
            .unwrap_or_default();
        eprintln!("M2M_HOST enabled=true socket_bound=true endpoint={endpoint}");
        Ok(())
    }

    fn disable(&self) -> Result<(), String> {
        let mut host = self
            .host
            .lock()
            .map_err(|_| "MCP_HOST_STATE_LOCKED".to_owned())?;
        if host.take().is_some() {
            eprintln!("M2M_HOST enabled=false socket_bound=false");
        }
        Ok(())
    }

    fn endpoint(&self) -> Result<Option<String>, String> {
        let host = self
            .host
            .lock()
            .map_err(|_| "MCP_HOST_STATE_LOCKED".to_owned())?;
        Ok(host
            .as_ref()
            .map(|host| host.endpoint().to_string_lossy().into_owned()))
    }
}

#[cfg(feature = "mcp-host")]
fn store_error_code(error: mesh2motion_mcp::StoreError) -> &'static str {
    match error {
        mesh2motion_mcp::StoreError::TooLarge => "SNAPSHOT_TOO_LARGE",
        mesh2motion_mcp::StoreError::InvalidSnapshot => "INVALID_SNAPSHOT",
        mesh2motion_mcp::StoreError::IncompatibleVersion => "INCOMPATIBLE_VERSION",
        mesh2motion_mcp::StoreError::NonMonotonicSequence => "NON_MONOTONIC_SEQUENCE",
        mesh2motion_mcp::StoreError::Unavailable => "STORE_UNAVAILABLE",
    }
}

#[cfg(feature = "mcp-host")]
fn host_error_code(error: &mesh2motion_mcp::HostError) -> &'static str {
    #[cfg(unix)]
    {
        match error {
            mesh2motion_mcp::HostError::RuntimeDirectory => "MCP_RUNTIME_DIRECTORY",
            mesh2motion_mcp::HostError::UnsafeRuntimeDirectory => "MCP_PERMISSION_DENIED",
            mesh2motion_mcp::HostError::AlreadyRunning => "MCP_SOCKET_IN_USE",
            mesh2motion_mcp::HostError::Bind(_) => "MCP_SOCKET_BIND_FAILED",
            mesh2motion_mcp::HostError::Runtime(_) => "MCP_HOST_START_FAILED",
        }
    }
    #[cfg(not(unix))]
    {
        let _ = error;
        "MCP_UNSUPPORTED_PLATFORM"
    }
}

#[cfg(feature = "mcp-host")]
fn configured_sidecar_path() -> Option<String> {
    std::env::var_os("M2M_MCP_SIDECAR_PATH")
        .map(std::path::PathBuf::from)
        .filter(|path| path.is_absolute())
        .map(|path| path.to_string_lossy().into_owned())
}

#[cfg(not(feature = "mcp-host"))]
impl McpHostState {
    fn new() -> Self {
        Self
    }
}

#[tauri::command]
fn mcp_host_status(state: State<'_, McpHostState>) -> Result<McpHostStatus, String> {
    #[cfg(feature = "mcp-host")]
    {
        state.status()
    }
    #[cfg(not(feature = "mcp-host"))]
    {
        let _ = state;
        Ok(McpHostStatus::default())
    }
}

#[tauri::command]
fn mcp_host_enable(state: State<'_, McpHostState>) -> Result<McpHostStatus, String> {
    #[cfg(feature = "mcp-host")]
    {
        state.enable()?;
        state.status()
    }
    #[cfg(not(feature = "mcp-host"))]
    {
        let _ = state;
        Err("MCP_UNSUPPORTED_FEATURE".to_owned())
    }
}

#[tauri::command]
fn mcp_host_disable(state: State<'_, McpHostState>) -> Result<McpHostStatus, String> {
    #[cfg(feature = "mcp-host")]
    {
        state.disable()?;
        state.status()
    }
    #[cfg(not(feature = "mcp-host"))]
    {
        let _ = state;
        Err("MCP_UNSUPPORTED_FEATURE".to_owned())
    }
}

/// UI adapter contract: the frontend supplies only a boolean and receives no
/// endpoint or snapshot data as part of the mutation result.
#[tauri::command]
fn mcp_set_enabled(enabled: bool, state: State<'_, McpHostState>) -> Result<(), String> {
    #[cfg(feature = "mcp-host")]
    {
        if enabled {
            state.enable()
        } else {
            state.disable()
        }
    }
    #[cfg(not(feature = "mcp-host"))]
    {
        let _ = (enabled, state);
        Err("MCP_UNSUPPORTED_FEATURE".to_owned())
    }
}

#[tauri::command]
fn mcp_host_paths(state: State<'_, McpHostState>) -> Result<McpHostPaths, String> {
    #[cfg(feature = "mcp-host")]
    {
        Ok(McpHostPaths {
            sidecar_path: configured_sidecar_path(),
            socket_path: state.endpoint()?,
        })
    }
    #[cfg(not(feature = "mcp-host"))]
    {
        let _ = state;
        Err("MCP_UNSUPPORTED_FEATURE".to_owned())
    }
}

#[tauri::command]
async fn mcp_connection_test(state: State<'_, McpHostState>) -> Result<McpConnectionTest, String> {
    #[cfg(feature = "mcp-host")]
    {
        let endpoint = match state.endpoint() {
            Ok(Some(endpoint)) => endpoint,
            Ok(None) => {
                return Ok(McpConnectionTest {
                    supported: true,
                    enabled: false,
                    socket_available: false,
                    connected: false,
                    snapshot_available: false,
                    snapshot_fresh: false,
                    snapshot_age_ms: None,
                    error_code: Some("MCP_HOST_DISABLED".to_owned()),
                });
            }
            Err(error) => {
                return Ok(McpConnectionTest {
                    supported: true,
                    enabled: true,
                    socket_available: false,
                    connected: false,
                    snapshot_available: false,
                    snapshot_fresh: false,
                    snapshot_age_ms: None,
                    error_code: Some(error),
                });
            }
        };
        let result = mesh2motion_mcp::IpcClient::with_endpoint(endpoint)
            .snapshot()
            .await;
        match result {
            Ok(snapshot) => {
                let age = snapshot.meta.age_ms;
                Ok(McpConnectionTest {
                    supported: true,
                    enabled: true,
                    socket_available: true,
                    connected: true,
                    snapshot_available: true,
                    snapshot_fresh: age <= 5_000,
                    snapshot_age_ms: Some(age),
                    error_code: (age > 5_000).then(|| "STALE_STATE".to_owned()),
                })
            }
            Err(mesh2motion_mcp::IpcError::AppUnavailable) => Ok(McpConnectionTest {
                supported: true,
                enabled: true,
                socket_available: true,
                connected: true,
                snapshot_available: false,
                snapshot_fresh: false,
                snapshot_age_ms: None,
                error_code: Some("APP_UNAVAILABLE".to_owned()),
            }),
            Err(mesh2motion_mcp::IpcError::NotReady) => Ok(McpConnectionTest {
                supported: true,
                enabled: true,
                socket_available: true,
                connected: true,
                snapshot_available: false,
                snapshot_fresh: false,
                snapshot_age_ms: None,
                error_code: Some("APP_NOT_READY".to_owned()),
            }),
            Err(mesh2motion_mcp::IpcError::PermissionDenied) => Ok(McpConnectionTest {
                supported: true,
                enabled: true,
                socket_available: true,
                connected: true,
                snapshot_available: false,
                snapshot_fresh: false,
                snapshot_age_ms: None,
                error_code: Some("PERMISSION_DENIED".to_owned()),
            }),
            Err(mesh2motion_mcp::IpcError::IncompatibleVersion) => Ok(McpConnectionTest {
                supported: true,
                enabled: true,
                socket_available: true,
                connected: true,
                snapshot_available: false,
                snapshot_fresh: false,
                snapshot_age_ms: None,
                error_code: Some("INCOMPATIBLE_VERSION".to_owned()),
            }),
            Err(error) => Ok(McpConnectionTest {
                supported: true,
                enabled: true,
                socket_available: false,
                connected: false,
                snapshot_available: false,
                snapshot_fresh: false,
                snapshot_age_ms: None,
                error_code: Some(format!("{error:?}").to_ascii_uppercase()),
            }),
        }
    }
    #[cfg(not(feature = "mcp-host"))]
    {
        let _ = state;
        Ok(McpConnectionTest {
            supported: false,
            enabled: false,
            socket_available: false,
            connected: false,
            snapshot_available: false,
            snapshot_fresh: false,
            snapshot_age_ms: None,
            error_code: Some("MCP_UNSUPPORTED_FEATURE".to_owned()),
        })
    }
}

#[tauri::command]
async fn mcp_test_connection(state: State<'_, McpHostState>) -> Result<McpTestConnection, String> {
    #[cfg(feature = "mcp-host")]
    {
        let started = std::time::Instant::now();
        let result = mcp_connection_test(state).await?;
        let ok = result.connected && result.snapshot_fresh;
        let message = match result.error_code.as_deref() {
            Some(code) => code.to_owned(),
            None if ok => "Connected; snapshot is fresh".to_owned(),
            None => "Connected; snapshot is unavailable".to_owned(),
        };
        Ok(McpTestConnection {
            ok,
            message,
            latency_ms: Some(started.elapsed().as_millis().min(u64::MAX as u128) as u64),
        })
    }
    #[cfg(not(feature = "mcp-host"))]
    {
        let _ = state;
        Ok(McpTestConnection {
            ok: false,
            message: "MCP_UNSUPPORTED_FEATURE".to_owned(),
            latency_ms: None,
        })
    }
}

#[tauri::command]
fn mcp_publish_snapshot(snapshot: Value, state: State<'_, McpHostState>) -> Result<(), String> {
    #[cfg(feature = "mcp-host")]
    {
        let guard = state
            .host
            .lock()
            .map_err(|_| "MCP_HOST_STATE_LOCKED".to_owned())?;
        if guard.is_none() {
            return Err("MCP_HOST_DISABLED".to_owned());
        }
        let sequence = snapshot.pointer("/meta/sequence").and_then(Value::as_u64);
        let captured_at = snapshot
            .pointer("/meta/captured_at")
            .or_else(|| snapshot.pointer("/meta/capturedAt"))
            .and_then(Value::as_str)
            .filter(|value| {
                value.len() <= 64
                    && value.bytes().all(|byte| {
                        byte.is_ascii_digit()
                            || matches!(byte, b'T' | b'Z' | b'+' | b'-' | b'.' | b':')
                    })
            })
            .unwrap_or("invalid")
            .to_owned();
        match state.store.publish_json(snapshot) {
            Ok(()) => {
                let diagnostics = state.store.diagnostics();
                eprintln!(
                    "M2M_PUBLISH result=accepted sequence={} captured_at={} age_ms={}",
                    diagnostics.last_accepted_sequence.unwrap_or_default(),
                    diagnostics.last_accepted_at.as_deref().unwrap_or("unknown"),
                    diagnostics.snapshot_age_ms.unwrap_or_default()
                );
                Ok(())
            }
            Err(error) => {
                eprintln!(
                    "M2M_PUBLISH result=rejected code={} sequence={} captured_at={}",
                    store_error_code(error),
                    sequence.map_or_else(|| "missing".to_owned(), |value| value.to_string()),
                    captured_at
                );
                Err(store_error_code(error).to_owned())
            }
        }
    }
    #[cfg(not(feature = "mcp-host"))]
    {
        let _ = (snapshot, state);
        Err("MCP_UNSUPPORTED_FEATURE".to_owned())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app.manage(McpHostState::new());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            animation_preview_asset_path,
            mcp_publish_snapshot,
            mcp_host_status,
            mcp_host_enable,
            mcp_host_disable,
            mcp_connection_test,
            mcp_set_enabled,
            mcp_test_connection,
            mcp_host_paths
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod contract_tests {
    use super::{preview_asset_relative_path, McpHostStatus};

    #[cfg(feature = "mcp-host")]
    #[test]
    fn poisoned_host_status_returns_an_error() {
        let state = super::McpHostState {
            store: mesh2motion_mcp::SnapshotStore::default(),
            host: std::sync::Mutex::new(None),
        };
        let poison = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = state.host.lock().unwrap();
            panic!("poison host state for test");
        }));

        assert!(poison.is_err());
        assert!(matches!(
            state.status(),
            Err(error) if error == "MCP_HOST_STATE_LOCKED"
        ));
    }

    #[test]
    fn preview_asset_keys_are_narrow_and_relative() {
        assert_eq!(
            preview_asset_relative_path("human/dark_Angry.mp4").unwrap(),
            std::path::PathBuf::from("human/dark_Angry.mp4")
        );
        for invalid in [
            "../secret.mp4",
            "human/../../secret.mp4",
            "human/dark_Angry.mp4/extra",
            "human/dark_../secret.mp4",
            "unknown/dark_Walk.mp4",
            "human/blue_Walk.mp4",
            "human/dark_.mp4",
        ] {
            assert!(
                preview_asset_relative_path(invalid).is_err(),
                "accepted {invalid}"
            );
        }
    }

    #[test]
    fn mcp_status_matches_frontend_snake_case_contract() {
        let status = serde_json::to_value(McpHostStatus::default()).unwrap();
        for field in [
            "supported",
            "enabled",
            "endpoint",
            "client_count",
            "commands",
            "sidecar_path",
            "tools",
            "socket_available",
            "publish_attempts",
            "publish_accepted",
            "publish_rejected",
            "last_accepted_sequence",
            "last_accepted_at",
            "snapshot_age_ms",
            "last_rejection_code",
        ] {
            assert!(status.get(field).is_some(), "missing status field {field}");
        }
        assert_eq!(status["supported"], false);
        assert_eq!(status["client_count"], serde_json::Value::Null);
    }
}
