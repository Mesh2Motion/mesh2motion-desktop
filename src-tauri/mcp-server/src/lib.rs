#![forbid(unsafe_code)]

mod ipc;
mod model;
mod server;

pub use ipc::{
    AppSnapshotSource, IpcClient, IpcError, IpcRequest, IpcResponse, SnapshotFuture,
    IPC_PROTOCOL_VERSION,
};
pub use model::*;
pub use server::Mesh2MotionMcp;

#[cfg(test)]
mod tests;
