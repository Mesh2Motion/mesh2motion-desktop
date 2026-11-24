# M2M-031 — New Animation workspace (frontend)

## Source and provenance

| Item | Value |
| --- | --- |
| Branch | `m2m-030-031-observation-animation` |
| Worktree | `worktrees/m2m030-031-observation/frontend` |
| Implementation commits | … `d773d85` (persistence + guarded deletion), `6227d98` (persist MCP clips), **`e484c32`** (previews + delete confirm) |
| Baseline | `75aae01` (natively verified view-helper fix) on `0505109` |
| Staged desktop pin | `m2m-030-031-integration` @ **`47962b6`**, frontend gitlink `0920f70` |
| Installed app / canonical branches / Rust | untouched |

## Top-level tab

A new top-level page `src/newAnimation/index.html` ("New Animation") is linked from
all four existing pages and marks itself active. It is registered as a Vite build
input; `dist/newAnimation/index.html` is emitted. Remix and Use Your Rigged Model
remain unchanged and independently reachable. The normal Three.js viewport and
`CustomViewHelper` are preserved; no observation camera, button, label or overlay is
shown in this tab (or anywhere else in the UI).

The page bootstraps `Mesh2MotionEngine`, loads a built-in rigged model through the
existing `GLTFLoader`/`RigConfig` assets (Human, Fox, Horse), owns its skinned meshes
through `set_active_skinned_meshes_provider`, and installs the existing desktop/authoring
bridge via `maybe_attach_desktop_control`. It uses the Rust-compatible `unknown` page
value because the frozen Rust snapshot enum has no `new-animation` variant (found and
fixed during the staged native launch; no Rust file changed).

## Two creation modes

- **From Scratch** — `SharedDraftController.create_from_scratch(name, duration)` calls
  `draft.create` on the live bridge, producing an empty original `AnimationDocument`
  bound to the live rig. No source animation is consulted.
- **From Existing Animation** — `create_from_existing` calls `draft.import` with a
  document explicitly built by `clipImport.import_clip_as_document()` from the
  selected library clip. Tracks are mapped by live bone display names, quaternions
  are normalized, duplicate step times are collapsed, unmatched tracks are reported
  as skipped (never invented). Source name is preserved via
  `FrontendAuthoringBridge.set_source_attribution` → `RealApplicationOwner` library
  tag `source:<name>` where metadata is supported.

The library list reuses `AnimationLibrary`/`AnimationLoader` with a search filter, so
Remix-style animation search and the real application library stay the same source.
No autoplay: preview starts only on explicit button click.

## Keyframe editor

`src/newAnimation/keyframeEditor.ts` provides:

- Track/channel list (bone display name · property · keyframe count), selectable.
- Keyframe timeline with current-time cursor (range input) and clickable keyframe markers.
- Add/remove keyframe at the current time; local position/quaternion/scale numeric
  editing; explicit interpolation select (`linear`/`step`/`slerp`).
- Undo/redo of draft edits, preview toggle, save/export draft, import saved document,
  commit and cancel.
- Bone selection/transform gizmos and skeleton visualization come from the existing
  engine/rigging step modules; the editor does not reimplement them.

Keyframe values are evaluated with the same `evaluate_document_at_time()` helper used
by MCP observation, so the editor, preview and observation all read one pose model.

## One authoritative document (critical requirement)

`src/animation-document/sharedDraft.ts` — `SharedDraftController` dispatches **every**
GUI action through the installed `FrontendAuthoringBridge.handle_gui_request()`. That
bridge owns the single `InMemoryAnimationDocumentStore` MCP also uses. There is no
GUI-only document, no timeline-only source of truth, and no second store.

- Revision tracking comes from that store; `expected_revision` is checked before every
  atomic edit, so stale MCP/GUI revisions are rejected rather than overwriting edits.
- `bridge.on_change()` propagates MCP mutations to the GUI through `attach()`; the
  workspace and keyframe editor re-render on the same event.
- GUI-originated operations bypass only the MCP permission gate — they do not grant
  MCP permission and do not expose the bridge on `window`.
- Undo/redo of editor edits re-imports prior snapshots as new revisions (append-only);
  MCP `draft.undo` remains the committed-library undo. Commit uses the existing
  compensating rollback and application-owned library insertion.
- `compose_rest_delta()` composes an authored local rotation delta with the live bone's
  rest quaternion (`rest * delta`, XYZW) instead of replacing the bind pose, addressing
  the first-wave pose failure without changing the frozen document convention.

## Mouse bone manipulation (added after user feedback)

The first workspace version lacked the rigging tab's mouse interaction. Commit
`9e59737` reuses the existing machinery on the **live GLB skeleton**:

- `Mesh2MotionEngine.transform_controls` (the same `CustomTransformControls` used by
  Use Your Model) is enabled on this page; `Utility.raycast_closest_bone_test` performs
  screen-space joint picking on the live `Skeleton.bones` (root excluded).
- Clicking a joint attaches the gizmo to that live bone and shows its name. Move /
  Rotate / Scale buttons call `setMode`, and the Local/World select calls `setSpace`.
- On gizmo drag end (`dragging-changed` false) the bone's actual local
  position/quaternion/scale is read and written as a keyframe at the current timeline
  cursor through `SharedDraftController` (same authoritative document, revision-checked).
- The timeline cursor now evaluates the authored pose onto the live rig via
  `apply_evaluated_pose`, so gizmo edits start from the correct pose; preview is stopped
  automatically before editing so the mixer cannot overwrite the drag.
- No clone armature is created: the live `SkinnedMesh.skeleton` is edited directly, so
  the authored transform and the viewport stay in sync.
- A searchable live-bone list (`upperarm_l`, `lowerarm_r`, …) provides deterministic
  selection; viewport picking remains as a convenience.
- Fix after user feedback: clicking the left elbow selected the right arm because the
  shared `Utility.normalized_mouse_position()` uses `window.innerWidth/innerHeight`
  against a sidebar-shifted canvas. The New Animation picker now computes NDC from
  `renderer.domElement.getBoundingClientRect()`, the same class of fix as the earlier
  XYZ-helper anchoring fix. This is local to this page; the shared utility is untouched.

## Clip editor rework (user feedback, commit `4e30909`)

The original minimal timeline was replaced with a Godot-style clip editor:

- **Time ruler** with second ticks/labels; click or drag to scrub.
- **One lane per channel** with diamond keyframes at their exact time; the lane
  name column is aligned with the lanes. Click a keyframe to jump to its time,
  click a lane to scrub, double-click a lane to add a keyframe there.
- **Playhead** marker spanning all lanes, updated on every scrub/playback frame.
- **Transport**: play/pause and stop/rewind buttons. Playback uses the engine's
  existing frame listener to advance the cursor and `evaluate_document_at_time`
  to drive the real rig, so the viewport shows the actual motion at the playhead.
- **Bone list** CSS fix: rows no longer collapse to slivers (`flex-shrink: 0`,
  `min-height`), and the search field uses the themed surface.

Four focused tests cover ruler/lane/marker rendering, playhead movement and pose
application, playback listener registration/removal, and the empty state.
Commit `7bb3a30` added Godot-style **keyframe retiming**: diamonds can be dragged
horizontally on their lane to change the keyframe time; the move is committed through
the shared draft controller with strict ordering and overlap rejection. The ruler
supports press-drag scrubbing, and pointer capture is guarded for WebKit/JSDOM.

Commit `06b5edc` fixed the decisive blocker: the page loads pre-rigged GLBs
directly, so `load_skeleton_step.skeleton_type()` stayed `none` and the bridge
rejected `draft.create` / keyframe writes with `unsupported rig identifier "none"`.
The page now calls `set_skeleton_type()` for the selected rig on load, so Create
clip and + Keyframe work.

Commit `04e9293` followed user feedback that adding keyframes did not work:
`+ Keyframe` now lives in the visible transport toolbar, auto-starts an empty
draft when none exists, and writes the **live bone's actual local transform**
instead of an identity quaternion. Selecting a bone or changing gizmo mode focuses
the matching channel. The stray green vertical playhead is hidden and an empty
hint is shown until the first keyframe exists (fifth focused test added).

## Save, Explore inspection and protected deletion (commits `d773d85`, `6227d98`)

- **Save clip** is now the explicit library-commit action (previously the ambiguous
  "Commit animation"); "Export JSON" remains a separate file download. Save commits
  the authoritative document through the bridge into the application-owned library
  and persists it locally (bounded to 20 clips / 4 MiB).
- **Explore inspection**: persisted authored documents are rehydrated after rig load
  into the engine's `StepAnimationsListing` via `restore_authored_documents`, using
  the frozen rig-fingerprint compatibility check (mismatched rigs are skipped, never
  remapped). They appear as custom tiles with a "C" badge and are playable.
- **MCP clips** are persisted too from `RealApplicationOwner.apply_committed_clip`
  (deduplicated by name/revision/channel count), so `mcp_jumping jack` is inspectable
  and deletable after a fresh launch.
- **Deletion protection**: `StepAnimationsListing.delete_animation` only accepts
  entries with `source_type: 'custom-import'`, no `file`, and a generated
  `custom-import-` id. Bundled manifest animations are refused (`protected: true`)
  and have no delete button. Explore custom tiles get a ✕ button; New Animation has
  a "Saved clips" list with per-clip Delete.
- Four focused tests cover protection, persistence round-trip, id tags, and idempotent
  library restoration.

## Authored clip previews and delete confirmation (commit `e484c32`)

- Custom/authored tiles previously showed "No preview". They now get a real PNG
  poster and a hover flipbook (8 frames) rendered from the live rig through the
  same internal camera/offscreen-capture path used by MCP observations. Frames are
  persisted with the document and mapped to the restored library entry id.
- The Explore ✕ button now asks `window.confirm("Delete …? This cannot be undone.
  Bundled animations are protected.")` before deleting. Cancelling leaves the clip
  untouched; bundled entries remain unreachable through this path.
- Verified on the staged native build: after relaunch, the persisted
  `mcp_jumping jack` was automatically restored into the library (the demo client
  reported it already existed) with no errors in the app log.

## Tests and real results

| Check | Result |
| --- | --- |
| ESLint `--max-warnings 0` | 0 / 0 |
| TypeScript `tsc --noEmit` | 0 errors |
| Vitest full suite | **67 files / 454 tests passed** |
| Vite production build | exit 0; new page emitted |
| Runtime import cycles | 2 / 2 |
| ast-grep explicit `any` | 0 matches |

New focused tests: empty original document creation, explicit existing-animation import,
track copy and source attribution, timeline revisions, stale-revision conflict after an
MCP mutation, subscriber notification, editor undo/redo, export/reimport equivalence,
preview lifecycle, rest-aware rotation composition (non-identity rest, shoulder/arm,
parent-child, finger joints, rig switching), and clip-import normalization/validation.

## Final native preview confirmation (2026-11-23)

The user confirmed the authored animation preview on the staged native build. Chat
evidence was supplied as `Bildschirmfoto_20261123_215020.png`; the retained matching
workspace screenshot is `artifacts/M2M-031/screenshots/06-working-preview-verified.png`.
The earlier authored-preview visual acceptance blocker is resolved. This does not
verify the separate six-camera MCP observation transport end to end.

## Known limitations

- The editor edits local transform channels of the loaded rig; it does not expose the
  full per-bone IK/gizmo internals of the rigging page (those remain in that page).
- "From Existing" copies keyframe tracks, not arbitrary clip metadata that has no v1
  document field; source attribution is preserved as a library tag where supported.
- Native visual acceptance of the tab is recorded separately in `native-acceptance.md`.
