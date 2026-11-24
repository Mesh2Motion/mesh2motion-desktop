# Mesh2Motion MCP client

The optional stdio sidecar exposes the running native application's read, animation-authoring,
and pose-observation tools. It does not open a TCP listener. The native app owns a private local
Unix socket; access is limited to the same user account. Tool calls do not accept shell commands
or arbitrary filesystem paths.

## Start and permission

The usual setup is to open Mesh2Motion and enable its MCP host in **Settings → Integrations / MCP**.
The host is disabled by default. The client may opt in to starting the fixed application executable
by setting `M2M_MCP_AUTOSTART_APP=1`; this must be an explicit local client configuration choice.

On Linux, the default application path is `/usr/lib/mesh2motion/mesh2motion`. The app's **Copy
config** button supplies the correct per-user socket path and executable settings. A typical
configuration is:

```json
{
  "mcpServers": {
    "mesh2motion": {
      "command": "/usr/bin/mesh2motion-mcp",
      "args": [],
      "env": {
        "M2M_MCP_IPC_ENDPOINT": "/run/user/1000/mesh2motion/mcp.sock"
      }
    }
  }
}
```

Replace `1000` with the value from `id -u`, or use **Copy config**. If the app is closed, the host
is disabled, or the frontend has not published a fresh snapshot, tools return an explicit
unavailable/stale result.

## Tool groups

- Read and inspect: `application_get_status`, `renderer_get_state`, `rigs_list`, `animations_list`,
  `scene_inspect`, `skeleton_get_catalog`.
- Draft authoring: `animation_create_draft`, `animation_import_draft`,
  `animation_set_bone_keyframes`, `animation_validate_draft`, `animation_preview_draft`,
  `animation_commit_draft`, `animation_cancel_draft`, `animation_undo_commit`,
  `animation_export_draft`, `animation_session_status`, `animation_cancel_session`,
  `application_set_page`.
- Pose vision: `animation_observe_pose` returns synchronized joint projections and capture metadata;
  `animation_capture_view` returns actual retained image content for a capture. Images are real
  PNG/JPEG MCP image blocks, with camera matrices, dimensions, rig fingerprint, draft revision and
  timestamp. The six internal cameras are front/back/left/right/top/bottom. Request the metadata
  and pixels for matching captures before changing a pose.

Draft writes are revision-checked and validated by Rust. Cancel discards the draft; commit saves it
to the animation library. Review pose images and projected joints before committing animation.
