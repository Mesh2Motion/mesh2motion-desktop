# M2M-025B — AnimationDocument contract (for Codex)

Published early from the isolated frontend branch `m2m-025b-animation-document` (base `c417eb2`).
This is the interface contract for future MCP authoring; it does not claim that MCP write tools work.

## Schema (version `animation-document/v1`)

```ts
interface AnimationDocument {
  schema_version: 'animation-document/v1'
  rig: { identifier: string; fingerprint: string }
  bones: Array<{ id: string; display_name: string; index: number; parent_id: string | null }>
  clip: { name: string; duration_seconds: number }
  channels: Array<{
    bone_id: string
    property: 'position' | 'quaternion' | 'scale'
    keyframes: Array<{ time_seconds: number; values: number[]; interpolation: 'linear' | 'step' | 'slerp' }>
  }>
  revision: number
  validation: { state: 'draft' | 'valid' | 'invalid'; errors: string[] }
}
```

## Bone-ID contract

- Stable IDs are derived from the **actual current rig**, not from display names:
  `<rig.identifier>:bone:<zero-padded skeleton index>` (for example `human:dark:bone:005`).
- `bones[].index` is the index in `Skeleton.bones`; `parent_id` is the parent bone's stable ID or null.
- `bones[].display_name` is informational only. Renaming a bone does not change its stable ID.
- The rig fingerprint is a deterministic SHA-256 over ordered bone names, parent indices and local rest
  transforms. A document is only convertible when its `rig.fingerprint` matches the live rig.

## Coordinate / interpolation rules

- All times are seconds, strictly increasing within a channel, and <= `clip.duration_seconds`.
- Positions/scales are three-component XYZ; quaternions are four-component XYZW-normalized (x,y,z,w),
  stored with unit length; `quaternion` channels use slerp, others linear; `step` is hold-previous.
- Transforms are **local bone transforms** relative to the parent bone, matching Three.js
  `PropertyBinding` semantics (`<boneName>.position|quaternion|scale`).

## Revision semantics

- `revision` starts at 1 and increments by exactly 1 on every accepted atomic commit.
- A rejected commit leaves the revision and stored document unchanged.
- `undo(revision)` restores the document snapshot of that revision; it is itself an atomic commit and
  increments the revision again (history is append-only, never rewritten).
- Validation is pure and deterministic: identical documents always produce identical error lists.

## Limits (v1)

- Max 512 channels, 4096 keyframes total, 256 bones per document, duration 0 < d <= 3600 s.
- No NaN/Infinity, no unknown bone IDs, no mismatched fingerprint, no empty clip name.
