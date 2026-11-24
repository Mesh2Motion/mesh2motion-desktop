# M2M-030B — Agent_Wave_01 native vision demo

Date: 2026-11-23  
Status: **The staged native workflow reached commit and export/reimport, but synchronized image/pose parity and undo remain BLOCKED. Do not count the wave as accepted.**

## Native setup and transport

- Built the Tauri app and standalone MCP sidecar from the isolated Rust feature worktree with the built frontend from the separate image-transport feature branch.
- Connected a real JSON-RPC stdio client through the sidecar to the app’s existing private Unix socket. The client read the live Human rig catalog (66 bones), created a native `AnimationDocument` draft, and submitted channels using the live IDs and local rest quaternions.
- Requested actual front/left/top captures at timestamp 0.5. The MCP client received PNG `ImageContent` blocks and joint projections in the same observation. Preview start/stop and draft cancellation succeeded.
- The full per-iteration request/response evidence is in `coordination/audits/M2M-030/vision-agent-wave-iteration-{3,4,5,6}.ndjson`; the decoded PNGs and their hashes are in `artifacts/M2M-030B/agent-wave-iterations/iteration-{3,4,5,6}/`.

## Pose parity finding and outcome

The first iterations 3–6 used new channels composed from live rest quaternions and were inspected, but OpenCode later found that their rendered skinned mesh still used stale skin matrices. Those images cannot be used as proof of pose quality or accepted visual-guided keyframe correction.

After OpenCode added `skeleton.update()` and I rebuilt the native app with that frontend dist, I captured a fresh draft with preview inactive at 0.25, 0.75, and 1.25 seconds. MCP returned actual PNGs and 66-joint records. At 0.25, `hand_l` metadata reports world y=1.919 and `head` world y=1.567, while the front PNG visibly shows the model bent forward with both hands low. This means image and joint data still do not describe the same visible pose. The exact t=0.25 image is `artifacts/M2M-030B/agent-wave-iterations/post-skin-matrix-no-preview/0.25/capture-1a20aa19373-ea6ff13a-front.png`; the JSON-RPC transcript is `coordination/audits/M2M-030/vision-agent-wave-post-skin-matrix.ndjson`.

OpenCode's staged native run then exercised the full workflow: original draft, six rest-composed channels, front/right/top images at three timestamps, actual pixel review, channel overrides, recapture at the same times, native preview, commit, export/reimport, and session cancel. The pixel review is saved at `artifacts/M2M-030/vision-guided/image-review.json`; it identified the lateral/down arms and requested a raised left arm with the right arm tucked. The staged clip `Agent_Wave_01_vision` was committed at revision 14 (`custom-import-2`, six tracks), and export/reimport preserved six tracks / 54 keys with valid status.

However, the separate MCP joint metadata still disagreed with rendered pixels after the first `skeleton.update()` change, so the resulting clip's pose cannot be verified as the pose described by the captured joints. This means the images were genuinely inspected and used to revise channels, but the required synchronized observation contract is not yet demonstrated. The wave's visual quality is **not accepted**.

## Acceptance not completed

- A staged app commit succeeded: `Agent_Wave_01_vision`, revision 14, `custom-import-2`, six tracks. Do not confuse this with visual acceptance.
- Export/reimport equality succeeded (six tracks, 54 keys, valid); native preview and session cancel succeeded.
- Undo to revision 13 returned `BAD_REQUEST`; rollback and post-undo library verification remain BLOCKED. The staged app's current library still has the committed clip.
- A second observation attempt later returned `RIG_UNAVAILABLE` after four seconds; lifecycle stability needs a regression test and fix.
- Existing M2M-028 wave demonstrations remain historical and do not close these gaps.

## Integration next steps

The integration agent should use the separate Rust and frontend feature commits plus OpenCode's frontend skin-matrix fix. First correct image/geometry parity, then verify the existing staged clip's visible pose and playback. Fix repeated-capture rig lifecycle and valid undo/rollback; then rerun acceptance and audit. Preserve the existing Unix socket and 256 KiB frame ceiling. Do not merge branches, install, or update the canonical frontend gitlink from this feature work.

## OpenCode addendum — executed vision round and library evidence (2026-11-24)

A real JSON-RPC run against the staged native app completed more of the workflow than the note above
records. It created an original draft from the live 66-bone Human catalog, authored six rest-composed
quaternion channels, captured front/right/top images at t=0.25/0.75/1.25 (9 real PNGs), **inspected
those pixels**, then used that inspection to revise the channels (raise left arm 70→110°, right arm
72→85°, elbow swing ±25→±45°, torso ±3→±6°, head ±2→±4°). It recaptured the same timestamps (9 more
PNGs, new pose digests), started the native preview, and **committed** the draft:

- Library entry: `Agent_Wave_01_vision`, duration 2.0 s, 6 tracks, `sourceType: custom-import`,
  tags `revision:14`, `authored-id:authored-mwtva0h0-dswu2r`, entry `custom-import-2`.
- Export/reimport comparison: 6 tracks / 54 keyframes, byte-for-byte equivalent track tuples,
  validation `state: valid`.
- Session cancellation succeeded. `animation_undo_commit` to revision 13 returned `BAD_REQUEST`,
  so undo/rollback remains BLOCKED.

All 18 PNGs and manifests are in `artifacts/M2M-030/vision-guided/`; the JSON-RPC transcripts are
`coordination/audits/M2M-030/mcp-vision-transcript-opencode-live.ndjson` and
`...-final-steps.ndjson`. The image review that drove the correction is
`artifacts/M2M-030/vision-guided/image-review.json`.

**Caveat:** because MCP geometry and PNG pixels still disagree (see the parity finding above), this
round proves the *workflow and transport*, not visual wave quality. Visual acceptance stays BLOCKED
until pixel/pose parity is proven.

## RESOLUTION + valid image-guided run (2026-11-24)

Parity was proven and fixed (see `mcp-vision-implementation.md` § RESOLUTION). A fresh run against
the rebuilt native app then completed the entire accepted workflow:

1. Live Human catalog (66 bones, `1a167d80…`), original 6-channel wave draft, Rust validation.
2. Captured front/right/top at t=0.25/0.75/1.25; repeated timestamps produced identical PNG bytes.
3. **Inspected the actual pixels**: the raised arm was diagonal and the elbow nearly straight.
4. Revision from that inspection: left arm 70→95°, right arm 72→80°, elbow swing ±25→±55°,
   torso ±3→±5°, head ±2→±4°. New pose digests and 9 new real PNGs.
5. Native preview started; draft committed as `Agent_Wave_01_vision`, revision 14
   (`custom-import-5`, 6 tracks, 2.0 s); library entry verified.
6. **Undo exercised in-session** to document revision 13 → revision 15 (`custom-import-6`).
7. Export/reimport comparison matched (6 tracks); session cancelled cleanly.

The final front/right PNGs show an upright figure with one arm raised and the free arm at the side —
a plausible greeting wave. Final visual sign-off still belongs to a human, but the image/geometry
parity gate that previously invalidated all visual evidence is now **PASS**. Evidence:
`coordination/audits/M2M-030/mcp-vision-transcript-parity-fixed.ndjson` and
`artifacts/M2M-030/vision-guided-parity-fixed/` (18 PNGs, manifests, image review, final.json).

## User-reported GUI defects fixed (2026-11-24 00:28 screenshot)

The user supplied `Bilder/Bildschirmfotos/Bildschirmfoto_20261124_002855.png` showing (a) two
`Agent Wave 01 vision` tiles with "No preview" and (b) a contorted viewport playback (base-library
animation still driving unauthored bones). Both are fixed in the frontend worktree:

- **Contorted playback:** `draft_document_to_clip()` now appends constant bind-pose quaternion
  tracks for every bone the document does not author. A from-scratch 6-channel draft therefore
  produces a fully pinned clip; the freshly committed `Agent Wave 01 clean` reports **trackCount 66**
  in `animations_list`. Regression test added in `liveRigAdapter.test.ts`.
- **Broken previews:** `RealApplicationOwner.apply_committed_clip()` now generates the real
  offscreen poster/flipbook (with 2D skeleton fallback), registers it with the catalog and persists
  it with the stored authored document. Previously previews were only generated during startup
  restore, so a fresh commit showed "No preview" in the current session. `set_stored_preview_frames()`
  was added to `authoredLibrary.ts`.
- Old already-committed clips in the running app keep their old 6-track data and missing posters;
  use the GUI delete action on those two tiles, then select `Agent Wave 01 clean`.

Frontend gates after these fixes: **71 files / 475 tests**, tsc, eslint, vite build PASS.


## Integrated acceptance update (2026-11-24)

The integration branch now includes the OpenCode parity/determinism fix (`98fcf60`) and authored-preview/full-track playback fix, plus Codex's bounded image transport and stable authoring adapter. OpenCode's native JSON-RPC run confirmed actual image review drove channel changes; repeated same-pose captures were byte-identical; preview, commit, export/reimport, and undo to the earlier document revision succeeded before session cancellation. It also caught that a sparse six-channel draft produced an incomplete clip; `draft_document_to_clip()` now pins unauthored bones to bind pose and commit generates/persists preview frames. The follow-up native clip `Agent Wave 01 clean` is selectable with 66 tracks.

I independently rebuilt the merged integration pair and connected its sidecar to the real native app. A temporary draft received all six genuine 512×512 PNG MCP image blocks with matching live rig/revision/time and structured joint/camera data; the session was canceled without committing. See `coordination/audits/M2M-030/mcp-vision-integration-native.ndjson` and `artifacts/M2M-030/integration-native/`.

The earlier `Agent_Wave_01_vision` six-track clip remains in the staged library for compatibility/history; the full-track `Agent Wave 01 clean` is the corrected playable clip. The final pose is an upright one-arm greeting with the free arm lowered. Human aesthetic sign-off is still pending, and no package was installed or branch promoted to canonical `main`.


## Final native-safe integration revision (2026-11-24)

The final frontend integration SHA is `c83c210b63a8fee0680ca563d460a32950665997`; the final desktop integration SHA is `94518dc5cdd093f66693f4364af3cffd026cfb9e` and its frontend gitlink matches. The M2M-031 cleanup branch was merged, removing Vite's deprecated `__dirname` warning and fixing the existing TypeScript/JSDOM validation noise. A Rolldown size-based splitting experiment removed the >500 KiB warning in Vite but caused the native frontend authoring listener not to start; that change was reverted. The final native-safe build starts `frontend_ready=true`; the large offline/Three chunks advisory remains (largest 960 KiB, gzip 267 KiB). Final locked desktop build passed. With an isolated blank profile, real MCP `tools/list` returned 19 tools and `animation_cancel_session` succeeded; no live rig was loaded in that blank profile. The six-view real-image capture transcript and PNGs above came from the same observation/application code before the build-config-only cleanup.

The exact integration-native app and MCP sidecar hashes after the final rebuild: see the final handoff. No packages were installed or canonical branches changed.
