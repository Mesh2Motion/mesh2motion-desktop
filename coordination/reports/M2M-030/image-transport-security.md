# M2M-030B — image transport security

Date: 2026-11-23

## Transport and limits

Image bytes are requested by a Rust event over the current private Unix IPC bridge; frontend replies through `mcp_pose_observation_image_respond`. The new event/command pair carries only one bounded chunk per exchange. It creates no HTTP endpoint, filesystem path, TCP port, shell execution, or arbitrary JavaScript route. Existing same-UID checks, socket mode 0600, runtime directory mode 0700 and 256 KiB frame maximum are unchanged.

| Resource | Bound |
| --- | ---: |
| Outer request/response frame | 256 KiB |
| Concurrent observation/image chunk | 1 |
| Chunk | 32 KiB |
| Retained captures per session | 4 |
| Aggregate image memory | 8 MiB |
| Capture lifetime | 30 seconds |
| Image dimensions | 64..1024 per side |
| PNG / JPEG bytes | 1 MiB / 256 KiB |
| Supported views | front, back, left, right, top, bottom |
| Request deadline | 100 ms..30 s |

The frontend owns captured bytes; Rust stores opaque references and validated pose/capture metadata. Capture handles are bound to the active authoring session. A rig change invalidates old captures; cancel/dispose, normal client disconnect, dropped request and app host shutdown clear in-flight requests and signal cancellation. Ungraceful process termination relies on the frontend's 30-second TTL.

## Validation and errors

Both sides enforce request/session/capture/image-ref/view/offset correlation. Rust verifies each response has exact identifiers and offset, stable MIME and total length across chunks, non-empty decoded bytes within the requested chunk size, declared image size within MIME caps, valid image signature/dimensions, expected pose digest, exact timestamp, current revision and matching rig fingerprint. MCP images use standard base64-backed `ImageContent`; no text substitute is used.

Observation errors use the closed contract set: `BAD_PROTOCOL`, `UNKNOWN_OPERATION`, `BAD_REQUEST`, `PAYLOAD_TOO_LARGE`, `DEADLINE_EXCEEDED`, `DUPLICATE_REQUEST_ID`, `SESSION_CONFLICT`, `RIG_UNAVAILABLE`, `RIG_MISMATCH`, `STALE_REVISION`, `UNSUPPORTED_BONE`, `CAPTURE_CANCELLED`, `CAPTURE_EXPIRED`, `CAPTURE_NOT_FOUND`, `RESOURCE_LIMIT_EXCEEDED`, `PREVIEW_ACTIVE`, `RENDER_FAILED`, `CONTEXT_LOST`, and `INTERNAL`. Draft lifecycle also returns existing authoring errors such as `INVALID_DOCUMENT`; those remain on the existing authoring envelope.

## Tests present and remaining

Implemented tests cover invalid operation/protocol/IDs/sequence/deadline and UTF-8 byte size, oversize envelopes, unsupported image view, invalid chunk size and path-like image ref, exact response correlation, base64 malformed padding, image MIME signatures, private IPC observation/chunk roundtrip, duplicate/stale/cross-session observation guard behavior, disconnect cancellation, session authorization and bounded socket frames. The latest MCP crate run passed **63 library + 3 binary tests**; the Tauri host passed **4 tests**. Frontend service/adapter tests also cover exact bytes, cross-session rejection, listener disposal and large response pruning.

Several requested race/lifecycle cases remain without native fault-injection evidence: actual renderer failure, disconnect during GPU capture, shutdown during GPU capture, rig replacement during GPU capture, cancellation races, and viewport/privacy isolation. Cache expiry/count/aggregate tests are not exhaustive at the MCP registry level. The initial “frontend does not handle chunk event” status is superseded: the separate frontend feature branch now implements and tests the bounded event/command adapter, and the rebuilt native app delivered six real images to an MCP client.

## Mandatory tool evidence

- Ruff PASS; pytest 3 PASS (external Dash deprecation warning); ty PASS.
- ast-grep unsafe Rust block scan PASS, no results.
- Bacon invoked headless under a 20-second timeout, but did not exit; mark **BLOCKED**. Cargo check passes independently.
- BugStalker 0.4.9 availability PASS. Bounded TUI starts but displays `Process: not running`; the non-TTY attempt reported `CannotToggleRawMode`. This is availability/smoke evidence only, not fault debugging.
- Cargo fmt, MCP tests, MCP Clippy, Tauri tests and Tauri Clippy pass as described in the implementation report. The Tauri checks use a temporary placeholder dist only to satisfy build-time configuration; it was removed and provides no frontend or native acceptance evidence.

## Native image delivery verification — 2026-11-23

The current native test used the feature-branch Tauri binary and standalone MCP sidecar, with the frontend `dist` produced from the separate `m2m-030b-image-transport` worktree. A real stdio JSON-RPC client called `initialize`, `tools/list`, live catalog, draft creation, `animation_observe_pose`, and `animation_capture_view` for each view. All six MCP responses contained actual `ImageContent` PNG bytes and correlated metadata. The draft was cancelled after capture.

Shared metadata: capture ID `capture-1a20a96c9c5-953e5b01`; view set front/back/left/right/top/bottom; each 512×512; revision 1; timestamp 0; Human rig fingerprint `1a167d80d91326d763af6c04914790d814010341d843008129ca2102d7f4c31d`. The transcript with actual MCP image blocks is `coordination/audits/M2M-030/mcp-vision-native-test.ndjson`; decoded test images are under `artifacts/M2M-030B/native-images/final-smoke/`.

| View | PNG bytes | SHA-256 |
| --- | ---: | --- |
| front | 28,665 | `1208eb9c1b8e4f171c02ee4719a99beb0d89cb2c4fe5f0d5ce7265744a2cef08` |
| back | 32,303 | `c94a1d63088255e2e1e60b57cf26e6b53dc3e571b7a845ebd2e91b03a880e648` |
| left | 25,641 | `bd42bffa83cfeefbb00ff3cb03a0b324cb1e64fcfb5b00b1934c976fc887785a` |
| right | 28,541 | `eafa20e1b147d3354f1a56748814f598639418a2ffefa5e014f93cd408ca97c9` |
| top | 23,752 | `209e3acb3d0b4a9083d14468fbb47749b5b02b06fc8dd5f7073c86f89832fd62` |
| bottom | 15,239 | `2756e46cf8c69a7d6d7bc67b3c498a03ffa536755b0dd87da7d7545f8ead945e` |

Additional Agent_Wave_01 native workflow evidence from OpenCode is in `coordination/audits/M2M-030/mcp-vision-transcript-opencode-live.ndjson`, `mcp-vision-transcript-final-steps.ndjson`, and `artifacts/M2M-030/vision-guided/`. The staged app committed `Agent_Wave_01_vision` at revision 14 and export/reimport preserved six tracks / 54 keys. Undo to revision 13 returned `BAD_REQUEST`; the staged library currently contains the clip. Visual acceptance remains blocked because the rendered image does not match MCP joint metadata; a later second capture returned `RIG_UNAVAILABLE` after four seconds.

**Correction to the iteration note above:** OpenCode found stale Three.js skin matrices in those pre-fix images. After their `skeleton.update()` fix, Codex rebuilt the app and recaptured the live wave draft with preview inactive. The new t=0.25 front image still shows a crouched mesh while MCP geometry reports `hand_l` y=1.919 and head y=1.567. This is a real image/geometry parity defect, so the earlier iteration screenshots are not valid pose-quality evidence and should not be used for visual-guided refinement. Latest transcript: `coordination/audits/M2M-030/vision-agent-wave-post-skin-matrix.ndjson`; latest files: `artifacts/M2M-030B/agent-wave-iterations/post-skin-matrix-no-preview/`. Investigate the active SkinnedMesh skeleton binding or another animation writer before claiming synchronized observation.

### OpenCode addendum — transport integrity and lifecycle finding (2026-11-24)

- Transport integrity independently verified: for three views, inline `image_base64` from
  `animation_observe_pose` and the bytes returned through `image_ref`/`animation_capture_view` were
  byte-identical (same length and SHA-256). No image path, HTTP endpoint or TCP port is involved.
- Cross-session and correlation behaviour was exercised through the live client: a second sidecar
  process initially failed with `DUPLICATE_REQUEST_ID`, confirming app-lifetime request-ID
  uniqueness is required; fixed in Rust with session-scoped IDs.
- **New lifecycle finding:** a repeat capture of the *same draft, revision and timestamp* four
  seconds apart returned `RIG_UNAVAILABLE: no live supported rig is loaded` on the second call.
  The rig-lifecycle watcher can drop a live rig between observations; this needs a test and fix
  before capture availability can be trusted.
- Live limits were respected: 512×512 PNGs of 22–29 KiB each, far below the 1 MiB/MIME and 8 MiB
  aggregate ceilings; 9 images per phase across 3 retained captures.
- Correction to "no animation was committed": my separate run committed `Agent_Wave_01_vision`
  (revision 14, `custom-import-2`) and verified export/reimport equality. Undo returned
  `BAD_REQUEST`, so rollback remains BLOCKED.

### RESOLUTION (2026-11-24)

All three lifecycle/parity blockers are closed in the frontend worktree:
- The observation service now re-reads the **live rendered skeleton** on every capture; a native
  diagnostic changed from `skeleton_is_rendered: false` to `true` with identical source/rendered
  bone world positions. The transport adapter is installed once and dispatches to the current
  service, so teardown/reattach can no longer leave a stale listener.
- Transient null rig snapshots are debounced (750 ms) before teardown. A second capture of the
  same draft/timestamp four seconds later now succeeds and is **byte-identical**.
- Unauthored bones are reset to bind pose before evaluation, so an active library animation can no
  longer bleed into captures. Determinism is verified natively and by a regression test.
- Undo succeeded in-session with `target_revision` = an earlier document revision
  (`custom-import-6`, revision 15); the earlier `BAD_REQUEST` was an incorrect library revision.
- Frontend 71/474 tests, tsc, eslint, Vite build; MCP 63+3 tests, fmt, denied-warning Clippy;
  Ruff, pytest 4, ty; ast-grep unsafe scan clean. Evidence:
  `coordination/audits/M2M-030/mcp-vision-{parity-diagnostics,determinism,undo}.ndjson`.


## Integrated native transport verification (2026-11-24)

The merged integration desktop/sidecar pair was exercised against a real native Tauri app over the existing same-UID private Unix socket. The disposable draft observation returned a bounded capture ID, `mcp-pose-observation/v1`, the live Human rig fingerprint, revision 1, exact timestamp 0, six camera records, camera view/projection matrices, and projected joint positions. The real MCP client decoded six PNG image content blocks and validated the PNG signature and 512×512 dimensions; all payloads were 12.4–30.4 KiB. It then canceled the uncommitted session. No HTTP/TCP image endpoint or arbitrary path was used.

Evidence is in `coordination/audits/M2M-030/mcp-vision-integration-native.ndjson` and `artifacts/M2M-030/integration-native/`. Frontend merge includes four-capture adapter retention, cross-session authorization and eviction tests; the MCP Rust suite and frontend suite passed. Native fault injection for renderer failure, disconnect/shutdown races, cancellation races, and rig replacement remains unperformed; report this as a test coverage limitation, not a proven runtime failure.


## Final native-safe integration revision (2026-11-24)

The final frontend integration SHA is `c83c210b63a8fee0680ca563d460a32950665997`; the final desktop integration SHA is `94518dc5cdd093f66693f4364af3cffd026cfb9e` and its frontend gitlink matches. The M2M-031 cleanup branch was merged, removing Vite's deprecated `__dirname` warning and fixing the existing TypeScript/JSDOM validation noise. A Rolldown size-based splitting experiment removed the >500 KiB warning in Vite but caused the native frontend authoring listener not to start; that change was reverted. The final native-safe build starts `frontend_ready=true`; the large offline/Three chunks advisory remains (largest 960 KiB, gzip 267 KiB). Final locked desktop build passed. With an isolated blank profile, real MCP `tools/list` returned 19 tools and `animation_cancel_session` succeeded; no live rig was loaded in that blank profile. The six-view real-image capture transcript and PNGs above came from the same observation/application code before the build-config-only cleanup.

The exact integration-native app and MCP sidecar hashes after the final rebuild: see the final handoff. No packages were installed or canonical branches changed.
