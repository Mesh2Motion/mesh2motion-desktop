# M2M-030B — Greeting Wave anatomy and native MCP review

Date: 2026-11-24  
Desktop integration branch: `m2m-030b-integration`  
Frontend integration pin: `50ef55e11a69b46ba94fbf1acbbbf75071d2ae30` (includes OpenCode `657e418`)  
Acceptance target: Human rig, live MCP authoring, readable palm in the front observation camera, a visibly flexed elbow, and an authored clip in the native animation library.

## Result

OpenCode authored and committed a fresh `Greeting Wave` clip through the integration app's MCP sidecar. The live catalog identified the Human rig with fingerprint `1a167d80d91326d763af6c04914790d814010341d843008129ca2102d7f4c31d` and 66 bones. The resulting library entry is `Greeting Wave`, `custom-import-5`, revision 7, duration 2.0 seconds, 66 tracks. The JSON-RPC run validated the draft, requested native preview, committed it, observed the library entry, and canceled the authoring session.

The first image review showed the raised arm too shallow and almost straight. The revised greeting keeps a raised shoulder and a steady elbow bend while the wrist turns between two palm-readable orientations. This is based on the actual rendered MCP PNGs and a wrist-axis grid, not a preset wave generator.

## Image-guided decisions

The trial grid captured 15 real front-camera images: local wrist rotations on X, Y, and Z at −60°, −30°, 0°, +30°, and +60°. Visual comparison showed local Y rotation produced the broadest palm/finger silhouette toward the front camera; the X and Z samples stayed edge-on or oblique. The authored hand channel therefore rocks on local Y between +30° and +60°.

The live draft used the rig's catalog local quaternions as rest transforms and MCP's existing rest-composed quaternion keyframe path. Authored channels:

- `upperarm_l`: local Z 95° across the two-second clip.
- `lowerarm_l`: local Z 60° across the clip, giving visible elbow flexion in the front render.
- `hand_l`: local Y alternates +30° / +60° at 0.25-second intervals.
- `upperarm_r`: local Z 82°; `lowerarm_r`: local Z 8° to keep the free arm relaxed at the side.

The two front MCP images at 0.00 and 0.25 seconds are distinct (SHA-256 `57c5f15bc602de388da37d934673bd2513f67e831a6352cc6fe4759bbf177ee6` and `e5ff7486a39b571b4106b28690f785b98133edb5bf25e28a8ff95dd1dc1aff51`). They show the bent raised arm and changed wrist/hand silhouette. The contact sheet is `artifacts/M2M-030/greeting-wave/wrist-grid/contact-sheet.png` (SHA-256 `fb92b265a1841a7ee0443f3336d34b70cc90cd03b36752702d388d4c1987606e`). Original PNGs, trial manifest, MCP transcript and report are alongside it.

## Verification and limits

Passed on this integration pair:

- Frontend: Vitest 71 files / 479 tests; ESLint with `--max-warnings 0`; TypeScript `--noEmit`; Vite production build.
- Rust: `cargo fmt --check --all`; locked offline workspace tests; locked offline MCP tests (41 library + 3 binary); Clippy with `-D warnings` for the desktop workspace and MCP sidecar.
- Python MCP client: Ruff; pytest (4 passed, with one external Dash/pytest deprecation warning); ty.
- ast-grep: Rust unsafe-block, TCP-listener, Unix-listener and filesystem-permission scans found no matches in the implementation.
- Bacon: bounded headless check reached Cargo `Finished` successfully; Bacon remained in its watch loop and was stopped at 20 seconds (exit 124), so the watcher itself is not reported as a clean exit.
- BugStalker: `bs --help` worked. A safe `/bin/true` debugger launch failed with `ENOTTY` in this non-interactive tool session; mark debugger inspection **BLOCKED**. Existing unrelated BugStalker TUI processes were not counted as a debugger run.

A follow-up cross-view/export round-trip attempt reconstructed the authored AnimationDocument from the recorded MCP requests. It was not allowed to proceed after the running native app stopped publishing its control snapshot: the sidecar returned `APP_UNAVAILABLE` and `RIG_UNAVAILABLE`. The captured failure is in `verification-blocked-transcript.ndjson`; it did not alter the library. Consequently this exact `Greeting Wave` revision still needs a front/right/top re-observation and exact-clip export/reimport comparison. The older Agent_Wave_01 integration run has separate successful commit/export/reimport/undo evidence, but that does not substitute for a round-trip of this new clip.

The front render looks anatomically improved, but the current fresh run did not preserve a side-view image or quantitative elbow angle; do not treat it as complete multi-view biomechanical sign-off. The user should review the two saved front frames and the native clip before package installation.

## Build and tool notices

- Production Vite build passed with the known large-chunk advisory. The earlier code-splitting attempt was reverted because it stopped native frontend readiness.
- The system pytest environment emitted a Dash plugin deprecation warning; this is external to the maintained MCP client.
- Rust MCP observation feature source commit: `622ebe804ca3b25cc20e764a54ffa3a542e1ecf8`.
- Rebuilt debug desktop SHA-256: `b8b61d1df3e1043165487e860d4cd7180b2faf7e2c68f1ea8d3ad8711a2de0c5`; MCP sidecar SHA-256: `87218aa85eea1ea6a08780f70d55712f8fa968b9cc22edfac380c9d0eb149b7e`.
- The isolated desktop integration branch and frontend integration branch are clean after commit. The OpenCode dependency symlink remains local-only and is excluded through that worktree’s `.git/info/exclude`; it was neither staged nor removed.
- No packages were installed. No release package was installed. Canonical branches were not modified.

## Remaining native acceptance steps

The exact committed clip still needs a live app window with the Human rig loaded to finish the blocked cross-view/export check:

1. Run `./run-integration.sh` from this desktop worktree and load the Human rig in the native app. Keep that app open while the MCP client runs.
2. Confirm `skeleton_get_catalog` returns the Human fingerprint recorded above, then select and play `Greeting Wave` from the native animation library.
3. Inspect the saved 0.00 s and 0.25 s front frames and the native playback. Confirm the elbow stays naturally flexed through the wrist sway and the palm faces the front camera; inspect the side view for forearm/elbow alignment.
4. Use the MCP observation tools on the matching draft at 0.00, 0.25 and 0.50 seconds and save front/right/top images with their synchronized joint projections. Compare repeated observations for matching pose digest and elbow/wrist positions.
5. Export and reimport that exact AnimationDocument and compare duration, rig fingerprint, bone IDs and representative keyframes. Preserve the current committed library entry unless the user asks to remove it.

Until those steps pass, treat visual-quality and exact-clip round-trip acceptance as **BLOCKED**. The code/test branch is clean and buildable; this is a native acceptance gate, not a clean-tree failure.

## Correction and closed acceptance (2026-11-24, OpenCode)

The channel values in this report are wrong for the elbow: `lowerarm_l.z = +60°` folds the forearm
**behind** the head. A 9-angle elbow probe (`elbow-grid-contact-sheet.png`, real MCP images) showed the
anatomically correct forward bend is **negative**: `lowerarm_l.z ≈ −60°` with `upperarm_l.z = 95°`
puts the forearm vertical beside the head. The user was right that the earlier clip contorted the
elbow. A second grid with that corrected elbow showed the palm stays toward the camera for
`hand_l.y` in roughly −45…+45°, so a small ±30° rock is sufficient; large elbow swings must never be
used to wave.

Corrected clip: **`Greeting Wave v3`** (`custom-import-5`, 66 tracks, revision 7), authored with
`upperarm_l.z=95`, `lowerarm_l.z=−60` constant, `upperarm_r.z=82`, `lowerarm_r.z=8`,
`hand_l.y=−30/+30` every 0.25 s.

Two problems in the install candidate were also fixed:
1. The integration `dist` was the reverted chunk-split bundle; rebuilding it from the final frontend
   HEAD restored native startup (`M2M_HOST enabled=true`, `frontend_ready=true`). A stale `dist`
   makes WebKit spin with no MCP socket.
2. The MCP tools gave agents no anatomical conventions, forcing axis guessing from pixels.
   Commit `67d4f85d` publishes validated Human pose conventions and camera-direction caveats in the
   `animation_set_bone_keyframes` / `animation_observe_pose` tool descriptions, verified in `tools/list`.

Native acceptance for the corrected pose is now **PASS**: front/right/top observations at
t=0.00/0.25/0.50 with matching rig fingerprint `1a167d80…` and distinct digests, exact
export/reimport round-trip (5 tracks / 45 keys / 2 s / same rig), and clean session cancellation.
Evidence: `coordination/audits/M2M-030/mcp-greeting-wave-acceptance.ndjson` and
`artifacts/M2M-030/greeting-wave/acceptance/`. Final visual sign-off by the user is still required.
