use std::{
    collections::{HashMap, HashSet},
    fs,
    os::unix::{
        fs::{FileTypeExt, MetadataExt, PermissionsExt},
        net::{UnixListener as StdListener, UnixStream as StdStream},
    },
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
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
    authoring_bridge::{AuthoringErrorCode, AuthoringOperation, AuthoringRequest},
    ipc::{
        AuthoringHandler, IpcErrorCode, IpcMethod, IpcRequest, IpcResponse, PoseImageChunkHandler,
        PoseObservationHandler, IPC_MAX_FRAME_BYTES, IPC_PROTOCOL_VERSION,
    },
    store::{SnapshotStore, StoreStatus, MAX_SNAPSHOT_BYTES},
};

const MAX_REQUEST_BYTES: usize = IPC_MAX_FRAME_BYTES;
const MAX_AUTHORING_REQUEST_IDS: usize = 4096;

#[derive(Default)]
struct AuthoringGuardState {
    seen_ids: HashSet<String>,
    last_sequence: HashMap<String, u64>,
    active_session: Option<String>,
}

#[derive(Clone, Default)]
struct AuthoringGuard(Arc<Mutex<AuthoringGuardState>>);

impl AuthoringGuard {
    fn reserve(&self, request: &AuthoringRequest) -> Result<(), AuthoringErrorCode> {
        let mut state = self.0.lock().map_err(|_| AuthoringErrorCode::Internal)?;
        if state.seen_ids.contains(&request.request_id) {
            return Err(AuthoringErrorCode::DuplicateRequestId);
        }
        if state.seen_ids.len() >= MAX_AUTHORING_REQUEST_IDS {
            return Err(AuthoringErrorCode::SessionConflict);
        }
        if state
            .last_sequence
            .get(&request.session_id)
            .is_some_and(|last| request.sequence <= *last)
        {
            return Err(AuthoringErrorCode::SessionConflict);
        }
        if state
            .active_session
            .as_deref()
            .is_some_and(|active| active != request.session_id)
        {
            return Err(AuthoringErrorCode::SessionConflict);
        }
        let starts_session = matches!(
            request.operation,
            AuthoringOperation::DraftCreate | AuthoringOperation::DraftImport
        );
        let read_without_session = matches!(
            request.operation,
            AuthoringOperation::RigCatalogGet
                | AuthoringOperation::DraftValidate
                | AuthoringOperation::SessionStatus
                | AuthoringOperation::SessionCancel
                | AuthoringOperation::ApplicationSetPage
        );
        if state.active_session.is_none() && !starts_session && !read_without_session {
            return Err(AuthoringErrorCode::SessionConflict);
        }
        state.seen_ids.insert(request.request_id.clone());
        state
            .last_sequence
            .insert(request.session_id.clone(), request.sequence);
        if starts_session && state.active_session.is_none() {
            state.active_session = Some(request.session_id.clone());
        }
        Ok(())
    }

    fn finish(&self, request: &AuthoringRequest, response: &crate::AuthoringResponse) {
        if let Ok(mut state) = self.0.lock() {
            let session_start_failed = matches!(
                request.operation,
                AuthoringOperation::DraftCreate | AuthoringOperation::DraftImport
            ) && !response.ok;
            let cancelled = request.operation == AuthoringOperation::SessionCancel && response.ok;
            if cancelled {
                if state.active_session.as_deref() == Some(request.session_id.as_str()) {
                    state.active_session = None;
                }
                state.last_sequence.remove(&request.session_id);
            } else if session_start_failed
                && state.active_session.as_deref() == Some(request.session_id.as_str())
            {
                state.active_session = None;
                state.last_sequence.remove(&request.session_id);
            }
        }
    }

    fn disconnected(&self, request: &AuthoringRequest) {
        if let Ok(mut state) = self.0.lock() {
            if state.active_session.as_deref() == Some(request.session_id.as_str()) {
                state.active_session = None;
                state.last_sequence.remove(&request.session_id);
            }
        }
    }

    fn authorizes(&self, session_id: &str) -> bool {
        self.0
            .lock()
            .is_ok_and(|state| state.active_session.as_deref() == Some(session_id))
    }
}

#[derive(Default)]
struct ObservationGuardState {
    seen_ids: HashMap<String, String>,
    last_sequence: HashMap<String, u64>,
}

#[derive(Clone, Default)]
struct ObservationGuard(Arc<Mutex<ObservationGuardState>>);

impl ObservationGuard {
    fn reserve(
        &self,
        request: &crate::PoseObservationRequest,
        authorized: bool,
    ) -> Result<(), String> {
        if !authorized {
            return Err("SESSION_CONFLICT".into());
        }
        let mut state = self.0.lock().map_err(|_| "INTERNAL".to_owned())?;
        if state.seen_ids.contains_key(&request.request_id) {
            return Err("DUPLICATE_REQUEST_ID".into());
        }
        if state.seen_ids.len() >= MAX_AUTHORING_REQUEST_IDS {
            return Err("SESSION_CONFLICT".into());
        }
        if state
            .last_sequence
            .get(&request.session_id)
            .is_some_and(|last| request.sequence <= *last)
        {
            return Err("SESSION_CONFLICT".into());
        }
        if state
            .last_sequence
            .keys()
            .any(|session| session != &request.session_id)
        {
            return Err("SESSION_CONFLICT".into());
        }
        state
            .seen_ids
            .insert(request.request_id.clone(), request.session_id.clone());
        state
            .last_sequence
            .insert(request.session_id.clone(), request.sequence);
        Ok(())
    }

    fn clear_session(&self, session_id: &str) {
        if let Ok(mut state) = self.0.lock() {
            state.last_sequence.remove(session_id);
            state.seen_ids.retain(|_, owner| owner != session_id);
        }
    }
}

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
    owner_uid: u32,
}

#[derive(Clone)]
struct HostServices {
    store: SnapshotStore,
    authoring_handler: Option<AuthoringHandler>,
    observation_handler: Option<PoseObservationHandler>,
    image_chunk_handler: Option<PoseImageChunkHandler>,
}

impl McpHost {
    pub fn start(store: SnapshotStore) -> Result<Self, HostError> {
        Self::start_with_authoring(store, None)
    }

    pub fn start_with_authoring(
        store: SnapshotStore,
        authoring_handler: Option<AuthoringHandler>,
    ) -> Result<Self, HostError> {
        Self::start_with_handlers(store, authoring_handler, None, None)
    }

    pub fn start_with_handlers(
        store: SnapshotStore,
        authoring_handler: Option<AuthoringHandler>,
        observation_handler: Option<PoseObservationHandler>,
        image_chunk_handler: Option<PoseImageChunkHandler>,
    ) -> Result<Self, HostError> {
        let root = runtime_directory().ok_or(HostError::RuntimeDirectory)?;
        Self::start_at_with_handlers(
            root,
            store,
            authoring_handler,
            observation_handler,
            image_chunk_handler,
        )
    }

    #[cfg(test)]
    fn start_at(root: PathBuf, store: SnapshotStore) -> Result<Self, HostError> {
        Self::start_at_with_authoring(root, store, None)
    }

    #[cfg(test)]
    fn start_at_with_authoring(
        root: PathBuf,
        store: SnapshotStore,
        authoring_handler: Option<AuthoringHandler>,
    ) -> Result<Self, HostError> {
        Self::start_at_with_handlers(root, store, authoring_handler, None, None)
    }

    fn start_at_with_handlers(
        root: PathBuf,
        store: SnapshotStore,
        authoring_handler: Option<AuthoringHandler>,
        observation_handler: Option<PoseObservationHandler>,
        image_chunk_handler: Option<PoseImageChunkHandler>,
    ) -> Result<Self, HostError> {
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
        let services = HostServices {
            store,
            authoring_handler,
            observation_handler,
            image_chunk_handler,
        };
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
                        serve(listener, services, thread_stop, expected_uid).await;
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
            owner_uid: expected_uid,
        })
    }

    pub fn endpoint(&self) -> &std::path::Path {
        &self.endpoint
    }

    /// True only while the expected user-owned socket path is present with
    /// private permissions. This is a filesystem check, not a client-auth claim.
    pub fn socket_available(&self) -> bool {
        let Ok(metadata) = fs::symlink_metadata(&self.endpoint) else {
            return false;
        };
        metadata.file_type().is_socket()
            && metadata.uid() == self.owner_uid
            && metadata.permissions().mode() & 0o777 == 0o600
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
    services: HostServices,
    stop: Arc<AtomicBool>,
    expected_uid: u32,
) {
    let permits = Arc::new(Semaphore::new(16));
    let authoring_slot = Arc::new(Semaphore::new(1));
    let authoring_guard = AuthoringGuard::default();
    let observation_guard = ObservationGuard::default();
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
        let services = services.clone();
        let authoring_slot = authoring_slot.clone();
        let authoring_guard = authoring_guard.clone();
        let observation_guard = observation_guard.clone();
        tokio::spawn(async move {
            let _permit = permit;
            let _ = serve_client(
                stream,
                services,
                authoring_slot,
                authoring_guard,
                observation_guard,
                expected_uid,
            )
            .await;
        });
    }
}

async fn serve_client(
    mut stream: UnixStream,
    services: HostServices,
    authoring_slot: Arc<Semaphore>,
    authoring_guard: AuthoringGuard,
    observation_guard: ObservationGuard,
    expected_uid: u32,
) -> std::io::Result<()> {
    let HostServices {
        store,
        authoring_handler,
        observation_handler,
        image_chunk_handler,
    } = services;
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
    let request = match serde_json::from_slice::<IpcRequest>(&bytes) {
        Ok(request) => request,
        Err(_) => return Ok(()),
    };
    let response = if !peer_uid_allowed(expected_uid, peer_uid) {
        IpcResponse::error(request.request_id, IpcErrorCode::PermissionDenied)
    } else if request.protocol_version != IPC_PROTOCOL_VERSION {
        IpcResponse::error(request.request_id, IpcErrorCode::IncompatibleVersion)
    } else {
        match request.method {
            IpcMethod::SnapshotGet => match store.status() {
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
            IpcMethod::AuthoringRequest {
                request: bridge_request,
            } => {
                if let Err(error) = bridge_request.validate() {
                    IpcResponse::authoring(request.request_id, bridge_request.error_response(error))
                } else if let Err(code) = authoring_guard.reserve(&bridge_request) {
                    IpcResponse::authoring(
                        request.request_id,
                        bridge_request.error_response(bridge_request.error(
                            code,
                            "authoring request is replayed, stale, or conflicts with the active session",
                        )),
                    )
                } else if let Some(handler) = authoring_handler {
                    match authoring_slot.clone().try_acquire_owned() {
                        Ok(_slot) => {
                            let deadline = Duration::from_millis(bridge_request.deadline_ms as u64);
                            let in_flight = handler(bridge_request.clone());
                            let response = tokio::select! {
                                result = timeout(deadline, in_flight) => {
                                    match result {
                                        Ok(response) if response.validate_for(&bridge_request) => {
                                            IpcResponse::authoring(request.request_id, response)
                                        }
                                        Ok(_) => IpcResponse::authoring(
                                            request.request_id,
                                            bridge_request.error_response(bridge_request.error(
                                                AuthoringErrorCode::Internal,
                                                "frontend response correlation failed",
                                            )),
                                        ),
                                        Err(_) => IpcResponse::authoring(
                                            request.request_id,
                                            bridge_request.error_response(bridge_request.error(
                                                AuthoringErrorCode::DeadlineExceeded,
                                                "native authoring request exceeded its deadline",
                                            )),
                                        ),
                                    }
                                }
                                _ = wait_for_client_close(&mut stream) => {
                                    authoring_guard.disconnected(&bridge_request);
                                    return Ok(());
                                },
                            };
                            if let Some(authoring_response) = response.authoring_response.as_ref() {
                                authoring_guard.finish(&bridge_request, authoring_response);
                                if bridge_request.operation == AuthoringOperation::SessionCancel
                                    && authoring_response.ok
                                {
                                    observation_guard.clear_session(&bridge_request.session_id);
                                }
                            }
                            response
                        }
                        Err(_) => IpcResponse::authoring(
                            request.request_id,
                            bridge_request.error_response(bridge_request.error(
                                AuthoringErrorCode::SessionConflict,
                                "another authoring request is in flight",
                            )),
                        ),
                    }
                } else {
                    IpcResponse::authoring(
                        request.request_id,
                        bridge_request.error_response(bridge_request.error(
                            AuthoringErrorCode::AuthoringDisabled,
                            "native authoring bridge is disabled",
                        )),
                    )
                }
            }
            IpcMethod::PoseObservationRequest {
                request: pose_request,
            } => {
                if let Err(error) = pose_request.validate() {
                    IpcResponse::pose_observation(
                        request.request_id,
                        pose_request.error_response(error),
                    )
                } else if let Err(code) = observation_guard.reserve(
                    &pose_request,
                    authoring_guard.authorizes(&pose_request.session_id),
                ) {
                    IpcResponse::pose_observation(
                        request.request_id,
                        pose_request.error_response(
                            pose_request
                                .error(&code, "observation request is replayed or unauthorized"),
                        ),
                    )
                } else if let Some(handler) = observation_handler {
                    match authoring_slot.clone().try_acquire_owned() {
                        Ok(_slot) => {
                            let deadline = Duration::from_millis(pose_request.deadline_ms as u64);
                            let in_flight = handler(pose_request.clone());
                            tokio::select! {
                                result = timeout(deadline, in_flight) => match result {
                                    Ok(response) if pose_request.validate_response(&response) => {
                                        IpcResponse::pose_observation(request.request_id, response)
                                    }
                                    Ok(_) => IpcResponse::pose_observation(request.request_id, pose_request.error_response(pose_request.error("INTERNAL", "frontend observation response correlation failed"))),
                                    Err(_) => IpcResponse::pose_observation(request.request_id, pose_request.error_response(pose_request.error("DEADLINE_EXCEEDED", "observation exceeded its request deadline"))),
                                },
                                _ = wait_for_client_close(&mut stream) => {
                                    return Ok(());
                                },
                            }
                        }
                        Err(_) => IpcResponse::pose_observation(
                            request.request_id,
                            pose_request.error_response(pose_request.error(
                                "RESOURCE_LIMIT_EXCEEDED",
                                "another native bridge request is in flight",
                            )),
                        ),
                    }
                } else {
                    IpcResponse::pose_observation(
                        request.request_id,
                        pose_request.error_response(pose_request.error(
                            "RIG_UNAVAILABLE",
                            "native observation bridge is unavailable",
                        )),
                    )
                }
            }
            IpcMethod::PoseImageChunkRequest {
                request: chunk_request,
            } => {
                if chunk_request.validate().is_err()
                    || !authoring_guard.authorizes(&chunk_request.session_id)
                {
                    IpcResponse::error(request.request_id, IpcErrorCode::PermissionDenied)
                } else if let Some(handler) = image_chunk_handler {
                    let deadline = Duration::from_secs(30);
                    let in_flight = handler(chunk_request);
                    match timeout(deadline, in_flight).await {
                        Ok(Some(response)) => {
                            IpcResponse::pose_image_chunk(request.request_id, response)
                        }
                        _ => IpcResponse::error(request.request_id, IpcErrorCode::AppUnavailable),
                    }
                } else {
                    IpcResponse::error(request.request_id, IpcErrorCode::AppUnavailable)
                }
            }
        }
    };
    let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let (outcome, sequence, age_ms) = match (&response.snapshot, &response.error) {
        (Some(snapshot), None) => (
            "snapshot_served",
            Some(snapshot.meta.sequence),
            Some(snapshot.meta.age_ms),
        ),
        (None, Some(error)) => (
            match error {
                IpcErrorCode::AppUnavailable => "app_unavailable",
                IpcErrorCode::NotReady => "app_not_ready",
                IpcErrorCode::AppDisconnected => "app_disconnected",
                IpcErrorCode::IncompatibleVersion => "incompatible_version",
                IpcErrorCode::PermissionDenied => "permission_denied",
                IpcErrorCode::AuthoringDisabled => "authoring_disabled",
            },
            None,
            None,
        ),
        _ => ("invalid_response", None, None),
    };
    eprintln!(
        "M2M_IPC timestamp={timestamp} request_id={} peer={} outcome={outcome} sequence={} age_ms={}",
        response.request_id,
        if peer_uid_allowed(expected_uid, peer_uid) { "verified_same_uid" } else { "denied" },
        sequence.map_or_else(|| "none".to_owned(), |value| value.to_string()),
        age_ms.map_or_else(|| "none".to_owned(), |value| value.to_string())
    );
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

async fn wait_for_client_close(stream: &mut UnixStream) {
    let mut trailing_byte = [0_u8; 1];
    loop {
        match stream.read(&mut trailing_byte).await {
            Ok(_) => return,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return,
        }
    }
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
        assert!(host.socket_available());
        assert_eq!(
            fs::metadata(&socket).unwrap().permissions().mode() & 0o777,
            0o600
        );
        drop(host);
        assert!(!socket.exists());
        let _ = fs::remove_dir_all(path);
    }

    #[tokio::test]
    async fn client_distinguishes_a_connected_host_with_no_snapshot() {
        use crate::ipc::AppSnapshotSource;

        let path = private_runtime();
        let host = McpHost::start_at(path.clone(), SnapshotStore::default()).unwrap();
        let client = crate::IpcClient::with_endpoint(host.endpoint().to_string_lossy());
        assert_eq!(
            client.snapshot().await.unwrap_err(),
            crate::IpcError::AppUnavailable
        );
        drop(host);
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

    fn sample_authoring_request() -> AuthoringRequest {
        AuthoringRequest {
            protocol: crate::AUTHORING_BRIDGE_PROTOCOL.into(),
            request_id: "caller-id".into(),
            session_id: "session-1".into(),
            sequence: 1,
            deadline_ms: 500,
            operation: AuthoringOperation::DraftCreate,
            payload: serde_json::json!({"document": {}}),
        }
    }

    #[tokio::test]
    async fn observation_and_image_chunks_share_authorized_session_and_private_ipc() {
        use crate::{
            AppSnapshotSource, PoseImageChunkResponse, PoseObservationRequest,
            PoseObservationResponse,
        };

        let path = private_runtime();
        let authoring_handler: AuthoringHandler = Arc::new(|request| {
            Box::pin(async move { request.success_response(serde_json::json!({"revision":1})) })
        });
        let observation_handler: PoseObservationHandler = Arc::new(|request| {
            Box::pin(async move {
                PoseObservationResponse::success(
                    &request,
                    serde_json::json!({"capture_id":"opaque","revision":1}),
                )
            })
        });
        let image_handler: PoseImageChunkHandler = Arc::new(|request| {
            Box::pin(async move {
                Some(PoseImageChunkResponse {
                    request_id: request.request_id,
                    session_id: request.session_id,
                    capture_id: request.capture_id,
                    image_ref: request.image_ref,
                    view: request.view,
                    offset: request.offset,
                    mime: "image/png".into(),
                    total_bytes: 4,
                    bytes_base64: "AQIDBA==".into(),
                    error: None,
                })
            })
        });
        let host = McpHost::start_at_with_handlers(
            path.clone(),
            SnapshotStore::default(),
            Some(authoring_handler),
            Some(observation_handler),
            Some(image_handler),
        )
        .unwrap();
        let client = crate::IpcClient::with_endpoint(host.endpoint().to_string_lossy());
        client
            .authoring_request(sample_authoring_request())
            .await
            .unwrap();

        let pose_request = PoseObservationRequest {
            protocol: crate::POSE_OBSERVATION_PROTOCOL.into(),
            request_id: "caller-pose".into(),
            session_id: "session-1".into(),
            sequence: 1,
            deadline_ms: 1_000,
            operation: crate::PoseObservationOperation::Capture,
            payload: serde_json::json!({"draft_id":"session-1"}),
        };
        let pose_response = client
            .pose_observation_request(pose_request.clone())
            .await
            .unwrap();
        assert!(pose_request.validate_response(&pose_response));
        assert_eq!(pose_response.result.unwrap()["capture_id"], "opaque");

        let image_request = crate::PoseImageChunkRequest {
            request_id: "image-1".into(),
            session_id: "session-1".into(),
            capture_id: "opaque".into(),
            image_ref: "opaque:front".into(),
            view: "front".into(),
            offset: 0,
            max_bytes: 4,
        };
        let image_response = client
            .pose_image_chunk(image_request.clone())
            .await
            .unwrap();
        assert!(image_request.validate_response(&image_response));
        drop(host);
        let _ = fs::remove_dir_all(path);
    }

    #[tokio::test]
    async fn observation_disconnect_drops_the_pending_frontend_capture_future() {
        use crate::{AppSnapshotSource, PoseObservationRequest, PoseObservationResponse};
        use std::sync::atomic::AtomicBool;

        struct DropMarker(Arc<AtomicBool>);
        impl Drop for DropMarker {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Release);
            }
        }

        let path = private_runtime();
        let dropped = Arc::new(AtomicBool::new(false));
        let authoring_handler: AuthoringHandler = Arc::new(|request| {
            Box::pin(async move { request.success_response(serde_json::json!({"revision":1})) })
        });
        let drop_flag = dropped.clone();
        let observation_handler: PoseObservationHandler = Arc::new(move |_request| {
            let marker = DropMarker(drop_flag.clone());
            Box::pin(async move {
                let _marker = marker;
                std::future::pending::<PoseObservationResponse>().await
            })
        });
        let host = McpHost::start_at_with_handlers(
            path.clone(),
            SnapshotStore::default(),
            Some(authoring_handler),
            Some(observation_handler),
            None,
        )
        .unwrap();
        let client = crate::IpcClient::with_endpoint(host.endpoint().to_string_lossy());
        client
            .authoring_request(sample_authoring_request())
            .await
            .unwrap();
        let request = PoseObservationRequest {
            protocol: crate::POSE_OBSERVATION_PROTOCOL.into(),
            request_id: "disconnect-pose".into(),
            session_id: "session-1".into(),
            sequence: 1,
            deadline_ms: 10_000,
            operation: crate::PoseObservationOperation::Capture,
            payload: serde_json::json!({}),
        };
        let call = tokio::spawn({
            let client = client.clone();
            async move { client.pose_observation_request(request).await }
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        call.abort();
        for _ in 0..20 {
            if dropped.load(Ordering::Acquire) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        assert!(
            dropped.load(Ordering::Acquire),
            "client disconnect must cancel the active capture future"
        );
        drop(host);
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn replay_guard_enforces_unique_ids_sequence_session_and_bounded_capacity() {
        let guard = AuthoringGuard::default();
        let first = sample_authoring_request();
        assert_eq!(guard.reserve(&first), Ok(()));
        assert_eq!(
            guard.reserve(&first),
            Err(AuthoringErrorCode::DuplicateRequestId)
        );
        let mut stale = first.clone();
        stale.request_id = "new-id".into();
        assert_eq!(
            guard.reserve(&stale),
            Err(AuthoringErrorCode::SessionConflict)
        );
        let mut other_session = first.clone();
        other_session.request_id = "other-id".into();
        other_session.session_id = "session-2".into();
        other_session.sequence = 2;
        assert_eq!(
            guard.reserve(&other_session),
            Err(AuthoringErrorCode::SessionConflict)
        );
        let mut next = first.clone();
        next.request_id = "next-id".into();
        next.sequence = 2;
        assert_eq!(guard.reserve(&next), Ok(()));

        if let Ok(mut state) = guard.0.lock() {
            state
                .seen_ids
                .extend((0..MAX_AUTHORING_REQUEST_IDS).map(|n| format!("id-{n}")));
        }
        next.request_id = "overflow-id".into();
        next.sequence = 3;
        assert_eq!(
            guard.reserve(&next),
            Err(AuthoringErrorCode::SessionConflict)
        );
    }

    #[test]
    fn observation_guard_rejects_unauthorized_duplicate_stale_and_cross_session_requests() {
        let guard = ObservationGuard::default();
        let request = crate::PoseObservationRequest {
            protocol: crate::POSE_OBSERVATION_PROTOCOL.into(),
            request_id: "pose-1".into(),
            session_id: "session-1".into(),
            sequence: 1,
            deadline_ms: 1_000,
            operation: crate::PoseObservationOperation::Capture,
            payload: serde_json::json!({}),
        };
        assert_eq!(
            guard.reserve(&request, false),
            Err("SESSION_CONFLICT".into())
        );
        assert_eq!(guard.reserve(&request, true), Ok(()));
        assert_eq!(
            guard.reserve(&request, true),
            Err("DUPLICATE_REQUEST_ID".into())
        );
        let mut stale = request.clone();
        stale.request_id = "pose-stale".into();
        assert_eq!(guard.reserve(&stale, true), Err("SESSION_CONFLICT".into()));
        let mut foreign = request;
        foreign.request_id = "pose-foreign".into();
        foreign.session_id = "session-2".into();
        foreign.sequence = 1;
        assert_eq!(
            guard.reserve(&foreign, true),
            Err("SESSION_CONFLICT".into())
        );
        guard.clear_session("session-1");
        assert_eq!(guard.reserve(&foreign, true), Ok(()));
    }

    #[test]
    fn replay_guard_releases_failed_or_disconnected_session() {
        let guard = AuthoringGuard::default();
        let request = sample_authoring_request();
        assert_eq!(guard.reserve(&request), Ok(()));
        guard.disconnected(&request);
        let mut restarted = request.clone();
        restarted.request_id = "fresh-session".into();
        restarted.sequence = 1;
        restarted.session_id = "session-2".into();
        assert_eq!(guard.reserve(&restarted), Ok(()));
        let failure = restarted.error_response(
            restarted.error(AuthoringErrorCode::InvalidDocument, "test create failure"),
        );
        guard.finish(&restarted, &failure);
        let mut retry = restarted;
        retry.request_id = "retry-session".into();
        retry.sequence = 1;
        assert_eq!(guard.reserve(&retry), Ok(()));
    }

    #[test]
    fn session_cancel_cleans_up_a_status_only_session() {
        let guard = AuthoringGuard::default();
        let mut status = sample_authoring_request();
        status.request_id = "status-only".into();
        status.operation = AuthoringOperation::SessionStatus;
        status.payload = serde_json::json!({});
        assert_eq!(guard.reserve(&status), Ok(()));

        let mut cancel = status.clone();
        cancel.request_id = "cancel-status-only".into();
        cancel.sequence = 2;
        cancel.operation = AuthoringOperation::SessionCancel;
        assert_eq!(guard.reserve(&cancel), Ok(()));
        guard.finish(
            &cancel,
            &cancel.success_response(serde_json::json!({"cancelled": true})),
        );

        let state = guard.0.lock().unwrap();
        assert!(state.active_session.is_none());
        assert!(!state.last_sequence.contains_key(&status.session_id));
    }

    #[tokio::test]
    async fn sidecar_authoring_request_is_correlated_and_host_defaults_to_disabled() {
        use crate::AppSnapshotSource;

        let path = private_runtime();
        let host = McpHost::start_at(path.clone(), SnapshotStore::default()).unwrap();
        let client = crate::IpcClient::with_endpoint(host.endpoint().to_string_lossy());
        let response = client
            .authoring_request(sample_authoring_request())
            .await
            .unwrap();
        assert_eq!(
            response.error.unwrap().code,
            AuthoringErrorCode::AuthoringDisabled
        );
        drop(host);
        let _ = fs::remove_dir_all(path);
    }

    #[tokio::test]
    async fn sidecar_authoring_round_trip_checks_response_identity() {
        use crate::AppSnapshotSource;

        let path = private_runtime();
        let handler: AuthoringHandler = Arc::new(|request| {
            Box::pin(async move { request.success_response(serde_json::json!({"revision": 1})) })
        });
        let host =
            McpHost::start_at_with_authoring(path.clone(), SnapshotStore::default(), Some(handler))
                .unwrap();
        let client = crate::IpcClient::with_endpoint(host.endpoint().to_string_lossy());
        let response = client
            .authoring_request(sample_authoring_request())
            .await
            .unwrap();
        assert!(response.ok);
        assert_eq!(response.sequence, 1);
        assert_eq!(response.result.unwrap()["revision"], 1);
        drop(host);
        let _ = fs::remove_dir_all(path);
    }

    #[tokio::test]
    async fn authoring_deadline_expires_before_slow_frontend_response() {
        use crate::AppSnapshotSource;

        let path = private_runtime();
        let handler: AuthoringHandler = Arc::new(|request| {
            Box::pin(async move {
                tokio::time::sleep(Duration::from_millis(200)).await;
                request.success_response(serde_json::json!({"revision": 1}))
            })
        });
        let host =
            McpHost::start_at_with_authoring(path.clone(), SnapshotStore::default(), Some(handler))
                .unwrap();
        let client = crate::IpcClient::with_endpoint(host.endpoint().to_string_lossy());
        let mut request = sample_authoring_request();
        request.deadline_ms = 100;
        let response = client.authoring_request(request).await.unwrap();
        assert_eq!(
            response.error.unwrap().code,
            AuthoringErrorCode::DeadlineExceeded
        );
        drop(host);
        let _ = fs::remove_dir_all(path);
    }
}
