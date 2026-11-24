# M2M-031 release vision-tool gap verification

Date: 2026-11-24

## Result

The flag is accurate for the installed M2M-031 r82 release. Its MCP sidecar exposes
`animation_set_bone_keyframes` but does not expose `animation_observe_pose` or
`animation_capture_view`. This is a feature gap in the release, not a documentation-only
gap. The M2M-030B integration candidate exposes all three tools.

## Evidence

- `coordination/reports/M2M-031/final-release.md` identifies the desktop release as
  `m2m-031-release-desktop` at `e439eb1a638c136df594888bd25071776083c24d`, package
  `mesh2motion-mcp-git-0.1.10.r82.m2m031-1-x86_64.pkg.tar.zst`, and explicitly says
  “MCP-side tools still pending” / “no Rust MCP tool was added by this work.”
- A JSON-RPC `initialize` + `tools/list` exchange against the installed
  `/usr/bin/mesh2motion-mcp` returned 17 tools. It included
  `animation_set_bone_keyframes`; it did not include `animation_observe_pose` or
  `animation_capture_view`.
- Source inspection of `worktrees/m2m031-release/desktop/src-tauri/mcp-server/src/server.rs`
  confirms the same registrations: keyframe authoring exists, observation registrations do not.
- A JSON-RPC `tools/list` exchange against the M2M-030B integration sidecar returned 20 tools
  and included all three names.

## Decision and integration status

The existing M2M-031 r82 package remains the historical released build and is not modified
or backported here. Its documented scope delivered the New Animation UI and internal camera
rendering, while MCP image transport was explicitly deferred. The M2M-030B integration branch
is the candidate that closes this gap. Its Rust/frontend trees are integrated on
`m2m-030b-integration`; it has not been promoted to a canonical `main` branch or installed.

The release branch's uncommitted experimental backport remains isolated and untouched. A
future package intended to advertise M2M-030B vision support must be built from the integrated
branch (or a reviewed release promotion of it), and its installed sidecar should pass a
`tools/list` acceptance check for both observation tools.

