# M2M-028A — Frontend authoring bridge (OpenCode)

## 1. Start condition and provenance

The specified M2M-027 candidate and integration-lead contract did not exist in Project 3 at task start
(verified: no M2M-027 report/branch/tag, no `authoring-bridge/v1` file, 0 grep matches). Codex explicitly
asked to proceed because it was blocked. This task therefore:
- published the actionable contract itself at `coordination/contracts/authoring-bridge-v1.md` (marked
  **provisional pending Codex review**), and
- opened an append-only two-way log at `coordination/handoffs/M2M-028A-bridge-channel.md` and a handoff
  notice at `coordination/handoffs/M2M-028A-to-Codex.md`.

Base revision: `83f0439c2712ba9d65f4643d3666320e5fc92c20` (`m2m-026-native-fixes`), which preserves
`ae487348`, M2M-025D/026 fixes and frozen `animation-document/v1`. Codex's `m2m-025c-frontend-integration`
was deliberately **not** used because `5f6d010` changes frozen validation semantics.

## 2. Exact commits

| Item | Value |
| --- | --- |
| Branch | `m2m-028a-frontend-bridge` |
| Worktree | `worktrees/m2m028a-frontend-bridge` |
| **Commit** | **`393bd670f36946465458f6e70385e119647e73e1`** |
| Base | `83f0439c2712ba9d65f4643d3666320e5fc92c20` |
| Canonical branches | untouched; no package installed; no Rust/MCP file modified |

## 3. Source interfaces (all under `src/authoringBridge/`)

| File | Exports / role |
| --- | --- |
| `types.ts` | `AUTHORING_BRIDGE_PROTOCOL`, `BRIDGE_LIMITS`, closed `AuthoringOperation` / `AuthoringErrorCode` sets, `AuthoringRequest`, `AuthoringResponse`, `is_authoring_request`, `is_animation_document` |
| `dispatcher.ts` | `AuthoringDispatcher`, `AuthoringHandlers` — validates envelope, bounds, deadlines, duplicate IDs, one in-flight request and one session; maps handler errors to bridge codes; owns and clears all timers |
| `rigCatalog.ts` | `live_skeleton_from_engine`, `live_rig_identifier`, `publish_live_rig_catalog` — returns only the current live `Skeleton.bones` data and fingerprint |
| `liveWave.ts` | `create_live_agent_wave`, `map_live_wave_bones` — creates the original wave from live bone IDs |
| `livePreview.ts` | `LiveDraftPreview` — guarded converter + existing `DraftPreviewSession`/`create_engine_preview` |
| `confirmationPanel.ts` / `.css` | `NativeConfirmationPanel` — real DOM dialog; only click events can approve |
| `applicationOwner.ts` | `RealApplicationOwner`, `AUTHORED_TAG`, `is_authored_entry` — application-owned commit into the real library |
| `service.ts` | `FrontendAuthoringBridge` — wires all of the above to the real engine |
| `attach.ts` | `attachAuthoringBridge` — installs on all four page bootstraps via `maybe_attach_desktop_control` |

Two small application-owned additions in `StepAnimationsListing.ts`:
`add_authored_animation(pair)`, `current_selection()`, `select_animation(id)`. These keep the library,
search UI and count label in sync without exposing private state.

## 4. Native bridge operations

`rig.catalog.get`, `draft.create`, `draft.validate`, `draft.preview.start`, `draft.preview.stop`,
`draft.commit`, `draft.cancel`, `draft.undo`, `draft.export`, `draft.import`, `session.status`,
`session.cancel`. Envelope: `{ protocol:'authoring-bridge/v1', request_id, session_id, deadline_ms,
operation, payload }` → `{ protocol, request_id, session_id, ok, result?, error? }`.
Frontend listens for `m2m-authoring-request` CustomEvents and answers with `m2m-authoring-response`.

Safety: typed data only, no eval, no dynamic module names, no filesystem paths, ≤512 KiB payloads,
100–30000 ms deadlines, duplicate/concurrent request rejection. Client-supplied `approved` fields are
ignored; no operation can set approval.

## 5. Live rig mapping and Real Agent_Wave_01

`publish_live_rig_catalog()` reads `engine.active_skinned_meshes()[0].skeleton` (the skeleton actually
driven by the mixer), falls back to `edit_skeleton_step.skeleton()`, and returns stable IDs
`human:bone:NNN`, display names, parent IDs, local rest transforms and the SHA-256 fingerprint. No
synthetic catalog.

`create_live_agent_wave()` reads the live Human catalog, locates RightUpperArm / RightForeArm / RightHand
(and chest/head) by display-name tokens, then authors new 2-second quaternion/slerp keyframes against
those actual IDs and the actual fingerprint. It does not reuse the synthetic fixture's fingerprint or
hardcoded IDs and does not copy an existing clip. Verified in tests: mapping to `human:bone:010/011/012`
on the test rig, fingerprint equals live fingerprint, validation valid.

## 6. Testing

| Check | Result |
| --- | --- |
| ESLint `--max-warnings 0` | 0 errors / 0 warnings |
| TypeScript `--noEmit` | 0 errors |
| Vitest | **59 files / 385 tests** (baseline 56 / 364; +19 new tests) |
| Vite build | exit 0 |
| Runtime import cycles | 2/2 |
| ast-grep explicit `any` | 0 |

Focused tests cover: live catalog publication and IDs/fingerprint, invalid bone/fingerprint rejection,
rig-change during session, valid preview/stop, concurrent requests, duplicate IDs, deadline expiry,
client-approval spoof rejection, native-click rejection/approval, commit into the real library,
undo, session cancel, and export/reimport transform equivalence (track names and all values compared).

Preservation verified by existing suites: 684 WebP/MP4 assets, 20/20 ceilings, single `video.play()`,
zero `video.load()`, no play buttons, NATIVE-005/006 fixes, Props/Settings/rigging layouts.

## 7. Native visual evidence — BLOCKED

No native MCP client exists yet on the Rust side, and this session cannot drive the KDE Wayland window.
A native screenshot of the confirmation panel or played wave therefore cannot be produced here. The
provided screenshots from earlier tasks do not cover this feature. Native acceptance of the bridge is
**BLOCKED** until Codex drives a real request through the native application. No MCP success is claimed.

## 8. Known limitations / Codex handoff

- The Rust/MCP side must dispatch the CustomEvent or implement an IPC→event shim; only `snapshotGet`
  exists today. Contract and open questions are in `coordination/handoffs/M2M-028A-bridge-channel.md`.
- The typed bridge is installed from `maybe_attach_desktop_control` and is a no-op until a live skeleton
  exists; if a rig loads after that call, the page must be re-entered or the attach called again.
- `LiveDraftPreview` cannot remove the engine's frame listener (engine has no
  `remove_frame_listener`); the session self-gates on `active`, so a stopped preview is a no-op, but the
  callback remains registered for the page lifetime.
- Undo appends a new authored entry representing the restored document (audit-friendly); it does not
  delete the previous entry.
- `animation-document/v1` was not modified. Codex's `5f6d010` validation amendments are not included.

---

# M2M-028B amendments — final frontend implementation

**Commit:** **`0fe123562cd0d6fef7a49c7da4ccf86afbe1aba4`** on `m2m-028a-frontend-bridge`
(previous `393bd670…` preserved as ancestor). Canonical branches, Rust/MCP, packages unchanged.

## Contract decisions (recorded in `coordination/contracts/authoring-bridge-v1.md`, now FINAL)

1. **Transport:** whole serialized envelope measured as UTF-8 bytes (`TextEncoder`), hard cap 256 KiB.
   Replaces the provisional 512 KiB JS-char check. No TCP/shell/paths/eval added.
2. **Sequence:** `sequence` (safe integer) required in every request, strictly increasing per session,
   echoed in responses; replay/reorder rejected as `BAD_REQUEST`.
3. **Dedupe:** 4096 request IDs retained per app launch, then fail closed with `SESSION_CONFLICT`.
4. **`draft.set_keyframes`:** payload `{ expected_revision, bone_id, property, keyframes }` →
   `{ revision, validation }`. Replaces a channel atomically at the expected revision; stale revision
   → `BAD_REQUEST`, invalid data → `INVALID_DOCUMENT`, no state change on rejection.
   `draft.create` now also accepts `{ name, duration_seconds }` to create an empty live-rig draft,
   keeping every request well below 256 KiB.
5. **Tauri adapter:** subscribes to `m2m-authoring-request`, calls `window.m2mAuthoringBridge.handle()`,
   invokes `mcp_authoring_respond` with `{ response }`; handles `m2m-authoring-cancel` by clearing
   session, preview, confirmation and listeners. Browser CustomEvents retained only for tests.
6. **Permission:** `#m2m-authoring-toggle`, default OFF every start, only a user click enables it;
   MCP requests cannot enable it. Read-only MCP access unchanged.
7. **Confirmation binding:** approval acknowledges `{ request_id, session_id, operation, revision,
   document_digest: SHA-256(serialized document), target_revision }`; stale/replayed/mismatched
   approvals rejected. Panel uses the caller's remaining deadline, never a detached 30 s timer.
8. **State integrity:** commit uses a compensating boundary (`InMemoryAnimationDocumentStore.rollback`
   + owner `rollback_last`); undo removes the library entry for the undone revision and applies the
   restored document, restoring prior selection.

## Quality at `0fe12356`

ESLint 0 errors/0 warnings, TypeScript 0 errors, Vitest **60 files / 395 tests**, Vite build OK,
runtime import cycles 2/2, ast-grep 0 explicit `any`.

New tests cover UTF-8 byte boundaries, sequence replay, 4096-ID dedupe cap, incremental keyframe
revision atomicity, Tauri request/ack correlation, cancel/disconnect cleanup, permission default-off,
digest-bound approval, failed owner insertion rollback, and real undo entry removal.

## Handoff status

Frontend side of M2M-028B is complete. Codex must integrate `0fe12356` with the Rust authoring branch,
then run the native JSON-RPC/preview/approval/commit/export/reimport/cancel/undo acceptance.
No native Agent_Wave_01 demo is claimed by this task.

---

# M2M-028A addendum — final frontend commit

**Commit:** **`3941490818f10afaf39e0ed040d2248228ffbf57`** on `m2m-028a-frontend-bridge`
(ancestors `0fe12356…` and `393bd670…`, both preserved). Canonical branches, Rust/MCP, packages and
installed app untouched.

## Corrections delivered

- **Rig-ready attachment:** `rigLifecycle.ts` watches `skeletonLoaded`/`modelLoaded`/`modelLoadError`
  plus a removable per-frame identity check. The bridge is created when a real rig finishes loading and
  disposed/recreated on unload, replacement or model switch — no page re-entry needed. Repeated
  `attachAuthoringBridge` calls create only one bridge.
- **Preview cleanup:** `Mesh2MotionEngine.remove_frame_listener()` added; `DraftPreviewSession.dispose()`
  unregisters its frame callback and releases mixer/actions. `LiveDraftPreview.dispose()` uses it.
- **Tauri boundary:** Tauri events remain the authoritative transport; DOM CustomEvents are internal
  test/dev transport only and cannot grant permission or approval.
- **Permission display:** `#m2m-authoring-toggle` shows `Enable authoring` / `Authoring enabled`, starts
  disabled on every launch, and only a click enables it.
- **Approval binding:** `ConfirmationBinding` now includes `rig_fingerprint` in addition to request,
  session, operation, revision, document SHA-256 and undo target. Mismatched approvals are rejected.
- **Undo/state integrity:** unchanged from `0fe12356` — undone library entry is removed and prior
  selection restored; commit has store+owner rollback boundary.

## Validation at `39414908`

ESLint 0 errors/0 warnings, TypeScript 0 errors, Vitest **61 files / 405 tests**, Vite build OK,
runtime import cycles 2/2, ast-grep 0 explicit `any`. Asset/preservation checks: 684 WebP + 684 MP4,
0 `video.load()`, no play-button markup. No native authoring demo claimed.

---

# Codex support — Rust envelope reconciliation (commit `380cb70`)

Frontend aligned to Codex's `authoring_bridge.rs` at `6a6a809`: added `AUTHORING_DISABLED` and
`APP_DISCONNECTED`, UTF-8 byte-length ID validation, unknown-envelope-field rejection, and permission
denial now returns `AUTHORING_DISABLED`. All 13 operations, 18 error codes, the 256 KiB UTF-8 ceiling,
sequence/deadline bounds and response correlation now match field by field. The full comparison table
and decisions for Codex are in `coordination/handoffs/M2M-028A-bridge-channel.md`.

Quality at `380cb700c59dcff1f19699ca5b3c530be15d4c08`: ESLint 0/0, TypeScript 0, Vitest 61/405, Vite OK,
cycles 2/2, ast-grep 0 `any`. No Rust file or package touched.
