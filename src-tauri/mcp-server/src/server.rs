use std::sync::Arc;

use rmcp::{
    handler::server::wrapper::{Json, Parameters},
    tool, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    AnimationInfo, AnimationOrigin, AppPhase, AppSnapshotSource, ControlSnapshot, IpcError,
    RendererInfo, RigInfo, SceneInfo, SnapshotInfo, ToolEnvelope,
};

#[derive(Clone)]
pub struct Mesh2MotionMcp {
    source: Arc<dyn AppSnapshotSource>,
}

impl Mesh2MotionMcp {
    pub fn new(source: Arc<dyn AppSnapshotSource>) -> Self {
        Self { source }
    }

    async fn read_snapshot(&self) -> Result<ControlSnapshot, IpcError> {
        self.source.snapshot().await
    }
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
        match self.read_snapshot().await {
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
        match self.read_snapshot().await {
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
        match self.read_snapshot().await {
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
        match self.read_snapshot().await {
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
        match self.read_snapshot().await {
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
        IpcError::Unavailable => ToolEnvelope::app_unavailable(),
        IpcError::NotReady => ToolEnvelope::failure(
            "NOT_READY",
            "The application has not published a ready control snapshot.",
            true,
            None,
        ),
        IpcError::InvalidResponse => ToolEnvelope::ipc_error(),
    }
}
