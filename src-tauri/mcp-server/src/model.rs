use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SnapshotMeta {
    #[serde(alias = "schema_version")]
    pub schema_version: String,
    pub sequence: u64,
    #[serde(alias = "captured_at")]
    pub captured_at: String,
    /// Supplied age is ignored by the host and recomputed from `captured_at`.
    #[serde(alias = "age_ms")]
    #[serde(default)]
    pub age_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AppPhase {
    Uninitialized,
    Loading,
    Ready,
    Degraded,
    Fatal,
    Closed,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AppPage {
    Explore,
    Create,
    Retarget,
    PreviewGenerator,
    Remix,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplicationInfo {
    pub phase: AppPhase,
    pub page: AppPage,
    pub version: String,
    pub build: String,
    #[serde(alias = "uptime_ms")]
    pub uptime_ms: u64,
    #[serde(alias = "diagnostics_enabled")]
    pub diagnostics_enabled: bool,
    #[serde(alias = "last_error", default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderLoopInfo {
    pub active: bool,
    pub stopped: bool,
    #[serde(alias = "consecutive_failures")]
    pub consecutive_failures: u32,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CanvasInfo {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Default, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GlInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub renderer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RendererInfo {
    pub created: bool,
    #[serde(alias = "context_lost")]
    pub context_lost: bool,
    #[serde(alias = "loop")]
    pub loop_state: RenderLoopInfo,
    pub canvas: CanvasInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gl: Option<GlInfo>,
    #[serde(alias = "last_error", default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RigInfo {
    pub key: String,
    #[serde(alias = "display_name")]
    pub display_name: String,
    /// A bundle-relative asset key only; never an absolute filesystem path.
    #[serde(alias = "model_asset")]
    pub model_asset: String,
    #[serde(alias = "rig_asset")]
    pub rig_asset: String,
    #[serde(alias = "animation_assets")]
    pub animation_assets: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AnimationSource {
    DefaultLibrary,
    CustomImport,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AnimationOrigin {
    Loaded,
    Manifest,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnimationInfo {
    pub index: u32,
    pub name: String,
    #[serde(alias = "duration_s")]
    pub duration_s: f32,
    #[serde(alias = "track_count")]
    pub track_count: u32,
    #[serde(alias = "source_type")]
    pub source_type: AnimationSource,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rig: Option<String>,
    pub origin: AnimationOrigin,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelSummary {
    pub loaded: bool,
    #[serde(alias = "mesh_count")]
    pub mesh_count: u32,
    #[serde(alias = "vertex_count")]
    pub vertex_count: u64,
    #[serde(alias = "triangle_count")]
    pub triangle_count: u64,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkeletonSummary {
    pub loaded: bool,
    #[serde(alias = "type")]
    pub skeleton_type: String,
    #[serde(alias = "bone_count")]
    pub bone_count: u32,
    #[serde(alias = "bone_names", default)]
    pub bone_names: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
    #[serde(alias = "root_name", skip_serializing_if = "Option::is_none")]
    pub root_name: Option<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneAnimationSummary {
    #[serde(alias = "loaded_count")]
    pub loaded_count: u32,
    #[serde(alias = "active_index")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_index: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraSummary {
    pub position: [f32; 3],
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneInfo {
    #[serde(alias = "process_step")]
    pub process_step: String,
    pub model: ModelSummary,
    pub skeleton: SkeletonSummary,
    pub animations: SceneAnimationSummary,
    pub camera: CameraSummary,
    #[serde(alias = "object_count")]
    pub object_count: u32,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ControlSnapshot {
    pub meta: SnapshotMeta,
    pub application: ApplicationInfo,
    pub renderer: RendererInfo,
    pub rigs: Vec<RigInfo>,
    pub animations: Vec<AnimationInfo>,
    pub scene: SceneInfo,
}

impl ControlSnapshot {
    /// Reject unsafe asset identifiers before an IPC snapshot reaches a tool.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.meta.schema_version != "1.0" {
            return Err("incompatible snapshot schema version");
        }
        if self.meta.sequence == 0 || self.meta.sequence > 9_007_199_254_740_991 {
            return Err("invalid snapshot sequence");
        }
        if self.meta.captured_at.len() > 64 {
            return Err("invalid snapshot timestamp");
        }
        let captured = chrono::DateTime::parse_from_rfc3339(&self.meta.captured_at)
            .map_err(|_| "invalid snapshot timestamp")?;
        let now = chrono::Utc::now();
        let age = now.signed_duration_since(captured);
        if age.num_seconds() < -30 {
            return Err("snapshot timestamp is in the future");
        }
        if self.application.version.len() > 128
            || self.application.build.len() > 256
            || self
                .application
                .last_error
                .as_ref()
                .is_some_and(|s| s.len() > 2048)
            || self
                .renderer
                .last_error
                .as_ref()
                .is_some_and(|s| s.len() > 2048)
            || self.rigs.len() > 256
            || self.animations.len() > 4096
        {
            return Err("snapshot exceeds a field limit");
        }
        if self.animations.iter().any(|a| {
            !a.duration_s.is_finite()
                || a.duration_s < 0.0
                || a.name.len() > 512
                || a.tags.len() > 128
                || a.tags.iter().any(|t| t.len() > 128)
        }) {
            return Err("invalid animation entry");
        }
        if self.scene.process_step.len() > 256
            || !self.scene.camera.position.iter().all(|v| v.is_finite())
        {
            return Err("invalid scene values");
        }
        for rig in &self.rigs {
            if !is_bundle_asset_key(&rig.model_asset) || !is_bundle_asset_key(&rig.rig_asset) {
                return Err("invalid rig asset key");
            }
            if rig
                .animation_assets
                .iter()
                .any(|asset| !is_bundle_asset_key(asset))
            {
                return Err("invalid animation asset key");
            }
        }
        Ok(())
    }

    /// Freshness is computed by the sidecar, never trusted from IPC input.
    pub fn refresh_age(&mut self) {
        let Ok(captured) = chrono::DateTime::parse_from_rfc3339(&self.meta.captured_at) else {
            self.meta.age_ms = u64::MAX;
            return;
        };
        self.meta.age_ms = chrono::Utc::now()
            .signed_duration_since(captured)
            .num_milliseconds()
            .max(0) as u64;
    }
}

fn is_bundle_asset_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && !value.starts_with('/')
        && !value.starts_with('\\')
        && !value.contains('\\')
        && !value.contains(':')
        && !value.contains('\0')
        && value
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SnapshotInfo {
    pub sequence: u64,
    pub captured_at: String,
    pub age_ms: u64,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ToolError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ToolEnvelope<T> {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ToolError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<SnapshotInfo>,
}

impl<T> ToolEnvelope<T> {
    pub fn success(data: T, meta: &SnapshotMeta) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
            meta: Some(SnapshotInfo {
                sequence: meta.sequence,
                captured_at: meta.captured_at.clone(),
                age_ms: meta.age_ms,
            }),
        }
    }

    pub fn app_unavailable() -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(ToolError {
                code: "APP_UNAVAILABLE".to_owned(),
                message: "Mesh2Motion has not published a control snapshot.".to_owned(),
                retryable: true,
            }),
            meta: None,
        }
    }

    pub fn ipc_error() -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(ToolError {
                code: "IPC_ERROR".to_owned(),
                message: "The Mesh2Motion local control channel returned invalid data.".to_owned(),
                retryable: true,
            }),
            meta: None,
        }
    }

    pub fn failure(
        code: &str,
        message: &str,
        retryable: bool,
        meta: Option<&SnapshotMeta>,
    ) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(ToolError {
                code: code.to_owned(),
                message: message.to_owned(),
                retryable,
            }),
            meta: meta.map(|meta| SnapshotInfo {
                sequence: meta.sequence,
                captured_at: meta.captured_at.clone(),
                age_ms: meta.age_ms,
            }),
        }
    }
}
