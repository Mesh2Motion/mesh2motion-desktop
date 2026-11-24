# M2M-027 Final Integration

## Revisions and ancestry

- Frontend integration: `921b1dbbdb375dfe392d4d35f99059e4f43e48a1`, merge parents `a33015c082aa753f04b6685e94b57297cf375207` (AnimationDocument/MCP frontend) and `83f0439c2712ba9d65f4643d3666320e5fc92c20` (LAN AI frontend).
- Desktop integration: `28f8da03000c9027c94b4761b1aebe10f0ab406d` merges `cd973962...` with LAN desktop `a981b88bd4031d62d28bc409f55b3db1d7528137a`; release recipe tip `0f39a0688f974d3d23fa3c3f576d5403eb5d9702` pins frontend `921b1db...`.
- Both isolated integration worktrees were clean. The canonical desktop worktree and its old frontend gitlink were left untouched. Recovery tags and package archives are retained as recorded in `install-and-rollback.md`.

## Integrated behavior

The frontend merge retains the newer AnimationDocument v1 files and interfaces, live rig adapter and ApplicationCommitController. It brings in LAN preview changes and the NATIVE-005 scrollbar corner/footer theme fix with regression coverage. `CustomSkeletonHelper` retains the zero-scale instanced matrices initialization and its regression test; no duplicate implementation was introduced. Existing preview scheduler limits (20 active videos and 20 concurrent fetches), hover playback, posters, Props anchoring, settings, and rigging panel layout remain in place.

Desktop retains the typed AnimationDocument mirror/validator, atomic draft store, five read-only MCP tools, private same-UID Unix socket, opt-in host startup, and shutdown hook. Tauri production custom protocol is explicitly enabled in the package build. Authoring tools are not exposed because there is no verified live frontend authoring bridge.

## Checks

- Frontend: ESLint `--max-warnings 0` pass; TypeScript noEmit pass; Vitest 57 files / 368 tests pass; Vite production build pass; runtime import cycles 2/2; ast-grep found no `any` or `video.load()` patterns.
- Rust: fmt check pass; locked workspace/all-features tests pass (3); clippy denied warnings pass; MCP server/sidecar locked tests pass (25 library + 3 sidecar); bounded headless Bacon check completed successfully (17.72s underlying check).
- MCP: actual JSON-RPC stdio smoke against staged native app passed initialize, tools/list, and all five read-only tools.
- Python maintained MCP tooling: Ruff, pytest (3), and ty pass. Whole-root Ruff reported 17 findings in legacy audit/PyGObject probes; whole-root ty reported 10 missing Gtk/WebKit2/GLib/Gdk stub diagnostics. These are documented, not reported as passes.
- BugStalker `bs` 0.4.9 executable/DAP initialization smoke completed; no relevant crash/core was present, so GDB core analysis did not apply.

## Native status and limitations

The real Tauri/WebKitGTK staged application was opened. The user confirmed “all good” after the requested NATIVE-006 five-switch visual check and broader native checklist (Human/Fox/Horse, previews, themed Remix scroll area, Props resize anchoring, Settings, and panel fit). Final extracted package build was also opened natively. No crash was observed. NATIVE-006 is accepted on the staged build; installed post-launch smoke is recorded in the install report.

NATIVE-006 is not claimed as an Agent_Wave_01 animation end-to-end result. The Rust drafts alone are not app animations; M2M-028 still needs the live frontend bridge and play/commit/export/reimport verification.
