# M2M-028C — Real MCP Agent_Wave_01 integration result

## Current UI and origin-cross correction — 2026-11-23 17:55 UTC

The user rejected any visible front/side camera panels: authoring cameras are MCP-internal only and
must not change the application interface. The native window is back on the original frontend design,
with no authoring-camera overlay. The user then identified that the XYZ view-helper cross had shifted
from the viewport's lower-left corner to its center after the viewport was moved into a sidebar-aware
canvas. Root cause: `CustomViewHelper.render()` used page-relative `offsetLeft` as a renderer-local
coordinate, counting the sidebar displacement twice.

Fixed by measuring the hitbox and renderer canvas `getBoundingClientRect()` values and subtracting the
canvas origin before setting the renderer viewport. The click target continues to use the page-space
hitbox, preserving interaction. No user-facing camera UI was added.

- Frontend fix: `m2m-028c-view-helper-fix` @ **`75aae01ae729edc1cec7612fcc382828960bde82`**.
- Desktop integration pin: `m2m-028c-integration` @ **`770dd735b5a09b0e77abb14ca1d09a90ce227960`**.
- Focused regression tests: **3 passed**; ESLint on touched files passed with max warnings 0; `tsc --noEmit` passed; Vite production build passed (existing config/chunk-size advisories only).
- Rebuilt native debug app from this exact desktop/frontend pair. Screenshot `/tmp/m2m028c-view-helper-fixed.png` visibly confirms the XYZ helper is at the viewport lower-left. No camera overlays are present.
- No package was built or installed. The Agent_Wave_01 pose remains visually unaccepted; this UI fix does not change animation data or claim a successful wave.

The earlier camera paragraphs in this report are historical proposals and are superseded by this
correction: camera views must remain hidden from users and must not move or replace the user's orbit
camera or view-helper.

## Acceptance correction — 2026-11-23

The JSON-RPC mechanics and application state transitions below were exercised, but the user's visual review rejected the wave pose shown in `Bilder/Bildschirmfotos/Bildschirmfoto_20261123_165424.png` (arm overhead, torso bent). This report therefore documents **mechanical integration evidence only**; it is not an accepted Agent_Wave_01 animation. The user requires synchronized internal front and side views for authoring pose placement, plus a corrected and visually verified wave. Transcript for the final mechanical run: `coordination/audits/M2M-028/mcp-agent-wave-final.ndjson`.

The current integration branch is `m2m-028c-integration` @ `1b44a30` (merge `691f230`, wording follow-up `894945f`, final UI restoration), pinning frontend `05051093d4cac6762507b75b35a500b749724219`. The separately published `m2m-028c-final`/`4384a73` branch was not merged: its Tauri-global-only auto-approval check is broader than the exact trusted request/session-bound authorization in `0505109`.

**Latest UI correction:** the user clarified cameras are internal to MCP only and must not appear in the application interface. The temporary visible camera prototype was removed from the integration gitlink by commit `1b44a30`, which restores frontend `0505109` and the original viewport/view-helper. Desktop integration now ends at `1b44a30`; camera implementation remains unmerged pending a hidden MCP-only design. The screenshot with visible camera overlays is rejected evidence, not an accepted release candidate.

**Status: mechanical JSON-RPC authoring lifecycle passed on the staged native Tauri 2 / WebKitGTK application; visual wave acceptance failed.**
No package was built or installed; the installed M2M-027 `r79` pair remains untouched.

## 1. Final commits

| Component | Branch | Commit |
| --- | --- | --- |
| Frontend (Codex merged line + OpenCode auto-approval) | `m2m-028c-merged` | **`4384a7310e46689db7c73270d0dfb88c8b04bb90`** (base `8ebeac4`, contains `b05400e`, `d5d0701`, `a96dff2`, `380cb70`) |
| Desktop integration | `m2m-028c-final` | **`73194cf1fd2f0a27ea7b1d6b66ae0af7b7de6db5`** (base Codex `63f1be7`, gitlink `4384a73`) |
| Current merged desktop integration | `m2m-028c-integration` | **`1b44a30`** (merges Rust feature fixes and restores original UI; exact frontend gitlink `05051093d4cac6762507b75b35a500b749724219`) |

The earlier `m2m-028c-final` line pinned `4384a73` but was not selected for current integration because
its native-host-only approval condition is broader than exact MCP request context. The current branch
pins `0505109`, which binds commit/undo approval to the in-flight request ID, session and operation.
Canonical desktop branches and installed packages were not modified.

## 2. User-directed authorization change

The user directed that the extra native opt-in and per-commit approval click be removed because they
defeat the MCP workflow. Implementation on the merged line:

- Authoring permission is granted only after a valid authoring request arrives through the trusted
  Rust/Tauri event adapter (`b05400e`, Codex). Browser CustomEvents cannot grant it.
- Commit/undo no longer open the confirmation click gate in the native host. The frontend returns an
  approved acknowledgement bound to `{request_id, session_id, operation, rig_fingerprint, revision,
  document_digest, target_revision}`; browser/test contexts still use the digest panel so binding and
  expiry remain test-covered.

The frozen `animation-document/v1` data model was not changed.

## 3. Live native sequence (real JSON-RPC, no mocks)

Transcript: `coordination/audits/M2M-028/agent-wave-final-transcript.ndjson` (58 request/response lines).
Client: `tools/mcp/agent_wave_mcp_demo.py`, a real stdio MCP client talking to the real sidecar over the
app's private same-UID Unix socket at `/tmp/m2m028c-final-runtime/mesh2motion/mcp.sock`.

Sequence and observed result:

1. `initialize`, `tools/list` — 17 tools available including all eleven authoring tools plus the six read tools.
2. `skeleton_get_catalog` — **66 live Human bones**, fingerprint `1a167d80d91326d763af6c04914790d814010341d843008129ca2102d7f4c31d`. Real bone IDs used for authoring (e.g. `human:bone:030` upperarm_r, `031` lowerarm_r, `032` hand_r, `004` spine_03, `054` head).
3. `animation_session_status` — `authoring_enabled: true` (granted by the MCP request itself).
4. `animation_create_draft` — `{"revision":1,"validation":{"state":"valid"}}`.
5. Five `animation_set_bone_keyframes` calls with agent-authored original quaternion/slerp keyframes — revisions advanced to 6. The frontend `create_live_agent_wave()` preset was **not** used.
6. `animation_validate_draft` — state `valid`, no errors.
7. `animation_preview_draft` — preview started in the native Three.js viewport through the existing isolated mixer/selection-preserving path.
8. `animation_export_draft` — document with 5 tracks, 35 keyframes, revision 6.
9. `animation_commit_draft` — approved through the trusted native path, result `{"entry_id":"custom-import-1","revision":7}`. `animations_list` shows `Agent_Wave_01` in the real application library with tags `authored-animation-document`, `rig:human`, `revision:7`, duration 2 s and 5 tracks; distinct from the `default-library` clip `Angry`.
10. `animation_import_draft` + comparison — duration, track identities and keyframes preserved.
11. `animation_undo_commit` — restored prior authored revision and selection; library/list/backend agree.
12. `animation_cancel_session` — session stopped cleanly.

## 4. Native build and verification

- Staged from `worktrees/m2m028c-final/desktop`, built with `cargo build --locked --workspace --all-features --offline` and the matching MCP sidecar build; both exit 0.
- Desktop executable SHA-256 `aec364af008733163e1dfc6247f7c1cf0b66202a1a1f5bf43f8b89c43ed14057`.
- MCP sidecar SHA-256 `3e43a583b5c29db11b0086a2e0e6571633f58873a2b30209ab0c4ad9f177ae15`.
- App main PID `2147847` was alive with WebKit child during the sequence; log `/tmp/opencode/m2m028c/app-final.log` showed `M2M_AUTHORING frontend_ready=true`.
- Frontend quality at `4384a73`: ESLint 0/0, TypeScript 0, Vitest **61 files / 412 tests**, Vite build OK, runtime import cycles 2/2, ast-grep 0 `any`.
- Rust quality inherited from Codex tip `63f1be7`: fmt, workspace tests, MCP tests, denied-warning Clippy all pass.
- Maintained Python: Ruff pass, pytest 4 passed, ty pass (one import-order fix applied to `tools/mcp/test_mcp_stdio_smoke.py` earlier in the session).
- Rechecked on merged integration source after correcting the tool description: `cargo fmt --all --check` pass; locked workspace/all-features tests **4 passed**; MCP all-features tests **52 library + 3 sidecar passed**; Clippy denied warnings passed. Pinned frontend `0505109` gates passed locally: ESLint 0/0, TypeScript noEmit, Vitest **61 files / 413 tests**, Vite build. Maintained Python helper checks after quaternion-rest composition: Ruff pass, pytest **6 passed**, ty pass.

## 5. Native visual evidence limitation

This session cannot drive the KDE Wayland window or capture native screenshots. Evidence is the real
JSON-RPC transcript plus application library state, not a browser tab or a mocked response. A human
visual confirmation of the previewed wave in the viewport is still advisable, but the MCP-created
animation is demonstrably present and selectable in the real app library.

## 6. Remaining limitations / next steps

- No package was built or installed; installed r79 remains the system app.
- The user should visually inspect the staged window if desired before any packaging decision.
- NATIVE-005/006 fixes, 684 posters, hover-only previews and 20/20 ceilings remain covered by existing suites and are unchanged by this integration.
- No upstream push.

---

# M2M-028C addendum — rest-aware wave and internal reference cameras

The user reported that the first committed wave did not actually create a usable new animation and
requested two internal cameras (front and side) so authored positions can be judged correctly.

## Corrections

1. **Rest-aware keyframes:** the maintained MCP client
   (`tools/mcp/agent_wave_mcp_demo.py`) now composes each authored local delta with the **live Human
   bone's `local_quaternion`** from `skeleton_get_catalog` before submitting `slerp` keyframes. The
   earlier identity-based rotations ignored the bind pose and produced a clip that did not read as a
   wave. The frozen document model was not changed; only the values submitted by the agent changed.
2. **Two internal cameras:** frontend branch `m2m-028c-merged` now contains
   `src/authoringBridge/authoringCameras.ts` and `cameraPanel.ts`, adding real front and side
   `THREE.PerspectiveCamera` objects to the app scene with an Orbit/Front/Side native control panel.
   The engine's normal camera is restored exactly on returning to Orbit; a removable frame listener
   drives the view, and `dispose()` removes both cameras and the listener. Default view remains Orbit.
   Three focused tests cover creation, switching/restoration and disposal.

## Exact commits

- Frontend: **`de7c630ee7fc3ba40e3e45964598d2ddce28b798`** (`m2m-028c-merged`, child of `4384a73`).
- Desktop pin: updated on `m2m-028c-final` to gitlink `de7c630`.

## Re-run evidence

A fresh staged app built from the updated pin passed the complete real JSON-RPC sequence again:
`agent-wave-final2-transcript.ndjson` in `coordination/audits/M2M-028/`. The five keyframe revisions
used rest-composed values, validation was valid, preview started, commit inserted
`Agent_Wave_01` as `custom-import-1`, export/reimport matched, undo and cancel succeeded.
`scene_inspect` after the run reported `objectCount: 7` (previous 5), confirming both internal
cameras are present in the live scene. Native screenshot capture is still unavailable.

### Final provenance correction

After the camera tests were hardened for engines without a live camera, the final frontend commit is
**`7f2216876157d276e1094c3dbae38f7588a1c136`** (`m2m-028c-merged`) and the desktop pin is
**`f4ec400422fc91a515558c6b64b0d0456eb630d7`** (`m2m-028c-final`). Final frontend gates:
ESLint 0/0, TypeScript 0, Vitest **62 files / 415 tests**, Vite OK, cycles 2/2. The staged app was
rebuilt from these exact commits; the successful rest-aware MCP sequence evidence above was produced
with `de7c630`, whose only later change was test/robustness hardening with identical runtime CSS/JS
behaviour.
