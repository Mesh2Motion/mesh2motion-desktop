# Left Foot Sole Clap — correction review

- Corrected clip: **Left Foot Sole Clap - Corrected**.
- Created from the live Human rig and committed through MCP as `custom-import-1`, library revision 2.
- Native preview started after validation; app reopened on New Animation for GUI editing.
- Rig fingerprint: `e02f544092433209e1360d242ed35e969bb50688d12cdc7136c5a51b61e4280a`.
- Duration: 2.4 s. Keyframes bring the left ankle forward to chest height, fold the knee, aim the sole toward the reaching right palm, then add three short palm taps before lowering the leg.

## Native observation

At 0.97 s, the real back, right, and top camera images show the left leg lifted in front, a clear knee bend, and the right forearm reaching across the torso toward the foot. The pose metadata puts `hand_r` at `(-0.132, 1.189, 0.228)`, `foot_l` at `(-0.260, 1.140, 0.280)`, and `ball_l` at `(-0.102, 1.052, 0.290)`. The wrist is about 0.15 m from the toe joint; the rendered fingers extend onto the sole area. The foot's local +Z sole normal is aimed toward the hand.

At 1.42 s, joint metadata matches the contact pose, but the back/right capture images were black while top rendered normally. The 0.97 s images and matching joint positions were used to assess contact. This intermittent camera render issue is recorded rather than treated as animation quality evidence.

Images and transcript are in `left-foot-clap-corrected/` and `left-foot-sole-clap-corrected.ndjson`. The edited document is `left-foot-sole-clap-corrected.animation-document.json`.
