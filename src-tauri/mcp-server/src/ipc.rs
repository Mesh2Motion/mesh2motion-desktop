use std::{env, future::Future, io, pin::Pin, time::Duration};

use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout,
};

use crate::model::ControlSnapshot;

pub const IPC_PROTOCOL_VERSION: u16 = 1;
const IPC_MAX_FRAME_BYTES: usize = 256 * 1024;
const IPC_TIMEOUT: Duration = Duration::from_millis(500);
const IPC_ENDPOINT_ENV: &str = "M2M_MCP_IPC_ENDPOINT";

#[cfg(all(test, unix))]
static NEXT_TEST_SOCKET: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IpcRequest {
    pub protocol_version: u16,
    pub request_id: u64,
    pub method: IpcMethod,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IpcMethod {
    SnapshotGet,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IpcResponse {
    pub protocol_version: u16,
    pub request_id: u64,
    pub snapshot: Option<ControlSnapshot>,
    pub error: Option<IpcErrorCode>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IpcErrorCode {
    AppUnavailable,
    NotReady,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IpcError {
    Unavailable,
    NotReady,
    InvalidResponse,
}

pub type SnapshotFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ControlSnapshot, IpcError>> + Send + 'a>>;

pub trait AppSnapshotSource: Send + Sync {
    fn snapshot(&self) -> SnapshotFuture<'_>;
}

/// Client for the one-request/one-response local IPC contract. An endpoint is
/// accepted only from explicit MCP process configuration; there is no TCP
/// fallback and no filesystem query other than connecting to the socket name.
#[derive(Clone, Debug, Default)]
pub struct IpcClient {
    endpoint: Option<String>,
}

impl IpcClient {
    pub fn from_environment() -> Self {
        Self {
            endpoint: env::var(IPC_ENDPOINT_ENV)
                .ok()
                .filter(|value| !value.is_empty()),
        }
    }

    pub fn with_endpoint(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: Some(endpoint.into()),
        }
    }

    async fn request_snapshot(&self) -> Result<ControlSnapshot, IpcError> {
        let endpoint = self.endpoint.as_deref().ok_or(IpcError::Unavailable)?;
        let mut stream = connect_stream(endpoint).await?;

        let request = IpcRequest {
            protocol_version: IPC_PROTOCOL_VERSION,
            request_id: 1,
            method: IpcMethod::SnapshotGet,
        };
        let request_bytes = serde_json::to_vec(&request).map_err(|_| IpcError::InvalidResponse)?;
        write_frame(&mut stream, &request_bytes).await?;
        let response_bytes = read_frame(&mut stream).await?;
        let response: IpcResponse =
            serde_json::from_slice(&response_bytes).map_err(|_| IpcError::InvalidResponse)?;
        if response.protocol_version != IPC_PROTOCOL_VERSION || response.request_id != 1 {
            return Err(IpcError::InvalidResponse);
        }
        match (response.snapshot, response.error) {
            (Some(mut snapshot), None) if snapshot.validate().is_ok() => {
                snapshot.refresh_age();
                Ok(snapshot)
            }
            (Some(_), None) => Err(IpcError::InvalidResponse),
            (None, Some(IpcErrorCode::AppUnavailable)) => Err(IpcError::Unavailable),
            (None, Some(IpcErrorCode::NotReady)) => Err(IpcError::NotReady),
            _ => Err(IpcError::InvalidResponse),
        }
    }
}

impl AppSnapshotSource for IpcClient {
    fn snapshot(&self) -> SnapshotFuture<'_> {
        Box::pin(self.request_snapshot())
    }
}

#[cfg(unix)]
type IpcStream = tokio::net::UnixStream;

#[cfg(windows)]
type IpcStream = tokio::net::windows::named_pipe::NamedPipeClient;

#[cfg(unix)]
async fn connect_stream(endpoint: &str) -> Result<IpcStream, IpcError> {
    timeout(IPC_TIMEOUT, IpcStream::connect(endpoint))
        .await
        .map_err(|_| IpcError::Unavailable)?
        .map_err(|_| IpcError::Unavailable)
}

#[cfg(windows)]
async fn connect_stream(endpoint: &str) -> Result<IpcStream, IpcError> {
    use tokio::{net::windows::named_pipe::ClientOptions, time::sleep};

    timeout(IPC_TIMEOUT, async {
        loop {
            match ClientOptions::new().open(endpoint) {
                Ok(stream) => return Ok(stream),
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::NotFound | io::ErrorKind::WouldBlock
                    ) =>
                {
                    sleep(Duration::from_millis(20)).await;
                }
                Err(_) => return Err(IpcError::Unavailable),
            }
        }
    })
    .await
    .map_err(|_| IpcError::Unavailable)?
}

#[cfg(not(any(unix, windows)))]
async fn connect_stream(_endpoint: &str) -> Result<(), IpcError> {
    Err(IpcError::Unavailable)
}

async fn write_frame<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    payload: &[u8],
) -> Result<(), IpcError> {
    if payload.is_empty() || payload.len() > IPC_MAX_FRAME_BYTES {
        return Err(IpcError::InvalidResponse);
    }
    let length = u32::try_from(payload.len()).map_err(|_| IpcError::InvalidResponse)?;
    timeout(IPC_TIMEOUT, async {
        writer.write_all(&length.to_be_bytes()).await?;
        writer.write_all(payload).await?;
        writer.flush().await
    })
    .await
    .map_err(|_| IpcError::Unavailable)?
    .map_err(|_| IpcError::Unavailable)
}

async fn read_frame<R: AsyncReadExt + Unpin>(reader: &mut R) -> Result<Vec<u8>, IpcError> {
    timeout(IPC_TIMEOUT, async {
        let mut prefix = [0_u8; 4];
        reader.read_exact(&mut prefix).await?;
        let length = u32::from_be_bytes(prefix) as usize;
        if length == 0 || length > IPC_MAX_FRAME_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid local IPC frame length",
            ));
        }
        let mut payload = vec![0_u8; length];
        reader.read_exact(&mut payload).await?;
        Ok(payload)
    })
    .await
    .map_err(|_| IpcError::Unavailable)?
    .map_err(|_| IpcError::InvalidResponse)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipc_request_rejects_unknown_fields() {
        let request =
            r#"{"protocolVersion":1,"requestId":7,"method":"snapshot_get","extra":"bad"}"#;
        assert!(serde_json::from_str::<IpcRequest>(request).is_err());
    }

    #[test]
    fn ipc_request_round_trips_versioned_snapshot_method() {
        let request = IpcRequest {
            protocol_version: IPC_PROTOCOL_VERSION,
            request_id: 7,
            method: IpcMethod::SnapshotGet,
        };
        let encoded = serde_json::to_string(&request).unwrap();
        let decoded: IpcRequest = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.protocol_version, IPC_PROTOCOL_VERSION);
        assert_eq!(decoded.request_id, 7);
    }

    #[tokio::test]
    async fn no_endpoint_is_explicitly_unavailable() {
        let client = IpcClient::default();
        assert_eq!(client.snapshot().await.unwrap_err(), IpcError::Unavailable);
    }

    #[tokio::test]
    async fn oversized_local_ipc_frame_is_rejected_before_allocation() {
        let (mut writer, mut reader) = tokio::io::duplex(8);
        writer
            .write_all(&((IPC_MAX_FRAME_BYTES as u32) + 1).to_be_bytes())
            .await
            .unwrap();
        assert_eq!(
            read_frame(&mut reader).await.unwrap_err(),
            IpcError::InvalidResponse
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn local_ipc_response_timeout_is_bounded() {
        use std::sync::atomic::Ordering;

        let suffix = NEXT_TEST_SOCKET.fetch_add(1, Ordering::Relaxed);
        let socket_path =
            env::temp_dir().join(format!("m2m-mcp-test-{}-{suffix}.sock", std::process::id()));
        let listener = tokio::net::UnixListener::bind(&socket_path).unwrap();
        let server_task = tokio::spawn(async move {
            let (_stream, _) = listener.accept().await.unwrap();
            tokio::time::sleep(IPC_TIMEOUT + Duration::from_millis(100)).await;
        });

        let endpoint = socket_path.to_string_lossy().into_owned();
        let result = IpcClient::with_endpoint(endpoint).snapshot().await;
        assert_eq!(result.unwrap_err(), IpcError::Unavailable);
        server_task.abort();
        let _ = std::fs::remove_file(socket_path);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn local_ipc_accepts_versioned_valid_snapshot() {
        use std::sync::atomic::Ordering;

        let suffix = NEXT_TEST_SOCKET.fetch_add(1, Ordering::Relaxed);
        let socket_path =
            env::temp_dir().join(format!("m2m-mcp-test-{}-{suffix}.sock", std::process::id()));
        let listener = tokio::net::UnixListener::bind(&socket_path).unwrap();
        let server_task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let request_bytes = read_frame(&mut stream).await.unwrap();
            let request: IpcRequest = serde_json::from_slice(&request_bytes).unwrap();
            assert_eq!(request.protocol_version, IPC_PROTOCOL_VERSION);
            assert!(matches!(request.method, IpcMethod::SnapshotGet));
            let response = IpcResponse {
                protocol_version: IPC_PROTOCOL_VERSION,
                request_id: request.request_id,
                snapshot: Some(crate::tests::mock_snapshot()),
                error: None,
            };
            let response_bytes = serde_json::to_vec(&response).unwrap();
            write_frame(&mut stream, &response_bytes).await.unwrap();
        });

        let endpoint = socket_path.to_string_lossy().into_owned();
        let snapshot = IpcClient::with_endpoint(endpoint).snapshot().await.unwrap();
        assert_eq!(snapshot.meta.sequence, 9);
        assert!(snapshot.meta.age_ms < 5_000);
        server_task.await.unwrap();
        let _ = std::fs::remove_file(socket_path);
    }
}
