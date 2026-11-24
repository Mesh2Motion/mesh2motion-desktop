# M2M-025B — Agent_Wave_01 fixture

**Commit:** `55d7e349181c452ae0dccfe1d785782d1c5127db` on `m2m-025b-animation-document`.
**File:** `src/animation-document/fixtures/AgentWave01.ts` (tests: `AgentWave01.test.ts`).

## Origin

The clip is authored from scratch. No existing `AnimationClip`, FBX, GLB or retargeted clip is read,
copied or used as a source. Every keyframe value is computed here from an authored Euler angle via
`Quaternion.setFromEuler()` and normalized through `normalize_quaternion()`.

## Rig

`create_agent_wave_skeleton()` builds a deterministic minimal Human upper-body skeleton so the fixture
is self-contained and reproducible. Bone order (and therefore stable ID index):

| Index | Bone | Parent |
| --- | --- | --- |
| 0 | hips | — |
| 1 | spine | hips |
| 2 | chest | spine |
| 3 | neck | chest |
| 4 | head | neck |
| 5 | shoulder.L | chest |
| 6 | upperarm.L | shoulder.L |
| 7 | forearm.L | upperarm.L |
| 8 | hand.L | forearm.L |
| 9 | shoulder.R | chest |
| 10 | upperarm.R | shoulder.R |
| 11 | forearm.R | upperarm.R |
| 12 | hand.R | forearm.R |

Rig identifier `human`; stable IDs are `human:bone:000` … `human:bone:012`.

## Clip

- Name: `Agent_Wave_01`
- Duration: **2.000 s**, revision 1, validation state `valid`
- **5 quaternion channels, 35 rotation keyframes total**, all `slerp`:
  - `upperarm.R` (index 10) — 9 keys, raises and sweeps side-to-side twice (−10° → −55° → −70° → … → −10°).
  - `forearm.R` (index 11) — 9 keys, alternating elbow bend.
  - `hand.R` (index 12) — 5 keys, wrist flick at each sweep extreme.
  - `chest` (index 2) — 5 keys, subtle counter-rotation.
  - `head` (index 4) — 5 keys, counter-rotation.

## Tests

`AgentWave01.test.ts` asserts:
- originality/determinism (two independently built documents are identical),
- two-second duration, Human rig, revision 1, all-quaternion/slerp channels,
- validity through `validate_document()`,
- JSON export/reimport transform equivalence at 0 / 0.25 / 0.5 / 1 / 1.5 / 2 s,
- transform equivalence through the existing `ExportAnimationCleanupService.clean_clips_for_export()`
  pipeline using the skeleton root as export root.

All 5 fixture tests pass as part of 54 files / 339 tests.

## Visual acceptance

**BLOCKED** in this session — no native GUI control available. To accept manually: load the fixture
document, call `create_engine_preview(engine).show(clip)` from `enginePreview.ts`, confirm a two-second
right-arm upper-body wave in the Tauri window, and confirm the selected animation in the existing
listing is unchanged while and after the draft plays.
