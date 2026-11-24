# HANDOFF NOTICE — OpenCode M2M-028A → Codex

The `authoring-bridge/v1` contract is now published and actionable:

- **Contract:** `coordination/contracts/animation-document-v1.md` (frozen, unchanged) and
  **`coordination/contracts/authoring-bridge-v1.md`** (new, provisional but ready to implement).
- **Frontend branch:** `m2m-028a-frontend-bridge` (isolated from reviewed integrated candidate
  `m2m-026-native-fixes` @ `83f0439c2712ba9d65f4643d3666320e5fc92c20`, which preserves
  `ae487348`).
- **Important:** do not base this work on `m2m-025c-frontend-integration` @ `a33015c`; its commit
  `5f6d010` changes frozen `animation-document/v1` validation (min duration, interpolation rules).
  Those changes are not part of the reviewed integration line.

## Contract essentials for Codex

- Request envelope: `{ protocol:'authoring-bridge/v1', request_id, session_id, deadline_ms, operation, payload }`.
- Closed operations: `rig.catalog.get`, `draft.create`, `draft.validate`, `draft.preview.start`,
  `draft.preview.stop`, `draft.commit`, `draft.cancel`, `draft.undo`, `draft.export`, `draft.import`,
  `session.status`, `session.cancel`.
- Responses: `{ protocol, request_id, session_id, ok, result?, error? { code, message } }`.
- Confirmation: the MCP client cannot supply approval. The frontend shows a native DOM panel; only a
  user click resolves commit/undo. Reject/expiry leaves the library unchanged.
- Bounds: payload ≤512 KiB, request/session IDs ≤64 chars, deadline 100..30000 ms, one in-flight request
  and one active session, duplicate IDs rejected.

Please review the contract and confirm the Rust IPC method name/casing. The TypeScript implementation
uses string request IDs. No Rust code was modified by this task.

**Two-way communication:** append replies and questions to
`coordination/handoffs/M2M-028A-bridge-channel.md` (append-only; do not rewrite earlier entries). That
file is the live coordination channel for this bridge.
