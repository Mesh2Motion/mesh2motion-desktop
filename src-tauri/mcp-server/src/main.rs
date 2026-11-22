use std::sync::Arc;

use mesh2motion_mcp::{IpcClient, Mesh2MotionMcp};
use rmcp::{transport::stdio, ServiceExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // This binary is opt-in: no Tauri code launches it. MCP protocol bytes use
    // stdout exclusively; diagnostics must never be written to stdout.
    let service = Mesh2MotionMcp::new(Arc::new(IpcClient::from_environment()))
        .serve(stdio())
        .await?;
    service.waiting().await?;
    Ok(())
}
