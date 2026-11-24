# Known Issues and Release Blockers

## M2M-030/031 release gate (2026-11-23)

- Features complete; authored preview visually confirmed by the user on the staged
  build (`Bildschirmfoto_20261123_215020.png`; retained project evidence:
  `artifacts/M2M-031/screenshots/06-working-preview-verified.png`). Promoted to the
  selected M2M-031 release refs and installed as package pair
  `0.1.10.r82.m2m031-1`. See `reports/M2M-031/final-release.md`.
- Observation transport event names are PROVISIONAL pending Codex; no Rust MCP tool was
  added. Generated preview images are PNG flipbooks, not MP4 videos.
- Authored-clip persistence uses browser-origin localStorage (20 clips / 4 MiB) and is
  skipped on rig-fingerprint mismatch rather than remapped.
- The installed r82 sidecar was verified with the release's post-install JSON-RPC smoke
  (17 tools, including 11 authoring tools). The separate observation-camera transport
  remains pending Codex confirmation.
- Authored preview acceptance passed by user confirmation. Automated pixel assertions,
  internal observation transport round-trip, and long-run GPU telemetry remain
  unverified; see `reports/M2M-031/native-acceptance.md`.


## M2M-024 native acceptance is incomplete

On the package-extracted r78 Tauri 2/WebKitGTK app, Human static posters visibly rendered and decoded at 100×120. Fox poster URL resolution/decoding was verified in an earlier run of the same integrated frontend. The Props dock measured a 7 CSS px inset from the real viewport top/right at the wide observed size. See `coordination/reports/M2M-024/native-acceptance.md`.

Still unverified: narrow resize, KDE tiling, fullscreen/restore, DPR change, Settings typography, Horse switching on this exact package, hover playback/pause/loop behavior, resource ceilings under load, MCP interaction and WebKit child stability. The historical WebKit child SIGABRT is not claimed fixed. The app’s window-size API was ACL denied; no bypass was attempted.

Installed packages remain r72; the r78 package candidate was only extracted and staged. Do not treat the candidate as release accepted until the remaining native observations pass.

## Build and test notices

- Vite emits the existing `__dirname` future-config-loader notice and an approximately 1.7 MB minified offline chunk warning.
- Pytest passes with an installed Dash plugin deprecation warning.
- Bacon’s bounded Cargo check succeeded before the 25-second watcher bound; the timeout is not a crash.
- The earlier r72 pacman installation completed, but configured Snapper hooks reported snapshot errors. M2M-024 did not install or upgrade packages. No global graphics setting was changed.

## Runtime cycles

The three historically reported edges used import type declarations and are erased by TypeScript. A regression test verifies that behavior; the integrated emitted-JavaScript scan found zero cycles among 260 production modules.

## Packaging caveat

The r78 desktop package contains 684 WebP + 684 MP4 resources. `makepkg` reported a `$srcdir` reference; the absolute staging build path is embedded in the executable. Bit-for-bit reproducibility is not established. Review `coordination/reports/M2M-024/package-manifest.md` before release.

## M2M-025 baseline findings

- Installed `mesh2motion-mcp-git 0.1.10.r73.gc417eb2-1` sidecar is stale: SHA-256 `5756529c…` matches r72/r63 and contains no `M2M_MCP_AUTOSTART` strings, despite autostart source existing at `68616f3`. Correct rebuilt sidecar from r75/r77/r78 is `24883bc6…`. Requires a future approved release to repackage from source.
- `r78.gd53041b` (Codex candidate, not installed) and installed `r73.gc417eb2` use different revision schemes; document before future packaging.
- Installed desktop binary contains one absolute Project 2 build-path string (reproducibility caveat).
- Native hover/fullscreen/tiling/DPR/fetch-telemetry cases remain unverified; no GUI automation available. Historical WebKit SIGABRT not claimed fixed.
- Vite >500 kB chunk warning and npm dev advisories are pre-existing; dependencies not upgraded.


## M2M-025B findings

- M2M-025B work is on isolated branch `m2m-025b-animation-document` (`55d7e34`); it is not integrated into canonical `m2m-025-dev`. Do not treat the new authoring domain as available in the installed app.
- Atomic commit/undo is defined only as an interface and reference in-memory store; live integration with `StepAnimationsListing` (the animation state owner) is pending and must preserve application ownership.
- `step` interpolation channels encode duplicate keyframe times; the existing `ExportAnimationCleanupService.clean_clips_for_export()` removes non-increasing times, so step holds would be lost on export. Agent_Wave_01 uses slerp only.
- Native visual acceptance of the draft preview remains BLOCKED without GUI automation.

## NATIVE-005 — RESOLVED

- Remix catalog first-paint background fixed and visually confirmed on the native staged build; evidence `Bildschirmfoto_20261123_034321.png`. Final frontend `5f81340`, desktop pin `6b247d2`. Details: `coordination/reports/M2M-025/remix-first-paint.md`.

## M2M-025B handoff limitations

- `ApplicationCommitController` is an interface + reference implementation; it is not wired into `StepAnimationsListing`. The integration layer must supply `AnimationApplicationOwner` without bypassing app ownership.
- Native confirmation transport (Tauri command/Rust payload/capabilities) is provisional and must be agreed with Codex; only the `native-confirmation/v1` request/result shape is frozen.
- Bone IDs are index-derived per the frozen contract. A reordered skeleton keeps the same IDs at each index, so compatibility relies on the stored fingerprint/display names/parents — the adapter rejects mismatches instead of redirecting by name.
- All 358 tests are JSDOM/Vitest; no native GUI verification of the authored wave exists.

- **NATIVE-006 (open, user-observed, Codex-owned):** switching to “Use Your Rigged Model” briefly shows a solid black triangle in the 3D viewport before the preview skeleton appears. Evidence: `Bildschirmfoto_20261123_032617.png` and `_032531.png`. Separate from NATIVE-005; assigned to Codex, not to the Remix workstream.

# M2M-027 follow-up

- MCP animation authoring remains intentionally unavailable until M2M-028 implements and verifies the live frontend IPC bridge, native confirmation, commit, cancel, and undo flow.
- Whole-root Python Ruff/ty checks still report findings in legacy GTK/WebKit audit probes; maintained `tools/mcp` sources pass scoped Ruff, pytest, and ty checks. See M2M-027 reports.

## M2M-030B — MCP vision: agent misread limb/palm positioning (Hermes investigation, 2026-11-24)

Symptom: an agent driving `animation_observe_pose` misjudged limb/palm/joint positions because
the tool descriptions gave no camera-orientation or bone-axis reference, so it read the "front"
view as the model's front and inferred bone motion from a single view. This is a *description*
clarity gap, distinct from the still-open renderer pose-parity issue below.

Live `server.rs` state (canonical tree `worktrees/m2m030b-integration/desktop/`, 83389 B;
`cargo check` exit 0; fix markers verified present; preserved through the 04:27 rustfmt pass):

1. `animation_observe_pose` description now states the six cameras are **WORLD-aligned, not
   rig-forward aligned**: front=world -Z→+Z, back=+Z→-Z, left=+X, right=-X, top=+Y, bottom=-Y.
   Key note: on the Human rig the model faces +Z, so the **"front" view shows the model's back**
   — use "back" or "right" to judge a palm that must face the +Z default viewport camera. Added:
   "compare projected joints with the pixels before accepting a pose; do not infer bone motion
   from a single view."
2. `animation_set_bone_keyframes` description now carries **HUMAN RIG POSE CONVENTIONS (validated
   on the live 66-bone rig)**: values are local quaternions composed as `rest * delta` (not
   absolute); upperarm Z for arm raise/lower (per-side signs); lowerarm Z **NEGATIVE** for an
   anatomical forward elbow bend (worked example `lowerarm_l.z≈-60` + `upperarm_l.z≈95` → forearm
   vertical, hand beside head; a POSITIVE `lowerarm.z` folds the forearm behind the head and
   contorts it); `hand_l.y` in −30..+30 for wrist/palm; a validated 2 s greeting-wave recipe
   (`upperarm_l.z=95`, `lowerarm_l.z=−60` constant, `upperarm_r.z=82`, `lowerarm_r.z=8`,
   `hand_l.y` alternating −30/+30 every 0.25 s). Plus: request front/right images AND joint
   projections, inspect pixels, confirm the elbow bends forward and the palm faces the intended
   camera before committing.
3. View validation error now names the six valid views explicitly: "each view must be one of:
   front, back, left, right, top, bottom (1..6 unique)".

STILL OPEN (deeper root cause — NOT fixed by description clarity; frontend/Codex-owned): the
handoff `M2M-030-031-observation-channel.md` records repeated "captured pixels did not match the
evaluated pose" — the front PNG shows a crouched, both-hands-low pose while metadata reports
`hand_l` world y≈1.919 and `head` y≈1.567. Root cause is the renderer drawing a different
`SkinnedMesh.skeleton` than `source.skeleton` (skin-matrix / skeleton identity). Related open
gates: `RIG_UNAVAILABLE` after 4 s; `animation_undo_commit` → `BAD_REQUEST`.

Propagation (b) — 2026-11-24: the description fixes were propagated to EVERY vision-capable
`server.rs` copy and verified byte-consistent (1 unique description fingerprint across all 5):
canonical live `m2m030b-integration` (83389 B) + `m2m030b-codex-user-visible` (80361 B) +
`m2m031-codex-preview` (83201 B) + `sources/m2m030b-stage` (80361 B) +
`sources/m2m030b-mcp-vision` (80361 B). Pre-fix backups kept as `*.pre-descfix.bak`.

Intentionally NOT touched: pre-vision trees that have the older `animation_set_bone_keyframes`
but lack `animation_observe_pose` (`m2m028b-authoring`, `m2m028c-final`, `m2m028c-integration`,
`m2m030-031-observation`, `m2m031-release`) and pre-tool trees (`m2m025/026/027`, the 14081 B
base). The fixed `animation_set_bone_keyframes` text *references* `animation_observe_pose`, so
pasting it into a tree without that tool would create a broken tool reference and re-introduce
the divergence we are removing.

NOTE (flagged, not fixed — feature question, out of scope for the description fix):
`m2m031-release` (43960 B) has `animation_set_bone_keyframes` but NOT `animation_observe_pose`.
If that release is meant to include the M2M-030B vision feature, it has a feature gap that
someone must confirm/backport.

Status: investigation did NOT merge, promote, package, install, or claim complete. The
description fixes do not unblock the three open gates above (renderer pose-parity /
`RIG_UNAVAILABLE` / `animation_undo_commit` BAD_REQUEST).

## M2M-032 — Dragon-to-Horse rig switching fails (open, user-reported, 2026-11-24)

When changing the selected rig from Dragon to Horse, the Horse rig does not load and its
skeleton is not exposed in the authoring UI. This blocks editing/previewing animations for
the Horse rig and prevents confirming that the selected rig and its bone catalog agree.
The screenshot supplied with the report was not available as a local file to archive.

Acceptance: switch Dragon → Horse and Horse → Dragon repeatedly; confirm the correct model
renders, the skeleton/bone catalog becomes available for each rig, animation tracks bind to
the selected rig fingerprint, and no stale skeleton or previous-rig pose remains visible.
Status: OPEN. Needs reproduction and frontend/native diagnosis; no fix or verification claimed.
See `reports/M2M-032/dragon-horse-rig-loading.md`.
