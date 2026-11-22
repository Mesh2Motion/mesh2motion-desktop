use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SnapshotMeta {
    pub schema_version: String,
    pub sequence: u64,
    pub captured_at: String,
    /// Unix epoch milliseconds from the Tauri publisher; used only to compute
    /// freshness on the server and omitted from MCP tool outputs.
    pub captured_at_unix_ms: u64,
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
    Unknown,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplicationInfo {
    pub phase: AppPhase,
    pub page: AppPage,
    pub version: String,
    pub build: String,
    pub uptime_ms: u64,
    pub diagnostics_enabled: bool,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderLoopInfo {
    pub active: bool,
    pub stopped: bool,
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
    pub context_lost: bool,
    pub loop_state: RenderLoopInfo,
    pub canvas: CanvasInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gl: Option<GlInfo>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RigInfo {
    pub key: String,
    pub display_name: String,
    /// A bundle-relative asset key only; never an absolute filesystem path.
    pub model_asset: String,
    pub rig_asset: String,
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
    pub duration_s: f32,
    pub track_count: u32,
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
    pub mesh_count: u32,
    pub vertex_count: u64,
    pub triangle_count: u64,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkeletonSummary {
    pub loaded: bool,
    pub skeleton_type: String,
    pub bone_count: u32,
    pub bone_names: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_name: Option<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneAnimationSummary {
    pub loaded_count: u32,
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
    pub process_step: String,
    pub model: ModelSummary,
    pub skeleton: SkeletonSummary,
    pub animations: SceneAnimationSummary,
    pub camera: CameraSummary,
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
        if self.meta.captured_at.is_empty() || self.meta.captured_at.len() > 64 {
            return Err("invalid snapshot timestamp");
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
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .min(u128::from(u64::MAX)) as u64;
        self.meta.age_ms = now_ms.saturating_sub(self.meta.captured_at_unix_ms);
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
