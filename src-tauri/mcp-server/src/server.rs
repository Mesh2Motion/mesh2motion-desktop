use std::sync::{Arc, Mutex};

use rmcp::{
    handler::server::wrapper::{Json, Parameters},
    model::{CallToolResult, ContentBlock},
    tool, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::{
    AnimationBone, AnimationChannel, AnimationClip, AnimationDocument, AnimationDraftStore,
    AnimationInfo, AnimationKeyframe, AnimationOrigin, AnimationProperty, AnimationRig,
    AnimationValidation, AppPhase, AppSnapshotSource, AuthoringErrorCode, AuthoringOperation,
    AuthoringRequest, ControlSnapshot, IpcError, LiveRigCatalog, PoseImageChunkRequest,
    PoseObservationOperation, PoseObservationRequest, RendererInfo, RigInfo, SceneInfo,
    SnapshotInfo, ToolEnvelope, ToolError, ValidationState, AUTHORING_BRIDGE_PROTOCOL,
    MAX_DOCUMENT_BYTES, MAX_IMAGE_CHUNK_BYTES, POSE_OBSERVATION_PROTOCOL,
};

#[derive(Clone)]
pub struct Mesh2MotionMcp {
    source: Arc<dyn AppSnapshotSource>,
    authoring_session_id: String,
    drafts: AnimationDraftStore,
    live_catalog: Arc<Mutex<Option<CachedLiveCatalog>>>,
    observation_captures: Arc<Mutex<HashMap<String, CachedObservation>>>,
    expired_observation_ids: Arc<Mutex<std::collections::HashSet<String>>>,
    next_observation_sequence: Arc<std::sync::atomic::AtomicU64>,
    next_authoring_sequence: Arc<std::sync::atomic::AtomicU64>,
}

#[derive(Clone)]
struct CachedObservation {
    session_id: String,
    result: Value,
    expires_at_ms: u64,
    retained_image_bytes: usize,
}

#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AnimationObservePoseArgs {
    pub draft_id: String,
    pub expected_revision: u64,
    pub timestamp_seconds: f64,
    #[serde(default)]
    pub views: Vec<String>,
    #[serde(default)]
    pub focus_bone_id: Option<String>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub projection: Option<String>,
    #[serde(default)]
    pub image_mime: Option<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AnimationCaptureViewArgs {
    pub capture_id: String,
    pub view: String,
}

#[derive(Clone)]
struct CachedLiveCatalog {
    identifier: String,
    fingerprint: String,
    bones: Vec<AnimationBone>,
}

type LocalToolError = Box<Json<ToolEnvelope<serde_json::Value>>>;

impl Mesh2MotionMcp {
    pub fn new(source: Arc<dyn AppSnapshotSource>) -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        Self {
            source,
            authoring_session_id: format!("mcp-{}-{nonce}", std::process::id()),
            drafts: AnimationDraftStore::default(),
            live_catalog: Arc::new(Mutex::new(None)),
            observation_captures: Arc::new(Mutex::new(HashMap::new())),
            expired_observation_ids: Arc::new(Mutex::new(std::collections::HashSet::new())),
            next_observation_sequence: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            next_authoring_sequence: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    fn cached_live_catalog(&self) -> Result<CachedLiveCatalog, LocalToolError> {
        self.live_catalog
            .lock()
            .map_err(|_| {
                Box::new(Json(ToolEnvelope::failure(
                    "INTERNAL",
                    "live rig catalog is unavailable",
                    false,
                    None,
                )))
            })?
            .clone()
            .ok_or_else(|| {
                Box::new(Json(ToolEnvelope::failure(
                    "RIG_UNAVAILABLE",
                    "read the live skeleton catalog before authoring",
                    true,
                    None,
                )))
            })
    }

    fn document_for_validation(
        &self,
        candidate: Option<AnimationDocument>,
    ) -> Result<(CachedLiveCatalog, AnimationDocument), LocalToolError> {
        let catalog = self.cached_live_catalog()?;
        let document = match candidate {
            Some(document) => document,
            None => self.drafts.get(&self.authoring_session_id).map_err(|_| {
                Box::new(Json(ToolEnvelope::failure(
                    "INVALID_DOCUMENT",
                    "create or import a draft before this operation",
                    false,
                    None,
                )))
            })?,
        };
        if let Err(errors) = document.validate(&catalog.fingerprint, &catalog.bones) {
            return Err(Box::new(Json(ToolEnvelope::failure(
                "INVALID_DOCUMENT",
                &errors.join("; "),
                false,
                None,
            ))));
        }
        Ok((catalog, document))
    }

    async fn read_snapshot(&self, tool: &'static str) -> Result<ControlSnapshot, IpcError> {
        let result = self.source.snapshot().await;
        let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        match &result {
            Ok(snapshot) => eprintln!(
                "M2M_MCP_TOOL timestamp={timestamp} tool={tool} result=snapshot sequence={} age_ms={}",
                snapshot.meta.sequence, snapshot.meta.age_ms
            ),
            Err(error) => eprintln!(
                "M2M_MCP_TOOL timestamp={timestamp} tool={tool} result=error code={error:?}"
            ),
        }
        result
    }

    async fn authoring_call(
        &self,
        operation: AuthoringOperation,
        payload: serde_json::Value,
    ) -> Json<ToolEnvelope<serde_json::Value>> {
        let response = self.authoring_call_once(operation, payload.clone()).await;
        // A reconnecting MCP client can meet a stale frontend session left by a
        // previous peer. The frontend accepts session.cancel from any trusted
        // native request, so clear it once and retry instead of stalling.
        if response
            .0
            .error
            .as_ref()
            .is_some_and(|error| error.code == "SESSION_CONFLICT")
        {
            let _ = self
                .authoring_call_once(AuthoringOperation::SessionCancel, serde_json::json!({}))
                .await;
            return self.authoring_call_once(operation, payload).await;
        }
        response
    }

    async fn authoring_call_once(
        &self,
        operation: AuthoringOperation,
        payload: serde_json::Value,
    ) -> Json<ToolEnvelope<serde_json::Value>> {
        let sequence = self
            .next_authoring_sequence
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            .saturating_add(1);
        let request = AuthoringRequest {
            protocol: AUTHORING_BRIDGE_PROTOCOL.to_owned(),
            request_id: format!("mcp-{}-{sequence}", self.authoring_session_id),
            session_id: self.authoring_session_id.clone(),
            sequence,
            deadline_ms: 30_000,
            operation,
            payload,
        };
        match self.source.authoring_request(request).await {
            Ok(response) if response.ok && response.result.is_some() => Json(ToolEnvelope {
                ok: true,
                data: response.result,
                error: None,
                meta: None,
            }),
            Ok(response) if !response.ok => {
                let (code, message) = response
                    .error
                    .map(|error| (authoring_error_code(error.code), error.message))
                    .unwrap_or(("INTERNAL", "native authoring request failed".to_owned()));
                Json(ToolEnvelope::failure(code, &message, false, None))
            }
            Ok(_) => Json(ToolEnvelope::ipc_error()),
            Err(error) => Json(error_envelope(error)),
        }
    }

    async fn observe_pose(&self, args: AnimationObservePoseArgs) -> CallToolResult {
        let fail = |code: &str, message: &str| {
            CallToolResult::error(vec![ContentBlock::text(
                json!({"ok":false,"error":{"code":code,"message":message}}).to_string(),
            )])
        };
        if args.draft_id.is_empty()
            || args.draft_id.len() > 64
            || !args.timestamp_seconds.is_finite()
        {
            return fail("BAD_REQUEST", "draft_id or timestamp is invalid");
        }
        let document = match self.drafts.get(&self.authoring_session_id) {
            Ok(document) => document,
            Err(_) => {
                return fail(
                    "SESSION_CONFLICT",
                    "no authorized draft exists for this MCP session",
                )
            }
        };
        if self.authoring_session_id != args.draft_id {
            return fail(
                "SESSION_CONFLICT",
                "draft_id is not owned by this MCP session",
            );
        }
        if document.revision != args.expected_revision {
            return fail(
                "STALE_REVISION",
                "expected_revision does not match the authoritative draft",
            );
        }
        if args.timestamp_seconds < 0.0 || args.timestamp_seconds > document.clip.duration_seconds {
            return fail(
                "BAD_REQUEST",
                "timestamp_seconds is outside the draft duration",
            );
        }
        let catalog = match self.cached_live_catalog() {
            Ok(catalog) => catalog,
            Err(_) => return fail("RIG_UNAVAILABLE", "live rig catalog is unavailable"),
        };
        if document.rig.fingerprint != catalog.fingerprint {
            return fail(
                "RIG_MISMATCH",
                "draft rig fingerprint no longer matches the live rig",
            );
        }
        if let Some(bone_id) = args.focus_bone_id.as_ref() {
            if !catalog.bones.iter().any(|bone| &bone.id == bone_id) {
                return fail(
                    "UNSUPPORTED_BONE",
                    "focus_bone_id is not present in the live rig",
                );
            }
        }
        let views = if args.views.is_empty() {
            vec!["front", "right", "top"]
                .into_iter()
                .map(str::to_owned)
                .collect()
        } else {
            args.views.clone()
        };
        let mut unique_views = std::collections::HashSet::new();
        if views.is_empty()
            || views.len() > 6
            || views.iter().any(|v| {
                !matches!(
                    v.as_str(),
                    "front" | "back" | "left" | "right" | "top" | "bottom"
                ) || !unique_views.insert(v)
            })
        {
            return fail("BAD_REQUEST", "views must be unique supported camera names");
        }
        if args.width.is_some_and(|v| !(64..=1024).contains(&v))
            || args.height.is_some_and(|v| !(64..=1024).contains(&v))
        {
            return fail("BAD_REQUEST", "image dimensions must be 64..1024");
        }
        if args
            .projection
            .as_deref()
            .is_some_and(|v| !matches!(v, "orthographic" | "perspective"))
        {
            return fail("BAD_REQUEST", "projection is unsupported");
        }
        let image_mime = args.image_mime.as_deref().unwrap_or("image/png");
        if !matches!(image_mime, "image/png" | "image/jpeg") {
            return fail("BAD_REQUEST", "image MIME type is unsupported");
        }
        // Allow the contract maximum per image. Inline bytes are only optional;
        // the frontend strips them when the complete observation approaches the
        // 256 KiB response ceiling, leaving image_ref retrieval for MCP clients.
        let max_image_bytes = if image_mime == "image/png" {
            1024 * 1024
        } else {
            256 * 1024
        };
        let payload = json!({
            "rig_fingerprint": catalog.fingerprint,
            "draft_id": self.authoring_session_id,
            "expected_revision": document.revision,
            "timestamp_seconds": args.timestamp_seconds,
            "views": views,
            "focus_bone_id": args.focus_bone_id,
            "width": args.width.unwrap_or(512),
            "height": args.height.unwrap_or(512),
            "projection": args.projection.unwrap_or_else(|| "orthographic".into()),
            "image": {"mime": image_mime},
            "include_joints": true,
            "max_image_bytes": max_image_bytes
        });
        let sequence = self
            .next_observation_sequence
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            .saturating_add(1);
        let request = PoseObservationRequest {
            protocol: POSE_OBSERVATION_PROTOCOL.into(),
            request_id: pose_request_id(&self.authoring_session_id, sequence),
            session_id: self.authoring_session_id.clone(),
            sequence,
            deadline_ms: 30_000,
            operation: PoseObservationOperation::Capture,
            payload,
        };
        if request.validate().is_err() {
            return fail(
                "PAYLOAD_TOO_LARGE",
                "observation request exceeds the v1 envelope limit",
            );
        }
        let response = match self.source.pose_observation_request(request.clone()).await {
            Ok(response) if request.validate_response(&response) => response,
            Ok(_) => {
                return fail(
                    "INTERNAL",
                    "frontend observation response correlation failed",
                )
            }
            Err(error) => {
                let code = match error {
                    IpcError::Unavailable | IpcError::AppUnavailable => "RIG_UNAVAILABLE",
                    IpcError::Disconnected => "RIG_UNAVAILABLE",
                    IpcError::NotReady => "RIG_UNAVAILABLE",
                    IpcError::IncompatibleVersion => "BAD_PROTOCOL",
                    IpcError::PermissionDenied | IpcError::AuthoringDisabled => "SESSION_CONFLICT",
                    IpcError::DeadlineExceeded => "DEADLINE_EXCEEDED",
                    IpcError::InvalidResponse => "INTERNAL",
                };
                return fail(code, &format!("observation bridge failed: {error:?}"));
            }
        };
        if !response.ok {
            let error = response.error.unwrap_or(crate::PoseObservationError {
                code: "INTERNAL".into(),
                message: "observation failed".into(),
            });
            return fail(&error.code, &error.message);
        }
        let Some(result) = response.result else {
            return fail("INTERNAL", "observation response omitted result");
        };
        let valid_header = result.get("protocol_version").and_then(Value::as_str)
            == Some(POSE_OBSERVATION_PROTOCOL)
            && result.get("draft_id").and_then(Value::as_str)
                == Some(self.authoring_session_id.as_str())
            && result.get("revision").and_then(Value::as_u64) == Some(document.revision)
            && result
                .get("rig")
                .and_then(|v| v.get("fingerprint"))
                .and_then(Value::as_str)
                == Some(catalog.fingerprint.as_str())
            && result.get("timestamp_seconds").and_then(Value::as_f64)
                == Some(args.timestamp_seconds)
            && result
                .get("pose_digest")
                .and_then(Value::as_str)
                .is_some_and(|digest| {
                    digest.len() == 64
                        && digest
                            .bytes()
                            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
                })
            && result
                .get("width")
                .and_then(Value::as_u64)
                .is_some_and(|width| (64..=1024).contains(&width))
            && result
                .get("height")
                .and_then(Value::as_u64)
                .is_some_and(|height| (64..=1024).contains(&height));
        if !valid_header {
            return fail(
                "INTERNAL",
                "capture metadata does not match the validated draft and pose",
            );
        }
        let Some(capture_id) = result.get("capture_id").and_then(Value::as_str) else {
            return fail("INTERNAL", "capture_id missing");
        };
        let Some(expires_at_ms) = result.get("expires_at_ms").and_then(Value::as_u64) else {
            return fail("INTERNAL", "capture expiry missing");
        };
        let Some(captured_views) = result.get("views").and_then(Value::as_array) else {
            return fail("INTERNAL", "captured views missing");
        };
        if captured_views.len() != views.len() {
            return fail("INTERNAL", "renderer returned a partial view set");
        }
        let mut returned_views = std::collections::HashSet::new();
        let mut aggregate_bytes = 0usize;
        for view in captured_views {
            let Some(view_name) = view.get("view").and_then(Value::as_str) else {
                return fail("INTERNAL", "view metadata missing name");
            };
            if !views.iter().any(|requested| requested == view_name)
                || !returned_views.insert(view_name)
            {
                return fail("INTERNAL", "renderer returned an unrequested view");
            }
            let mime = view.get("mime").and_then(Value::as_str).unwrap_or_default();
            let bytes = view
                .get("image_bytes")
                .and_then(Value::as_u64)
                .unwrap_or_default() as usize;
            let limit = if mime == "image/png" {
                1024 * 1024
            } else if mime == "image/jpeg" {
                256 * 1024
            } else {
                0
            };
            if mime != image_mime
                || limit == 0
                || bytes == 0
                || bytes > limit
                || view.get("image_ref").and_then(Value::as_str).is_none()
            {
                return fail(
                    "RENDER_FAILED",
                    "renderer returned invalid image reference metadata",
                );
            }
            if !validate_view_geometry(view, &catalog.bones) {
                return fail(
                    "INTERNAL",
                    "camera or joint geometry does not match the live rig catalog",
                );
            }
            aggregate_bytes = match aggregate_bytes.checked_add(bytes) {
                Some(total) => total,
                None => return fail("RESOURCE_LIMIT_EXCEEDED", "image byte sum overflow"),
            };
        }
        if returned_views.len() != views.len() {
            return fail("INTERNAL", "renderer returned duplicate or missing views");
        }
        if aggregate_bytes > 8 * 1024 * 1024 {
            return fail(
                "RESOURCE_LIMIT_EXCEEDED",
                "capture exceeds aggregate image memory limit",
            );
        }
        let now = chrono::Utc::now().timestamp_millis().max(0) as u64;
        if expires_at_ms <= now || expires_at_ms > now.saturating_add(30_000) {
            return fail(
                "CAPTURE_EXPIRED",
                "capture expiry is invalid or already elapsed",
            );
        }
        if let Ok(mut captures) = self.observation_captures.lock() {
            let expired = captures
                .iter()
                .filter(|(_, capture)| capture.expires_at_ms <= now)
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>();
            for id in &expired {
                captures.remove(id);
            }
            if let Ok(mut ids) = self.expired_observation_ids.lock() {
                ids.extend(expired);
                while ids.len() > 4096 {
                    if let Some(id) = ids.iter().next().cloned() {
                        ids.remove(&id);
                    }
                }
                ids.remove(capture_id);
            }
            if captures.len() >= 4 {
                if let Some(oldest) = captures
                    .iter()
                    .min_by_key(|(_, capture)| capture.expires_at_ms)
                    .map(|(id, _)| id.clone())
                {
                    captures.remove(&oldest);
                }
            }
            let retained: usize = captures
                .values()
                .map(|capture| capture.retained_image_bytes)
                .sum();
            if retained.saturating_add(aggregate_bytes) > 8 * 1024 * 1024 {
                return fail(
                    "RESOURCE_LIMIT_EXCEEDED",
                    "retained captures exceed aggregate image memory limit",
                );
            }
            captures.insert(
                capture_id.to_owned(),
                CachedObservation {
                    session_id: self.authoring_session_id.clone(),
                    result: result.clone(),
                    expires_at_ms,
                    retained_image_bytes: aggregate_bytes,
                },
            );
        } else {
            return fail("INTERNAL", "capture registry is unavailable");
        }
        CallToolResult::structured(result)
    }

    async fn capture_view(&self, args: AnimationCaptureViewArgs) -> CallToolResult {
        let fail = |code: &str, message: &str| {
            CallToolResult::error(vec![ContentBlock::text(
                json!({"ok":false,"error":{"code":code,"message":message}}).to_string(),
            )])
        };
        if !matches!(
            args.view.as_str(),
            "front" | "back" | "left" | "right" | "top" | "bottom"
        ) {
            return fail("BAD_REQUEST", "unsupported view");
        }
        let now = chrono::Utc::now().timestamp_millis().max(0) as u64;
        let capture = match self.observation_captures.lock() {
            Ok(mut captures) => {
                let capture = captures.get(&args.capture_id).cloned();
                if capture
                    .as_ref()
                    .is_some_and(|item| item.expires_at_ms <= now)
                {
                    captures.remove(&args.capture_id);
                    if let Ok(mut ids) = self.expired_observation_ids.lock() {
                        ids.insert(args.capture_id.clone());
                    }
                    None
                } else {
                    capture
                }
            }
            Err(_) => return fail("INTERNAL", "capture registry is unavailable"),
        };
        let Some(capture) = capture else {
            let expired = self
                .expired_observation_ids
                .lock()
                .is_ok_and(|ids| ids.contains(&args.capture_id));
            return if expired {
                fail("CAPTURE_EXPIRED", "capture ID has expired")
            } else {
                fail("CAPTURE_NOT_FOUND", "capture ID is unknown or was evicted")
            };
        };
        if capture.session_id != self.authoring_session_id {
            return fail(
                "CAPTURE_NOT_FOUND",
                "capture is not authorized for this session",
            );
        }
        let Some(views) = capture.result.get("views").and_then(Value::as_array) else {
            return fail("INTERNAL", "capture view metadata missing");
        };
        let Some(view_data) = views
            .iter()
            .find(|item| item.get("view").and_then(Value::as_str) == Some(args.view.as_str()))
        else {
            return fail(
                "CAPTURE_NOT_FOUND",
                "requested view is not retained by this capture",
            );
        };
        let Some(image_ref) = view_data.get("image_ref").and_then(Value::as_str) else {
            return fail("CAPTURE_NOT_FOUND", "image reference missing");
        };
        let Some(mime) = view_data.get("mime").and_then(Value::as_str) else {
            return fail("INTERNAL", "image MIME missing");
        };
        let Some(expected_bytes) = view_data
            .get("image_bytes")
            .and_then(Value::as_u64)
            .map(|n| n as usize)
        else {
            return fail("INTERNAL", "image byte count missing");
        };
        let Some(capture_id) = capture.result.get("capture_id").and_then(Value::as_str) else {
            return fail("INTERNAL", "capture ID missing");
        };
        let mut image = Vec::with_capacity(expected_bytes);
        let mut offset = 0usize;
        let mut request_number = 0u64;
        while offset < expected_bytes {
            request_number += 1;
            let max_bytes = MAX_IMAGE_CHUNK_BYTES.min(expected_bytes - offset);
            let request = PoseImageChunkRequest {
                request_id: format!(
                    "img-{}-{request_number}",
                    capture_id.chars().take(36).collect::<String>()
                ),
                session_id: self.authoring_session_id.clone(),
                capture_id: capture_id.to_owned(),
                image_ref: image_ref.to_owned(),
                view: args.view.clone(),
                offset,
                max_bytes,
            };
            let chunk = match self.source.pose_image_chunk(request.clone()).await {
                Ok(chunk) if request.validate_response(&chunk) => chunk,
                Ok(_) => {
                    return fail(
                        "INTERNAL",
                        "image chunk response failed correlation or size validation",
                    )
                }
                Err(_) => {
                    return fail(
                        "CAPTURE_NOT_FOUND",
                        "image bytes are no longer available from the frontend",
                    )
                }
            };
            if let Some(error) = chunk.error.as_ref() {
                return fail(&error.code, &error.message);
            }
            if chunk.mime != mime || chunk.total_bytes != expected_bytes {
                return fail("INTERNAL", "image metadata changed between chunks");
            }
            let decoded = match decode_base64(&chunk.bytes_base64) {
                Some(bytes) => bytes,
                None => return fail("RENDER_FAILED", "image chunk is invalid base64"),
            };
            if decoded.is_empty()
                || decoded.len() > max_bytes
                || image.len().saturating_add(decoded.len()) > expected_bytes
            {
                return fail(
                    "PAYLOAD_TOO_LARGE",
                    "image chunk exceeded its declared bounds",
                );
            }
            offset += decoded.len();
            image.extend_from_slice(&decoded);
        }
        let expected_dimensions = capture
            .result
            .get("width")
            .and_then(Value::as_u64)
            .zip(capture.result.get("height").and_then(Value::as_u64));
        if image.len() != expected_bytes
            || !valid_encoded_image(&image, mime)
            || encoded_image_dimensions(&image, mime)
                != expected_dimensions.map(|(width, height)| (width as u32, height as u32))
        {
            return fail(
                "RENDER_FAILED",
                "image bytes do not match the declared MIME or size",
            );
        }
        let metadata = json!({"capture_id":capture_id,"view":args.view,"mime":mime,"width":capture.result.get("width"),"height":capture.result.get("height"),"rig":capture.result.get("rig"),"draft_id":capture.result.get("draft_id"),"revision":capture.result.get("revision"),"timestamp_seconds":capture.result.get("timestamp_seconds"),"pose_digest":capture.result.get("pose_digest"),"camera":view_data.get("camera"),"joints":view_data.get("joints")});
        CallToolResult::success(vec![
            ContentBlock::text(metadata.to_string()),
            ContentBlock::image(encode_base64(&image), mime),
        ])
    }

    pub async fn dispose_observation_captures(&self) {
        let capture_ids = self
            .observation_captures
            .lock()
            .map(|captures| captures.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        for (index, capture_id) in capture_ids.into_iter().enumerate() {
            let sequence = self
                .next_observation_sequence
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                .saturating_add(1);
            let request = PoseObservationRequest {
                protocol: POSE_OBSERVATION_PROTOCOL.into(),
                request_id: pose_dispose_request_id(&self.authoring_session_id, sequence, index),
                session_id: self.authoring_session_id.clone(),
                sequence,
                deadline_ms: 1_000,
                operation: PoseObservationOperation::Dispose,
                payload: json!({"capture_id":capture_id}),
            };
            let _ = self.source.pose_observation_request(request).await;
        }
        if let Ok(mut captures) = self.observation_captures.lock() {
            captures.clear();
        }
    }

    /// Release the single app-side authoring session when this stdio peer exits.
    /// A later MCP process receives a new session ID and otherwise cannot replace
    /// the frontend dispatcher's retained prior session.
    pub async fn cancel_authoring_session(&self) {
        let response = self
            .authoring_call(AuthoringOperation::SessionCancel, serde_json::json!({}))
            .await;
        if response.0.ok {
            let _ = self.drafts.remove(&self.authoring_session_id);
            if let Ok(mut captures) = self.observation_captures.lock() {
                captures.clear();
            }
        }
    }
}

fn authoring_error_code(code: AuthoringErrorCode) -> &'static str {
    match code {
        AuthoringErrorCode::BadProtocol => "BAD_PROTOCOL",
        AuthoringErrorCode::UnknownOperation => "UNKNOWN_OPERATION",
        AuthoringErrorCode::BadRequest => "BAD_REQUEST",
        AuthoringErrorCode::PayloadTooLarge => "PAYLOAD_TOO_LARGE",
        AuthoringErrorCode::DeadlineExceeded => "DEADLINE_EXCEEDED",
        AuthoringErrorCode::DuplicateRequestId => "DUPLICATE_REQUEST_ID",
        AuthoringErrorCode::SessionConflict => "SESSION_CONFLICT",
        AuthoringErrorCode::RigUnavailable => "RIG_UNAVAILABLE",
        AuthoringErrorCode::RigMismatch => "RIG_MISMATCH",
        AuthoringErrorCode::InvalidDocument => "INVALID_DOCUMENT",
        AuthoringErrorCode::PreviewActive => "PREVIEW_ACTIVE",
        AuthoringErrorCode::ConfirmationRequired => "CONFIRMATION_REQUIRED",
        AuthoringErrorCode::ConfirmationDenied => "CONFIRMATION_DENIED",
        AuthoringErrorCode::ConfirmationExpired => "CONFIRMATION_EXPIRED",
        AuthoringErrorCode::CommitFailed => "COMMIT_FAILED",
        AuthoringErrorCode::UndoFailed => "UNDO_FAILED",
        AuthoringErrorCode::Internal => "INTERNAL",
        AuthoringErrorCode::AuthoringDisabled => "AUTHORING_DISABLED",
        AuthoringErrorCode::AppDisconnected => "APP_DISCONNECTED",
    }
}

fn pose_request_id(session_id: &str, sequence: u64) -> String {
    format!("pose-{session_id}-{sequence}")
}

fn pose_dispose_request_id(session_id: &str, sequence: u64, index: usize) -> String {
    format!("dispose-{session_id}-{sequence}-{index}")
}

#[cfg(test)]
mod live_catalog_tests {
    use super::*;

    fn catalog() -> LiveRigCatalogData {
        LiveRigCatalogData {
            rig: LiveRigIdentity {
                identifier: "human".to_owned(),
                fingerprint: "a".repeat(64),
            },
            bones: vec![LiveBoneData {
                id: "human:bone:000".to_owned(),
                display_name: "root".to_owned(),
                index: 0,
                parent_id: None,
                local_position: [0.0, 0.0, 0.0],
                local_quaternion: [0.0, 0.0, 0.0, 1.0],
                local_scale: [1.0, 1.0, 1.0],
            }],
        }
    }

    #[test]
    fn live_catalog_requires_a_real_shaped_fingerprint_and_normalized_bones() {
        assert!(catalog().is_well_formed());
        let mut invalid = catalog();
        invalid.bones[0].local_quaternion = [0.0, 0.0, 0.0, 2.0];
        assert!(!invalid.is_well_formed());
        let mut invalid = catalog();
        invalid.rig.fingerprint = "fixture".to_owned();
        assert!(!invalid.is_well_formed());
        let mut invalid = catalog();
        invalid.bones[0].parent_id = Some("missing".to_owned());
        assert!(!invalid.is_well_formed());
    }
}

impl LiveRigCatalogData {
    fn is_well_formed(&self) -> bool {
        use std::collections::HashSet;
        if self.rig.identifier.trim().is_empty()
            || self.rig.fingerprint.len() != 64
            || !self
                .rig
                .fingerprint
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || self.bones.is_empty()
            || self.bones.len() > crate::MAX_BONES
        {
            return false;
        }
        let mut ids = HashSet::new();
        let mut indices = HashSet::new();
        if self.bones.iter().any(|bone| {
            bone.id.trim().is_empty()
                || !ids.insert(bone.id.as_str())
                || !indices.insert(bone.index)
                || bone.local_position.iter().any(|value| !value.is_finite())
                || bone.local_scale.iter().any(|value| !value.is_finite())
                || bone.local_quaternion.iter().any(|value| !value.is_finite())
                || (bone
                    .local_quaternion
                    .iter()
                    .map(|value| value * value)
                    .sum::<f64>()
                    .sqrt()
                    - 1.0)
                    .abs()
                    > 1e-4
        }) {
            return false;
        }
        self.bones.iter().all(|bone| {
            bone.parent_id
                .as_ref()
                .is_none_or(|parent_id| parent_id != &bone.id && ids.contains(parent_id.as_str()))
        })
    }
}

fn validate_view_geometry(view: &Value, bones: &[AnimationBone]) -> bool {
    let Some(camera) = view.get("camera") else {
        return false;
    };
    let valid_vector = |value: Option<&Value>, length: usize| {
        value.and_then(Value::as_array).is_some_and(|values| {
            values.len() == length
                && values
                    .iter()
                    .all(|v| v.as_f64().is_some_and(f64::is_finite))
        })
    };
    if !matches!(
        camera.get("projection").and_then(Value::as_str),
        Some("orthographic" | "perspective")
    ) || !valid_vector(camera.get("position"), 3)
        || !valid_vector(camera.get("target"), 3)
        || !valid_vector(camera.get("up"), 3)
        || !valid_vector(camera.get("view_matrix"), 16)
        || !valid_vector(camera.get("projection_matrix"), 16)
        || !camera
            .get("near")
            .and_then(Value::as_f64)
            .is_some_and(|v| v.is_finite() && v > 0.0)
        || !camera
            .get("far")
            .and_then(Value::as_f64)
            .is_some_and(|v| v.is_finite() && v > 0.0)
    {
        return false;
    }
    let Some(joints) = view.get("joints").and_then(Value::as_array) else {
        return false;
    };
    if joints.len() > bones.len() || joints.len() > crate::MAX_BONES {
        return false;
    }
    let mut seen = std::collections::HashSet::new();
    joints.iter().all(|joint| {
        let Some(bone_id) = joint.get("bone_id").and_then(Value::as_str) else {
            return false;
        };
        let Some(bone) = bones.iter().find(|bone| bone.id == bone_id) else {
            return false;
        };
        if !seen.insert(bone_id)
            || joint.get("parent_id")
                != Some(&serde_json::to_value(&bone.parent_id).unwrap_or(Value::Null))
            || !valid_vector(joint.get("world_position"), 3)
            || !valid_vector(joint.get("local_position"), 3)
            || !valid_vector(joint.get("local_quaternion"), 4)
            || !valid_vector(joint.get("local_scale"), 3)
        {
            return false;
        }
        let Some(rotation) = joint.get("local_quaternion").and_then(Value::as_array) else {
            return false;
        };
        let norm = rotation
            .iter()
            .filter_map(Value::as_f64)
            .map(|v| v * v)
            .sum::<f64>()
            .sqrt();
        let Some(projected) = joint.get("projected") else {
            return false;
        };
        let valid_occlusion = matches!(
            projected.get("occlusion").and_then(Value::as_str),
            Some("none" | "self" | "other" | "unknown")
        );
        valid_occlusion
            && (norm - 1.0).abs() <= 1e-4
            && projected
                .get("u")
                .and_then(Value::as_f64)
                .is_some_and(f64::is_finite)
            && projected
                .get("v")
                .and_then(Value::as_f64)
                .is_some_and(f64::is_finite)
            && projected
                .get("depth")
                .and_then(Value::as_f64)
                .is_some_and(f64::is_finite)
            && projected.get("in_frame").and_then(Value::as_bool).is_some()
            && projected.get("visible").and_then(Value::as_bool).is_some()
    })
}

#[derive(Clone, Debug, serde::Deserialize, JsonSchema, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct LiveRigCatalogData {
    pub rig: LiveRigIdentity,
    pub bones: Vec<LiveBoneData>,
}

#[derive(Clone, Debug, serde::Deserialize, JsonSchema, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct LiveRigIdentity {
    pub identifier: String,
    pub fingerprint: String,
}

#[derive(Clone, Debug, serde::Deserialize, JsonSchema, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct LiveBoneData {
    pub id: String,
    pub display_name: String,
    pub index: u16,
    pub parent_id: Option<String>,
    pub local_position: [f64; 3],
    pub local_quaternion: [f64; 4],
    pub local_scale: [f64; 3],
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateDraftArgs {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub duration_seconds: Option<f64>,
    #[serde(default)]
    pub document: Option<AnimationDocument>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetBoneKeyframesArgs {
    pub expected_revision: u64,
    pub bone_id: String,
    pub property: AnimationProperty,
    pub keyframes: Vec<AnimationKeyframe>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ValidateDraftArgs {
    #[serde(default)]
    pub document: Option<AnimationDocument>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct PreviewDraftArgs {
    pub action: PreviewAction,
    #[serde(default)]
    pub document: Option<AnimationDocument>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PreviewAction {
    Start,
    Stop,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommitDraftArgs {
    #[serde(default)]
    pub document: Option<AnimationDocument>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UndoCommitArgs {
    pub target_revision: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PageArgs {
    pub page: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ImportDraftArgs {
    pub document: AnimationDocument,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EmptyArgs {}

#[derive(Clone, Debug, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplicationStatusArgs {
    /// Include the diagnostics-enabled flag. This never enables diagnostics.
    #[serde(default)]
    pub include_diagnostics: bool,
}

#[derive(Clone, Debug, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RendererStateArgs {}

#[derive(Clone, Debug, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RigsListArgs {
    /// Reserved for a future permission-gated debug mode; currently denied.
    #[serde(default)]
    pub include_asset_paths: bool,
}

#[derive(Clone, Debug, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnimationsListArgs {
    #[serde(default)]
    pub rig: Option<RigKey>,
    #[serde(default)]
    pub include_custom: bool,
    #[serde(default)]
    pub include_manifest: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RigKey {
    Human,
    Fox,
    Bird,
    Dragon,
    Kaiju,
    Spider,
    Snake,
    Fish,
    Horse,
    Error,
    None,
}

impl RigKey {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Fox => "fox",
            Self::Bird => "bird",
            Self::Dragon => "dragon",
            Self::Kaiju => "kaiju",
            Self::Spider => "spider",
            Self::Snake => "snake",
            Self::Fish => "fish",
            Self::Horse => "horse",
            Self::Error => "error",
            Self::None => "none",
        }
    }
}

#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneInspectArgs {
    #[serde(default = "default_true")]
    pub include_counts: bool,
    #[serde(default = "default_max_bones")]
    #[schemars(range(min = 1, max = 256))]
    pub max_bones: u16,
}

impl Default for SceneInspectArgs {
    fn default() -> Self {
        Self {
            include_counts: true,
            max_bones: default_max_bones(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_max_bones() -> u16 {
    64
}

#[derive(Clone, Debug, serde::Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationStatusData {
    pub phase: AppPhase,
    pub page: crate::AppPage,
    pub version: String,
    pub build: String,
    pub uptime_ms: u64,
    pub diagnostics_enabled: bool,
    pub snapshot: SnapshotInfo,
}

#[derive(Clone, Debug, serde::Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListData<T> {
    pub items: Vec<T>,
    pub count: usize,
}

#[derive(Clone, Debug, serde::Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SceneInspectData {
    pub scene: SceneInfo,
    pub truncated: bool,
}

#[tool_router(server_handler)]
impl Mesh2MotionMcp {
    #[tool(
        name = "application_get_status",
        description = "Read the current Mesh2Motion application phase and version."
    )]
    pub async fn application_get_status(
        &self,
        Parameters(args): Parameters<ApplicationStatusArgs>,
    ) -> Json<ToolEnvelope<ApplicationStatusData>> {
        let _ = args.include_diagnostics;
        match self.read_snapshot("application_get_status").await {
            Ok(snapshot) => {
                let data = ApplicationStatusData {
                    phase: snapshot.application.phase,
                    page: snapshot.application.page,
                    version: snapshot.application.version,
                    build: snapshot.application.build,
                    uptime_ms: snapshot.application.uptime_ms,
                    diagnostics_enabled: snapshot.application.diagnostics_enabled,
                    snapshot: SnapshotInfo {
                        sequence: snapshot.meta.sequence,
                        captured_at: snapshot.meta.captured_at.clone(),
                        age_ms: snapshot.meta.age_ms,
                    },
                };
                Json(ToolEnvelope::success(data, &snapshot.meta))
            }
            Err(error) => Json(error_envelope(error)),
        }
    }

    #[tool(
        name = "renderer_get_state",
        description = "Read measured WebGL and render-loop state; unavailable fields are omitted."
    )]
    pub async fn renderer_get_state(
        &self,
        Parameters(_args): Parameters<RendererStateArgs>,
    ) -> Json<ToolEnvelope<RendererInfo>> {
        match self.read_snapshot("renderer_get_state").await {
            Ok(snapshot) => {
                if let Some(error) = freshness_error(&snapshot) {
                    return Json(error);
                }
                if matches!(
                    snapshot.application.phase,
                    AppPhase::Uninitialized | AppPhase::Closed
                ) {
                    return Json(ToolEnvelope::failure(
                        "NOT_READY",
                        "Renderer state is not available in this application phase.",
                        true,
                        Some(&snapshot.meta),
                    ));
                }
                Json(ToolEnvelope::success(
                    snapshot.renderer.clone(),
                    &snapshot.meta,
                ))
            }
            Err(error) => Json(error_envelope(error)),
        }
    }

    #[tool(
        name = "rigs_list",
        description = "List configured rig names and bundle-relative asset keys."
    )]
    pub async fn rigs_list(
        &self,
        Parameters(args): Parameters<RigsListArgs>,
    ) -> Json<ToolEnvelope<ListData<RigInfo>>> {
        if args.include_asset_paths {
            return Json(ToolEnvelope::failure(
                "PERMISSION_DENIED",
                "Absolute asset paths are not exposed by this server.",
                false,
                None,
            ));
        }
        match self.read_snapshot("rigs_list").await {
            Ok(snapshot) => {
                if let Some(error) = freshness_error(&snapshot) {
                    return Json(error);
                }
                if !matches!(
                    snapshot.application.phase,
                    AppPhase::Loading | AppPhase::Ready | AppPhase::Degraded
                ) {
                    return Json(not_ready(&snapshot));
                }
                let count = snapshot.rigs.len();
                Json(ToolEnvelope::success(
                    ListData {
                        items: snapshot.rigs,
                        count,
                    },
                    &snapshot.meta,
                ))
            }
            Err(error) => Json(error_envelope(error)),
        }
    }

    #[tool(
        name = "animations_list",
        description = "List loaded or configured animation metadata without clip data."
    )]
    pub async fn animations_list(
        &self,
        Parameters(args): Parameters<AnimationsListArgs>,
    ) -> Json<ToolEnvelope<ListData<AnimationInfo>>> {
        match self.read_snapshot("animations_list").await {
            Ok(snapshot) => {
                if let Some(error) = freshness_error(&snapshot) {
                    return Json(error);
                }
                if snapshot.application.phase == AppPhase::Fatal {
                    return Json(ToolEnvelope::failure(
                        "ENGINE_FATAL",
                        "Animation state is unavailable after a fatal engine error.",
                        false,
                        Some(&snapshot.meta),
                    ));
                }
                if !matches!(
                    snapshot.application.phase,
                    AppPhase::Ready | AppPhase::Degraded
                ) {
                    return Json(not_ready(&snapshot));
                }
                let items = snapshot
                    .animations
                    .into_iter()
                    .filter(|animation| {
                        (args.include_custom
                            || animation.source_type != crate::AnimationSource::CustomImport)
                            && (args.include_manifest
                                || animation.origin != AnimationOrigin::Manifest)
                            && args
                                .rig
                                .as_ref()
                                .is_none_or(|rig| animation.rig.as_deref() == Some(rig.as_str()))
                    })
                    .collect::<Vec<_>>();
                let count = items.len();
                Json(ToolEnvelope::success(
                    ListData { items, count },
                    &snapshot.meta,
                ))
            }
            Err(error) => Json(error_envelope(error)),
        }
    }

    #[tool(
        name = "scene_inspect",
        description = "Read safe scene counts, names, and camera position; never returns geometry or paths."
    )]
    pub async fn scene_inspect(
        &self,
        Parameters(args): Parameters<SceneInspectArgs>,
    ) -> Json<ToolEnvelope<SceneInspectData>> {
        if !(1..=256).contains(&args.max_bones) {
            return Json(ToolEnvelope::failure(
                "INVALID_PARAMS",
                "maxBones must be between 1 and 256.",
                false,
                None,
            ));
        }
        match self.read_snapshot("scene_inspect").await {
            Ok(snapshot) => {
                if let Some(error) = freshness_error(&snapshot) {
                    return Json(error);
                }
                if snapshot.application.phase == AppPhase::Fatal {
                    return Json(ToolEnvelope::failure(
                        "ENGINE_FATAL",
                        "Scene state is unavailable after a fatal engine error.",
                        false,
                        Some(&snapshot.meta),
                    ));
                }
                if !matches!(
                    snapshot.application.phase,
                    AppPhase::Ready | AppPhase::Degraded
                ) {
                    return Json(not_ready(&snapshot));
                }
                let mut scene = snapshot.scene.clone();
                let max_bones = usize::from(args.max_bones);
                let truncated = scene.skeleton.bone_names.len() > max_bones;
                scene.skeleton.bone_names.truncate(max_bones);
                if !args.include_counts {
                    scene.model.mesh_count = 0;
                    scene.model.vertex_count = 0;
                    scene.model.triangle_count = 0;
                    scene.skeleton.bone_count = 0;
                    scene.animations.loaded_count = 0;
                    scene.object_count = 0;
                }
                Json(ToolEnvelope::success(
                    SceneInspectData { scene, truncated },
                    &snapshot.meta,
                ))
            }
            Err(error) => Json(error_envelope(error)),
        }
    }

    #[tool(
        name = "skeleton_get_catalog",
        description = "Read the current live rig and bone catalog from the native Three.js skeleton."
    )]
    pub async fn skeleton_get_catalog(
        &self,
        Parameters(_args): Parameters<RendererStateArgs>,
    ) -> Json<ToolEnvelope<LiveRigCatalogData>> {
        let response = self
            .authoring_call(AuthoringOperation::RigCatalogGet, serde_json::json!({}))
            .await
            .0;
        if !response.ok {
            return Json(ToolEnvelope {
                ok: false,
                data: None,
                error: response.error,
                meta: None,
            });
        }
        match response
            .data
            .and_then(|value| serde_json::from_value::<LiveRigCatalogData>(value).ok())
        {
            Some(data) if data.is_well_formed() => {
                let cached = CachedLiveCatalog {
                    identifier: data.rig.identifier.clone(),
                    fingerprint: data.rig.fingerprint.clone(),
                    bones: data
                        .bones
                        .iter()
                        .map(|bone| AnimationBone {
                            id: bone.id.clone(),
                            display_name: bone.display_name.clone(),
                            index: bone.index,
                            parent_id: bone.parent_id.clone(),
                        })
                        .collect(),
                };
                if let Ok(mut catalog) = self.live_catalog.lock() {
                    *catalog = Some(cached);
                }
                Json(ToolEnvelope {
                    ok: true,
                    data: Some(data),
                    error: None,
                    meta: None,
                })
            }
            _ => Json(ToolEnvelope {
                ok: false,
                data: None,
                error: Some(ToolError {
                    code: "INVALID_LIVE_CATALOG".to_owned(),
                    message: "The native application returned an invalid live rig catalog."
                        .to_owned(),
                    retryable: false,
                }),
                meta: None,
            }),
        }
    }

    #[tool(
        name = "animation_create_draft",
        description = "Create an empty draft for the current live rig; requires native authoring opt-in."
    )]
    pub async fn animation_create_draft(
        &self,
        Parameters(args): Parameters<CreateDraftArgs>,
    ) -> Json<ToolEnvelope<serde_json::Value>> {
        let catalog = match self.cached_live_catalog() {
            Ok(catalog) => catalog,
            Err(error) => return *error,
        };
        let document = args.document.clone().unwrap_or_else(|| AnimationDocument {
            schema_version: crate::ANIMATION_DOCUMENT_SCHEMA.to_owned(),
            rig: AnimationRig {
                identifier: catalog.identifier.clone(),
                fingerprint: catalog.fingerprint.clone(),
            },
            bones: catalog.bones.clone(),
            clip: AnimationClip {
                name: args
                    .name
                    .clone()
                    .unwrap_or_else(|| "Untitled Authored Clip".to_owned()),
                duration_seconds: args.duration_seconds.unwrap_or(2.0),
            },
            channels: Vec::new(),
            revision: 1,
            validation: AnimationValidation {
                state: ValidationState::Draft,
                errors: Vec::new(),
            },
        });
        if let Err(errors) = document.validate(&catalog.fingerprint, &catalog.bones) {
            return Json(ToolEnvelope::failure(
                "INVALID_DOCUMENT",
                &errors.join("; "),
                false,
                None,
            ));
        }
        if serde_json::to_vec(&document).map_or(true, |bytes| bytes.len() > MAX_DOCUMENT_BYTES) {
            return Json(ToolEnvelope::failure(
                "PAYLOAD_TOO_LARGE",
                "AnimationDocument exceeds the 256 KiB document ceiling",
                false,
                None,
            ));
        }
        if self.drafts.get(&self.authoring_session_id).is_ok() {
            return Json(ToolEnvelope::failure(
                "SESSION_CONFLICT",
                "an authoring draft already exists for this session",
                false,
                None,
            ));
        }
        if let Err(error) = self.drafts.create(
            &self.authoring_session_id,
            document,
            &catalog.fingerprint,
            &catalog.bones,
        ) {
            return Json(ToolEnvelope::failure(
                "INVALID_DOCUMENT",
                &format!("draft creation failed: {error:?}"),
                false,
                None,
            ));
        }
        let mut response = self
            .authoring_call(
                AuthoringOperation::DraftCreate,
                serde_json::to_value(args).unwrap_or_default(),
            )
            .await;
        if !response.0.ok {
            let _ = self.drafts.remove(&self.authoring_session_id);
        } else if let Some(data) = response.0.data.as_mut() {
            if let Some(object) = data.as_object_mut() {
                object.insert(
                    "draft_id".to_owned(),
                    Value::String(self.authoring_session_id.clone()),
                );
            }
        }
        response
    }

    #[tool(
        name = "animation_set_bone_keyframes",
        description = "Atomically replace one live-rig bone channel at an expected draft revision. HUMAN RIG POSE CONVENTIONS (validated on the live 66-bone rig): values are local quaternions composed as rest * delta, NOT absolute identity rotations. Arm raising/lowering uses the upperarm Z axis: upperarm_l.z positive raises the left arm, upperarm_r.z positive lowers the right arm. Elbow flexion uses lowerarm Z with a NEGATIVE sign for an anatomically correct forward bend: lowerarm_l.z about -60 with upperarm_l.z about 95 puts the forearm vertical and the hand beside the head; a POSITIVE lowerarm.z folds the forearm behind the head and contorts the hand. Wrist/palm orientation uses hand_l Y in a small range such as -30..+30; large elbow swings must NOT be used to wave. Validated greeting wave (2 s): upperarm_l.z=95, lowerarm_l.z=-60 constant, upperarm_r.z=82, lowerarm_r.z=8, hand_l.y alternating -30/+30 every 0.25 s. Always request animation_observe_pose front/right images AND joint projections, inspect the actual pixels, and confirm the elbow bends forward and the palm faces the intended camera before committing."
    )]
    pub async fn animation_set_bone_keyframes(
        &self,
        Parameters(args): Parameters<SetBoneKeyframesArgs>,
    ) -> Json<ToolEnvelope<serde_json::Value>> {
        let catalog = match self.cached_live_catalog() {
            Ok(catalog) => catalog,
            Err(error) => return *error,
        };
        let current = match self.drafts.get(&self.authoring_session_id) {
            Ok(document) => document,
            Err(_) => {
                return Json(ToolEnvelope::failure(
                    "INVALID_DOCUMENT",
                    "create or import a draft before setting keyframes",
                    false,
                    None,
                ));
            }
        };
        if current.revision != args.expected_revision {
            return Json(ToolEnvelope::failure(
                "BAD_REQUEST",
                "stale expected_revision",
                false,
                None,
            ));
        }
        let mut candidate = current.clone();
        let replacement = AnimationChannel {
            bone_id: args.bone_id.clone(),
            property: args.property,
            keyframes: args.keyframes.clone(),
        };
        if let Some(channel) = candidate
            .channels
            .iter_mut()
            .find(|channel| channel.bone_id == args.bone_id && channel.property == args.property)
        {
            channel.keyframes = args.keyframes.clone();
        } else {
            candidate.channels.push(replacement.clone());
        }
        let Some(next_revision) = args.expected_revision.checked_add(1) else {
            return Json(ToolEnvelope::failure(
                "BAD_REQUEST",
                "draft revision overflow",
                false,
                None,
            ));
        };
        candidate.revision = next_revision;
        if let Err(errors) = candidate.validate(&catalog.fingerprint, &catalog.bones) {
            return Json(ToolEnvelope::failure(
                "INVALID_DOCUMENT",
                &errors.join("; "),
                false,
                None,
            ));
        }
        if serde_json::to_vec(&candidate).map_or(true, |bytes| bytes.len() > MAX_DOCUMENT_BYTES) {
            return Json(ToolEnvelope::failure(
                "PAYLOAD_TOO_LARGE",
                "AnimationDocument exceeds the 256 KiB document ceiling",
                false,
                None,
            ));
        }
        let response = self
            .authoring_call(
                AuthoringOperation::DraftSetKeyframes,
                serde_json::to_value(&args).unwrap_or_default(),
            )
            .await;
        if response.0.ok {
            let current = self.drafts.get(&self.authoring_session_id);
            if current.is_ok_and(|document| document.revision == args.expected_revision)
                && self
                    .drafts
                    .set_bone_keyframes(
                        &self.authoring_session_id,
                        args.expected_revision,
                        replacement,
                        LiveRigCatalog {
                            fingerprint: &catalog.fingerprint,
                            bones: &catalog.bones,
                        },
                    )
                    .is_err()
            {
                return Json(ToolEnvelope::failure(
                    "INTERNAL",
                    "frontend accepted keyframes but Rust draft revision could not commit",
                    false,
                    None,
                ));
            }
        }
        response
    }

    #[tool(
        name = "animation_validate_draft",
        description = "Validate the current draft against the live rig and AnimationDocument v1 rules."
    )]
    pub async fn animation_validate_draft(
        &self,
        Parameters(args): Parameters<ValidateDraftArgs>,
    ) -> Json<ToolEnvelope<serde_json::Value>> {
        if let Err(error) = self.document_for_validation(args.document.clone()) {
            return *error;
        }
        self.authoring_call(
            AuthoringOperation::DraftValidate,
            serde_json::to_value(args).unwrap_or_default(),
        )
        .await
    }

    #[tool(
        name = "animation_preview_draft",
        description = "Start or stop the native viewport preview for the current draft."
    )]
    pub async fn animation_preview_draft(
        &self,
        Parameters(args): Parameters<PreviewDraftArgs>,
    ) -> Json<ToolEnvelope<serde_json::Value>> {
        let operation = match args.action {
            PreviewAction::Start => AuthoringOperation::DraftPreviewStart,
            PreviewAction::Stop => AuthoringOperation::DraftPreviewStop,
        };
        if matches!(args.action, PreviewAction::Start) {
            if let Err(error) = self.document_for_validation(args.document.clone()) {
                return *error;
            }
        }
        self.authoring_call(operation, serde_json::to_value(args).unwrap_or_default())
            .await
    }

    #[tool(
        name = "animation_commit_draft",
        description = "Commit a draft through the trusted native MCP path with authorization bound to the exact request and document digest."
    )]
    pub async fn animation_commit_draft(
        &self,
        Parameters(args): Parameters<CommitDraftArgs>,
    ) -> Json<ToolEnvelope<serde_json::Value>> {
        let (catalog, mut document) = match self.document_for_validation(args.document.clone()) {
            Ok(value) => value,
            Err(error) => return *error,
        };
        let current = match self.drafts.get(&self.authoring_session_id) {
            Ok(document) => document,
            Err(_) => {
                return Json(ToolEnvelope::failure(
                    "INVALID_DOCUMENT",
                    "create or import a draft before committing",
                    false,
                    None,
                ));
            }
        };
        if document.revision != current.revision {
            return Json(ToolEnvelope::failure(
                "BAD_REQUEST",
                "commit document revision does not match the Rust draft revision",
                false,
                None,
            ));
        }
        let Some(next_revision) = current.revision.checked_add(1) else {
            return Json(ToolEnvelope::failure(
                "BAD_REQUEST",
                "draft revision overflow",
                false,
                None,
            ));
        };
        document.revision = next_revision;
        let response = self
            .authoring_call(
                AuthoringOperation::DraftCommit,
                serde_json::to_value(args).unwrap_or_default(),
            )
            .await;
        if response.0.ok
            && self
                .drafts
                .commit(
                    &self.authoring_session_id,
                    current.revision,
                    document,
                    &catalog.fingerprint,
                    &catalog.bones,
                )
                .is_err()
        {
            return Json(ToolEnvelope::failure(
                "INTERNAL",
                "frontend committed the clip but Rust draft history could not advance",
                false,
                None,
            ));
        }
        response
    }

    #[tool(
        name = "animation_cancel_draft",
        description = "Cancel and discard the current native animation draft."
    )]
    pub async fn animation_cancel_draft(
        &self,
        Parameters(_args): Parameters<EmptyArgs>,
    ) -> Json<ToolEnvelope<serde_json::Value>> {
        let response = self
            .authoring_call(AuthoringOperation::DraftCancel, serde_json::json!({}))
            .await;
        if response.0.ok {
            let _ = self.drafts.remove(&self.authoring_session_id);
        }
        response
    }

    #[tool(
        name = "application_set_page",
        description = "Switch the visible native app page so the user can observe and correct an MCP draft: page='new-animation' opens the New Animation editor, page='explore' returns to the library. Use this before authoring so the user sees every change before it is saved. No filesystem path or script is accepted."
    )]
    pub async fn application_set_page(
        &self,
        Parameters(args): Parameters<PageArgs>,
    ) -> Json<ToolEnvelope<serde_json::Value>> {
        if !matches!(args.page.as_str(), "new-animation" | "explore") {
            return Json(ToolEnvelope::failure(
                "BAD_REQUEST",
                "page must be 'new-animation' or 'explore'",
                false,
                None,
            ));
        }
        self.authoring_call(
            AuthoringOperation::ApplicationSetPage,
            serde_json::json!({"page": args.page}),
        )
        .await
    }

    #[tool(
        name = "animation_undo_commit",
        description = "Undo an authored animation revision after digest-bound native user approval."
    )]
    pub async fn animation_undo_commit(
        &self,
        Parameters(args): Parameters<UndoCommitArgs>,
    ) -> Json<ToolEnvelope<serde_json::Value>> {
        let (catalog, current) = match self.document_for_validation(None) {
            Ok(value) => value,
            Err(error) => return *error,
        };
        if current.revision == 0 || args.target_revision >= current.revision {
            return Json(ToolEnvelope::failure(
                "BAD_REQUEST",
                "target_revision must identify an earlier retained revision",
                false,
                None,
            ));
        }
        let target_revision = args.target_revision;
        let response = self
            .authoring_call(
                AuthoringOperation::DraftUndo,
                serde_json::to_value(args).unwrap_or_default(),
            )
            .await;
        if response.0.ok
            && self
                .drafts
                .undo(
                    &self.authoring_session_id,
                    current.revision,
                    target_revision,
                    &catalog.fingerprint,
                    &catalog.bones,
                )
                .is_err()
        {
            return Json(ToolEnvelope::failure(
                "INTERNAL",
                "frontend committed undo but Rust draft history could not restore the revision",
                false,
                None,
            ));
        }
        response
    }

    #[tool(
        name = "animation_export_draft",
        description = "Export the current authored draft as AnimationDocument v1."
    )]
    pub async fn animation_export_draft(
        &self,
        Parameters(_args): Parameters<EmptyArgs>,
    ) -> Json<ToolEnvelope<serde_json::Value>> {
        self.authoring_call(AuthoringOperation::DraftExport, serde_json::json!({}))
            .await
    }

    #[tool(
        name = "animation_import_draft",
        description = "Import and validate an AnimationDocument v1 draft for the current live rig."
    )]
    pub async fn animation_import_draft(
        &self,
        Parameters(args): Parameters<ImportDraftArgs>,
    ) -> Json<ToolEnvelope<serde_json::Value>> {
        let catalog = match self.cached_live_catalog() {
            Ok(catalog) => catalog,
            Err(error) => return *error,
        };
        if let Err(errors) = args.document.validate(&catalog.fingerprint, &catalog.bones) {
            return Json(ToolEnvelope::failure(
                "INVALID_DOCUMENT",
                &errors.join("; "),
                false,
                None,
            ));
        }
        if serde_json::to_vec(&args.document).map_or(true, |bytes| bytes.len() > MAX_DOCUMENT_BYTES)
        {
            return Json(ToolEnvelope::failure(
                "PAYLOAD_TOO_LARGE",
                "AnimationDocument exceeds the 256 KiB document ceiling",
                false,
                None,
            ));
        }
        if self.drafts.get(&self.authoring_session_id).is_ok() {
            return Json(ToolEnvelope::failure(
                "SESSION_CONFLICT",
                "cancel the current draft before importing another document",
                false,
                None,
            ));
        }
        let document = args.document.clone();
        let response = self
            .authoring_call(
                AuthoringOperation::DraftImport,
                serde_json::to_value(args).unwrap_or_default(),
            )
            .await;
        if response.0.ok {
            // The frontend remains the application owner; Rust retains a
            // validated mirror for subsequent expected-revision operations.
            if self
                .drafts
                .create(
                    &self.authoring_session_id,
                    document,
                    &catalog.fingerprint,
                    &catalog.bones,
                )
                .is_err()
            {
                return Json(ToolEnvelope::failure(
                    "INTERNAL",
                    "frontend imported a document but Rust draft mirror could not be created",
                    false,
                    None,
                ));
            }
        }
        response
    }

    #[tool(
        name = "animation_observe_pose",
        description = "Capture one synchronized pose from 1..6 hidden model cameras and return live joint projection metadata. Requires the draft_id returned by animation_create_draft. Cameras are WORLD-aligned, not rig-forward aligned: 'front' looks from world -Z toward +Z, 'back' from +Z toward -Z, 'left' from +X, 'right' from -X, 'top' from +Y, 'bottom' from -Y. On the Human rig the model faces +Z, so the 'front' view can show the model's back; use 'back' or 'right' to judge a palm that must face the default viewport camera at +Z. Images are real PNG/JPEG MCP image blocks; image_ref chunk bytes are byte-identical to the inline capture. Compare projected joints with the pixels before accepting a pose; do not infer bone motion from a single view."
    )]
    pub async fn animation_observe_pose(
        &self,
        Parameters(args): Parameters<AnimationObservePoseArgs>,
    ) -> CallToolResult {
        self.observe_pose(args).await
    }

    #[tool(
        name = "animation_capture_view",
        description = "Return an actual retained camera image as MCP image content with matching camera matrices, joint projections, rig fingerprint, revision and timestamp."
    )]
    pub async fn animation_capture_view(
        &self,
        Parameters(args): Parameters<AnimationCaptureViewArgs>,
    ) -> CallToolResult {
        self.capture_view(args).await
    }

    #[tool(
        name = "animation_session_status",
        description = "Read the current authoring session, live rig fingerprint, preview and permission state."
    )]
    pub async fn animation_session_status(
        &self,
        Parameters(_args): Parameters<EmptyArgs>,
    ) -> Json<ToolEnvelope<serde_json::Value>> {
        self.authoring_call(AuthoringOperation::SessionStatus, serde_json::json!({}))
            .await
    }

    #[tool(
        name = "animation_cancel_session",
        description = "Cancel the authoring session and stop preview, confirmation and draft state."
    )]
    pub async fn animation_cancel_session(
        &self,
        Parameters(_args): Parameters<EmptyArgs>,
    ) -> Json<ToolEnvelope<serde_json::Value>> {
        let response = self
            .authoring_call(AuthoringOperation::SessionCancel, serde_json::json!({}))
            .await;
        self.clear_session_state_if_cancelled(response.0.ok);
        response
    }

    fn clear_session_state_if_cancelled(&self, cancelled: bool) {
        if !cancelled {
            return;
        }
        let _ = self.drafts.remove(&self.authoring_session_id);
        if let Ok(mut captures) = self.observation_captures.lock() {
            captures.clear();
        }
    }
}

fn encode_base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let a = chunk[0];
        let b = *chunk.get(1).unwrap_or(&0);
        let c = *chunk.get(2).unwrap_or(&0);
        out.push(TABLE[(a >> 2) as usize] as char);
        out.push(TABLE[(((a & 3) << 4) | (b >> 4)) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[(((b & 15) << 2) | (c >> 6)) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(c & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

fn decode_base64(input: &str) -> Option<Vec<u8>> {
    if !input.len().is_multiple_of(4) {
        return None;
    }
    let mut output = Vec::with_capacity(input.len() / 4 * 3);
    for (index, chunk) in input.as_bytes().chunks(4).enumerate() {
        let last = index == input.len() / 4 - 1;
        let value = |byte| match byte {
            b'A'..=b'Z' => Some(byte - b'A'),
            b'a'..=b'z' => Some(byte - b'a' + 26),
            b'0'..=b'9' => Some(byte - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        };
        let a = value(chunk[0])?;
        let b = value(chunk[1])?;
        output.push((a << 2) | (b >> 4));
        if chunk[2] == b'=' {
            if !last || chunk[3] != b'=' || b & 15 != 0 {
                return None;
            }
            continue;
        }
        let c = value(chunk[2])?;
        output.push((b << 4) | (c >> 2));
        if chunk[3] == b'=' {
            if !last || c & 3 != 0 {
                return None;
            }
            continue;
        }
        let d = value(chunk[3])?;
        output.push((c << 6) | d);
    }
    Some(output)
}

fn valid_encoded_image(bytes: &[u8], mime: &str) -> bool {
    match mime {
        "image/png" => {
            bytes.starts_with(b"\x89PNG\r\n\x1a\n")
                && bytes.len() >= 45
                && bytes.get(12..16) == Some(b"IHDR")
                && bytes.ends_with(b"\0\0\0\0IEND\xaeB`\x82")
        }
        "image/jpeg" => {
            bytes.starts_with(&[0xff, 0xd8]) && bytes.ends_with(&[0xff, 0xd9]) && bytes.len() >= 4
        }
        _ => false,
    }
}

fn encoded_image_dimensions(bytes: &[u8], mime: &str) -> Option<(u32, u32)> {
    match mime {
        "image/png" if valid_encoded_image(bytes, mime) => {
            let width = u32::from_be_bytes(bytes.get(16..20)?.try_into().ok()?);
            let height = u32::from_be_bytes(bytes.get(20..24)?.try_into().ok()?);
            (width > 0 && height > 0).then_some((width, height))
        }
        "image/jpeg" if valid_encoded_image(bytes, mime) => {
            let mut offset = 2usize;
            while offset + 4 <= bytes.len() - 2 {
                if bytes[offset] != 0xff {
                    return None;
                }
                let marker = bytes[offset + 1];
                offset += 2;
                if marker == 0xd9 || marker == 0xda {
                    return None;
                }
                if matches!(marker, 0x01 | 0xd0..=0xd7) {
                    continue;
                }
                let segment_len =
                    u16::from_be_bytes([*bytes.get(offset)?, *bytes.get(offset + 1)?]) as usize;
                if segment_len < 2 || offset.checked_add(segment_len)? > bytes.len() {
                    return None;
                }
                if matches!(
                    marker,
                    0xc0 | 0xc1
                        | 0xc2
                        | 0xc3
                        | 0xc5
                        | 0xc6
                        | 0xc7
                        | 0xc9
                        | 0xca
                        | 0xcb
                        | 0xcd
                        | 0xce
                        | 0xcf
                ) {
                    if segment_len < 7 {
                        return None;
                    }
                    let height = u16::from_be_bytes([bytes[offset + 3], bytes[offset + 4]]) as u32;
                    let width = u16::from_be_bytes([bytes[offset + 5], bytes[offset + 6]]) as u32;
                    return (width > 0 && height > 0).then_some((width, height));
                }
                offset += segment_len;
            }
            None
        }
        _ => None,
    }
}

#[cfg(test)]
mod image_transport_tests {
    use super::*;

    #[test]
    fn base64_transport_round_trips_and_rejects_malformed_padding() {
        for bytes in [vec![1], vec![1, 2], vec![1, 2, 3], (0..=255).collect()] {
            assert_eq!(decode_base64(&encode_base64(&bytes)), Some(bytes));
        }
        assert_eq!(decode_base64("AQ=Z"), None);
        assert_eq!(decode_base64("AQ==AAAA"), None);
        assert_eq!(decode_base64("A==="), None);
    }

    #[test]
    fn observation_request_ids_are_unique_across_sidecar_sessions() {
        let first = pose_request_id("mcp-100-123456", 1);
        let second = pose_request_id("mcp-101-123456", 1);
        assert_ne!(first, second);
        assert!(first.len() <= crate::MAX_POSE_ID_BYTES);
        assert_ne!(
            pose_dispose_request_id("mcp-100-123456", 1, 0),
            pose_dispose_request_id("mcp-101-123456", 1, 0)
        );
    }

    #[test]
    fn image_magic_must_match_the_claimed_mime_type() {
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend_from_slice(&[
            0, 0, 0, 13, b'I', b'H', b'D', b'R', 0, 0, 2, 0, 0, 0, 2, 0, 8, 6, 0, 0, 0, 0, 0, 0, 0,
        ]);
        png.extend_from_slice(b"\0\0\0\0IEND\xaeB`\x82");
        assert!(valid_encoded_image(&png, "image/png"));
        assert_eq!(
            encoded_image_dimensions(&png, "image/png"),
            Some((512, 512))
        );
        assert!(!valid_encoded_image(&png, "image/jpeg"));
        assert!(valid_encoded_image(
            &[0xff, 0xd8, 0x01, 0xff, 0xd9],
            "image/jpeg"
        ));
        assert!(!valid_encoded_image(&[0xff, 0xd8, 0x01], "image/jpeg"));
    }
}

fn freshness_error<T>(snapshot: &ControlSnapshot) -> Option<ToolEnvelope<T>> {
    (snapshot.meta.age_ms > 5_000).then(|| {
        ToolEnvelope::failure(
            "STALE_STATE",
            "The latest application snapshot is older than five seconds.",
            true,
            Some(&snapshot.meta),
        )
    })
}

fn not_ready<T>(snapshot: &ControlSnapshot) -> ToolEnvelope<T> {
    ToolEnvelope::failure(
        "NOT_READY",
        "This tool is not available in the current application phase.",
        true,
        Some(&snapshot.meta),
    )
}

fn error_envelope<T>(error: IpcError) -> ToolEnvelope<T> {
    match error {
        IpcError::Unavailable | IpcError::AppUnavailable => ToolEnvelope::app_unavailable(),
        IpcError::NotReady => ToolEnvelope::failure(
            "NOT_READY",
            "The application has not published a ready control snapshot.",
            true,
            None,
        ),
        IpcError::Disconnected => ToolEnvelope::failure(
            "APP_DISCONNECTED",
            "Mesh2Motion disconnected during the local control request.",
            true,
            None,
        ),
        IpcError::IncompatibleVersion => ToolEnvelope::failure(
            "INCOMPATIBLE_VERSION",
            "The application and MCP sidecar use incompatible control protocol versions.",
            false,
            None,
        ),
        IpcError::PermissionDenied => ToolEnvelope::failure(
            "PERMISSION_DENIED",
            "The local control peer did not pass same-user verification.",
            false,
            None,
        ),
        IpcError::AuthoringDisabled => ToolEnvelope::failure(
            "AUTHORING_DISABLED",
            "Animation authoring is disabled in the native application.",
            false,
            None,
        ),
        IpcError::DeadlineExceeded => ToolEnvelope::failure(
            "DEADLINE_EXCEEDED",
            "The native application did not answer before the request deadline.",
            true,
            None,
        ),
        IpcError::InvalidResponse => ToolEnvelope::ipc_error(),
    }
}
