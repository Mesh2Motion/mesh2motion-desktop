use std::{
    fs,
    os::unix::{
        fs::{FileTypeExt, MetadataExt, PermissionsExt},
        net::{UnixListener as StdListener, UnixStream as StdStream},
    },
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
    sync::Semaphore,
    time::timeout,
};

use crate::{
    ipc::{IpcErrorCode, IpcRequest, IpcResponse, IPC_PROTOCOL_VERSION},
    store::{SnapshotStore, StoreStatus, MAX_SNAPSHOT_BYTES},
};

const MAX_REQUEST_BYTES: usize = 4096;

#[derive(Debug)]
pub enum HostError {
    RuntimeDirectory,
    UnsafeRuntimeDirectory,
    AlreadyRunning,
    Bind(std::io::Error),
    Runtime(std::io::Error),
}

pub struct McpHost {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    endpoint: PathBuf,
}

impl McpHost {
    pub fn start(store: SnapshotStore) -> Result<Self, HostError> {
        let root = runtime_directory().ok_or(HostError::RuntimeDirectory)?;
        Self::start_at(root, store)
    }

    fn start_at(root: PathBuf, store: SnapshotStore) -> Result<Self, HostError> {
        let root_meta = fs::symlink_metadata(&root).map_err(|_| HostError::RuntimeDirectory)?;
        if !root_meta.is_dir()
            || root_meta.file_type().is_symlink()
            || root_meta.permissions().mode() & 0o077 != 0
        {
            return Err(HostError::UnsafeRuntimeDirectory);
        }
        let directory = root.join("mesh2motion");
        match fs::create_dir(&directory) {
            Ok(()) => fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
                .map_err(HostError::Runtime)?,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(HostError::Runtime(e)),
        }
        let dir_meta = fs::symlink_metadata(&directory).map_err(HostError::Runtime)?;
        if !dir_meta.is_dir()
            || dir_meta.file_type().is_symlink()
            || dir_meta.uid() != root_meta.uid()
            || dir_meta.permissions().mode() & 0o077 != 0
        {
            return Err(HostError::UnsafeRuntimeDirectory);
        }
        let endpoint = directory.join("mcp.sock");
        if let Ok(meta) = fs::symlink_metadata(&endpoint) {
            if !meta.file_type().is_socket() || meta.uid() != dir_meta.uid() {
                return Err(HostError::UnsafeRuntimeDirectory);
            }
            match StdStream::connect(&endpoint) {
                Ok(_) => return Err(HostError::AlreadyRunning),
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::NotFound
                    ) => {}
                Err(_) => return Err(HostError::UnsafeRuntimeDirectory),
            }
            fs::remove_file(&endpoint).map_err(HostError::Runtime)?;
        }
        let listener = StdListener::bind(&endpoint).map_err(HostError::Bind)?;
        fs::set_permissions(&endpoint, fs::Permissions::from_mode(0o600))
            .map_err(HostError::Runtime)?;
        listener.set_nonblocking(true).map_err(HostError::Runtime)?;
        let expected_uid = dir_meta.uid();
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let thread_endpoint = endpoint.clone();
        let thread = thread::Builder::new()
            .name("m2m-mcp-ipc".into())
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(_) => {
                        let _ = fs::remove_file(&thread_endpoint);
                        return;
                    }
                };
                runtime.block_on(async move {
                    if let Ok(listener) = UnixListener::from_std(listener) {
                        serve(listener, store, thread_stop, expected_uid).await;
                    }
                });
                let _ = fs::remove_file(&thread_endpoint);
                if let Some(parent) = thread_endpoint.parent() {
                    let _ = fs::remove_dir(parent);
                }
            })
            .map_err(HostError::Runtime)?;
        Ok(Self {
            stop,
            thread: Some(thread),
            endpoint,
        })
    }

    pub fn endpoint(&self) -> &std::path::Path {
        &self.endpoint
    }
}

#[cfg(target_os = "linux")]
fn runtime_directory() -> Option<PathBuf> {
    std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from)
}

#[cfg(all(unix, not(target_os = "linux")))]
fn runtime_directory() -> Option<PathBuf> {
    std::env::var_os("XDG_RUNTIME_DIR")
        .or_else(|| std::env::var_os("TMPDIR"))
        .map(PathBuf::from)
}

impl Drop for McpHost {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

async fn serve(
    listener: UnixListener,
    store: SnapshotStore,
    stop: Arc<AtomicBool>,
    expected_uid: u32,
) {
    let permits = Arc::new(Semaphore::new(16));
    loop {
        if stop.load(Ordering::Acquire) {
            break;
        }
        let accepted = timeout(Duration::from_millis(100), listener.accept()).await;
        let Ok(Ok((stream, _))) = accepted else {
            continue;
        };
        let Ok(permit) = permits.clone().try_acquire_owned() else {
            continue;
        };
        let store = store.clone();
        tokio::spawn(async move {
            let _permit = permit;
            let _ = serve_client(stream, store, expected_uid).await;
        });
    }
}

async fn serve_client(
    mut stream: UnixStream,
    store: SnapshotStore,
    expected_uid: u32,
) -> std::io::Result<()> {
    let peer_uid = stream.peer_cred()?.uid();
    let mut prefix = [0_u8; 4];
    timeout(Duration::from_millis(500), stream.read_exact(&mut prefix))
        .await
        .map_err(timeout_error)??;
    let length = u32::from_be_bytes(prefix) as usize;
    if length == 0 || length > MAX_REQUEST_BYTES {
        return Ok(());
    }
    let mut bytes = vec![0; length];
    timeout(Duration::from_millis(500), stream.read_exact(&mut bytes))
        .await
        .map_err(timeout_error)??;
    let response = match serde_json::from_slice::<IpcRequest>(&bytes) {
        Ok(request) if !peer_uid_allowed(expected_uid, peer_uid) => {
            IpcResponse::error(request.request_id, IpcErrorCode::PermissionDenied)
        }
        Ok(request) if request.protocol_version != IPC_PROTOCOL_VERSION => {
            IpcResponse::error(request.request_id, IpcErrorCode::IncompatibleVersion)
        }
        Ok(request) => match store.status() {
            StoreStatus::IncompatibleVersion => {
                IpcResponse::error(request.request_id, IpcErrorCode::IncompatibleVersion)
            }
            StoreStatus::Unavailable => {
                IpcResponse::error(request.request_id, IpcErrorCode::AppUnavailable)
            }
            StoreStatus::Ready => match store.snapshot() {
                Ok(snapshot) => IpcResponse::snapshot(request.request_id, snapshot),
                Err(_) => IpcResponse::error(request.request_id, IpcErrorCode::AppUnavailable),
            },
        },
        Err(_) => return Ok(()),
    };
    let payload = serde_json::to_vec(&response)?;
    if payload.len() > MAX_SNAPSHOT_BYTES {
        return Ok(());
    }
    timeout(Duration::from_millis(500), async {
        stream
            .write_all(&(payload.len() as u32).to_be_bytes())
            .await?;
        stream.write_all(&payload).await?;
        stream.flush().await
    })
    .await
    .map_err(timeout_error)??;
    Ok(())
}

fn timeout_error(_: tokio::time::error::Elapsed) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::TimedOut, "MCP IPC request timed out")
}

fn peer_uid_allowed(expected_uid: u32, peer_uid: u32) -> bool {
    expected_uid == peer_uid
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    static NEXT_RUNTIME: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn private_runtime() -> PathBuf {
        let suffix = NEXT_RUNTIME.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("m2m009-runtime-{}-{suffix}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    }

    #[test]
    fn rejects_runtime_directory_with_broad_permissions() {
        let path = private_runtime();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(matches!(
            McpHost::start_at(path.clone(), SnapshotStore::default()),
            Err(HostError::UnsafeRuntimeDirectory)
        ));
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn peer_verification_denies_different_uid_and_missing_runtime_identity() {
        assert!(peer_uid_allowed(1000, 1000));
        assert!(!peer_uid_allowed(1000, 1001));
    }

    #[test]
    fn socket_is_private_and_removed_on_shutdown() {
        let path = private_runtime();
        let host = McpHost::start_at(path.clone(), SnapshotStore::default()).unwrap();
        let socket = host.endpoint().to_path_buf();
        assert_eq!(
            fs::metadata(&socket).unwrap().permissions().mode() & 0o777,
            0o600
        );
        drop(host);
        assert!(!socket.exists());
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn refuses_to_replace_an_active_socket() {
        let path = private_runtime();
        let host = McpHost::start_at(path.clone(), SnapshotStore::default()).unwrap();
        assert!(matches!(
            McpHost::start_at(path.clone(), SnapshotStore::default()),
            Err(HostError::AlreadyRunning)
        ));
        assert!(host.endpoint().exists());
        drop(host);
        let _ = fs::remove_dir_all(path);
    }

    #[tokio::test]
    async fn ipc_host_serves_live_snapshots_to_concurrent_sidecar_requests() {
        use crate::AppSnapshotSource;

        let path = private_runtime();
        let store = SnapshotStore::default();
        let mut snapshot = crate::tests::mock_snapshot();
        snapshot.meta.sequence = 42;
        store
            .publish_json(serde_json::to_value(snapshot).unwrap())
            .unwrap();
        let host = McpHost::start_at(path.clone(), store).unwrap();
        let client = crate::IpcClient::with_endpoint(host.endpoint().to_string_lossy());
        let requests: Vec<_> = (0..8)
            .map(|_| {
                let client = client.clone();
                tokio::spawn(async move { client.snapshot().await.unwrap().meta.sequence })
            })
            .collect();
        for request in requests {
            assert_eq!(request.await.unwrap(), 42);
        }
        drop(host);
        let _ = fs::remove_dir_all(path);
    }

    #[tokio::test]
    async fn sidecar_reconnects_after_application_host_restart() {
        use crate::AppSnapshotSource;

        let path = private_runtime();
        let first_store = SnapshotStore::default();
        let mut first_snapshot = crate::tests::mock_snapshot();
        first_snapshot.meta.sequence = 9;
        first_store
            .publish_json(serde_json::to_value(first_snapshot).unwrap())
            .unwrap();
        let first_host = McpHost::start_at(path.clone(), first_store).unwrap();
        let client = crate::IpcClient::with_endpoint(first_host.endpoint().to_string_lossy());
        assert_eq!(client.snapshot().await.unwrap().meta.sequence, 9);
        drop(first_host);

        let second_store = SnapshotStore::default();
        let mut second_snapshot = crate::tests::mock_snapshot();
        second_snapshot.meta.sequence = 1;
        second_store
            .publish_json(serde_json::to_value(second_snapshot).unwrap())
            .unwrap();
        let second_host = McpHost::start_at(path.clone(), second_store).unwrap();
        assert_eq!(client.snapshot().await.unwrap().meta.sequence, 1);
        drop(second_host);
        let _ = fs::remove_dir_all(path);
    }
}
