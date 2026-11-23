#![forbid(unsafe_code)]

mod animation_document;
mod authoring_bridge;
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

pub use animation_document::*;
pub use authoring_bridge::{
    AuthoringError, AuthoringErrorCode, AuthoringOperation, AuthoringRequest, AuthoringResponse,
    AUTHORING_BRIDGE_PROTOCOL, MAX_AUTHORING_DEADLINE_MS, MAX_AUTHORING_ID_BYTES,
    MAX_AUTHORING_REQUEST_BYTES, MIN_AUTHORING_DEADLINE_MS,
};
#[cfg(all(feature = "ipc-host", unix))]
pub use host::{HostError, McpHost};
#[cfg(all(feature = "ipc-host", not(unix)))]
pub use host_stub::{HostError, McpHost};
pub use ipc::{
    AppSnapshotSource, AuthoringFuture, AuthoringHandler, IpcClient, IpcError, IpcRequest,
    IpcResponse, SnapshotFuture, IPC_MAX_FRAME_BYTES, IPC_PROTOCOL_VERSION,
};
pub use model::*;
#[cfg(feature = "mcp-server")]
pub use server::Mesh2MotionMcp;
#[cfg(feature = "ipc-host")]
pub use store::{SnapshotStore, StoreDiagnostics, StoreError, StoreStatus};

#[cfg(all(test, feature = "mcp-server"))]
mod tests;
