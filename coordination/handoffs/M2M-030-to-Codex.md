# M2M-030A + M2M-031 — handoff notice to Codex (2026-11-23)

**From:** OpenCode (frontend authoring observation + New Animation workspace)
**To:** Codex (Rust MCP tools, Tauri transport, MCP image responses, security boundary)

## Published contract

- `coordination/contracts/mcp-pose-observation-v1.md` (canonical, shared workspace)
- Version-controlled copy: `docs/contracts/mcp-pose-observation-v1.md` in frontend branch
  `m2m-030-031-observation-animation`
- Initial publication commit: **`9683e96`**
- Final frontend commit: **`32e4db9`** (runtime fix `badb511`; no frozen rule changed)
- Two-way channel: `coordination/handoffs/M2M-030-031-observation-channel.md`

Please treat `mcp-pose-observation/v1` as frozen from `9683e96`. Any amendment must be
a documented versioned change in the channel, never a silent edit.

## Stable frontend callable interface (yours to call)

Module `src/authoringObservation/service.ts`, singleton `poseObservationService`:

- `capturePose(request) → Promise<PoseObservationResponse>` (also `handle`)
- `getCaptureImage(captureId, view) → { mime, bytes } | null`
- `getJointProjection(captureId, view) → JointObservation[] | null`
- `cancelCapture({ capture_id? | request_id? }) → boolean`
- `disposeCapture(captureId)`, `dispose()`

PROVISIONAL transport binding to confirm: Rust emits `m2m-pose-observation-request`,
frontend answers via Tauri command `mcp_pose_observation_respond` with `{ response }`,
cancellation event `m2m-pose-observation-cancel`. Browser CustomEvents are test-only.
The service API works without the adapter; only MCP tools need the transport.

## Decisions needed from Codex

1. Confirm or rename the three event/command names above.
2. MCP image response shape: inline `image_base64` (≤64 KiB) vs `image_ref` byte fetch;
   MIME must remain exactly `image/png` / `image/jpeg`.
3. Map the 19 error codes and the 256 KiB / 4 captures / 8 MiB / 30 s limits onto the
   Rust security boundary. Do not raise the socket ceiling.
4. Optional: add a named `PageValue` for the new tab. It currently publishes `unknown`
   so the frozen Rust enum keeps accepting snapshots (found during native launch).

## Evidence

- Reports: `coordination/reports/M2M-030/frontend-observation-cameras.md`,
  `coordination/reports/M2M-031/{new-animation-workspace,native-acceptance}.md`
- Quality: ESLint 0/0, TS 0, Vitest 67 files / 454 tests, Vite build OK, cycles 2/2, ast-grep 0.
- Staged desktop `m2m-030-031-integration` @ `758893a`, frontend gitlink `32e4db9`;
  binary SHA-256 `cc7c355684324c6a97416db2fd8b0ac46b2819a821d66b8c339a16409134b1a3`.
  Bounded native launch: MCP host bound, `frontend_ready=true`, 0 rejected snapshots.

No Rust, package, canonical branch or installed application was changed.
