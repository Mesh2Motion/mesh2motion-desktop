use std::sync::Arc;

use rmcp::handler::server::wrapper::Parameters;

use crate::{
    server::{AnimationsListArgs, ApplicationStatusArgs, RendererStateArgs, SceneInspectArgs},
    AnimationInfo, AnimationOrigin, AnimationSource, AppPage, AppPhase, AppSnapshotSource,
    ApplicationInfo, CameraSummary, CanvasInfo, ControlSnapshot, Mesh2MotionMcp, ModelSummary,
    RenderLoopInfo, RendererInfo, RigInfo, SceneAnimationSummary, SceneInfo, SkeletonSummary,
    SnapshotMeta,
};

#[derive(Clone)]
struct MockSnapshotSource(ControlSnapshot);

impl AppSnapshotSource for MockSnapshotSource {
    fn snapshot(&self) -> crate::ipc::SnapshotFuture<'_> {
        let snapshot = self.0.clone();
        Box::pin(async move { Ok(snapshot) })
    }
}

pub(crate) fn mock_snapshot() -> ControlSnapshot {
    ControlSnapshot {
        meta: SnapshotMeta {
            schema_version: "1.0".to_owned(),
            sequence: 9,
            captured_at: chrono::Utc::now().to_rfc3339(),
            age_ms: 42,
        },
        application: ApplicationInfo {
            phase: AppPhase::Ready,
            page: AppPage::Explore,
            version: "0.1.10".to_owned(),
            build: "test-fixture".to_owned(),
            uptime_ms: 5_000,
            diagnostics_enabled: false,
            last_error: None,
        },
        renderer: RendererInfo {
            created: true,
            context_lost: false,
            loop_state: RenderLoopInfo {
                active: true,
                stopped: false,
                consecutive_failures: 0,
            },
            canvas: CanvasInfo {
                width: 640,
                height: 480,
            },
            gl: None,
            last_error: None,
        },
        rigs: vec![RigInfo {
            key: "human".to_owned(),
            display_name: "Human".to_owned(),
            model_asset: "models/model-human.glb".to_owned(),
            rig_asset: "rigs/rig-human.glb".to_owned(),
            animation_assets: vec!["animations/human-base-animations.glb".to_owned()],
        }],
        animations: vec![
            AnimationInfo {
                index: 0,
                name: "Walk".to_owned(),
                duration_s: 1.2,
                track_count: 3,
                source_type: AnimationSource::DefaultLibrary,
                tags: vec!["locomotion".to_owned()],
                rig: Some("human".to_owned()),
                origin: AnimationOrigin::Loaded,
            },
            AnimationInfo {
                index: 1,
                name: "Private Import".to_owned(),
                duration_s: 2.0,
                track_count: 2,
                source_type: AnimationSource::CustomImport,
                tags: Vec::new(),
                rig: Some("human".to_owned()),
                origin: AnimationOrigin::Loaded,
            },
            AnimationInfo {
                index: 2,
                name: "Configured Clip".to_owned(),
                duration_s: 0.0,
                track_count: 0,
                source_type: AnimationSource::DefaultLibrary,
                tags: Vec::new(),
                rig: Some("human".to_owned()),
                origin: AnimationOrigin::Manifest,
            },
        ],
        scene: SceneInfo {
            process_step: "load-model".to_owned(),
            model: ModelSummary {
                loaded: true,
                mesh_count: 1,
                vertex_count: 100,
                triangle_count: 50,
            },
            skeleton: SkeletonSummary {
                loaded: true,
                skeleton_type: "human".to_owned(),
                bone_count: 3,
                truncated: None,
                bone_names: vec!["root".to_owned(), "hip".to_owned(), "knee".to_owned()],
                root_name: Some("root".to_owned()),
            },
            animations: SceneAnimationSummary {
                loaded_count: 1,
                active_index: Some(0),
            },
            camera: CameraSummary {
                position: [0.0, 1.0, 5.0],
            },
            object_count: 2,
        },
    }
}

fn server() -> Mesh2MotionMcp {
    server_with_snapshot(mock_snapshot())
}

fn server_with_snapshot(snapshot: ControlSnapshot) -> Mesh2MotionMcp {
    Mesh2MotionMcp::new(Arc::new(MockSnapshotSource(snapshot)))
}

#[tokio::test]
async fn status_and_renderer_return_only_mock_protocol_state() {
    let app = server();
    let status = app
        .application_get_status(Parameters(ApplicationStatusArgs::default()))
        .await
        .0;
    assert!(status.ok);
    let status_json = serde_json::to_value(status).unwrap();
    assert_eq!(status_json["data"]["version"], "0.1.10");
    assert_eq!(status_json["data"]["snapshot"]["sequence"], 9);

    let renderer = app
        .renderer_get_state(Parameters(RendererStateArgs::default()))
        .await
        .0;
    assert!(renderer.ok);
    let renderer_json = serde_json::to_value(renderer).unwrap();
    assert!(renderer_json["data"]["gl"].is_null());
}

#[tokio::test]
async fn animation_filters_exclude_custom_and_manifest_by_default() {
    let response = server()
        .animations_list(Parameters(AnimationsListArgs::default()))
        .await
        .0;
    assert!(response.ok);
    let json = serde_json::to_value(response).unwrap();
    assert_eq!(json["data"]["count"], 1);
    assert_eq!(json["data"]["items"][0]["name"], "Walk");
}

#[tokio::test]
async fn scene_rejects_out_of_range_bone_limit_and_caps_valid_results() {
    let bad = server()
        .scene_inspect(Parameters(SceneInspectArgs {
            include_counts: true,
            max_bones: 0,
        }))
        .await
        .0;
    assert_eq!(bad.error.unwrap().code, "INVALID_PARAMS");

    let capped = server()
        .scene_inspect(Parameters(SceneInspectArgs {
            include_counts: true,
            max_bones: 2,
        }))
        .await
        .0;
    assert!(capped.ok);
    let json = serde_json::to_value(capped).unwrap();
    assert_eq!(
        json["data"]["scene"]["skeleton"]["boneNames"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(json["data"]["truncated"], true);
}

#[tokio::test]
async fn absolute_asset_paths_are_denied() {
    let response = server()
        .rigs_list(Parameters(crate::server::RigsListArgs {
            include_asset_paths: true,
        }))
        .await
        .0;
    assert_eq!(response.error.unwrap().code, "PERMISSION_DENIED");
}

#[tokio::test]
async fn rigs_list_returns_only_relative_asset_keys() {
    let response = server()
        .rigs_list(Parameters(crate::server::RigsListArgs::default()))
        .await
        .0;
    assert!(response.ok);
    let json = serde_json::to_value(response).unwrap();
    assert_eq!(json["data"]["count"], 1);
    assert_eq!(
        json["data"]["items"][0]["modelAsset"],
        "models/model-human.glb"
    );
}

#[test]
fn ipc_snapshot_rejects_absolute_and_traversing_asset_keys() {
    let mut snapshot = mock_snapshot();
    assert!(snapshot.validate().is_ok());
    snapshot.rigs[0].model_asset = "/home/user/private.glb".to_owned();
    assert!(snapshot.validate().is_err());

    snapshot.rigs[0].model_asset = "models/../../private.glb".to_owned();
    assert!(snapshot.validate().is_err());
}

#[test]
fn rig_filter_is_a_closed_enum() {
    assert!(serde_json::from_str::<AnimationsListArgs>(r#"{"rig":"human"}"#).is_ok());
    assert!(serde_json::from_str::<AnimationsListArgs>(r#"{"rig":"/home/user"}"#).is_err());
}

#[tokio::test]
async fn stale_and_fatal_snapshots_return_explicit_errors() {
    let mut stale = mock_snapshot();
    stale.meta.age_ms = 5_001;
    let stale_server = server_with_snapshot(stale);
    let status = stale_server
        .application_get_status(Parameters(ApplicationStatusArgs::default()))
        .await
        .0;
    assert!(
        status.ok,
        "status remains available when its snapshot is stale"
    );
    let rigs = stale_server
        .rigs_list(Parameters(crate::server::RigsListArgs::default()))
        .await
        .0;
    assert_eq!(rigs.error.unwrap().code, "STALE_STATE");

    let mut fatal = mock_snapshot();
    fatal.application.phase = AppPhase::Fatal;
    let fatal_server = server_with_snapshot(fatal);
    let scene = fatal_server
        .scene_inspect(Parameters(SceneInspectArgs::default()))
        .await
        .0;
    assert_eq!(scene.error.unwrap().code, "ENGINE_FATAL");
}

#[tokio::test]
async fn production_without_app_ipc_reports_unavailable_without_mock_state() {
    let app = Mesh2MotionMcp::new(Arc::new(crate::IpcClient::default()));
    let response = app
        .application_get_status(Parameters(ApplicationStatusArgs::default()))
        .await
        .0;
    assert!(!response.ok);
    assert_eq!(response.error.unwrap().code, "APP_UNAVAILABLE");
}

#[test]
fn arguments_reject_unknown_properties() {
    let unknown = r#"{"includeDiagnostics":false,"filesystemPath":"/private"}"#;
    assert!(serde_json::from_str::<ApplicationStatusArgs>(unknown).is_err());
}
