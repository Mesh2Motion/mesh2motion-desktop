# M2M-025B — Original AnimationDocument and Three.js authoring adapter

**Branch:** `m2m-025b-animation-document` (isolated frontend worktree at
`worktrees/m2m025b-animation/mesh2motion-app`), based on the M2M-025A baseline `c417eb2`.
**Commit:** **`55d7e349181c452ae0dccfe1d785782d1c5127db`**.
**Contract published before implementation:** `coordination/reports/M2M-025/animation-document-contract.md`.

No Rust, MCP sidecar, package, installed application, preview scheduler, poster path, Props layout,
rigging panel layout or settings CSS was touched. No existing animation clip was used as a source.

## 1. Delivered interfaces (for Codex)

All under `src/animation-document/`:

| File | Exports |
| --- | --- |
| `types.ts` | `AnimationDocument`, `AnimationDocumentRig`, `AnimationBoneIdentity`, `AnimationChannel`, `AnimationKeyframe`, `AnimationDocumentValidation`, `DocumentCommitReceipt`, `BoneIdResolver`, `ANIMATION_DOCUMENT_SCHEMA`, `DOCUMENT_LIMITS` |
| `rigIdentity.ts` | `stable_bone_id(rig, index)`, `compute_rig_fingerprint(Skeleton)`, `rig_descriptor(Skeleton, id)`, `bone_identities(Skeleton, id)`, `make_bone_id_resolver(bones)` |
| `sha256.ts` | `sha256_hex(string)` — synchronous, deterministic FIPS 180-4 |
| `conventions.ts` | `normalize_quaternion`, `identity_quaternion`, `slerp_quaternion`, `lerp_vector` |
| `validation.ts` | `validate_document(doc, { live_fingerprint? })`, `with_validation(doc, options)` |
| `create.ts` | `create_empty_document(options)`, `with_channel(doc, ...)`, `with_animation_channel(doc, channel)` |
| `conversion.ts` | `document_to_animation_clip(doc, { resolve_bone_name, binding_prefix?, require_valid? })` |
| `preview.ts` | `DraftPreviewSession`, `PreviewHost` |
| `enginePreview.ts` | `create_engine_preview(engine)`, `PreviewEngine` |
| `store.ts` | `AnimationDocumentStore`, `InMemoryAnimationDocumentStore`, `InvalidDocumentCommitError`, `RevisionMismatchError`, `RevisionNotFoundError`, `next_revision_of` |
| `fixtures/AgentWave01.ts` | `create_agent_wave_01_document()`, `create_agent_wave_skeleton()`, constants |

## 2. How the requirements were met

1. **Schema v1** — version string, rig identifier + SHA-256 fingerprint, stable bone IDs with parent
   links, clip name/duration, position/quaternion/scale channels, ordered keyframes with interpolation,
   integer revision and a computed validation state.
2. **Stable bone identity** — `stable_bone_id()` uses the index in `Skeleton.bones`, not `bone.name`.
   The fingerprint covers ordered names, parent indices and quantized (6-decimal) local rest TRS.
   Renaming a display name keeps the ID; changing structure or rest pose changes the fingerprint.
3. **Conventions** — times in seconds; local XYZ / XYZW transforms; unit quaternions; quaternion
   channels slerp, position/scale linear, `step` holds until the next keyframe. Documented in
   `conventions.ts` and the published contract.
4. **Deterministic validation** — rejects unknown/invalid bones, NaN/Infinity/zero-length quaternions,
   negative/decreasing/out-of-range times, invalid duration/name/schema, wrong value arity, oversized
   documents (256 bones / 512 channels / 4096 keyframes / 3600 s) and an incompatible live fingerprint.
   Identical documents always produce identical error lists; validation never mutates its input.
5. **Pure conversion** — `document_to_animation_clip()` creates a standalone `AnimationClip` with
   `QuaternionKeyframeTrack` / `VectorKeyframeTrack` and an optional PropertyBinding prefix. No app state.
6. **Isolated preview** — `DraftPreviewSession` uses its own `AnimationMixer` and the engine's
   `add_frame_listener`. `create_engine_preview()` is the only bridge to the app; it pauses/resumes the
   engine's player through its own API and never calls `StepAnimationsListing.play_animation()` or
   `AnimationPlayer.set_animation()`. `current_playing_id` and the animation library are untouched.
7. **Preservation** — no existing source file was modified; all previous functionality is unchanged.
8. **Atomic commit/undo** — `AnimationDocumentStore` defines the contract; `InMemoryAnimationDocumentStore`
   is a reference implementation with validation-before-mutation, exact `+1` revision checks, deep-copy
   isolation and append-only undo. It is explicitly *not* wired into the live app: integration with
   `StepAnimationsListing` remains an application-ownership decision for the next cycle.
9. **Fixture** — see `agent-wave-fixture.md`.
10. **Export/reimport** — JSON round-trip and `ExportAnimationCleanupService.clean_clips_for_export()`
    transform-equivalence tests both pass.

## 3. Bug found and fixed during development

The first `slerp_quaternion()` implementation used `cos(t·theta)` as the first weight. The
transform-equivalence test caught a ~0.128 component error at the midpoint of a −10°→−55° wave.
Corrected to `sin((1−t)·theta)/sin(theta)` and `sin(t·theta)/sin(theta)`; it now matches Three's
`QuaternionKeyframeTrack` slerp to 4 decimals. This is why the equivalence test is valuable.

## 4. Known limitation (not fixed here)

`step` channels are encoded with duplicate keyframe times. The existing
`ExportAnimationCleanupService` drops non-increasing times before export, so a `step` track would lose
its hold when exported. The Agent_Wave_01 fixture uses only `slerp`, so this does not affect it.
A future fix should either encode `step` with a tiny epsilon or teach the cleanup service about
document-authored step tracks.

## 5. Quality results (commit `55d7e34`)

| Check | Result |
| --- | --- |
| ESLint `--max-warnings 0` | 0 errors / 0 warnings |
| TypeScript `tsc --noEmit` | 0 errors |
| Vitest (full suite) | **54 files / 339 tests passed** (baseline 48 / 299) |
| Vite build | exit 0 (pre-existing >500 kB chunk warning) |
| Runtime import cycles | 2/2 passed |
| ast-grep explicit `any` in new folder | 0 |
| Native Tauri visual acceptance | **BLOCKED** — no GUI automation in this session |

## 6. Native acceptance status

**BLOCKED.** The installed application was not modified and no new package was built. Visual
acceptance of the authored wave in the real Tauri 2 / WebKitGTK window requires manual playback of the
draft preview; this session cannot observe the Wayland window. No browser tab was substituted.

## 7. STOP

Domain, conversion, isolated preview and atomic commit/undo foundations pass their tests. The full MCP
authoring workflow is **not** claimed to work; it requires a separate integration cycle with Codex.

---

# Update — authoritative handoff commit `ae48734813fa92788f391a6d1ab4001cae4fb0d7`

The foundation commit `55d7e34` was followed by the handoff commit `ae48734` on the same branch
`m2m-025b-animation-document`. Nothing was restarted or discarded.

## Added in `ae48734`

| File | Exports |
| --- | --- |
| `liveRigAdapter.ts` | `read_live_rig`, `SUPPORTED_RIG_IDENTIFIERS`, `is_supported_rig`, `UnsupportedRigError`, `create_document_from_live_rig`, `check_rig_compatibility`, `draft_document_to_clip` |
| `confirmation.ts` | `NATIVE_CONFIRMATION_SCHEMA`, `NativeConfirmationRequest`, `NativeConfirmationResult`, `NativeConfirmationBridge`, `create_confirmation_request`, `DenyConfirmationBridge` |
| `applicationCommit.ts` | `ApplicationCommitController`, `AnimationApplicationOwner`, `CommitOutcome`, `ApplicationCommitDependencies` |

### Live rig adapter safety
`read_live_rig()` reads the real `Skeleton.bones` order, parent links and local rest transforms and
returns an immutable catalog with a SHA-256 fingerprint. `check_rig_compatibility()` reports
`renamed`, `reordered-or-missing-bones`, `reparented`, `rest-transform-changed`, `bone-count-changed`
or `unsupported-rig`; `draft_document_to_clip()` refuses conversion on any mismatch. There is no
name-based fallback, so a changed rig cannot silently redirect keyframes to another bone.

### Application-owned commit/cancel/undo
`ApplicationCommitController` works on a separate draft, previews it with an isolated mixer, and
requires `native-confirmation/v1` approval for commit and undo. `AnimationApplicationOwner` is injected:
the controller never calls `StepAnimationsListing.play_animation()`, never edits the animation library
and never touches Remix/composer state. Denied confirmation keeps the draft; explicit `cancel()`
discards it and restores the captured application selection.

### Updated quality
ESLint 0/0, TypeScript 0, Vitest **56 files / 358 tests**, Vite build OK, runtime import cycles 2/2.
All are JSDOM/Vitest tests — **no native GUI test** was performed.

### Contract publication
- Versioned frozen contract: `coordination/contracts/animation-document-v1.md`.
- Codex notification: `coordination/handoffs/M2M-025B-to-Codex.md`, with pointers appended to
  `mcp-authoring.md` and `agent-wave-end-to-end.md`.
- Frozen: schema, limits, conventions, bone-ID/fingerprint rules, revision/undo semantics.
- Provisional: confirmation transport, `AnimationApplicationOwner` wiring, PropertyBinding prefix,
  `step` export encoding, catalog read-tool shape, MCP write op names.
