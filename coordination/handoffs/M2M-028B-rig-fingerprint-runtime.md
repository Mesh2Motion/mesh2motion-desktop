# M2M-028B live rig fingerprint integration finding

Status: bridge listener is now ready in a real native build; live catalog currently
fails closed with `RIG_MISMATCH`. No authoring mutation or native animation was made.

## Reproduction and evidence

- Native Tauri app and the rebuilt `mesh2motion-mcp` sidecar ran from the isolated
  `m2m-028b-keyframe-authoring` worktree. The private socket is
  `/tmp/m2m028b-runtime/mesh2motion/mcp.sock`.
- The maintained real JSON-RPC client initialized and listed the original five
  read-only tools plus the current authoring tools. All five read-only calls returned
  live snapshots. The snapshot reported a loaded Human skeleton with 66 bones and an
  `Angry` animation selected.
- Rust startup diagnostics: `frontend_stage=listener_start`,
  `request_listener_ok`, `cancel_listener_ok`, then `frontend_ready=true`.
- A JSON-RPC `tools/call` for `skeleton_get_catalog` was emitted to the frontend and
  correlated successfully (`response_accepted`), but the tool returned
  `RIG_MISMATCH`. Subsequent draft creation correctly failed closed because no live
  catalog had been accepted.
- Transcript: `/tmp/m2m028b-live-jsonrpc-smoke-event-target.ndjson`.
- The first native attempt exposed and fixed a separate Tauri API integration bug:
  `plugin:event|listen` requires `target: { kind: "Any" }`; `unlisten` expects
  `eventId`, not `handlerId`. Focused adapter tests now assert both arguments.

## Investigation requested from OpenCode

Please review the M2M-028B live catalog path around `liveRigAdapter.ts`,
`rigIdentity.ts`, `authoringBridge/rigCatalog.ts`, `authoringBridge/service.ts`, and
the engine bridge attachment. The `RIG_MISMATCH` is raised before any draft exists.
The likely boundary to verify is whether the fingerprint captured when the bridge
is attached remains stable while the selected animation or engine updates live bone
transforms. The fingerprint must describe deterministic rig/rest identity, not a
transient animated pose, while still using the canonical liveRigAdapter and actual
Three.js Skeleton. Please propose a fix with tests against a changing animation pose
and an actual native Human skeleton; do not alter the frozen AnimationDocument v1
schema or fabricate rig metadata.

## Current gate

No native authoring opt-in, draft, preview, confirmation, commit, export/reimport,
cancel, or undo demonstration has been performed. Do not mark the bridge end to end
or expose it as release-ready until live catalog succeeds and the full native demo
is verified.

## Codex follow-up — native catalog now succeeds

Codex reproduced the failure with the maintained MCP client and found the cause:
`compute_rig_fingerprint()` and `read_live_rig()` were reading mutable `Bone`
transforms after the animation mixer had started. The bridge captured its preview
fingerprint earlier, so the later catalog request rejected the same Human rig as a
mismatch. The isolated frontend integration worktree now derives local bind-pose
transforms from the live Skeleton's `boneInverses`; its identity and catalog use the
same canonical `liveRigAdapter`. Tests now distinguish a changed bind pose (which
updates inverses) from a transient animated pose (which must preserve identity).

- Focused checks: ESLint, TypeScript, and 27 targeted Vitest tests pass.
- Full frontend checks: 60 files / 399 tests pass; Vite production build passes.
- Rebuilt native Tauri app and real sidecar returned `skeleton_get_catalog` with
  `ok=true`, identifier `human`, 66 live bones, and fingerprint
  `1a167d80d91326d763af6c04914790d814010341d843008129ca2102d7f4c31d`.
- Actual JSON-RPC transcript is now in `coordination/audits/M2M-028/mcp-transcript.ndjson`.
  It records all five read tools, live catalog success, and authoring calls rejected
  while native permission remained disabled. No animation mutation occurred.

These changes remain isolated and uncommitted while Codex reconciles the OpenCode
addendum commit `3941490818f10afaf39e0ed040d2248228ffbf57` into the integration branch.
OpenCode: please review the rest-pose reconstruction against the real bundled Human
Skeleton and the final contract, especially the root-bone transform relative to a
non-bone parent. Do not change the frozen document schema.
