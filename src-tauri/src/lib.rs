#[cfg(feature = "mcp-host")]
use mesh2motion_mcp::{
    AppSnapshotSource, AuthoringErrorCode, AuthoringHandler, AuthoringOperation, AuthoringRequest,
    AuthoringResponse, PoseImageChunkHandler, PoseImageChunkRequest, PoseImageChunkResponse,
    PoseObservationHandler, PoseObservationRequest, PoseObservationResponse,
    AUTHORING_BRIDGE_PROTOCOL, OBSERVATION_IMAGE_REQUEST_EVENT,
};
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;
#[cfg(feature = "mcp-host")]
use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender},
        Arc, Mutex,
    },
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};
use tauri::path::BaseDirectory;
#[cfg(feature = "mcp-host")]
use tauri::Emitter;
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
        || !(filename.ends_with(".mp4") || filename.ends_with(".webp"))
    {
        return Err("INVALID_PREVIEW_ASSET".to_owned());
    }

    let animation = filename
        .strip_prefix("dark_")
        .or_else(|| filename.strip_prefix("light_"))
        .and_then(|name| {
            name.strip_suffix(".mp4")
                .or_else(|| name.strip_suffix(".webp"))
        })
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
    authoring_enabled: bool,
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
    authoring: Arc<AuthoringBridgeState>,
    observation: Arc<PoseObservationBridgeState>,
}

#[cfg(feature = "mcp-host")]
struct AuthoringBridgeState {
    app: AppHandle,
    enabled: AtomicBool,
    frontend_ready: AtomicBool,
    pending: Mutex<HashMap<String, PendingAuthoring>>,
}

#[cfg(feature = "mcp-host")]
struct PendingAuthoring {
    session_id: String,
    sequence: u64,
    operation: AuthoringOperation,
    sender: SyncSender<AuthoringResponse>,
}

#[cfg(feature = "mcp-host")]
struct PoseObservationBridgeState {
    app: AppHandle,
    authoring: Arc<AuthoringBridgeState>,
    pending_pose: Mutex<HashMap<String, PendingPoseObservation>>,
    pending_image: Mutex<HashMap<String, PendingPoseImage>>,
    captures: Mutex<HashMap<String, String>>,
}

#[cfg(feature = "mcp-host")]
struct PendingPoseObservation {
    request: PoseObservationRequest,
    session_id: String,
    sequence: u64,
    sender: SyncSender<PoseObservationResponse>,
}

#[cfg(feature = "mcp-host")]
struct PendingPoseImage {
    request: PoseImageChunkRequest,
    sender: SyncSender<Option<PoseImageChunkResponse>>,
}

#[cfg(feature = "mcp-host")]
fn mcp_operation_grants_authoring(operation: AuthoringOperation) -> bool {
    operation != AuthoringOperation::RigCatalogGet
}

#[cfg(feature = "mcp-host")]
#[derive(Clone, Serialize)]
#[serde(rename_all = "snake_case")]
struct AuthoringCancelNotice {
    protocol: &'static str,
    request_id: String,
    session_id: String,
}

#[cfg(feature = "mcp-host")]
impl AuthoringBridgeState {
    fn new(app: AppHandle) -> Self {
        Self {
            app,
            enabled: AtomicBool::new(false),
            frontend_ready: AtomicBool::new(false),
            pending: Mutex::new(HashMap::new()),
        }
    }

    fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }

    fn authorize_mcp_request(&self, operation: AuthoringOperation) {
        if mcp_operation_grants_authoring(operation) {
            self.enabled.store(true, Ordering::Release);
        }
    }

    fn disable(&self) {
        self.enabled.store(false, Ordering::Release);
        if let Ok(mut pending) = self.pending.lock() {
            for (request_id, entry) in pending.iter() {
                let _ = self.app.emit(
                    "m2m-authoring-cancel",
                    AuthoringCancelNotice {
                        protocol: AUTHORING_BRIDGE_PROTOCOL,
                        request_id: request_id.clone(),
                        session_id: entry.session_id.clone(),
                    },
                );
            }
            pending.clear();
        }
    }
}

#[cfg(feature = "mcp-host")]
impl PoseObservationBridgeState {
    fn new(app: AppHandle, authoring: Arc<AuthoringBridgeState>) -> Self {
        Self {
            app,
            authoring,
            pending_pose: Mutex::new(HashMap::new()),
            pending_image: Mutex::new(HashMap::new()),
            captures: Mutex::new(HashMap::new()),
        }
    }

    fn cancel_all(&self) {
        if let Ok(mut pending) = self.pending_pose.lock() {
            for (request_id, entry) in pending.iter() {
                let _ = self.app.emit(
                    "m2m-pose-observation-cancel",
                    serde_json::json!({"protocol":"mcp-pose-observation/v1","request_id":request_id,"session_id":entry.session_id}),
                );
                let _ = entry.sender.try_send(
                    entry.request.error_response(
                        entry
                            .request
                            .error("CAPTURE_CANCELLED", "observation bridge is shutting down"),
                    ),
                );
            }
            pending.clear();
        }
        if let Ok(mut pending) = self.pending_image.lock() {
            for (request_id, entry) in pending.iter() {
                let _ = self.app.emit(
                    "m2m-pose-observation-cancel",
                    serde_json::json!({"protocol":"mcp-pose-observation/v1","request_id":request_id,"session_id":entry.request.session_id,"capture_id":entry.request.capture_id}),
                );
                let _ = entry.sender.try_send(None);
            }
            pending.clear();
        }
        if let Ok(mut captures) = self.captures.lock() {
            for (capture_id, session_id) in captures.iter() {
                let _ = self.app.emit("m2m-pose-observation-cancel", serde_json::json!({"protocol":"mcp-pose-observation/v1","session_id":session_id,"capture_id":capture_id}));
            }
            captures.clear();
        }
    }

    fn cancel_session(&self, session_id: &str) {
        if let Ok(mut pending) = self.pending_pose.lock() {
            let ids = pending
                .iter()
                .filter(|(_, entry)| entry.session_id == session_id)
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>();
            for request_id in ids {
                if let Some(entry) = pending.remove(&request_id) {
                    let _ = entry.sender.try_send(
                        entry.request.error_response(
                            entry
                                .request
                                .error("CAPTURE_CANCELLED", "authoring session was cancelled"),
                        ),
                    );
                    let _ = self.app.emit("m2m-pose-observation-cancel", serde_json::json!({"protocol":"mcp-pose-observation/v1","request_id":request_id,"session_id":session_id}));
                }
            }
        }
        if let Ok(mut pending) = self.pending_image.lock() {
            let ids = pending
                .iter()
                .filter(|(_, entry)| entry.request.session_id == session_id)
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>();
            for request_id in ids {
                if let Some(entry) = pending.remove(&request_id) {
                    let _ = entry.sender.try_send(None);
                    let _ = self.app.emit("m2m-pose-observation-cancel", serde_json::json!({"protocol":"mcp-pose-observation/v1","request_id":request_id,"session_id":session_id,"capture_id":entry.request.capture_id}));
                }
            }
        }
        if let Ok(mut captures) = self.captures.lock() {
            let ids = captures
                .iter()
                .filter(|(_, owner)| owner.as_str() == session_id)
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>();
            for capture_id in ids {
                captures.remove(&capture_id);
                let _ = self.app.emit("m2m-pose-observation-cancel", serde_json::json!({"protocol":"mcp-pose-observation/v1","session_id":session_id,"capture_id":capture_id}));
            }
        }
    }
}

#[cfg(feature = "mcp-host")]
struct PoseWaitState {
    result: Option<PoseObservationResponse>,
    waker: Option<Waker>,
}

#[cfg(feature = "mcp-host")]
struct PoseWait {
    state: Arc<Mutex<PoseWaitState>>,
    cancelled: Arc<AtomicBool>,
    bridge: Arc<PoseObservationBridgeState>,
    request_id: String,
    session_id: String,
    completed: bool,
}

#[cfg(feature = "mcp-host")]
impl Future for PoseWait {
    type Output = PoseObservationResponse;
    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let Ok(mut state) = this.state.lock() else {
            return Poll::Pending;
        };
        if let Some(response) = state.result.take() {
            this.completed = true;
            return Poll::Ready(response);
        }
        state.waker = Some(context.waker().clone());
        Poll::Pending
    }
}

#[cfg(feature = "mcp-host")]
impl Drop for PoseWait {
    fn drop(&mut self) {
        if self.completed {
            return;
        }
        self.cancelled.store(true, Ordering::Release);
        let _ = self.bridge.app.emit(
            "m2m-pose-observation-cancel",
            serde_json::json!({"protocol":"mcp-pose-observation/v1","request_id":self.request_id,"session_id":self.session_id}),
        );
        if let Ok(mut pending) = self.bridge.pending_pose.lock() {
            pending.remove(&self.request_id);
        }
    }
}

#[cfg(feature = "mcp-host")]
struct ImageWaitState {
    result: Option<Option<PoseImageChunkResponse>>,
    waker: Option<Waker>,
}

#[cfg(feature = "mcp-host")]
struct ImageWait {
    state: Arc<Mutex<ImageWaitState>>,
    cancelled: Arc<AtomicBool>,
    bridge: Arc<PoseObservationBridgeState>,
    request: PoseImageChunkRequest,
    completed: bool,
}

#[cfg(feature = "mcp-host")]
impl Future for ImageWait {
    type Output = Option<PoseImageChunkResponse>;
    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let Ok(mut state) = this.state.lock() else {
            return Poll::Pending;
        };
        if let Some(response) = state.result.take() {
            this.completed = true;
            return Poll::Ready(response);
        }
        state.waker = Some(context.waker().clone());
        Poll::Pending
    }
}

#[cfg(feature = "mcp-host")]
impl Drop for ImageWait {
    fn drop(&mut self) {
        if self.completed {
            return;
        }
        self.cancelled.store(true, Ordering::Release);
        let _ = self.bridge.app.emit(
            "m2m-pose-observation-cancel",
            serde_json::json!({"protocol":"mcp-pose-observation/v1","request_id":self.request.request_id,"session_id":self.request.session_id,"capture_id":self.request.capture_id}),
        );
        if let Ok(mut pending) = self.bridge.pending_image.lock() {
            pending.remove(&self.request.request_id);
        }
    }
}

#[cfg(feature = "mcp-host")]
fn make_pose_observation_handler(
    bridge: &Arc<PoseObservationBridgeState>,
) -> PoseObservationHandler {
    let bridge = Arc::downgrade(bridge);
    Arc::new(move |request: PoseObservationRequest| {
        let Some(bridge) = bridge.upgrade() else {
            return Box::pin(async move {
                request.error_response(
                    request.error("RIG_UNAVAILABLE", "native application is shutting down"),
                )
            }) as Pin<Box<dyn Future<Output = PoseObservationResponse> + Send>>;
        };
        if !bridge.authoring.is_enabled()
            || !bridge.authoring.frontend_ready.load(Ordering::Acquire)
        {
            return Box::pin(async move {
                request.error_response(request.error(
                    "SESSION_CONFLICT",
                    "trusted authoring session is not active",
                ))
            });
        }
        let (sender, receiver) = mpsc::sync_channel(1);
        let inserted = bridge
            .pending_pose
            .lock()
            .map(|mut pending| {
                if !pending.is_empty() || pending.contains_key(&request.request_id) {
                    return false;
                }
                pending.insert(
                    request.request_id.clone(),
                    PendingPoseObservation {
                        request: request.clone(),
                        session_id: request.session_id.clone(),
                        sequence: request.sequence,
                        sender,
                    },
                );
                true
            })
            .unwrap_or(false);
        if !inserted {
            return Box::pin(async move {
                request.error_response(request.error(
                    "RESOURCE_LIMIT_EXCEEDED",
                    "another observation request is pending",
                ))
            });
        }
        if bridge
            .app
            .emit("m2m-pose-observation-request", request.clone())
            .is_err()
        {
            if let Ok(mut pending) = bridge.pending_pose.lock() {
                pending.remove(&request.request_id);
            }
            return Box::pin(async move {
                request.error_response(request.error(
                    "RIG_UNAVAILABLE",
                    "frontend observation listener is unavailable",
                ))
            });
        }
        let state = Arc::new(Mutex::new(PoseWaitState {
            result: None,
            waker: None,
        }));
        let worker_state = state.clone();
        let worker_bridge = bridge.clone();
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = cancelled.clone();
        let worker_request = request.clone();
        std::thread::spawn(move || {
            let response = receiver
                .recv_timeout(Duration::from_millis(worker_request.deadline_ms as u64))
                .unwrap_or_else(|_| {
                    worker_request.error_response(worker_request.error(
                        "DEADLINE_EXCEEDED",
                        "frontend observation response exceeded the request deadline",
                    ))
                });
            if let Ok(mut pending) = worker_bridge.pending_pose.lock() {
                pending.remove(&worker_request.request_id);
            }
            if !worker_cancelled.load(Ordering::Acquire) {
                if let Ok(mut shared) = worker_state.lock() {
                    shared.result = Some(response);
                    if let Some(waker) = shared.waker.take() {
                        waker.wake();
                    }
                }
            }
        });
        Box::pin(PoseWait {
            state,
            cancelled,
            bridge,
            request_id: request.request_id,
            session_id: request.session_id,
            completed: false,
        })
    })
}

#[cfg(feature = "mcp-host")]
fn make_pose_image_chunk_handler(
    bridge: &Arc<PoseObservationBridgeState>,
) -> PoseImageChunkHandler {
    let bridge = Arc::downgrade(bridge);
    Arc::new(move |request: PoseImageChunkRequest| {
        let Some(bridge) = bridge.upgrade() else {
            return Box::pin(async { None })
                as Pin<Box<dyn Future<Output = Option<PoseImageChunkResponse>> + Send>>;
        };
        if !bridge.authoring.is_enabled()
            || !bridge.authoring.frontend_ready.load(Ordering::Acquire)
            || request.validate().is_err()
        {
            return Box::pin(async { None });
        }
        let (sender, receiver) = mpsc::sync_channel(1);
        let inserted = bridge
            .pending_image
            .lock()
            .map(|mut pending| {
                if !pending.is_empty() || pending.contains_key(&request.request_id) {
                    return false;
                }
                pending.insert(
                    request.request_id.clone(),
                    PendingPoseImage {
                        request: request.clone(),
                        sender,
                    },
                );
                true
            })
            .unwrap_or(false);
        if !inserted
            || bridge
                .app
                .emit(OBSERVATION_IMAGE_REQUEST_EVENT, request.clone())
                .is_err()
        {
            if let Ok(mut pending) = bridge.pending_image.lock() {
                pending.remove(&request.request_id);
            }
            return Box::pin(async { None });
        }
        let state = Arc::new(Mutex::new(ImageWaitState {
            result: None,
            waker: None,
        }));
        let worker_state = state.clone();
        let worker_bridge = bridge.clone();
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = cancelled.clone();
        let worker_request = request.clone();
        std::thread::spawn(move || {
            let response = receiver
                .recv_timeout(Duration::from_secs(30))
                .unwrap_or(None);
            if let Ok(mut pending) = worker_bridge.pending_image.lock() {
                pending.remove(&worker_request.request_id);
            }
            if !worker_cancelled.load(Ordering::Acquire) {
                if let Ok(mut shared) = worker_state.lock() {
                    shared.result = Some(response);
                    if let Some(waker) = shared.waker.take() {
                        waker.wake();
                    }
                }
            }
        });
        Box::pin(ImageWait {
            state,
            cancelled,
            bridge,
            request,
            completed: false,
        })
    })
}

#[cfg(feature = "mcp-host")]
struct BridgeWaitState {
    result: Option<AuthoringResponse>,
    waker: Option<Waker>,
}

#[cfg(feature = "mcp-host")]
struct BridgeWait {
    state: Arc<Mutex<BridgeWaitState>>,
    cancelled: Arc<AtomicBool>,
    bridge: Arc<AuthoringBridgeState>,
    request_id: String,
    session_id: String,
    completed: bool,
}

#[cfg(feature = "mcp-host")]
fn should_cancel_authoring_wait(completed: bool) -> bool {
    !completed
}

#[cfg(feature = "mcp-host")]
impl Future for BridgeWait {
    type Output = AuthoringResponse;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let Ok(mut state) = this.state.lock() else {
            return Poll::Pending;
        };
        if let Some(response) = state.result.take() {
            this.completed = true;
            return Poll::Ready(response);
        }
        state.waker = Some(context.waker().clone());
        Poll::Pending
    }
}

#[cfg(feature = "mcp-host")]
impl Drop for BridgeWait {
    fn drop(&mut self) {
        if !should_cancel_authoring_wait(self.completed) {
            return;
        }
        self.cancelled.store(true, Ordering::Release);
        let _ = self.bridge.app.emit(
            "m2m-authoring-cancel",
            AuthoringCancelNotice {
                protocol: AUTHORING_BRIDGE_PROTOCOL,
                request_id: self.request_id.clone(),
                session_id: self.session_id.clone(),
            },
        );
        if let Ok(mut pending) = self.bridge.pending.lock() {
            pending.remove(&self.request_id);
        }
    }
}

#[cfg(feature = "mcp-host")]
fn make_authoring_handler(state: &Arc<AuthoringBridgeState>) -> AuthoringHandler {
    let state = Arc::downgrade(state);
    Arc::new(move |request: AuthoringRequest| -> Pin<Box<dyn Future<Output = AuthoringResponse> + Send>> {
        let Some(state) = state.upgrade() else {
            return Box::pin(async move {
                request.error_response(request.error(
                    AuthoringErrorCode::AppDisconnected,
                    "native application is shutting down",
                ))
            });
        };
        if !state.frontend_ready.load(Ordering::Acquire) {
            return Box::pin(async move {
                request.error_response(request.error(
                    AuthoringErrorCode::RigUnavailable,
                    "frontend authoring event listener is not ready",
                ))
            });
        }
        // A valid authoring request over the private same-UID MCP socket
        // authorizes this app session. Live catalog discovery stays read-only.
        state.authorize_mcp_request(request.operation);
        let (sender, receiver) = mpsc::sync_channel(1);
        let inserted = state
            .pending
            .lock()
            .map(|mut pending| {
                if pending.is_empty() && !pending.contains_key(&request.request_id) {
                    pending.insert(
                        request.request_id.clone(),
                        PendingAuthoring {
                            session_id: request.session_id.clone(),
                            sequence: request.sequence,
                            operation: request.operation,
                            sender,
                        },
                    );
                    true
                } else {
                    false
                }
            })
            .unwrap_or(false);
        if !inserted {
            return Box::pin(async move {
                request.error_response(request.error(
                    AuthoringErrorCode::SessionConflict,
                    "another native bridge request is pending",
                ))
            });
        }
        if state.app.emit("m2m-authoring-request", request.clone()).is_err() {
            if let Ok(mut pending) = state.pending.lock() {
                pending.remove(&request.request_id);
            }
            return Box::pin(async move {
                request.error_response(request.error(
                    AuthoringErrorCode::AppDisconnected,
                    "frontend bridge is unavailable",
                ))
            });
        }
        eprintln!(
            "M2M_AUTHORING request_emitted request_id={} operation={:?}",
            request.request_id, request.operation
        );

        let deadline = Instant::now() + Duration::from_millis(u64::from(request.deadline_ms));
        let wait_state = Arc::new(Mutex::new(BridgeWaitState {
            result: None,
            waker: None,
        }));
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_state = wait_state.clone();
        let worker_cancelled = cancelled.clone();
        let worker_bridge = state.clone();
        let request_for_worker = request.clone();
        std::thread::spawn(move || {
            let response = match receiver.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Ok(response) => response,
                Err(_) => request_for_worker.error_response(request_for_worker.error(
                    AuthoringErrorCode::DeadlineExceeded,
                    "frontend bridge response exceeded the request deadline",
                )),
            };
            if let Ok(mut pending) = worker_bridge.pending.lock() {
                pending.remove(&request_for_worker.request_id);
            }
            if !worker_cancelled.load(Ordering::Acquire) {
                if let Ok(mut shared) = worker_state.lock() {
                    shared.result = Some(response);
                    if let Some(waker) = shared.waker.take() {
                        waker.wake();
                    }
                }
            }
        });
        Box::pin(BridgeWait {
            state: wait_state,
            cancelled,
            bridge: state,
            request_id: request.request_id,
            session_id: request.session_id,
            completed: false,
        })
    })
}

#[cfg(not(feature = "mcp-host"))]
#[derive(Default)]
struct McpHostState;

#[cfg(feature = "mcp-host")]
impl McpHostState {
    fn new(app: AppHandle) -> Self {
        let store = mesh2motion_mcp::SnapshotStore::default();
        let authoring = Arc::new(AuthoringBridgeState::new(app.clone()));
        let observation = Arc::new(PoseObservationBridgeState::new(app, authoring.clone()));
        let host = if std::env::var("M2M_MCP_ENABLE").as_deref() == Ok("1") {
            let handler = make_authoring_handler(&authoring);
            let observation_handler = make_pose_observation_handler(&observation);
            let image_handler = make_pose_image_chunk_handler(&observation);
            match mesh2motion_mcp::McpHost::start_with_handlers(
                store.clone(),
                Some(handler),
                Some(observation_handler),
                Some(image_handler),
            ) {
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
            authoring,
            observation,
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
            authoring_enabled: self.authoring.is_enabled(),
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
        let handler = make_authoring_handler(&self.authoring);
        let observation_handler = make_pose_observation_handler(&self.observation);
        let image_handler = make_pose_image_chunk_handler(&self.observation);
        let started = mesh2motion_mcp::McpHost::start_with_handlers(
            self.store.clone(),
            Some(handler),
            Some(observation_handler),
            Some(image_handler),
        )
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
        self.observation.cancel_all();
        self.authoring.disable();
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

/// Frontend event acknowledgment. It can resolve only a currently pending
/// request and must echo the bridge identity fields for exact correlation.
#[cfg(feature = "mcp-host")]
#[tauri::command]
fn mcp_authoring_respond(
    response: AuthoringResponse,
    state: State<'_, McpHostState>,
) -> Result<(), String> {
    let pending = state
        .authoring
        .pending
        .lock()
        .map_err(|_| "AUTHORING_STATE_LOCKED".to_owned())?;
    let Some(entry) = pending.get(&response.request_id) else {
        return Err("AUTHORING_REQUEST_NOT_PENDING".to_owned());
    };
    if response.protocol != AUTHORING_BRIDGE_PROTOCOL
        || response.session_id != entry.session_id
        || response.sequence != entry.sequence
    {
        eprintln!("M2M_AUTHORING response_rejected reason=correlation");
        return Err("AUTHORING_RESPONSE_MISMATCH".to_owned());
    }
    let cancel_observation = entry.operation == AuthoringOperation::SessionCancel && response.ok;
    let session_id = entry.session_id.clone();
    entry
        .sender
        .try_send(response)
        .map_err(|_| "AUTHORING_RESPONSE_REJECTED".to_owned())?;
    drop(pending);
    if cancel_observation {
        state.observation.cancel_session(&session_id);
    }
    eprintln!("M2M_AUTHORING response_accepted");
    Ok(())
}

/// Frontend event-listener readiness; permission is granted by an MCP authoring request.
#[cfg(feature = "mcp-host")]
#[tauri::command]
fn mcp_authoring_frontend_set_ready(
    ready: bool,
    state: State<'_, McpHostState>,
) -> Result<(), String> {
    state
        .authoring
        .frontend_ready
        .store(ready, Ordering::Release);
    eprintln!("M2M_AUTHORING frontend_ready={ready}");
    Ok(())
}

#[cfg(feature = "mcp-host")]
#[tauri::command]
fn mcp_authoring_frontend_stage(stage: String, detail: Option<String>) -> Result<(), String> {
    if !matches!(
        stage.as_str(),
        "listener_start" | "request_listener_ok" | "cancel_listener_ok" | "ready_failed"
    ) {
        return Err("AUTHORING_STAGE_INVALID".to_owned());
    }
    let detail = detail
        .unwrap_or_default()
        .chars()
        .filter(|character| {
            character.is_ascii_alphanumeric() || matches!(character, ' ' | '_' | '-' | ':' | '.')
        })
        .take(160)
        .collect::<String>();
    eprintln!("M2M_AUTHORING frontend_stage={stage} detail={detail}");
    Ok(())
}

#[cfg(feature = "mcp-host")]
#[tauri::command]
fn mcp_pose_observation_respond(
    response: PoseObservationResponse,
    state: State<'_, McpHostState>,
) -> Result<(), String> {
    let pending = state
        .observation
        .pending_pose
        .lock()
        .map_err(|_| "POSE_OBSERVATION_STATE_LOCKED".to_owned())?;
    let Some(entry) = pending.get(&response.request_id) else {
        return Err("POSE_OBSERVATION_REQUEST_NOT_PENDING".to_owned());
    };
    if response.protocol != "mcp-pose-observation/v1"
        || response.session_id != entry.session_id
        || response.sequence != entry.sequence
        || response.ok != response.result.is_some()
        || response.ok == response.error.is_some()
        || serde_json::to_vec(&response).map_or(true, |bytes| {
            bytes.len() > mesh2motion_mcp::MAX_POSE_REQUEST_BYTES - 1024
        })
    {
        return Err("POSE_OBSERVATION_RESPONSE_MISMATCH".to_owned());
    }
    let capture_id = response
        .result
        .as_ref()
        .and_then(|result| result.get("capture_id"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    let session_id = response.session_id.clone();
    let successful = response.ok;
    entry
        .sender
        .try_send(response)
        .map_err(|_| "POSE_OBSERVATION_RESPONSE_REJECTED".to_owned())?;
    drop(pending);
    if successful {
        if let Some(capture_id) = capture_id {
            let mut captures = state
                .observation
                .captures
                .lock()
                .map_err(|_| "POSE_OBSERVATION_STATE_LOCKED".to_owned())?;
            if captures.contains_key(&capture_id) {
                return Err("POSE_CAPTURE_ID_DUPLICATE".to_owned());
            }
            captures.insert(capture_id, session_id);
            while captures.len() > 4 {
                if let Some(oldest) = captures.keys().next().cloned() {
                    captures.remove(&oldest);
                }
            }
        }
    }
    Ok(())
}

#[cfg(feature = "mcp-host")]
#[tauri::command]
fn mcp_pose_observation_image_respond(
    response: PoseImageChunkResponse,
    state: State<'_, McpHostState>,
) -> Result<(), String> {
    let pending = state
        .observation
        .pending_image
        .lock()
        .map_err(|_| "POSE_IMAGE_STATE_LOCKED".to_owned())?;
    let Some(entry) = pending.get(&response.request_id) else {
        return Err("POSE_IMAGE_REQUEST_NOT_PENDING".to_owned());
    };
    let encoded_len_limit = entry.request.max_bytes.saturating_add(2) / 3 * 4 + 4;
    if !entry.request.validate_response(&response)
        || response.bytes_base64.len() > encoded_len_limit
        || !response
            .bytes_base64
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'='))
        || serde_json::to_vec(&response).map_or(true, |bytes| {
            bytes.len() > mesh2motion_mcp::IPC_MAX_FRAME_BYTES
        })
    {
        return Err("POSE_IMAGE_RESPONSE_MISMATCH".to_owned());
    }
    entry
        .sender
        .try_send(Some(response))
        .map_err(|_| "POSE_IMAGE_RESPONSE_REJECTED".to_owned())
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
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            #[cfg(feature = "mcp-host")]
            app.manage(McpHostState::new(app.handle().clone()));
            #[cfg(not(feature = "mcp-host"))]
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
            mcp_host_paths,
            #[cfg(feature = "mcp-host")]
            mcp_authoring_respond,
            #[cfg(feature = "mcp-host")]
            mcp_authoring_frontend_set_ready,
            #[cfg(feature = "mcp-host")]
            mcp_authoring_frontend_stage,
            #[cfg(feature = "mcp-host")]
            mcp_pose_observation_respond,
            #[cfg(feature = "mcp-host")]
            mcp_pose_observation_image_respond
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // Stop the optional MCP host at Tauri's explicit shutdown boundary so the
    // listener thread can remove its socket before managed state is torn down.
    app.run(|_app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            #[cfg(feature = "mcp-host")]
            if let Some(state) = _app_handle.try_state::<McpHostState>() {
                if let Err(error) = state.disable() {
                    eprintln!("M2M_HOST shutdown_error={error}");
                }
            }
        }
    });
}

#[cfg(test)]
mod contract_tests {
    #[cfg(feature = "mcp-host")]
    use super::{mcp_operation_grants_authoring, should_cancel_authoring_wait};
    use super::{preview_asset_relative_path, McpHostStatus};
    #[cfg(feature = "mcp-host")]
    use mesh2motion_mcp::AuthoringOperation;

    #[cfg(feature = "mcp-host")]
    #[test]
    fn live_catalog_is_read_only_but_mcp_authoring_operations_grant_session_access() {
        assert!(!mcp_operation_grants_authoring(
            AuthoringOperation::RigCatalogGet
        ));
        assert!(mcp_operation_grants_authoring(
            AuthoringOperation::SessionStatus
        ));
        assert!(mcp_operation_grants_authoring(
            AuthoringOperation::DraftCreate
        ));
    }

    #[cfg(feature = "mcp-host")]
    #[test]
    fn completed_authoring_response_drop_does_not_cancel_the_session() {
        assert!(!should_cancel_authoring_wait(true));
        assert!(should_cancel_authoring_wait(false));
    }

    #[test]
    fn preview_asset_keys_are_narrow_and_relative() {
        assert_eq!(
            preview_asset_relative_path("human/dark_Angry.mp4").unwrap(),
            std::path::PathBuf::from("human/dark_Angry.mp4")
        );
        assert_eq!(
            preview_asset_relative_path("fox/light_Scratch Ear.webp").unwrap(),
            std::path::PathBuf::from("fox/light_Scratch Ear.webp")
        );
        for invalid in [
            "../secret.mp4",
            "human/../../secret.mp4",
            "human/dark_Angry.mp4/extra",
            "human/dark_../secret.mp4",
            "unknown/dark_Walk.mp4",
            "human/blue_Walk.mp4",
            "human/dark_.mp4",
            "human/dark_Angry.png",
            "human/dark_Angry.webp/extra",
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
            "authoring_enabled",
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
