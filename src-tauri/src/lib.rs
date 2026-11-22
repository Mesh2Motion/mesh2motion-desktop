#[cfg(feature = "mcp-host")]
use mesh2motion_mcp::AppSnapshotSource;
use serde::Serialize;
use serde_json::Value;
#[cfg(feature = "mcp-host")]
use std::sync::Mutex;
use tauri::{Manager, State};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// Runtime-visible host truth. This contains counters and snapshot metadata
/// only; it never returns a model path or snapshot payload.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct McpHostStatus {
    supported: bool,
    enabled: bool,
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
#[serde(rename_all = "camelCase")]
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

    fn status(&self) -> McpHostStatus {
        let diagnostics = self.store.diagnostics();
        let host = self.host.lock().ok();
        let enabled = host.as_ref().is_some_and(|state| state.is_some());
        let socket_available = host
            .as_ref()
            .and_then(|state| state.as_ref())
            .is_some_and(mesh2motion_mcp::McpHost::socket_available);
        McpHostStatus {
            supported: true,
            enabled,
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
        }
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

#[cfg(not(feature = "mcp-host"))]
impl McpHostState {
    fn new() -> Self {
        Self
    }
}

#[tauri::command]
fn mcp_host_status(state: State<'_, McpHostState>) -> McpHostStatus {
    #[cfg(feature = "mcp-host")]
    {
        state.status()
    }
    #[cfg(not(feature = "mcp-host"))]
    {
        let _ = state;
        McpHostStatus::default()
    }
}

#[tauri::command]
fn mcp_host_enable(state: State<'_, McpHostState>) -> Result<McpHostStatus, String> {
    #[cfg(feature = "mcp-host")]
    {
        state.enable()?;
        Ok(state.status())
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
        Ok(state.status())
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
fn mcp_publish_snapshot(snapshot: Value, state: State<'_, McpHostState>) -> Result<(), String> {
    #[cfg(feature = "mcp-host")]
    {
        let guard = state
            .host
            .lock()
            .map_err(|_| "MCP_HOST_STATE_LOCKED".to_owned())?;
        if guard.is_none() {
            eprintln!("M2M_PUBLISH result=rejected code=MCP_HOST_DISABLED");
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
            mcp_publish_snapshot,
            mcp_host_status,
            mcp_host_enable,
            mcp_host_disable,
            mcp_connection_test
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
