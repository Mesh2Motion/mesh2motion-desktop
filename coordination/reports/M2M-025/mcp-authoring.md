# M2M-025C — MCP authoring foundations and integration boundary

**Status: backend contract mirror implemented; native authoring bridge not ready. No MCP write tools are exposed.** Work is on isolated branch `m2m-025c-mcp-authoring` at `725b93ad5a5d6206febadfd8032638e66c0a9ea6`; its isolated frontend gitlink is `a33015c082aa753f04b6685e94b57297cf375207`. Canonical desktop branch and gitlink remain unchanged. Installed r73 was not used or modified.

## Contract source and provenance

- Authoritative contract file: `coordination/contracts/animation-document-v1.md`; independent frontend checkout: `worktrees/m2m025b-animation/mesh2motion-app`.
- Contract foundation commit: `55d7e349181c452ae0dccfe1d785782d1c5127db`; completed OpenCode handoff: `ae48734813fa92788f391a6d1ab4001cae4fb0d7`.
- Integrated frontend chain contains M2M-025D first-paint work (`313d7ac` through `5f8134081883069abc35f538da212738bb40caa5`) and the C-owned frontend changes on `m2m-025c-frontend-integration`; final integration tip is recorded below.
- Published shared text: `coordination/reports/M2M-025/animation-document-contract.md`.
- `liveRigAdapter.ts`, the native-confirmation interface, and the application commit controller are committed by `ae48734`. The adapter and controller are still not connected to the actual MCP publisher, Tauri IPC, or live animation library.

## Field-by-field synchronization

| Contract field / behavior | TypeScript source | Rust mirror | Actual live engine / MCP boundary |
| --- | --- | --- | --- |
| `schema_version` | literal `animation-document/v1` | `AnimationDocument.schema_version: String`, validator enforces exact literal | Not in current MCP snapshot/request schema |
| `rig.identifier` | supported identifiers `human`, `fox`, `horse` | string; non-empty/length-limited | Real identifier comes from selected rig type; only the new adapter recognizes these three |
| `rig.fingerprint` | SHA-256 of ordered names, parent indices, local rest transforms quantized to six decimal places | string; compared to caller-provided live fingerprint; 64 hexadecimal characters required | Three.js skeleton available in uncommitted `liveRigAdapter.ts`; current committed `EngineControlHost` publishes only type/count/root, not fingerprint |
| `bones[]` | `id`, `display_name`, `index`, `parent_id`; ID is `<rig>:bone:<3-digit index>` from `Skeleton.bones` | matching serde struct with exact live catalog equality check, max 256 | Adapter derives entries from actual `Skeleton.bones`; it is not wired into committed MCP snapshot |
| `clip` | `name`, `duration_seconds` | matching struct; nonblank name; finite duration in `(0, 3600]` | No native library mutation bridge |
| `channels[]` | `bone_id`, property (`position`, `quaternion`, `scale`), ordered keyframes | serde enums/structs; max 512 channels; rejects duplicate bone/property channel and unknown bone | No MCP request schema exposes these yet |
| `keyframes[]` | seconds, numeric values, interpolation (`linear`, `step`, `slerp`) | finite values; strict time increase and in-duration; 3 values for position/scale, 4 XYZW for quaternion; unit quaternion tolerance `1e-4`; slerp only quaternion; max 4096 total | Conversion to Three.js clip is local to frontend adapter; no RPC preview request |
| `revision` | starts at 1, next atomic commit increments one, undo appends a revision | revision-positive validation, optimistic expected revision, exact next revision, unchanged on conflict | Native UI confirmation and native application undo are absent |
| `validation` | `draft` / `valid` / `invalid` with deterministic errors; `with_validation` recomputes it | mirror; accepted stored documents have validation reset to `valid` with an empty error list after successful structural validation | No MCP IPC envelope yet |
| atomic draft storage | in-memory reference frontend store, append-only undo history | session-keyed in-memory store, max 16 sessions, max 32 retained revisions per session, each serialized document <=256 KiB | Rust storage is not a real application animation and is not exposed over MCP |

### Actual rig publisher

Committed `EngineControlHost` receives the actual engine but currently publishes only skeleton type, bone count and root name. The committed adapter implements `read_live_rig(skeleton, identifier)` and derives stable IDs, hierarchy, local transforms and fingerprint from the real `Skeleton`; it supports Human/Fox/Horse. It is not yet connected to `EngineControlHost` or the Tauri snapshot, so the MCP publisher still does not return the catalog. We do not synthesize bone names or rig metadata in Rust.

### MCP schemas

The current Rust MCP request/response schemas contain only the five read tools (`application_get_status`, `renderer_get_state`, `rigs_list`, `animations_list`, `scene_inspect`). They have no AnimationDocument request/response fields. This is intentional until IPC and user confirmation are implemented.

## Differences and unresolved schema questions

1. Duration drift is reconciled on the C branch: TypeScript now rejects `d <= 0`, accepts every positive finite `d <= 3600`, and Rust follows the same frozen rule. Quaternion normalization and interpolation restrictions now match the frozen contract in both validators.
2. The frozen `native-confirmation/v1` request/result shape is implemented in TypeScript. Its Tauri transport, approval expiry, concurrent clients, disconnect cleanup, and session identity binding remain provisional.
3. MCP operation names and schemas, bounded Rust→frontend request/reply transport, live app-owner wiring, and whether to add a separate read tool for the bone catalog remain to be agreed before write tools are exposed.

## Rust implementation and limits

Added internal typed mirror and `AnimationDraftStore` in `src-tauri/mcp-server/src/animation_document.rs`. It enforces size/fingerprint/catalog/data bounds and revision-checked mutations. `lib.rs` exports these foundations for Rust use only. No tool router or Tauri command was added. Existing private Unix socket, same-UID policy, request limit and five read-only tools are unchanged. Authoring permissions are default-disabled because there is no authoring capability; native opt-in must be implemented as a separate explicit permission in the bridge before tools are registered.

The transport ceiling remains 256 KiB. A document is independently capped at 256 KiB; larger future edits must use bounded incremental keyframe operations and must be designed with frontend before publication. Store memory is bounded by 16 sessions × 32 revisions × 256 KiB (128 MiB maximum serialized history, excluding map/runtime overhead).

## Verification

- `cargo fmt --all -- --check`: passed.
- `cargo test --locked --manifest-path mcp-server/Cargo.toml`: passed, 25 library tests and 3 sidecar tests, including document validation, atomic commit and undo. The first sandboxed run could not bind two Unix-socket tests; the identical rerun outside the sandbox passed all tests.
- `cargo test --locked --workspace --all-features`: passed (3 Tauri contract/host tests); `cargo clippy --locked --workspace --all-features --all-targets -- -D warnings` and `cargo fmt --all -- --check` passed on the final source.
- Frontend at the integration tip: Vitest 57 files / 367 tests, ESLint `--max-warnings 0`, TypeScript, and Vite build passed. The native NATIVE-005 visual pass was confirmed by the user at `Bildschirmfoto_20261123_034321.png`.
- Real JSON-RPC MCP client smoke path against the running native host: initialize, `tools/list`, and `tools/call` for all five read-only tools passed. The transcript is `/tmp/m2m025c-mcp-smoke.ndjson`; sidecar exit `-15` was the expected controlled termination after the smoke test. No write path was run because no write operation exists; no client-side authoring behavior is claimed.
- Bounded Bacon `check` reached successful Cargo compilation; its watcher was stopped at 25 seconds (exit 124). ast-grep found no unsafe blocks in changed Rust files (no-match exit 1). `pytest -q`: 3 passed. Ruff reports 17 existing lint findings in audit/smoke Python files; `ty check .` reports missing PyGObject Gtk/WebKit2/GLib/Gdk type members. These Python findings are unrelated to this Rust/frontend change.

## Package provenance

- Exact Rust source for this work is desktop branch `m2m-025c-mcp-authoring`, commit `725b93ad5a5d6206febadfd8032638e66c0a9ea6`; exact frontend integration source is commit `a33015c082aa753f04b6685e94b57297cf375207`. The merged desktop integration parent is `6b247d2e0d85b2bc57aa3d17d5a1c0806113362c`.
- Installed r73 is stale as documented in the prior report; no package install occurred.
- A future release build must compile the sidecar from this release source. It must not copy a staged executable. Build and package hashes remain pending because no package was built.

## Required OpenCode work before authoring tools

1. Wire `liveRigAdapter.ts` into the real live publisher so MCP reads the actual rig fingerprint/catalog.
2. Agree and implement typed, bounded Rust→frontend IPC request/reply over the existing private Unix socket; retain the 256 KiB request limit.
3. Wire the native confirmation prompt and acknowledgement plus the application-owned commit/cancel/undo controller into the actual engine and animation library.
4. Add and run native adapter tests covering real draft appearance, confirmation, commit, cancellation, undo, and export/reimport. Authoring MCP tools remain unavailable until this path is tested.

## Commit record

- Rust implementation commit: `10e88fcb6a221654f9dc7818501fbc58a55edbba` (`M2M-025C: add AnimationDocument Rust foundations`).
- Final Rust contract synchronization commit: `b56ad0ab350516ec2f9387e0cdfb5282ea6d87ba`; integrated desktop branch tip: `725b93ad5a5d6206febadfd8032638e66c0a9ea6`.
- Desktop feature branch includes the integrated frontend gitlink. No package was installed.
- Source SHA-256: `src-tauri/mcp-server/src/animation_document.rs` = `d9c0727039eeac10cad6f7ae4ef0a5f1d8f0023d92f659b45d71f8a7c06126e2`; `src-tauri/mcp-server/src/lib.rs` = `a47af83faab64d202131252f1e1fe4267076f8302c9cccf31cfdf39e974804ef`.
- Rust source SHA-256 at final branch tip: `animation_document.rs` = `18eedd275388e397f91edc92bd4dfdcd8e21744940c5dcde7fe782a7ab166d72`; `lib.rs` = `a47af83faab64d202131252f1e1fe4267076f8302c9cccf31cfdf39e974804ef`.
- Rebuilt debug candidate binaries (not packaged or installed): `src-tauri/target/debug/mesh2motion` = `d51a340a49e17925340e0618b7effb075160e96ec273e89149f78b620c953aab`; `src-tauri/mcp-server/target/debug/mesh2motion-mcp` = `3fe484f8071c78f929d94d9f2dfc1162a285a07cb2707fd9e061fc60ff22532e`. Both were rebuilt from the exact isolated source with locked Cargo dependencies. A future package recipe must build the sidecar from this source and preserve these source/build inputs; do not reuse staged r73.

## Native visual bug updates

- NATIVE-005: OpenCode's final integrated frontend fix is `5f8134081883069abc35f538da212738bb40caa5`; the user confirms both the remaining black catalog box/padding residue and scrollbar are resolved in native visual inspection `Bildschirmfoto_20261123_034321.png`. No package was installed.
- NATIVE-006: C-branch source fix is present in the integrated frontend tip: initialize `CustomSkeletonHelper` instance matrices at zero scale before the first live update. Regression test is `src/lib/CustomSkeletonHelper.test.ts`; source-level test suite passed. Native confirmation remains **pending**: a Wayland screenshot of the current desktop showed terminals rather than the app, so it did not verify the rigging tab. Do not call the visual bug natively resolved yet.


---

## HANDOFF NOTICE (added by OpenCode M2M-025B — do not treat as your own finding)

The AnimationDocument v1 contract and implementation are now committed. Read
`coordination/handoffs/M2M-025B-to-Codex.md` and `coordination/contracts/animation-document-v1.md`.
Frontend branch `m2m-025b-animation-document`, authoritative commit
`ae48734813fa92788f391a6d1ab4001cae4fb0d7`. The previous "contract not published" blocker is resolved.
