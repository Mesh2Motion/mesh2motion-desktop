# Project 4 storage cleanup

Date: 2026-11-24

## Before and after

- Before cleanup: 8.4 GB.
- After cleanup: 312 MB.
- Reduction: approximately 8.1 GB (96%).

## Removed generated build output

- `src-tauri/target` — 6.4 GB Rust/Tauri target cache and debug app.
- `src-tauri/mcp-server/target` — 783 MB MCP sidecar target cache.
- `mesh2motion-app/dist` — 86 MB generated Vite output.
- `/tmp/m2m-app-117fdae.bundle` — temporary 151 MB submodule transfer bundle.

These are reproducible build outputs. The final OpenCode-integrated app and sidecar must be
rebuilt before installation.

## Removed from the project directory, retained outside it

The clean nested `desktop/` and `frontend/` checkouts totaled 810 MB and duplicated the project 4
repositories. Their refs were checked against project 4: desktop branch/tag refs were already
present in the parent repository, and frontend commit
`507192b01bd49c14aec5241b266afba59f245475` was present in the frontend repository. A local branch
`m2m-030b-integration-frontend` now preserves that frontend commit in the submodule. The duplicate
checkouts were moved intact to:

`/home/burstcachyhtpc/Schreibtisch/Mesh2motion wayland project 3/worktrees/project4-duplicate-checkouts-20261124/`

They were clean before the move. The pending uncommitted OpenCode edits in project 3's shared
integration worktree were not touched.

## Retained

Project 4 still contains the complete desktop and frontend Git histories, app assets, coordination
documents, reports, issue records and audit artifacts. The 312 MB size is not comparable to a
GitHub source ZIP under 50 MB: it includes full Git histories plus the frontend's bundled assets
and project evidence. No source asset or historical report was removed.

The desktop worktree and frontend submodule both report clean. No package was installed and no
GitHub push was made.
