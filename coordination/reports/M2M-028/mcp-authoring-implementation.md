# M2M-028B — Secure MCP authoring implementation

## Current integration/acceptance update — 2026-11-23

The `m2m-028c-integration` desktop branch now merges `m2m-028b-keyframe-authoring` at desktop merge commit `691f230`; current desktop tip is `1b44a30`, whose exact frontend gitlink is `05051093d4cac6762507b75b35a500b749724219`. The integration retains the exact-request/session/operation-bound trusted MCP commit and undo path. OpenCode `4384a73` was reviewed but not selected because checking only `window.__TAURI_INTERNALS__` is broader than the trusted MCP event context.

The real native JSON-RPC workflow reached commit, export/import comparison, undo, and cancel. The user then rejected the resulting wave pose after reviewing `Bilder/Bildschirmfotos/Bildschirmfoto_20261123_165424.png`; the arm is raised overhead and the torso bends, so **no visually successful Agent_Wave_01 is claimed**. The user requires synchronized front and side internal views of the live model for pose placement. Camera work and a corrected pose remain acceptance dependencies. See `coordination/reports/M2M-028/agent-wave-live-demo.md` and the transcript `coordination/audits/M2M-028/mcp-agent-wave-final.ndjson`. No package/install occurred.

Transcript diagnosis: authored arm/spine/hand/head channels contained absolute-local identity/axis-delta quaternions, while the actual live Human rig has non-identity local rest quaternions. That overwrote the bind pose and accounts for the distorted result. `tools/mcp/agent_wave_mcp_demo.py` now composes the desired local delta with each live `skeleton_get_catalog.local_quaternion`; added Python tests pass. This helper has not yet been used in a new native preview, so the visual bug is not yet verified fixed.

The user clarified that internal MCP cameras must not change the application UI. Codex reverted the camera-visible frontend from the integration gitlink, restoring the original viewport and view-helper. The prior visible camera prototype `558ee2f` remains unmerged; internal camera access needs a separate MCP-only contract and must return no UI panel, labels or controls.

**Status: native transport, live catalog and mechanical MCP authoring lifecycle verified; visual animation acceptance failed.** A clip was created/committed/undone through the native app, but the user rejected its pose. Do not claim a successful Agent_Wave_01 animation until a corrected pose is visually verified with front and side views. No package was built or installed.

## Source and ancestry

| Component | Branch / revision | Notes |
| --- | --- | --- |
| Desktop Rust | `m2m-028b-keyframe-authoring` @ **`cf3a94aa9854033dc1e9bc3257d605551db9e227`** | Based on M2M-027 and includes the authoring bridge, permission amendment, request lifecycle fix, cancellation cleanup and its regression test. |
| Integrated frontend | `m2m-028b-mcp-permission` @ **`05051093d4cac6762507b75b35a500b749724219`** | Contains the agreed bridge, permission amendment, live rig integration, null payload fix, diagnostic detail, and trusted-MCP auto-authorization bound to the exact request. Browser/test paths retain the panel. |
| Desktop gitlink | `mesh2motion-app` → **`05051093d4cac6762507b75b35a500b749724219`** | Verified exact equality with the checked-out submodule commit. Canonical branches and installed app remain untouched. |

OpenCode's field-by-field contract reconciliation is at `coordination/handoffs/M2M-028A-bridge-channel.md`; final contract is `coordination/contracts/authoring-bridge-v1.md`. `animation-document/v1` remains frozen.

## Schema and bridge

- Request envelope: `{ protocol, request_id, session_id, sequence, deadline_ms, operation, payload }`. Both sides reject unknown top-level keys; IDs are 1–64 UTF-8 bytes; sequence is positive and strictly increasing per session (frontend accepts the JSON-safe subset); deadline is 100–30,000 ms.
- Serialized request ceiling is **256 KiB UTF-8 bytes** across the whole envelope. Rust socket limits were not increased. Longer clips use bounded `draft.set_keyframes` channel replacements.
- Thirteen closed operations: `rig.catalog.get`, `draft.create`, `draft.set_keyframes`, `draft.validate`, `draft.preview.start`, `draft.preview.stop`, `draft.commit`, `draft.cancel`, `draft.undo`, `draft.export`, `draft.import`, `session.status`, `session.cancel`.
- Nineteen error variants are shared: `BAD_PROTOCOL`, `UNKNOWN_OPERATION`, `BAD_REQUEST`, `PAYLOAD_TOO_LARGE`, `DEADLINE_EXCEEDED`, `DUPLICATE_REQUEST_ID`, `SESSION_CONFLICT`, `RIG_UNAVAILABLE`, `RIG_MISMATCH`, `INVALID_DOCUMENT`, `PREVIEW_ACTIVE`, `CONFIRMATION_REQUIRED`, `CONFIRMATION_DENIED`, `CONFIRMATION_EXPIRED`, `COMMIT_FAILED`, `UNDO_FAILED`, `INTERNAL`, `AUTHORING_DISABLED`, `APP_DISCONNECTED`.
- Rust retains all five read-only tools and adds `skeleton_get_catalog` plus the eleven draft/session tools. Drafts are bounded and session-scoped; Rust validates AnimationDocument v1, fingerprint, bone references, revisions, duration, keyframe/channel/bone limits, finite values and normalized quaternions before forwarding.
- Requests travel over the existing private Unix socket with same-UID checks and 0700 runtime directory / 0600 socket. Rust emits typed `m2m-authoring-request` Tauri events and correlates `mcp_authoring_respond`; cancellation uses `m2m-authoring-cancel`. The frontend has no arbitrary JS, shell, network or filesystem execution path.
- A separate frontend readiness bit keeps requests from timing out before Tauri listeners attach. The event adapter supplies Tauri's required `target: { kind: "Any" }` and unregisters by `eventId`.
- **User-directed permission amendment (2026-11-23):** the native enable toggle and `mcp_authoring_set_enabled` command are removed. Each app launch starts with in-memory authoring disabled; the first valid authoring-bridge operation arriving over the private same-UID MCP socket grants access for that app session. Live catalog discovery remains read-only and does not grant access. The frontend grants only on the Rust-to-Tauri MCP event path; browser CustomEvents cannot grant it. Commit/undo approval remains bound to request, session, operation, rig fingerprint, revision, SHA-256 document digest and undo target.
- `AnimationDraftStore` is a validated Rust mirror. The Three.js frontend owner remains authoritative for viewport, library insertion, confirmation, cancel and undo; a Rust-only draft is not represented as an application animation.

## Live rig identity correction

The first real native catalog request failed `RIG_MISMATCH`: the frontend hashed mutable `Bone` transforms after the animation mixer started. The integration now derives rest transforms from the actual live `Skeleton.boneInverses` through the canonical `liveRigAdapter`; IDs and hierarchy still come from the actual `Skeleton.bones`. A regression test separates an animated pose from a changed bind pose. The Tauri listener's missing target argument was also fixed.

Native JSON-RPC now returns `ok=true` for Human with 66 bones and fingerprint `1a167d80d91326d763af6c04914790d814010341d843008129ca2102d7f4c31d`. The live app reports Human loaded and `Angry` selected; the catalog read made no application-state change.

## Verification performed

- Frontend: ESLint **0 errors / 0 warnings**, `tsc --noEmit` pass, Vitest **61 files / 411 tests**, Vite production build pass. Vite reports the existing `__dirname` config warning and >500 KiB offline bundle warning.
- Rust: `cargo fmt --all`; MCP locked suite **51 library + 3 sidecar tests passed**; workspace suite **2 Tauri tests passed**; Clippy with `-D warnings` passed. Dedicated tests include deny-unknown-fields parity, UTF-8 ID byte length, the two shared error codes, store revision/atomicity, malformed/non-finite documents, fingerprint/bone references, limits, duplicate IDs, sequence replay, deadline, disconnect cleanup, same-UID and socket permissions.
- Frontend security coverage: browser-event denial, MCP-request session grant, cross-session isolation, request replay/dedupe, digest-bound approval, expiry/denial, failed owner insertion rollback, undo restoration, client cancel/disconnect cleanup, and bridge lifecycle tests.
- Maintained MCP Python checks: Ruff pass, pytest **4 passed** (one installed Dash hook deprecation warning), `ty check tools/mcp` pass.
- ast-grep explicit TypeScript `any` and Rust `unsafe` scans returned no matches (ast-grep exit 1 means no matches). `bs --version` reported BugStalker 0.4.9; there was no failing Rust target/core requiring an interactive debugging session.
- Bounded Bacon `check --all-features`: Cargo check completed successfully; the watcher was stopped by the 30-second bound (exit 124), consistent with Bacon's persistent watch behavior.
- No new production dependency or system package was installed.

## Real MCP JSON-RPC boundary

`coordination/audits/M2M-028/mcp-transcript.ndjson` is the earlier real maintained stdio-client transcript from the isolated Tauri app and sidecar. `initialize`, `tools/list`, all five existing read tools and `skeleton_get_catalog` succeeded; catalog contained 66 live Human bones. The previous native toggle was clicked by the user, but the running binary predates this permission-flow change. A fresh native build and transcript must still verify that the first valid authoring MCP operation grants session permission. No animation creation is claimed.

## Remaining gate

The current staged binary predates the permission-flow change. Rebuild and relaunch it, then verify the first valid authoring MCP operation enables this session without a UI toggle. The maintained MCP client must create `Agent_Wave_01` from an empty live-rig draft, set original keyframes by catalog bone IDs, validate through Rust, preview, and request commit. The user must approve the digest-bound native confirmation. Afterward verify the actual animation library, export/reimport equivalence, cancel and undo restoration. No approval evidence, animation export/reimport or end-to-end success is recorded yet.

No package was built or installed. For the isolated staged candidate, `cargo build --locked --workspace --all-features` plus `cargo build --locked --manifest-path mcp-server/Cargo.toml --bin mesh2motion-mcp` used desktop source **`15b0f3a099e6176803788c6f28a18ec79687da5a`**, frontend gitlink/dist source **`a96dff2da3c42d012b5b86e9d1def1a96fa452c1`**, and locked dependencies. SHA-256: desktop executable `6d430954d304ace1146ed6cb7222b82ab347a7e015221cb87f89b72b1e7cb40a`; MCP sidecar `0b14450e14772bad5dab5e6c527f07d0fbef812a3a4c40e6e02c6c63f4f5538d`. These are local debug build hashes, not package payload hashes. Installed application and package state are unchanged.

## 2026-11-23 — current native integration update

The previous build hashes and partial-demo status above are historical and superseded by the feature
tip in the source table. One real native run created the empty live Human draft, authored five
quaternion channels through the bounded set-keyframes tool, validated it through Rust, started actual
viewport preview and exported the draft. Commit returned success, but the following `animations_list`
call omitted it because the tool excludes `custom-import` entries unless `includeCustom` is true. The
demo ended before undo/export-reimport. The maintained client now requests custom entries and polls
for visibility. The next staged run uses automatic authorization only through the trusted
Rust-to-Tauri MCP request path; browser/test paths retain the digest-bound panel.

The current feature commits fix `BridgeWait` cancellation after normal responses, Rust-null optional
document handling for preview/commit, and exact MCP-request authorization for commit/undo. Frontend
checks passed (ESLint, TypeScript, 61 files / 413 Vitest tests, Vite); Tauri checks passed (fmt, 4
workspace tests, Clippy with denied warnings); MCP host-feature tests passed (52 library + 3 sidecar
tests). The MCP host tests required the approved elevated run because sandboxed Unix socket bind
failed with `EPERM`. Maintained Python checks passed (Ruff, 4 pytest tests, ty). ast-grep explicit
TypeScript `any` and Rust `unsafe` scans returned no matches. No package or installation occurred.

---

# UPDATE — integration and live sequence complete

Frontend `m2m-028c-merged` @ `4384a7310e46689db7c73270d0dfb88c8b04bb90` (child of merged tip
`8ebeac4`) adds trusted-MCP auto-approval for commit/undo. Desktop `m2m-028c-final` @
`73194cf1fd2f0a27ea7b1d6b66ae0af7b7de6db5` pins that frontend. A real JSON-RPC client completed
catalog → create draft → five keyframe revisions → validate → preview → commit → export/reimport →
undo → cancel on the staged native app. See `coordination/reports/M2M-028/integration-final.md` and
`coordination/audits/M2M-028/agent-wave-final-transcript.ndjson`. No package was installed.
