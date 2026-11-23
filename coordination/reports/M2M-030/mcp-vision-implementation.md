# M2M-030B — MCP vision implementation

Date: 2026-11-23  
Status: **Rust transport, the isolated frontend image-byte listener, and real native MCP image delivery are implemented and verified. Agent_Wave_01 image/pose parity, valid undo, and full acceptance remain blocked.**

## Source and ownership

- Desktop worktree: `sources/mesh2motion-desktop/worktrees/m2m030b-mcp-vision/desktop`
- Rust branch: `m2m-030b-mcp-vision`; base: `e439eb1a638c136df594888bd25071776083c24d`; initial feature commit: `15017056d9f9467b3b81dfb0d19640443628a85d`; latest source commit: **`622ebe804ca3b25cc20e764a54ffa3a542e1ecf8`**.
- Codex transport adapter is on a separate worktree/branch `worktrees/m2m030b-codex-frontend/mesh2motion-app` / `m2m-030b-codex-image-transport`, commit `63b8b95ce9f9c3e125cc8e18ce75de904325d949`, based on `0920f7025773e68939fb329fc9d8be1f3d8dbf34`. OpenCode owns its independent frontend pose-evaluation branch/worktree and skin-matrix fix. Rust, Codex frontend, and OpenCode work are isolated; neither canonical branch nor gitlink changed.
- No new server or TCP listener. The feature uses the existing private same-UID Unix socket and its 256 KiB frame bound.
- `AnimationDocument` has no identifier in the published TS/Rust schema. For now, Rust binds additive MCP `draft_id` to its existing authoring-session key and checks the revision and live rig fingerprint. OpenCode should confirm this identity binding in the handoff.

## Implemented

- Added typed `mcp-pose-observation/v1` requests/responses and bounded image chunk IPC (`pose_observation.rs`, `ipc.rs`, `host.rs`, `lib.rs`). Requests reject unknown fields/operations, invalid UTF-8 byte-length IDs, unsafe sequence values, invalid deadlines, non-object payloads and bodies above 256 KiB.
- Added MCP tools `animation_observe_pose` and `animation_capture_view`; the JSON-RPC `tools/list` response confirms both names and their schemas alongside the existing authoring and six read tools.
- Pose observations validate draft/session, revision, rig fingerprint, timestamp, requested camera names, focus bone and geometry/capture metadata. Image retrieval returns an actual MCP image content block only after chunk correlation, byte limits, MIME signature/dimensions, camera metadata, and session-bound capture checks pass.
- Chunk path uses `m2m-pose-observation-image-request` and `mcp_pose_observation_image_respond`; chunk maximum 32 KiB. The outer request limit remains 256 KiB. Capture limits are 4 retained captures, 8 MiB aggregate, 30 s TTL, PNG <=1 MiB, JPEG <=256 KiB, dimensions 64..1024, and six closed camera names. Captures are frontend-held; Rust retains handles/metadata, not image files.
- Added pending-request correlation, single in-flight observation/capture guards, cancellation on request drop/session cancel/host shutdown, disconnect cleanup and frontend disposal on normal MCP stdio close. MCP observation request IDs include the authoring-session ID, preventing collisions when a second sidecar connects to the same app. App-kill cleanup is TTL-based if graceful shutdown cannot run.
- MCP `animation_create_draft` returns additive `draft_id`; no `AnimationDocument` schema change was made.

## Tool shapes

`animation_observe_pose` input: `{draft_id, expected_revision, timestamp_seconds, views, focus_bone_id?}`. Returns structured metadata with capture IDs/image refs, pose digest, rig fingerprint, revision/time, camera data, joints and projected positions.

`animation_capture_view` input: `{capture_id, view}`. Returns MCP text metadata plus `ImageContent {type:"image", mimeType, data}` with base64-encoded actual capture bytes. The actual response example is JSON-RPC response ID 5 in `coordination/audits/M2M-030/mcp-vision-native-test.ndjson` (front PNG, full MCP base64 included); exact schemas are in the `tools/list` exchange: `coordination/audits/M2M-030/mcp-vision-transcript.ndjson`.

Current transport errors include the contract codes `BAD_PROTOCOL`, `UNKNOWN_OPERATION`, `BAD_REQUEST`, `PAYLOAD_TOO_LARGE`, `RIG_UNAVAILABLE`, `RIG_MISMATCH`, `STALE_REVISION`, `UNSUPPORTED_BONE`, `CAPTURE_CANCELLED`, `CAPTURE_EXPIRED`, and `CAPTURE_NOT_FOUND`, plus authoring-bridge errors for draft validation/authorization. Full code mapping is summarized in `image-transport-security.md`.

## Verification — native integration update (2026-11-23)

- Built the Vite frontend on the Codex transport branch using existing local `node_modules` through a temporary symlink; no package install or lockfile change. Frontend tests: **71 files / 471 tests passed**; `tsc --noEmit` and ESLint (`--max-warnings 0`) passed. Vite emitted the existing native-config `__dirname` warning and large-chunk warning (784/963 KiB chunks). OpenCode separately reports its pose-evaluation branch at **71 files / 472 tests**, with TypeScript/ESLint/build clean.
- Rust MCP workspace: **63 library + 3 binary tests passed**, `cargo fmt --all -- --check` and denied-warning Clippy passed. Tauri workspace: **4 tests passed**, denied-warning Clippy and locked workspace build passed, using the actual built frontend `dist` through a temporary symlink; symlink removed after build.
- Python: Ruff passed; pytest **4 passed** (external Dash pytest-plugin deprecation notice); `ty check tools/mcp` passed.
- ast-grep `ast-grep run --kind unsafe_block -l rust src-tauri` returned exit 1 with no matches (the utility uses nonzero for an empty result). Manual IPC lifecycle/resource review completed. Bacon `timeout 20s bacon check --headless` ran `cargo check` successfully but stayed in its watch loop until timeout (exit 124); mark **BLOCKED as a terminating check**. BugStalker 0.4.9 is available; bounded TUI smoke started, but showed `Process: not running`. A non-TTY TUI attempt reported `CannotToggleRawMode`. This is availability/smoke evidence only, not fault debugging.
- Built a matching native Tauri app and standalone MCP sidecar from the feature worktree and connected a real JSON-RPC stdio client. `tools/list` reported 19 tools, including the two observation tools. The live Human catalog returned 66 bones and fingerprint `1a167d80d91326d763af6c04914790d814010341d843008129ca2102d7f4c31d`.
- Created and cancelled a native draft after requesting all six views. MCP returned genuine `image/png` ImageContent blocks, each **512×512**, with 66 joint records, camera/view/projection metadata, the same rig fingerprint, draft revision 1 and timestamp 0. This verifies real image transport and response correlation, but later pixel comparison found the rendered mesh does not yet match returned joint geometry. Synchronized image/geometry acceptance remains **BLOCKED**. Image bytes and JSON-RPC content are captured in `coordination/audits/M2M-030/mcp-vision-native-test.ndjson`; decoded images and SHA-256 values are recorded in the transport report.
- Fixed the replay collision found on reconnect by session-scoping observation/disposal request IDs, then rebuilt both the app and standalone sidecar. A fresh app process was required after earlier failed attempts left an in-memory session active.

## Superseded initial verification snapshot

The following records the first Rust-only pass, before frontend interoperability and native capture were completed. The later native integration update above supersedes all old tool counts and “no image/no native app” statements below.

- MCP crate: `cargo test --locked --workspace --all-features` — **61 library + 3 binary tests passed**.
- MCP crate: `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings` — **PASS**.
- App workspace with `mcp-host`: `cargo test --locked --workspace --features mcp-host` — **4 Tauri tests passed**. Since this pinned source checkout lacks generated `mesh2motion-app/dist`, a temporary minimal `dist/index.html` was created only for compilation and removed immediately. This is not a native-app or frontend test.
- App workspace Clippy: `cargo clippy --locked --workspace --all-targets --features mcp-host -- -D warnings` — **PASS**, same temporary build-only dist workaround.
- `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` — **PASS**.
- Ruff `ruff check tools/mcp` — **PASS**; pytest `pytest -q tools/mcp` — **3 passed**, with one pre-existing external Dash plugin deprecation warning; `ty check tools/mcp` — **PASS**.
- ast-grep Rust `unsafe_block` scan across MCP and Tauri IPC — **PASS, no matches**. IPC lifecycle and image resource cleanup were also reviewed.
- Bacon `timeout 20s bacon check --headless` — **BLOCKED**: it displayed a check task but did not exit within 20 seconds. A prior unbounded headless invocation also stayed open; Cargo check and Clippy themselves pass.
- BugStalker `bs --version` / `bs --help` — **PASS for availability** (`0.4.9`). A bounded TUI smoke against the compiled MCP unit-test binary opened successfully but displayed `Process: not running`; this is not fault debugging and yielded no runtime trace.
- Built the MCP sidecar and ran a real stdio JSON-RPC `initialize` + `tools/list` exchange. It listed 19 tools, including both new tools and existing tools. No running native app was connected, so no `tools/call` camera operation was made.
- Frontend Vitest/TypeScript contract tests — **BLOCKED**: no `node_modules` at the exact pinned frontend checkout; package installation was not performed.

## Integration status and remaining limitation

The new frontend branch listens for `m2m-pose-observation-image-request`, returns exact bounded byte ranges via `mcp_pose_observation_image_respond`, scopes capture IDs to their MCP session, and clears the listener on disposal. It also removes inline image bytes from oversized pose envelopes while preserving image references and the existing 256 KiB limit. Cross-session access, chunk correlation, cleanup, and envelope bounding have frontend tests. These changes are separate from the pinned frontend checkout and await integration-agent review.

Agent_Wave_01 draft creation, channel submission, Rust validation, preview, capture, actual image review, channel revision, recapture, commit and export/reimport were exercised in OpenCode's staged native app. The clip `Agent_Wave_01_vision` was committed at revision 14 (`custom-import-2`, six tracks); export/reimport preserved six tracks / 54 keys and valid status. The image review requested a higher left arm and tucked right arm. However, a new app build with preview inactive still showed a forward-crouched mesh while `hand_l` metadata was at world y=1.919 and the head at y=1.567. Therefore this workflow used genuine pixels to request changes but did not prove the resulting keyed pose matches the pixels. The visual result is not accepted. Undo to revision 13 returned `BAD_REQUEST`; a later repeated capture returned `RIG_UNAVAILABLE` after 4 seconds. The staged library currently contains the committed clip. Image/pose parity, rig lifecycle stability, valid undo/rollback, and race/privacy acceptance remain **BLOCKED**.

OpenCode's exact native run is recorded in `coordination/audits/M2M-030/mcp-vision-transcript-opencode-live.ndjson` and `mcp-vision-transcript-final-steps.ndjson`; 18 real view images, manifests and pixel-review JSON are under `artifacts/M2M-030/vision-guided/`.

Both feature branches are ready for integration review. The integration agent owns their merge and frontend gitlink update. Do not claim full mission completion until visual-quality refinement, commit/export/reimport/undo, and remaining race/lifecycle acceptance are complete. No canonical branch, package, or installation was changed.

## OpenCode addendum — full workflow run and residual defects (2026-11-24)

I worked with Codex on the same branches (`m2m-030b-mcp-vision`, now also `622ebe8`, and the still
uncommitted frontend `m2m-030b-image-transport`) and ran a real JSON-RPC client against a staged
app built from those exact sources.

- Confirmed genuine image delivery: inline `image_base64` and `image_ref` chunk bytes were
  byte-identical for front/right/top (same size and SHA-256). 19 tools and the live 66-bone Human
  catalog (`1a167d80…`) were present.
- Confirmed Codex's `DUPLICATE_REQUEST_ID` root cause and that his `pose_request_id(session_id,
  sequence)` fix resolves repeat MCP sessions.
- Applied the frontend skin-matrix fix (`skeleton.update()` in `poseEvaluation.ts` plus regression
  test). Frontend gates after the fix: 71 files / 472 tests, `tsc`, `eslint --max-warnings 0`,
  `vite build` all PASS. Python `ty` regression (`# ty: ignore[invalid-assignment]`) fixed;
  Ruff, pytest (4 passed) and ty PASS.
- **New evidence contradicting the "no clip committed" note above:** in my run the wave draft was
  actually **committed** to the real staged library as `Agent_Wave_01_vision`, revision **14**,
  entry `custom-import-2`, 6 tracks. `animations_list` confirmed it. Export/reimport comparison
  matched exactly (6 tracks / 54 keyframes, `state: valid`), and the session cancelled cleanly.
  `animation_undo_commit` returned `BAD_REQUEST` for target revision 13, so undo/rollback is still
  BLOCKED. Evidence: `coordination/audits/M2M-030/mcp-vision-transcript-opencode-live.ndjson`,
  `...-final-steps.ndjson`, and `artifacts/M2M-030/vision-guided/`.
- Image/geometry parity remains a real, unresolved defect (Codex independently reproduced it).
  A repeat-capture probe of the **same draft/revision/timestamp** returned `RIG_UNAVAILABLE` four
  seconds later, so both skeleton binding and rig-lifecycle stability need investigation before
  any visual-quality acceptance.

## RESOLUTION (2026-11-24) — all three engineering blockers closed

Owner note: Codex declared `m2m-030b-codex-image-transport` (`63b8b95`) its final frontend
contribution and handed the shared worktree to OpenCode. All fixes below are in OpenCode's
`m2m-030b-image-transport` worktree.

1. **Image/geometry parity — FIXED.** A temporary native diagnostic proved
   `skeleton_is_rendered: false`: the observation service held an early fallback (editor) skeleton,
   not the rendered `SkinnedMesh.skeleton` (`rendered_mesh_count: 1`). `attach.ts` now re-reads the
   live skeleton via `live_skeleton_from_engine()` on every capture and verifies its fingerprint;
   the Tauri adapter is installed once and dispatches to the *current* service. After the fix the
   diagnostic reported `skeleton_is_rendered: true`, `same_bone_object: true`, and identical source
   and rendered bone world positions. Real front/right PNGs now visibly show the raised arm that
   the joint metadata predicts.
2. **Rig lifecycle / `RIG_UNAVAILABLE` — FIXED.** Transient null rig snapshots were disposing the
   service and leaving a stale adapter bound to it. Null teardown is debounced 750 ms and the
   adapter dispatches dynamically. A repeat capture of the same draft/revision/timestamp four
   seconds apart now succeeds with identical bytes (`SAME_BYTES True`).
3. **Determinism — FIXED.** `apply_evaluated_pose()` now resets every bone to its bind-pose local
   transform before applying authored channels, so unauthored bones cannot retain the playing
   library animation. The documented "bones without authored channels keep their rest transform"
   invariant is now true. Repeated captures are byte-identical.
4. **Undo — user error, not a defect.** `target_revision` is an earlier **document** revision in
   the originating session, not a library revision. Undo in-session succeeded
   (`custom-import-6`, revision 15). Reimport before undo replaced the history and caused the
   earlier `BAD_REQUEST`.

Final gates: frontend **71 files / 474 tests**, tsc, eslint, vite build PASS; MCP **63 + 3 tests**,
fmt, denied-warning Clippy PASS; Ruff, pytest (4), ty PASS; ast-grep unsafe scan clean.
Full workflow evidence: `coordination/audits/M2M-030/mcp-vision-transcript-parity-fixed.ndjson`,
`...-parity-diagnostics.ndjson`, `...-determinism.ndjson`, `...-undo.ndjson`, and 18 PNGs in
`artifacts/M2M-030/vision-guided-parity-fixed/`.


## Integrated branch verification (2026-11-24)

All requested feature branches are now integrated on isolated feature branches (no canonical branch, package, or installation changed):

- Desktop/Rust integration: `m2m-030b-integration` at `d7e8f46734db7115233682c107308f99f44dc9ac`, includes Rust MCP feature merge `e98c817`; feature source commit `622ebe804ca3b25cc20e764a54ffa3a542e1ecf8`.
- Frontend integration: `m2m-030b-integration-frontend` at `9026d49db8c5adcae04be82d89b962a50c667986`, pins OpenCode fix `98fcf60c6e882a05a435f1a7df50ae22b6846357` and Codex transport `63b8b95ce9f9c3e125cc8e18ce75de904325d949`; desktop gitlink is `9026d49`.
- Frontend checks after merge: Vitest 71 files / 479 tests; TypeScript and ESLint (`--max-warnings 0`) pass; Vite build passes. Existing Vite `__dirname` and large-chunk notices remain. Rust app build passes with `--locked --all-features --offline`; MCP/workspace tests, fmt and denied-warning Clippy passed on the merged Rust source. Python Ruff/pytest (4)/ty and ast-grep checks passed. Bacon watcher timed out; BugStalker opened without an active debug process, so neither is claimed as a completed runtime fault trace.
- Native integration probe used the exact built integration desktop binary and MCP sidecar with a real Human rig (66 bones; fingerprint `1a167d80d91326d763af6c04914790d814010341d843008129ca2102d7f4c31d`). It created a disposable revision-1 draft, requested front/back/left/right/top/bottom, received six MCP `image/png` blocks at 512×512 (12,679–31,160 bytes each), verified camera matrices and joint projections in the observation response, then canceled the session without commit. Transcript: `coordination/audits/M2M-030/mcp-vision-integration-native.ndjson`; PNGs: `artifacts/M2M-030/integration-native/`.
- OpenCode's real native Agent_Wave_01 run verified six images per phase, image-guided revision, repeated-capture byte identity, native preview, commit, export/reimport, and in-session undo. A separate clean full-track clip was committed after the GUI playback issue was found. `Agent_Wave_01_vision` from the earlier six-track run remains in the staged library; use the newer `Agent Wave 01 clean` (66 tracks) for playback. Human aesthetic sign-off and cleanup of old staged clips remain outside this code integration.

The integration binary and sidecar hashes are recorded in the turn handoff. No install or promotion to `main` was performed.


## Final native-safe integration revision (2026-11-24)

The final frontend integration SHA is `c83c210b63a8fee0680ca563d460a32950665997`; the final desktop integration SHA is `94518dc5cdd093f66693f4364af3cffd026cfb9e` and its frontend gitlink matches. The M2M-031 cleanup branch was merged, removing Vite's deprecated `__dirname` warning and fixing the existing TypeScript/JSDOM validation noise. A Rolldown size-based splitting experiment removed the >500 KiB warning in Vite but caused the native frontend authoring listener not to start; that change was reverted. The final native-safe build starts `frontend_ready=true`; the large offline/Three chunks advisory remains (largest 960 KiB, gzip 267 KiB). Final locked desktop build passed. With an isolated blank profile, real MCP `tools/list` returned 19 tools and `animation_cancel_session` succeeded; no live rig was loaded in that blank profile. The six-view real-image capture transcript and PNGs above came from the same observation/application code before the build-config-only cleanup.

The exact integration-native app and MCP sidecar hashes after the final rebuild: see the final handoff. No packages were installed or canonical branches changed.
