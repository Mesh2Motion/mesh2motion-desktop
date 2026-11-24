# Left Foot Sole Clap — native review

- Native integration app was started and left on the New Animation page with preview active.
- Created against the live Human rig and validated through the existing MCP authoring path.
- Committed clip: `Left Foot Sole Clap` (`custom-import-1`, library revision 2).
- Live rig fingerprint: `e02f544092433209e1360d242ed35e969bb50688d12cdc7136c5a51b61e4280a`.
- Native MCP captures: 512×512 PNG from the back, right, and top cameras at 0.97 s and 1.42 s. Transcript: `left-foot-sole-clap-native.ndjson`.
- AnimationDocument: `left-foot-sole-clap-v1.animation-document.json`.

## Visual review

The captured pose raises the left leg and bends the right arm, but the right hand does not yet contact the left sole. Joint metadata at 0.97 s puts `hand_r` at approximately `(-0.23, 1.43, 0.50)` and `foot_l` at `(-0.25, 0.85, -0.32)`. The separation and rendered images show the clip is not yet a successful clap. The clip remains committed so it can be inspected in the app; visual acceptance is not claimed.

Images are in `left-foot-clap-refined/`. The final capture set uses `capture-1a20ba68bdd-1e99b74c` for the back view at 0.97 s.
