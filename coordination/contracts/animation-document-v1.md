# AnimationDocument v1 — authoritative frozen contract

**Status:** FROZEN for cross-language implementation. Published by OpenCode M2M-025B.
**Schema version string:** `animation-document/v1`
**Authoritative implementation commit:** `ae48734813fa92788f391a6d1ab4001cae4fb0d7`
**Frontend branch:** `m2m-025b-animation-document` (base `c417eb2`)
**Earlier foundation commit:** `55d7e349181c452ae0dccfe1d785782d1c5127db`

This file is the versioned copy of the agreed interface. The executable source is
`src/animation-document/` in the frontend repo. Any amendment below must be an explicit
new version (`animation-document/v2`) or a documented, agreed note — never a silent change.

## 1. FROZEN schema

```ts
interface AnimationDocument {
  schema_version: 'animation-document/v1'
  rig: { identifier: string; fingerprint: string }
  bones: Array<{
    id: string              // <rig.identifier>:bone:<3-digit Skeleton.bones index>
    display_name: string    // informational only; never an identity key
    index: number           // index in Skeleton.bones
    parent_id: string | null
  }>
  clip: { name: string; duration_seconds: number }
  channels: Array<{
    bone_id: string
    property: 'position' | 'quaternion' | 'scale'
    keyframes: Array<{
      time_seconds: number
      values: number[]      // 3 for position/scale, 4 for quaternion
      interpolation: 'linear' | 'step' | 'slerp'
    }>
  }>
  revision: number          // positive integer, starts at 1
  validation: { state: 'draft' | 'valid' | 'invalid'; errors: string[] }
}
```

## 2. FROZEN rules (do not change silently)

| Rule | Value |
| --- | --- |
| `schema_version` | `animation-document/v1` |
| Rig identifier | string; supported by the adapter: `human`, `fox`, `horse` |
| Rig fingerprint | SHA-256 over ordered bone names, parent indices and local rest transforms quantized to 6 decimals |
| Bone ID | `<rig.identifier>:bone:<zero-padded 3-digit Skeleton.bones index>` |
| Bone identity source | real `Skeleton.bones` sequence, never display names |
| Local transform channels | `position` (XYZ), `quaternion` (XYZW), `scale` (XYZ), relative to parent bone |
| Quaternion convention | XYZW, normalized to unit length |
| Keyframe times | seconds, strictly increasing within a channel, `>= 0`, `<= duration_seconds` |
| Interpolation | `slerp` for quaternion channels, `linear` for position/scale, `step` holds previous value |
| Max channels | **512** |
| Max keyframes (total) | **4096** |
| Max bones | **256** |
| Duration | `0 < duration_seconds <= 3600` |
| Revision | starts at 1; each accepted atomic commit increments by exactly 1 |
| Undo | restores a prior revision as an append-only commit; history is never rewritten |
| Validation | pure and deterministic; identical documents yield identical ordered error lists |

### Frozen invariants
- No NaN or Infinity anywhere; no zero-length quaternion.
- Unknown bone IDs, duplicate bone IDs/indices, self-parents and unknown parents are invalid.
- An incompatible rig fingerprint is a hard error. Renaming, reordering, reparenting or changing the
  rest pose of a rig invalidates documents bound to it; the adapter reports the specific kind and
  **never** falls back to a name-based lookup that could redirect keyframes to another bone.

## 3. Entry points (repository-relative)

| Purpose | Export | File |
| --- | --- | --- |
| Schema/limits/types | `AnimationDocument`, `DOCUMENT_LIMITS`, `ANIMATION_DOCUMENT_SCHEMA` | `src/animation-document/types.ts` |
| Validator | `validate_document`, `with_validation` | `src/animation-document/validation.ts` |
| Live rig/bone catalog | `read_live_rig`, `SUPPORTED_RIG_IDENTIFIERS`, `is_supported_rig` | `src/animation-document/liveRigAdapter.ts` |
| Live document creation | `create_document_from_live_rig` | `src/animation-document/liveRigAdapter.ts` |
| Rig compatibility | `check_rig_compatibility` | `src/animation-document/liveRigAdapter.ts` |
| Draft → Three clip | `draft_document_to_clip` | `src/animation-document/liveRigAdapter.ts` |
| Generic pure conversion | `document_to_animation_clip` | `src/animation-document/conversion.ts` |
| Bone identities/fingerprint | `bone_identities`, `compute_rig_fingerprint`, `stable_bone_id`, `make_bone_id_resolver` | `src/animation-document/rigIdentity.ts` |
| Conventions | `normalize_quaternion`, `slerp_quaternion`, `lerp_vector` | `src/animation-document/conventions.ts` |
| Draft store | `AnimationDocumentStore`, `InMemoryAnimationDocumentStore`, `next_revision_of` | `src/animation-document/store.ts` |
| Isolated preview | `DraftPreviewSession`, `create_engine_preview` | `src/animation-document/preview.ts`, `enginePreview.ts` |
| Commit/cancel/undo | `ApplicationCommitController`, `AnimationApplicationOwner` | `src/animation-document/applicationCommit.ts` |
| Native confirmation | `NativeConfirmationRequest`, `NativeConfirmationResult`, `NativeConfirmationBridge`, `create_confirmation_request` | `src/animation-document/confirmation.ts` |

## 4. Native confirmation contract (`native-confirmation/v1`)

```ts
interface NativeConfirmationRequest {
  schema_version: 'native-confirmation/v1'
  request_id: string
  operation: 'commit' | 'undo'
  clip_name: string
  proposed_revision: number
  target_revision: number | null   // undo target; null for commit
  reason: 'application-owned-animation-mutation'
  created_at_utc: string
}
interface NativeConfirmationResult {
  schema_version: 'native-confirmation/v1'
  request_id: string
  approved: boolean
  decided_by: 'user'
  decided_at_utc: string | null
  rejection_reason?: string
}
interface NativeConfirmationBridge {
  request_confirmation(request: NativeConfirmationRequest): Promise<NativeConfirmationResult>
}
```

Commit and undo require an approved result. Cancel is a local draft discard and needs no confirmation.
A denied confirmation keeps the draft so the user can revise it; no store mutation occurs.

## 5. PROVISIONAL — requires Codex agreement, not frozen

1. Concrete Tauri command name and Rust payload for `NativeConfirmationBridge` (the TypeScript
   request/result shape above is frozen; the transport is not).
2. How `AnimationApplicationOwner` is implemented against `StepAnimationsListing` so the real
   selection id and animation library remain application-owned.
3. Track binding prefix for PropertyBinding (default `''`; the app's skeleton node naming must be
   agreed before native playback of authored clips).
4. Encoding of `step` interpolation for export — current duplicate-time encoding is dropped by
   `ExportAnimationCleanupService.clean_clips_for_export()`.
5. Whether to publish the rig/bone catalog as a new read tool or extend an existing snapshot.
6. MCP write operation names (`create_draft`, `set_keyframes`, `commit`, etc.) and capability scope.

## 6. Proposed amendment log

None. No limit, convention or frozen rule has changed since the first publication
(`animation-document-contract.md`). The live-rig adapter was added as an implementation of the
frozen rules, not an amendment. Corrections made during development:
- `slerp_quaternion` weights corrected from `cos(t·theta)` to `sin((1−t)θ)/sinθ` — a bug fix to match
  the frozen slerp rule, not a convention change.
- Denied confirmation now preserves the draft (documented semantics, no schema impact).

## 7. Version history

| Version | Commit | Note |
| --- | --- | --- |
| `animation-document/v1` | `55d7e34` | schema, validation, conversion, preview, store, fixture |
| `animation-document/v1` (handoff complete) | `ae48734` | live rig adapter, compatibility guards, confirmation + commit controller |
