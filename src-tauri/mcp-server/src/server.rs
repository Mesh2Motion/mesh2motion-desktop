use std::sync::Arc;

use rmcp::{
    handler::server::wrapper::{Json, Parameters},
    tool, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    AnimationInfo, AnimationOrigin, AppPhase, AppSnapshotSource, AuthoringOperation,
    AuthoringRequest, ControlSnapshot, IpcError, RendererInfo, RigInfo, SceneInfo, SnapshotInfo,
    ToolEnvelope, ToolError, AUTHORING_BRIDGE_PROTOCOL,
};

#[derive(Clone)]
pub struct Mesh2MotionMcp {
    source: Arc<dyn AppSnapshotSource>,
}

impl Mesh2MotionMcp {
    pub fn new(source: Arc<dyn AppSnapshotSource>) -> Self {
        Self { source }
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

    async fn read_live_catalog(&self) -> Result<serde_json::Value, IpcError> {
        let session_id = format!(
            "catalog-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let request = AuthoringRequest {
            protocol: AUTHORING_BRIDGE_PROTOCOL.to_owned(),
            request_id: "catalog-request".to_owned(),
            session_id,
            sequence: 1,
            deadline_ms: 5_000,
            operation: AuthoringOperation::RigCatalogGet,
            payload: serde_json::json!({}),
        };
        let response = self.source.authoring_request(request).await?;
        if response.ok {
            response.result.ok_or(IpcError::InvalidResponse)
        } else {
            match response.error.map(|error| error.code) {
                Some(crate::AuthoringErrorCode::AuthoringDisabled) => {
                    Err(IpcError::AuthoringDisabled)
                }
                Some(crate::AuthoringErrorCode::DeadlineExceeded) => {
                    Err(IpcError::DeadlineExceeded)
                }
                Some(crate::AuthoringErrorCode::AppDisconnected) => Err(IpcError::Disconnected),
                _ => Err(IpcError::InvalidResponse),
            }
        }
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
        match self.read_live_catalog().await {
            Ok(value) => match serde_json::from_value::<LiveRigCatalogData>(value) {
                Ok(data) if data.is_well_formed() => Json(ToolEnvelope {
                    ok: true,
                    data: Some(data),
                    error: None,
                    meta: None,
                }),
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
            },
            Err(error) => Json(error_envelope(error)),
        }
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
