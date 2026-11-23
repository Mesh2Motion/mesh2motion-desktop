use std::sync::{Arc, Mutex};

use rmcp::{
    handler::server::wrapper::{Json, Parameters},
    tool, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    AnimationBone, AnimationChannel, AnimationClip, AnimationDocument, AnimationDraftStore,
    AnimationInfo, AnimationKeyframe, AnimationOrigin, AnimationProperty, AnimationRig,
    AnimationValidation, AppPhase, AppSnapshotSource, AuthoringErrorCode, AuthoringOperation,
    AuthoringRequest, ControlSnapshot, IpcError, LiveRigCatalog, RendererInfo, RigInfo, SceneInfo,
    SnapshotInfo, ToolEnvelope, ToolError, ValidationState, AUTHORING_BRIDGE_PROTOCOL,
    MAX_DOCUMENT_BYTES,
};

#[derive(Clone)]
pub struct Mesh2MotionMcp {
    source: Arc<dyn AppSnapshotSource>,
    authoring_session_id: String,
    drafts: AnimationDraftStore,
    live_catalog: Arc<Mutex<Option<CachedLiveCatalog>>>,
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
        let request = AuthoringRequest {
            protocol: AUTHORING_BRIDGE_PROTOCOL.to_owned(),
            request_id: "mcp-request".to_owned(),
            session_id: self.authoring_session_id.clone(),
            sequence: 1,
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
        let response = self
            .authoring_call(
                AuthoringOperation::DraftCreate,
                serde_json::to_value(args).unwrap_or_default(),
            )
            .await;
        if !response.0.ok {
            let _ = self.drafts.remove(&self.authoring_session_id);
        }
        response
    }

    #[tool(
        name = "animation_set_bone_keyframes",
        description = "Atomically replace one live-rig bone channel at an expected draft revision."
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
        if response.0.ok {
            let _ = self.drafts.remove(&self.authoring_session_id);
        }
        response
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
