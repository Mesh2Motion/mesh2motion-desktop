# M2M-030 — Authored preview gizmo regression

## Finding

Authored library previews and MCP observation images shared the offscreen renderer. It rendered the full Three.js scene, including editor transform controls, skeleton helpers, and debug visuals. Previously persisted authored thumbnails were reused indefinitely, so correcting only new renders would leave old contaminated previews visible.

## Change

The frontend feature branch `m2m031-preview-isolation` adds a shared exclusion list for editor-only objects. `OffscreenCapture` temporarily hides those objects only during its synchronous render and restores their exact prior visibility in `finally`, including when renderer capture fails. The engine supplies transform-control, skeleton-helper, and debug-visual objects to both authored-preview and MCP observation capture paths.

Authored previews now carry render version 2. Restoring an older/missing-version preview regenerates it from the live rig and persists the result. Deduplicated commits return the stored document ID, allowing refreshed frames to replace stale persisted frames without adding duplicate animation documents.

## Verification

- Focused capture/library tests: 19 passed.
- Frontend suite: 71 files, 484 tests passed.
- ESLint (`--max-warnings 0`): passed.
- TypeScript (`tsc --noEmit`): passed.
- Vite production build: passed; existing large-chunk advisory remains.
- Desktop Rust formatting: passed.
- Locked offline Tauri workspace tests: 2 passed.
- Locked offline Clippy with `-D warnings`: passed.
- Locked offline desktop build: passed.
- Ruff, pytest (`4 passed`, Dash plugin excluded), and ty: passed.
- ast-grep scans for Rust unsafe blocks and frontend `any`: no matches.
- Bacon check reached Cargo `Finished dev profile`; its watch loop required the 20-second timeout.
- BugStalker TUI launched, but no active debuggee was available; no runtime fault trace was produced.

The Rust checks/build used the exact isolated frontend worktree through a temporary local symlink; the cleanup trap restored the desktop tree after completion. No application window was launched, so there is no native visual confirmation of the fixed preview yet.

## Remaining issues / acceptance

- OpenCode has separate in-flight rig compatibility, rest-transform tolerance, and repeated-create draft-preservation edits in its own dirty worktree; they are not included in this branch and need integration/retesting.
- Native visual acceptance of newly generated and regenerated tiles remains outstanding. Inspect authored library tiles and MCP front/side/top images after integration.
- Front/back naming may not match user intuition for the Human rig; resolve through the shared observation contract before changing aliases.
- Previous wave clips should be evaluated by the user in the GUI. Do not auto-commit a replacement or delete library entries as part of this preview fix.
- Vite's existing large-chunk advisory remains.

## Integration

Cherry-pick the frontend branch commit into the integration frontend repository and the report commit into the integration desktop repository. Rebuild the pinned frontend/native app pair, inspect regenerated authored thumbnails, and run the native MCP observation acceptance before packaging. Do not update the canonical frontend gitlink from this feature worktree.
