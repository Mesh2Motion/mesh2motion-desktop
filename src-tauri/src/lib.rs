// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[cfg(feature = "mcp-host")]
use tauri::Manager;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg(feature = "mcp-host")]
struct McpHostState {
    store: mesh2motion_mcp::SnapshotStore,
    _host: Option<mesh2motion_mcp::McpHost>,
}

#[cfg(feature = "mcp-host")]
#[tauri::command]
fn mcp_publish_snapshot(
    snapshot: serde_json::Value,
    state: tauri::State<'_, McpHostState>,
) -> Result<(), String> {
    if state._host.is_none() {
        return Err("MCP host is disabled; set M2M_MCP_ENABLE=1 to opt in".to_owned());
    }
    state
        .store
        .publish_json(snapshot)
        .map_err(|error| format!("snapshot rejected: {error:?}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            #[cfg(feature = "mcp-host")]
            {
                let store = mesh2motion_mcp::SnapshotStore::default();
                let host = if std::env::var("M2M_MCP_ENABLE").as_deref() == Ok("1") {
                    match mesh2motion_mcp::McpHost::start(store.clone()) {
                        Ok(host) => {
                            eprintln!(
                                "Mesh2Motion MCP local socket ready: {}",
                                host.endpoint().display()
                            );
                            Some(host)
                        }
                        Err(error) => {
                            eprintln!("Mesh2Motion MCP host unavailable: {error:?}");
                            None
                        }
                    }
                } else {
                    None
                };
                app.manage(McpHostState { store, _host: host });
            }
            let _ = app;
            Ok(())
        });
    #[cfg(feature = "mcp-host")]
    {
        builder = builder.invoke_handler(tauri::generate_handler![greet, mcp_publish_snapshot]);
    }
    #[cfg(not(feature = "mcp-host"))]
    {
        builder = builder.invoke_handler(tauri::generate_handler![greet]);
    }
    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
