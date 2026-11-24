# M2M-030B — MCP authoring/observation gaps and GUI-observation fix

Date: 2026-11-24
Branches: desktop `m2m-030b-integration`, frontend `m2m-030b-integration-frontend`

## User requirement

The agent must author with the **New Animation** tab visible so the user can observe the procedure
and correct or modify the animation before saving. Mouse/keyboard injection into the desktop is a
**no-go** — it hijacks the user's pointer, is slow, and the app runs on native Wayland where X11
automation cannot see it.

## MCP problems found (all observed live, not hypothetical)

| # | Problem | Root cause | Fix / status |
| --- | --- | --- | --- |
| 1 | Agent cannot know which local bone axis raises an arm, flexes an elbow or turns a palm. Repeated attempts produced contorted elbows and a non-waving pose. | MCP exposed raw bone IDs and quaternions with **no anatomical conventions**; tool descriptions were minimal. | CLOSED — desktop `67d4f85d` publishes validated Human conventions in `animation_set_bone_keyframes` (rest·delta, `upperarm_l.z` raises, `lowerarm_l.z` **negative** ≈ −60° flexes forward, `hand_l.y` ±30° rocks the palm) and in `animation_observe_pose` (world-aligned camera directions, Human faces +Z so `front` can show the back). Verified in a real `tools/list` exchange. |
| 2 | MCP draft changes did not refresh an already-open New Animation tab; the tab showed "no active draft". | `SharedDraftController.attach()` subscribed to the bridge only if the bridge existed at construction; rig loading is asynchronous. | CLOSED — `FrontendAuthoringBridge.notify_change()` now emits a UI-only `m2m-authoring-document-changed` CustomEvent; the controller listens to it independently of bridge timing. No authorization meaning, no payload. |
| 3 | No MCP way to switch the visible page, so an agent could not make the GUI observable without hijacking the mouse. | No authoring operation or tool existed for navigation. | CLOSED — new `application.set_page` authoring operation and `application_set_page({page})` MCP tool. `new-animation` navigates to `/newAnimation/index.html`, `explore` to `/index.html`. Navigation is deferred 150 ms so the IPC reply is sent first. |
| 4 | First implementation of #3 timed out with `SESSION_CONFLICT` / no reply. | `authoring_call` reused a constant request ID and sequence; the host guard also treated the new op as session-only. | CLOSED — `authoring_call` now uses session-scoped unique request IDs and an increasing sequence; `ApplicationSetPage` is a session-free (read-like) operation in the host guard. |
| 5 | Stale integration `dist` made WebKit spin at ~37% CPU with no socket or `M2M_HOST` log — the app looked broken. | The worktree still contained the reverted chunk-split bundle. | CLOSED — rebuilt `dist` from the final frontend HEAD and re-embedded it (Tauri needs a rebuilt bundle copied into `mesh2motion-app/dist`, not just a Vite build). |
| 6 | Authored clip tiles could show "No preview". | Preview frames were generated only during startup restore, after tiles had rendered. | CLOSED earlier — commit-time generation/registration/persistence (`98fcf60`) plus catalog refresh (`657e418`). |
| 7 | `RIG_UNAVAILABLE` / stale skeleton and non-deterministic captures. | Observation service held an early fallback skeleton; transient null rig snapshots disposed the service; unauthored bones kept the playing animation. | CLOSED earlier — live-skeleton re-read per capture, debounced teardown, dynamic adapter dispatch, bind-pose reset (`98fcf60`). Repeat captures are byte-identical. |

## Verification of the GUI-observation path

- Real MCP `tools/list` exposes the new `application_set_page` tool.
- `application_set_page({page:"new-animation"})` returns `{"data":{"page":"new-animation"},"ok":true}`.
- `application_get_status` afterwards reports the New Animation page's published value `unknown`
  (the frozen Rust enum accepts `unknown` for that page), confirming navigation.
- No X11 pointer or keyboard injection is used for the authoring workflow; Wayland remains the normal
  session. `xdotool` was used only diagnostically to confirm why native Wayland cannot be automated
  that way, never as the solution.

## New Animation keyframe editing fix (user-reported critical bug)

The user reported that editing a bone's placement did not override the selected keyframe in the graph,
and that `- Keyframe` could not remove an upper-arm key. Root cause: the value inputs, interpolation
selector, gizmo writes and `remove_keyframe()` all used the **playhead time**, while the displayed
values came from that time (interpolated), ignoring the keyframe the user had selected in the graph.

Fixed in `src/newAnimation/keyframeEditor.ts`:
- `selected_key()` resolves the actually selected keyframe (time, values, interpolation).
- Value inputs, interpolation changes and gizmo edits now write to the selected keyframe's exact time
  and display its exact stored values; they no longer create a stray keyframe at the playhead.
- `- Keyframe` removes the selected keyframe, falling back to the nearest keyframe only when none is
  selected, and re-renders the graph.
- Regression test added: with the playhead at 0 s and keyframe 0.5 s selected, an edit overwrites
  exactly 0.5 s (3 keys remain) and removal leaves keys `[0, 1]`. Frontend gates: 71 files / 480
  tests, tsc, eslint, Vite all pass.

## GUI edits rejected while an MCP session is active (second editor bug)

The user still could not edit `upperarm_l` in the New Animation tab; no GUI edit requests appeared in
the app log at all. Root cause: after an MCP draft is created, the bridge's single authoring session
is `mcp-…`, while `SharedDraftController` sent GUI edits under its own `gui-…` session ID. The
dispatcher rejected every GUI operation as a competing session before any IPC was emitted.

Fixed:
- `AuthoringDispatcher.peek_sequence()` and `FrontendAuthoringBridge.active_session_id()` /
  `peek_authoring_sequence()` expose the active session and its next legal sequence.
- `SharedDraftController.operation()` now edits under the **active** session (MCP or GUI) and uses
  that session's next sequence, so GUI and MCP mutate the same authoritative draft.
- Regression test: with a fake MCP session owning the bridge at sequence 4, a GUI create succeeds and
  advances that session's sequence. Frontend gates: 71 files / 481 tests, tsc, eslint, Vite pass.

## MCP camera-orientation analysis (user question, 2026-11-24)

Measured from a real six-view observation (`/tmp/opencode/camera-orientation/report.json`):

| View | Camera position | Looks | Up |
| --- | --- | --- | --- |
| front | (x, 1.16, −5.29) | +Z | +Y |
| back | (x, 1.08, +5.38) | −Z | +Y |
| left | (+4.96, 1.12, z) | −X | +Y |
| right | (−5.72, 1.12, z) | +X | +Y |
| top | (x, +6.45, z) | −Y | +Z |
| bottom | (x, −4.22, z) | +Y | −Z |

The rig's own facing comes from foot→ball geometry: `foot_l` world z = −0.048, `ball_l` world z =
+0.113, so the Human model faces **+Z**. Left arm is +X, right arm is −X. Therefore:
- `left`/`right`/`top`/`bottom` are consistent with the rig's own axes.
- **`front` and `back` are inverted relative to the rig's facing**: `front` (world −Z) sees the
  model's **back**, and `back` (world +Z) sees the model's **front**. The app's viewport camera is at
  +Z, so it matches MCP `back`, not `front`.

This is the exact cause of my repeated orientation failures: I judged the palm in `front` images,
which show the back of the model, so every "palm toward camera" correction turned the palm away from
the real viewer. A controlled probe in the `back` view showed `hand_l.y ≈ −60°` gives an open palm
facing the model's front/viewer, while `+60°` points it away. Current tool descriptions warn about
this, but the durable fix is a documented contract amendment adding rig-forward-relative view aliases
(or swapping the world direction of `front`/`back`); until then, use `back` for face/palm checks.

## Reconnect stall fix

A reconnecting MCP client previously stalled the app because the frontend bridge kept the prior
session and the new sidecar received `SESSION_CONFLICT` for every request. `Mesh2MotionMcp::authoring_call`
now detects a session conflict, sends a trusted `session.cancel` (accepted from any native request by
the frontend), and retries the original operation once. Verified with two sequential real JSON-RPC
clients: both returned `ok:true` with the live 66-bone catalog.

## Save/restore root cause — RESOLVED (2026-11-24 03:50)

Switching the native app to New Animation produces an equivalent 66-bone skeleton whose ~1e-7 float
noise tips 6-decimal fingerprint quantization (`1a167d80…` → `e02f5440…`, IDs/hierarchy/names
identical). Documents saved in that page therefore failed restore on the Explore page.

Proper fix (no restore-time hash rewriting):
- `AnimationDocument` gained additive optional `rest_transforms`, filled from the live bind pose.
- `check_rig_compatibility` compares stored vs live bind-pose transforms with 1e-4 tolerance;
  structural identity (IDs/order/parents/names) remains authoritative. A genuine rest-pose change
  beyond tolerance still fails; float noise does not.
- `validate_document` accepts `fingerprint_equivalent` only when compatibility proved equivalence.
- All 9 rigs are selectable; `Create clip` no longer wipes an open draft when the name field changes.

Verified: fresh MCP clip `Wave Final 4` (66 tracks, revision 7) committed, app restarted, and
`animations_list` returned it from the real library. All mandatory tools pass (see handoff).

## Remaining limits

- Bacon watcher does not terminate on its own; Cargo `check` inside it succeeds.
- BugStalker 0.4.9 is available; only availability/smoke evidence, no active fault to debug.
- Final aesthetic sign-off belongs to the user in the New Animation tab. The agent must not commit a
  clip until the user confirms the pose in that tab.
