# M2M-028B Agent_Wave_01 live demo

## UI correction follow-up — 2026-11-23 17:55 UTC

The visible camera panels shown in the prior staged screenshot were rejected by the user. The cameras
are intended only for the MCP workflow, with no visible controls or layout changes. The staged
frontend was restored to `05051093d4cac6762507b75b35a500b749724219`; a separate follow-up fix corrected
the XYZ view-helper position regression. The helper used a page-space `offsetLeft` as if it were
relative to the viewport canvas, double-counting the left sidebar. Frontend commit
`75aae01ae729edc1cec7612fcc382828960bde82` now subtracts canvas bounds; desktop pin is
`770dd735b5a09b0e77abb14ca1d09a90ce227960`. Three regression tests, touched-file ESLint, TypeScript,
Vite build, and a native screenshot pass confirm lower-left placement with no camera UI.

This does not resolve the Agent_Wave_01 pose. The user-rejected pose remains rejected; no package or
installation was made.

## Acceptance update — 2026-11-23

**Mechanical MCP workflow passed; animation quality acceptance failed.** The final native JSON-RPC transcript is `coordination/audits/M2M-028/mcp-agent-wave-final.ndjson` (the earlier `agent-wave-final-transcript.ndjson` is a separate run). It records live rig discovery, draft/keyframe authoring, Rust validation, native preview, commit, export/import comparison, undo and cancel. The user inspected `Bilder/Bildschirmfotos/Bildschirmfoto_20261123_165424.png` and reported that the resulting pose is not a usable wave: the arm is overhead and the torso is bent. Therefore **Agent_Wave_01 is not accepted as a successful animation**.

The user additionally requires two internal authoring views of the live model, one frontal and one side view, to judge pose placement and translation correctly. Current renderer inspection found a single `PerspectiveCamera` owned by `SceneEnvironmentManager`; the authoring work needs a coordinated front/side camera design that observes the same live rig and animation. These views should validate placement; the AnimationDocument's local-bone transform conventions remain unchanged. Keep MCP creation/commit mechanics distinct from the still-pending visual-quality and camera acceptance. No package or install occurred.

### UI restoration clarification

The user clarified that cameras are internal to the MCP workflow and must not change the user-facing interface. The visible front/side overlays in the staged camera build were rejected. Desktop integration commit `1b44a30` repins frontend `05051093d4cac6762507b75b35a500b749724219`, restoring the original no-overlay UI and view-helper cross. Codex's visible camera prototype `558ee2f` remains isolated and is not part of the integration gitlink. Do not expose internal cameras to users as panes, labels or controls.

The follow-up preview used the corrected rest-composed keyframes, but the MCP client received `APP_DISCONNECTED` during the export request. It therefore does not establish successful corrected preview/commit/export or visual acceptance. That staged build included the now-rejected camera overlays; the integration gitlink and rebuilt dist have since been restored to `0505109`. The original viewport and lower-left orientation cross are visible in `/tmp/m2m028c-restored-original-ui.png`. No package was installed.

### Pose failure cause and demo correction

The captured draft's arm, spine, hand and head quaternion keys used pure identity/axis-delta values as absolute local rotations. The live catalog shows non-identity rest quaternions for those bones, so those channels replaced the bind pose and produced the bent/overhead pose. Updated `tools/mcp/agent_wave_mcp_demo.py` to compose local axis deltas with the live catalog's `local_quaternion` values in XYZW order (`rest * delta`) and added tests in `tools/mcp/test_agent_wave_mcp_demo.py`. Ruff passed, pytest **6 passed** (one installed Dash hook deprecation warning), and `ty check tools/mcp` passed. This fixes the keyframe-generation helper; it has not yet been rerun in the native application, so no corrected visual result is claimed.

The active desktop integration branch is `m2m-028c-integration` @ `1b44a30`, with the frontend gitlink pinned to `05051093d4cac6762507b75b35a500b749724219`. This preserves request-scoped trusted-MCP authorization and restores the original visible interface. OpenCode commit `4384a73` was not integrated because it approves any confirmation whenever the Tauri global exists, instead of binding approval to the exact in-flight MCP commit/undo request.

**IN PROGRESS — native keyframe authoring, preview and commit response verified; library visibility, undo and export/reimport remain pending.** No fixture, JSDOM test or Rust-only draft is counted as native authoring. The user requested MCP-triggered permission and automatic commit/undo authorization; the old enable toggle has been removed.

## Native state observed

Before and after the JSON-RPC probe, the real isolated Tauri app reported a loaded Human skeleton (66 bones), `Angry` selected, one loaded animation, and a ready renderer. The catalog request returned the live rig fingerprint `1a167d80d91326d763af6c04914790d814010341d843008129ca2102d7f4c31d`. The read-only probe did not alter selection or library state.

## JSON-RPC evidence

The captured transcript is `coordination/audits/M2M-028/mcp-transcript.ndjson` (19 request/response pairs; the `tools/list` response contains all registered tools). Verified:

- MCP initialize and tools/list over the actual stdio sidecar.
- All five existing read-only tool calls returned live application data.
- `skeleton_get_catalog` returned `ok=true` and 66 actual Human bones from Three.js.
- Native bridge diagnostics reported both listeners ready and the catalog request/response correlated.
- The captured transcript predates the permission amendment: `animation_create_draft` and `animation_session_status` returned `AUTHORING_DISABLED`. It proves the previous build's default-off behavior only. The updated source makes the first valid non-catalog MCP operation grant permission for the app session.

This transcript is **not** a completed Agent_Wave_01 sequence. It has no authoring opt-in, keyframe mutation, Rust document validation, viewport preview, native commit approval, library insertion, export/reimport comparison, or cancel/undo verification.

## Remaining sequence

1. Rebuild and relaunch the native app from the permission-amended source; verify the MCP session starts disabled and the first valid non-catalog operation grants it without a toggle.
2. Use the maintained JSON-RPC client to create an empty Human draft, then set original quaternion keyframes using the just-read stable bone IDs.
3. Validate through Rust, preview in the actual viewport, and request digest-bound native commit approval.
4. Human approves the exact revision/fingerprint/digest; verify the app animation library contains `Agent_Wave_01`.
5. Export/reimport and compare duration, track identities and representative transforms.
6. Exercise cancel and undo and verify the application returns to its prior library/selection state.

The user approval and the export/reimport result are required before any end-to-end success claim. No package is installed.

## 2026-11-23 — latest native flow and fixes

The isolated feature source now includes desktop `63f1be7961317e618613393b48b5089948699c3b` and
frontend `8ebeac4df77c54979ca22f5bbdd0d0470e619803` (desktop gitlink equals the frontend SHA). The
latest staged app was rebuilt from these contents. Two issues found in the first real JSON-RPC runs
were fixed:

- A normal completed `BridgeWait` emitted a cancellation event on drop, clearing the frontend draft
  between MCP operations. Cancellation now fires only for abandoned/incomplete requests.
- Rust represents omitted optional documents as JSON `null`; preview/commit now use the active draft
  for either `null` or `undefined`.

The maintained MCP stdio client now reached and verified: all five read-only tools remain present;
the live Human catalog has 66 bones; MCP use grants authoring; `Agent_Wave_01` empty draft created;
five original quaternion channels added using live bone IDs (revision 6); Rust validation returned
`valid`; actual native viewport preview started; draft export returned five channels. Transcript:
`coordination/audits/M2M-028/mcp-agent-wave-live.ndjson`.

The prior native commit request returned `{ entry_id: "custom-import-1", revision: 7 }`. The next
`animations_list` call omitted the new entry because custom imports are excluded by default, so the
demo stopped before undo and export/reimport. The maintained client now requests `includeCustom: true`
and polls for visibility. That staged app was closed after the incomplete flow to clear its in-memory
clip. The next run uses automatic authorization only through the trusted Tauri/MCP request path; no
additional user click is required for commit or undo. Do not claim full end-to-end completion until
library visibility, undo, and export/reimport pass.

---

# HISTORICAL UPDATE — mechanical live sequence completed (M2M-028C)

The previous "NOT RUN" status is superseded for mechanics only. A real MCP stdio client created and committed
`Agent_Wave_01` through the staged native Tauri application. Transcript:
`coordination/audits/M2M-028/agent-wave-final-transcript.ndjson`. Full evidence and exact commits:
`coordination/reports/M2M-028/integration-final.md`. The user-directed auto-authorization model was
used; no browser tab, fixture preset or mocked response was substituted. Native screenshot capture
remains unavailable in this session.
