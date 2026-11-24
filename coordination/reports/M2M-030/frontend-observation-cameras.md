# M2M-030A — Frontend internal MCP pose observation cameras

## Source and provenance

| Item | Value |
| --- | --- |
| Frontend branch | `m2m-030-031-observation-animation` (persistent, isolated) |
| Frontend worktree | `worktrees/m2m030-031-observation/frontend` |
| Contract publication commit | **`9683e96`** (`M2M-030: publish mcp-pose-observation/v1 shared contract`) |
| Camera implementation commit | **`d4fd2b5`** (`M2M-030A: internal six-camera MCP pose observation`) |
| Final frontend commit (both features) | **`32e4db9`** (runtime identical to `badb511`, which fixed the themed rig-picker select) |
| Safe baseline | `75aae01ae729edc1cec7612fcc382828960bde82` (natively verified XYZ helper fix), itself on `0505109` exact-request-scoped trusted MCP approval |
| Canonical branches / installed app / Rust | untouched; no package built or installed |

The rejected visible-camera prototypes (`de7c630`/`7f22168` OpenCode, `558ee2f` Codex)
were inspected for reusable framing math only. Their Front/Side panels, labels, CSS
and camera-into-visible-scene behaviour were **not** restored. `75aae01` was fetched
from the Codex/submodule frontend object store into the canonical store before basing
the branch on it.

## Shared contract

- Canonical: `coordination/contracts/mcp-pose-observation-v1.md`
- Version-controlled copy: `docs/contracts/mcp-pose-observation-v1.md` (commit `9683e96`)
- Handoff: `coordination/handoffs/M2M-030-031-observation-channel.md`
- Protocol `mcp-pose-observation/v1`; 19 closed error codes; 256 KiB UTF-8 envelope;
  views front/back/left/right/top/bottom; pose digest; joint/occlusion format;
  `image_ref` byte-fetch split. Transport event names are PROVISIONAL pending Codex.

## Implemented frontend API (`src/authoringObservation/`)

| File | Role / exports |
| --- | --- |
| `types.ts` | Frozen TS mirror of the contract: `OBSERVATION_PROTOCOL`, `OBSERVATION_VIEWS`, `OBSERVATION_LIMITS`, request/response/result/joint types, `ObservationError`, `utf8_byte_length` |
| `cameraGeometry.ts` | `rig_frame`, `posed_bone_bounds`, `bone_group_indices`, `compute_framing`, `view_basis`, `create_observation_camera`, `camera_metadata`, `project_joint` |
| `poseEvaluation.ts` | `evaluate_document_at_time`, `apply_evaluated_pose`, `pose_digest`, `collect_joints` |
| `capture.ts` | `OffscreenCapture`, `canvas_pixel_encoder`, injectable `PixelEncoder`, full GPU state save/restore |
| `service.ts` | `PoseObservationService` (`capturePose`/`handle`, `getCaptureImage`, `getJointProjection`, `cancel`, `disposeCapture`, `dispose`), module singleton accessors |
| `tauriAdapter.ts` | Optional event adapter (`m2m-pose-observation-request` → `mcp_pose_observation_respond`, cancel event) |
| `attach.ts` | Idempotent rig watcher installation from every page bootstrap; no DOM output |

The service is installed by `src/integrations/mcp/attachControl.ts`, so all four existing
pages plus the new one get internal observation capability. It reads the **same**
`AnimationDocument` as the GUI/MCP authoring bridge through the new
`get_active_authoring_bridge()` accessor (no second document model).

## Camera orientation and projection specification

- Default projection: orthographic (consistent pose measurements). Perspective is
  optional with fov 40°.
- Frame derived from the live `Skeleton` bones' world positions after pose evaluation,
  with a rig-derived orthonormal basis: up = pelvis→head, right = left/right arm pair,
  forward = right × up. World axes are fallback only (non-Humanoid rigs), never an
  unconditional "faces +Z" assumption.
- Directions: front +forward, back −forward, right +right, left −right,
  top +up (camera up = −forward), bottom −up (camera up = +forward).
  Top/bottom up vectors are explicit, so `lookAt` never degenerates.
- Frustum fitted from the 8 bone-bounds corners projected on the camera right/up
  axes, plus a deterministic 12% margin; aspect-correct; near 0.01,
  far = distance + 2×diagonal. `focus_bone_id` reframes to that bone and its
  descendants (hand, foot, finger chains); unsupported IDs return `UNSUPPORTED_BONE`.

## Pose identity and integrity

- One call to `evaluate_document_at_time(document, timestamp)` produces one evaluated
  transform set; the same `pose_digest` is returned for all six views.
- The live skeleton is mutated only inside `apply_evaluated_pose`'s synchronous
  save/restore transaction. There is no `await` inside the pose transaction, so the
  application frame loop cannot advance playback between views.
- Stale `expected_revision` → `STALE_REVISION`; fingerprint mismatch → `RIG_MISMATCH`;
  unknown focus bone → `UNSUPPORTED_BONE` with supported IDs.
- Renderer state saved/restored in `finally`: render target, viewport, scissor,
  scissor test, clear colour/alpha, auto-clear. No camera object is added to the
  visible scene; no new WebGL context; one reusable render target; no render loop.

## Capture images and joint metadata

- Real pixels from `renderer.readRenderTargetPixels` on the live scene, encoded by a
  2D canvas to `image/png` (default) or `image/jpeg`; image dimensions 64–1024
  (default 512×512). JSDOM tests inject a deterministic encoder.
- Optional `bone_labels`/`skeleton` overlays are drawn into the **capture image only**.
- Joints carry stable bone IDs, hierarchy, world/local transforms, `u/v/depth`,
  `in_frame`, `visible`, grounded `occlusion` (`none`/`self`/`unknown`) and reason.
  Raycast occlusion uses the live evaluated skinned meshes; if none are available the
  honest `unknown` value is returned. No joint positions are invented.

## Tests and real results

Commands run in `worktrees/m2m030-031-observation/frontend`:

| Check | Result |
| --- | --- |
| ESLint `--max-warnings 0` | 0 errors / 0 warnings |
| TypeScript `tsc --noEmit` | 0 errors |
| Vitest (full suite) | **67 files / 454 tests passed** (baseline 61 / 413 → +6 files / +41 tests) |
| Vite production build | exit 0; `dist/newAnimation/index.html` emitted (existing `__dirname`/chunk advisories only) |
| Runtime import cycles (`runtime-import-cycles.test.ts`) | 2 / 2 passed |
| ast-grep explicit `any` scan | 0 matches |

Focused tests cover: six camera orientations, top/bottom up vectors, rig-relative
orientation under a rotated rig, whole-model vs hand focus framing, column-major
matrices and aspect-correct frustum, perspective fov, world→image projection,
occlusion honesty, identical timestamp/digest across views, image dimensions/MIME/bytes,
GPU state restoration, no scene/DOM camera objects, orbit camera unchanged, stale
revision/fingerprint rejection, unsupported bone, deadline, cancel/dispose, duplicate
IDs and non-increasing sequences, and render-target disposal.

## Known limitations / Codex handoff

- Rust transport is not implemented here. Confirm `m2m-pose-observation-request`,
  `mcp_pose_observation_respond` and `m2m-pose-observation-cancel` in the channel
  before wiring MCP tools; the callable service API works without them.
- `image_base64` is included inline only for images ≤64 KiB; larger captures are
  fetched by `image_ref` via `getCaptureImage`.
- WebKitGTK/WebGL2 readback was exercised only via the staged native launch smoke and
  JSDOM/fake-renderer transactions; a real MCP image round-trip requires Codex's tools.
- **Capture image samples: PROVIDED.** Real images rendered by the six internal
  cameras are in `artifacts/M2M-030/pose-inspection/`. They were produced by the
  developer-only harness `src/authoringObservation/inspect.html` / `inspect.ts` using
  `OffscreenCapture` + `canvas_pixel_encoder` on the real skinned model in headless
  Chromium/WebGL2 (not fabricated diagrams or desktop screenshots). The harness is
  deliberately not linked from the application navigation and adds no UI to the app.
  - `01-malformed-t0.png` — original jumping-jack draft (T-pose at "closed").
  - `02-corrected-closed-t0.png` — corrected closed pose (arms down, legs together).
  - `03-crossed-legs-before-fix-t0.5.png` — exposed the wrong thigh signs.
  - `04-correct-jumping-jack-open-t0.5.png` — verified open pose (arms overhead,
    legs spread) from all six directions.
- An MCP end-to-end `capturePose` round-trip still requires Codex's transport/tools;
  the samples above exercise the same camera/capture code with a local harness.
# User preview confirmation addendum (2026-11-23)

The user confirmed the authored animation preview on the staged native build
(`Bildschirmfoto_20261123_215020.png`, supplied in chat). The matching retained image
is `artifacts/M2M-031/screenshots/06-working-preview-verified.png`. This confirms the
New Animation preview path; the distinct MCP six-camera image transport remains
provisional and is not claimed as an end-to-end verified feature.
