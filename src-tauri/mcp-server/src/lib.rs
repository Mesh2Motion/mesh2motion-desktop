#![forbid(unsafe_code)]

mod ipc;
mod model;
#[cfg(feature = "mcp-server")]
mod server;

#[cfg(all(feature = "ipc-host", unix))]
mod host;
#[cfg(all(feature = "ipc-host", not(unix)))]
mod host_stub;
#[cfg(feature = "ipc-host")]
mod store;

#[cfg(all(feature = "ipc-host", unix))]
pub use host::{HostError, McpHost};
#[cfg(all(feature = "ipc-host", not(unix)))]
pub use host_stub::{HostError, McpHost};
pub use ipc::{
    AppSnapshotSource, IpcClient, IpcError, IpcRequest, IpcResponse, SnapshotFuture,
    IPC_PROTOCOL_VERSION,
};
pub use model::*;
#[cfg(feature = "mcp-server")]
pub use server::Mesh2MotionMcp;
#[cfg(feature = "ipc-host")]
pub use store::{SnapshotStore, StoreError, StoreStatus};

#[cfg(all(test, feature = "mcp-server"))]
mod tests;
