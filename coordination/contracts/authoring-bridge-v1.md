# authoring-bridge/v1 — authoritative frontend/native contract (FINAL, M2M-028B amendments)

**Status:** FINAL for integration after the M2M-028B amendments recorded by Codex and the coordinator.
OpenCode implementation: branch `m2m-028a-frontend-bridge`. `animation-document/v1`
(`coordination/contracts/animation-document-v1.md`) remains frozen and unchanged.

Transport: Codex's existing local one-request/one-response Unix socket, bridged to the app through Tauri
events. No TCP, shell execution, filesystem path from a client, or JavaScript evaluation is permitted.

## 1. Envelope (amended)

```ts
interface AuthoringRequest {
  protocol: 'authoring-bridge/v1'
  request_id: string       // 1..64 chars, unique for app lifetime; duplicates rejected
  session_id: string       // 1..64 chars; one active authoring session per app
  sequence: number         // JSON-safe subset of Rust u64 (1..2^53-1), strictly increasing PER SESSION
  deadline_ms: number      // integer 100..30000
  operation: AuthoringOperation
  payload: Record<string, unknown>
}
interface AuthoringResponse {
  protocol: 'authoring-bridge/v1'
  request_id: string
  session_id: string
  sequence: number         // echoed exactly from the request
  ok: boolean
  result?: Record<string, unknown>
  error?: { code: AuthoringErrorCode, message: string }
}
```

**Hard ceiling:** **256 KiB total serialized IPC frame**, measured as UTF-8 bytes of the whole envelope
(`TextEncoder().encode(JSON.stringify(request)).byteLength`), not JavaScript string characters.
Request/session IDs are 1–64 **UTF-8 bytes** (Rust `str::len`), not JS code units; the envelope uses `#[serde(deny_unknown_fields)]` equivalent (no extra top-level keys). At most **4096** distinct request IDs are retained per app
launch; once the cap is reached, all further authoring requests fail closed with `SESSION_CONFLICT`
until restart. One in-flight request per app; closed operation set; payload shape and deadlines are
validated before dispatch.

## 2. Operations (closed set)

| Operation | Payload | Result |
| --- | --- | --- |
| `rig.catalog.get` | `{}` | `{ rig: { identifier, fingerprint }, bones: [{ id, display_name, index, parent_id, local_position, local_quaternion, local_scale }] }` from the **current live skeleton only** |
| `draft.create` | `{ name?, duration_seconds?, document? }` | `{ revision, validation }`. Creates an empty live-rig draft (preferred, keeps frames small) or accepts a full v1 document |
| `draft.set_keyframes` | `{ expected_revision:number, bone_id:string, property:'position'\|'quaternion'\|'scale', keyframes:AnimationKeyframe[] }` | `{ revision, validation }`. Atomic single-channel mutation at the expected revision |
| `draft.validate` | `{ document? }` | `{ state, errors }` |
| `draft.preview.start` | `{ document? }` | `{ previewing:true, clip_name }` |
| `draft.preview.stop` | `{}` | `{ previewing:false }` |
| `draft.commit` | `{ document? }` | `{ revision, entry_id }` after trusted MCP authorization |
| `draft.cancel` | `{}` | `{ cancelled:true }` |
| `draft.undo` | `{ target_revision:number }` | `{ revision, entry_id }` after trusted MCP authorization |
| `draft.export` | `{}` | `{ document }` |
| `draft.import` | `{ document }` | `{ revision, validation }` |
| `session.status` | `{}` | `{ active, session_id, revision, rig_fingerprint, previewing, authoring_enabled }` |
| `session.cancel` | `{}` | `{ cancelled:true }` |

`draft.set_keyframes` validates the current live rig, bone ID, property, value arity, quaternion
normalization/slerp rules, strictly increasing times and frozen channel/keyframe limits. A rejected
mutation leaves the draft, store and application state unchanged.

## 3. Tauri wiring (final)

- Rust emits the typed request on event **`m2m-authoring-request`**.
- Frontend subscribes through a closed-over handler and invokes Tauri command
  **`mcp_authoring_respond`** with `{ response }`; no bridge object is exposed on `window`.
- Rust emits **`m2m-authoring-cancel`** with `{ protocol, request_id, session_id }` on peer disconnect,
  deadline/cancellation, authoring disable and shutdown. Frontend dismisses confirmation UI, stops the
  preview, clears the matching session and releases listeners/timers.
- Rust IPC method name is `authoring_request` (snake_case, length-prefixed Unix frame). Bridge
  `operation` strings remain dot-separated as above. Bridge `request_id` is a string; the outer
  `IpcRequest.request_id` numeric field is unchanged.
- The browser `m2m-authoring-request`/`m2m-authoring-response` CustomEvent pair is kept only for tests
  and internal compatibility; it does not replace the Tauri adapter.

## 4. Permission and native confirmation (final)

- Authoring starts disabled in memory on every app launch. A valid authoring operation arriving over
  the existing private, same-UID Unix MCP socket grants authoring for that app session; live catalog
  discovery (`rig.catalog.get`) remains read-only and does not grant it. No toggle or persisted setting
  is used. Existing read-only MCP tools remain available independently.
- This is the user-directed permission policy amendment published 2026-11-23. The frontend grants its
  in-memory gate only on the Rust-to-Tauri MCP event path; browser CustomEvents do not grant it. Read-only
  MCP tools remain available independently.
- **User-directed commit/undo policy (2026-11-23):** an explicit MCP commit or undo request is itself
  the user's authorization. On the trusted Rust-to-Tauri event path, the frontend returns an approval
  acknowledgement bound to the exact MCP request ID, session, operation, live rig fingerprint,
  revision, document digest and undo target. No additional click is required. Browser/test transports
  continue to show `.m2m-authoring-confirmation`; client-supplied `approved` fields never authorize a
  mutation. `NativeConfirmationResult.decided_by` is `trusted-mcp` for this native route and `user` for
  a panel click.
- The confirmation panel, where used by browser/tests, shows animation name, target rig and fingerprint
  prefix, duration, track/keyframe count and revision/digest. Its expiry remains bounded by the caller's
  remaining deadline.
- Approval acknowledgement binding (returned in `NativeConfirmationResult.binding`):

```ts
interface ConfirmationBinding {
  request_id: string
  session_id: string
  operation: 'commit' | 'undo'
  rig_fingerprint: string   // fingerprint of the live rig at approval time
  revision: number
  document_digest: string   // SHA-256 of the serialized document being approved
  target_revision: number | null  // undo only
}
```

Stale, replayed, expired or mismatched approvals are rejected. The panel timer uses the **caller's
remaining deadline**, never a detached fixed timer.

## 5. Error codes (closed set)

`BAD_PROTOCOL`, `UNKNOWN_OPERATION`, `BAD_REQUEST`, `PAYLOAD_TOO_LARGE`, `DEADLINE_EXCEEDED`,
`DUPLICATE_REQUEST_ID`, `SESSION_CONFLICT`, `RIG_UNAVAILABLE`, `RIG_MISMATCH`, `INVALID_DOCUMENT`,
`PREVIEW_ACTIVE`, `CONFIRMATION_REQUIRED`, `CONFIRMATION_DENIED`, `CONFIRMATION_EXPIRED`,
`COMMIT_FAILED`, `UNDO_FAILED`, `INTERNAL`, `AUTHORING_DISABLED`, `APP_DISCONNECTED` (the last two are Rust-host additions, now mirrored in the frontend).

## 6. Application-state integrity (final)

- Commit is a compensating boundary: the store commit is rolled back if real library insertion fails.
- Undo removes the entry representing the undone revision and applies the restored document, then
  restores the prior selection — not merely an append.
- Cancel, disconnect and shutdown stop preview/mixers, dismiss confirmation UI and clear session state.
- Preview sessions register a removable per-frame callback (`remove_frame_listener`); disposal always
  unregisters it, with no inactive callbacks left on the engine.
- The bridge attaches when the real rig finishes loading (skeleton/model events plus identity polling)
  and re-attaches on rig unload, replacement or model switching. Re-entering a page is unnecessary.
- Authoring permission is session-local and reset on app restart. DOM CustomEvents are internal
  transport only and cannot grant MCP authoring permission or approve a commit/undo.
- Rig catalog data always comes from the live Three.js `Skeleton` through `read_live_rig()`.

## 7. Version history

| Version | Note |
| --- | --- |
| `authoring-bridge/v1` initial | envelope, 12 operations, 512 KiB provisional, no sequence/digest |
| `authoring-bridge/v1` M2M-028B FINAL | 256 KiB UTF-8, `sequence`, `draft.set_keyframes`, Tauri adapter, digest-bound approval, bounded dedupe, commit rollback, real undo |
| `authoring-bridge/v1` addendum | rig-ready attachment/replacement, removable preview frame listener, rig fingerprint in approval binding, explicit permission display |
| `authoring-bridge/v1` Rust reconciliation | 256 KiB UTF-8 both sides, 13 operations identical, 19 error codes aligned (`AUTHORING_DISABLED`, `APP_DISCONNECTED` added), ID byte-length validation, no unknown envelope fields, sequence JSON-safe subset of u64 |
| `authoring-bridge/v1` user-directed permission amendment (2026-11-23) | Remove enable toggle; private-socket MCP use grants session access; trusted MCP commit/undo operations provide digest-bound authorization; browser/test paths retain the panel |
