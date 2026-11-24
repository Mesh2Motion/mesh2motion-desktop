# MCP authoring speed and user control follow-up

## Changes

- `animation_import_draft` is the fast authoring path for a complete clip. A client reads `skeleton_get_catalog`, builds all channels from live bone IDs and local rest transforms, and submits one validated AnimationDocument instead of issuing one `animation_set_bone_keyframes` call per bone. Validation and atomic mutation remain in the existing Rust/authoring bridge. The request ceiling remains 256 KiB.
- The maintained Python MCP client now exposes `import_draft(document)` and requires structured revision metadata in the response. The helper does not generate a canned wave or bypass live-rig validation.
- The New Animation GUI now shows MCP operation activity and result state. On completion it refreshes revision/channel controls and reapplies the current draft pose to the visible model.
- Transform controls now take pointer capture only after a primary press actually starts a gizmo drag. They release/reset on pointer-up, pointer-cancel, lost capture, window blur, disable, and disconnect. Ordinary viewport presses remain available to OrbitControls and the user's normal pointer flow.

## Validation

- Frontend: 72 Vitest files / 481 tests passed; ESLint with zero warnings, TypeScript, and Vite production build passed. Vite still reports the existing >500 KiB chunks advisory.
- Python: Ruff passed; pytest passed (6 tests); ty passed. Pytest emitted the environment's existing Dash hook deprecation notice.
- Rust: `cargo fmt --all -- --check` passed; the locked offline Tauri workspace tests passed (2 tests); the locked offline MCP crate tests passed (41 library + 3 binary); denied-warning Clippy passed for both crates.
- Read-only live MCP smoke: the maintained stdio client connected to the running app's private Unix socket, listed 19 tools including `animation_import_draft`, and received the actual Human catalog (66 bones; fingerprint `1a167d80d91326d763af6c04914790d814010341d843008129ca2102d7f4c31d`). The JSON-RPC exchange is in `artifacts/M2M-030/user-control-mcp-read-smoke.ndjson`. No draft or library mutation was issued in this smoke run.
- BugStalker opened its TUI against safe `/bin/true`; the process exited immediately and no application fault trace was produced. Bacon's `check` job was attempted twice but timed out while rebuilding/waiting on Cargo's build lock. ast-grep's Rust unsafe-block scan completed with no matches; its pointer API AST pattern did not match the TypeScript optional-call form, so the pointer review was also done from source and regression tests.
- One initial concurrent full Vitest run hit the FBX fixture test's 5-second timeout during Rust compilation. A clean rerun passed all 481 tests.

## Remaining acceptance

The live smoke used the already-running integration app only for read-only JSON-RPC. It does not prove this feature branch's new frontend bundle is running there. Native confirmation still requires building and launching this exact frontend/Rust pair, then observing that pointer-up/cancel restores orbit interaction and a new MCP-imported draft appears immediately in the visible New Animation timeline and viewport. Do not infer that from unit tests. A real human animation review remains necessary for pose quality. No install, canonical branch update, or package change was made.
