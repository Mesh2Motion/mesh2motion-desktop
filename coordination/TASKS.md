# Development Tasks

- [x] M2M-031 cleanup — double-checked Dash warning origin, Vite chunk composition, BugStalker execution, and shared node_modules symlink. Fixed the JSDOM warning, TypeScript generated-output race, deprecated Vite `__dirname`, and untracked symlink status. Frontend branch `m2m-031-cleanup` commits `b0c9cf6` and `f254e01`; release refs and installed r82 unchanged. Results: `coordination/reports/M2M-031/cleanup.md`.

- [x] M2M-030A — six internal MCP observation cameras, offscreen capture, joint projection and dev harness complete. Contract `mcp-pose-observation/v1` @ `9683e96`; real images in `artifacts/M2M-030/pose-inspection/`. Codex transport confirmation still pending.
- [x] M2M-031 — New Animation tab complete: From Scratch/Existing, gizmo bone editing, Godot-style keyframe editor, playback, shared authoritative draft, saved-clip persistence, Explore inspection with real thumbnails/flipbook previews, protected deletion with confirmation. Final frontend `0920f70`; user-verified screenshot `artifacts/M2M-031/screenshots/06-working-preview-verified.png`.
- [x] M2M-030/031 RELEASE — user-confirmed preview documented; promoted to frontend `m2m-031-release` @ `0920f70` and desktop `m2m-031-release-desktop` @ `e439eb1` (pins `47962b6` / frontend `0920f70`). Built and installed r82 package pair `0.1.10.r82.m2m031-1`; verified hashes, native launch and post-install MCP smoke (17 tools, 11 authoring). There is no literal `main` branch. Full record: `reports/M2M-031/final-release.md`.

- [ ] M2M-028B — Finish native authoring acceptance. User-directed permission amendment removes the native enable toggle; a valid MCP authoring request grants per-app-session access. Staged JSON-RPC permission smoke passed with live 66-bone Human catalog, all read-only tools, enabled session status, and empty draft create/cancel. Frontend 61/411, Rust MCP 51+3, Tauri 3, Clippy, Ruff, pytest 4, ty pass. Remaining: Agent_Wave_01 keyframes/validation/preview, native digest approval, library commit, export/reimport, cancel and undo. No package installed. See `reports/M2M-028/mcp-authoring-implementation.md`, `agent-wave-live-demo.md`, and `integration-status.md`.

- [ ] M2M-025C — Await published AnimationDocument v1 schema and frontend adapter. Then implement Rust validation/permission integration and run real Agent_Wave_01 MCP end-to-end tests. Current blocker/details: `reports/M2M-025/{mcp-authoring,agent-wave-end-to-end}.md`.

- [ ] M2M-024 native acceptance: verify poster fallback and hover/pause/loop behavior, virtualization and 20/20 ceilings; Props inset during resize, tiling, fullscreen, sidebar and DPR; Settings at normal/narrow/fullscreen with keyboard/MCP controls.
- [ ] On the exact r78 staged package, test Human/Fox/Horse switching, fullscreen and KDE tiling; investigate prior WebKit child SIGABRT only with new evidence.
- [ ] Review the r78 package’s embedded `$srcdir` path before calling the package reproducible.
- [ ] Recheck the prior WebKit child-process crash with native evidence.
- [x] Install candidate after explicit user request; rollback packages preserved.
- [ ] Continue upstream compatibility and offline-operation QA.

- [x] M2M-022 — Create independent Project-2 workspace, integrate architecture into verified RC, run frontend/Rust/Python checks and install candidate at user request. Native feature acceptance partial; see M2M-022 and M2M-023 reports.
- [x] M2M-023 — Record native acceptance findings NATIVE-001 through NATIVE-004; fixes and complete release acceptance remain open.
- [x] M2M-023B — Verify installed native MCP read tools end to end; commit isolated host-exit cleanup and explicit MCP-client app autostart; validate staged native launch, all five read tools, default-off behavior and normal-close socket cleanup. Write operations remain blocked on Agent 3 AnimationDocument agreement. Details: `reports/M2M-023/codex-mcp.md`.
- [x] M2M-021C — Zero-lint integration baseline 3909f18 / ce06c9f; native acceptance blocked.
- [x] M2M-021A/B/D — Parser reports copied; Codex parser retained, OpenCode alternative archived and not merged.
- [x] M2M-021B architecture — Integrated from ce073a5; see reports/M2M-022/integration.md.
- [x] M2M-023A — Native UI regression repair. Isolated frontend `m2m-023a-native-ui` at `3fac9db` (preceded by corruption repair `25b4f3f`); 684 offline WebP posters + poster-aware tiles, CSS viewport-anchored Props dock, MCP/settings layout fix, 8 new tests. ESLint 0/0, tsc 0, Vitest 47/288, Vite build OK. Staged Tauri binary `e22bbe70…` with matching resources; native GUI verification BLOCKED with fish steps. Report `reports/M2M-023/opencode-native-ui.md`.
- [x] M2M-023C — Poster correction integrated at `e06d6bb`; removed per-tile ▶ controls and resolved WebP paths through the existing MP4 allowlist key.
- [x] M2M-024A source integration and r78 package preparation. Native Human posters and wide-layout Props position directly observed; package not installed. Overall milestone remains open until native checks pass. Reports: `reports/M2M-024/`.

- M2M-023D — settings toggle rebound after late nav markup injection; right-rail insets set for tool-panel pages at all widths. Isolated branch `m2m-023d-settings-and-layout` at `c417eb2` (Codex branch untouched). ESLint 0/0, tsc 0, Vitest 48/299, Vite OK. Staged candidate `worktrees/m2m023d-ui-fix/stage` binary `6efca568…`; native verification still BLOCKED.
- [x] M2M-024 — Merge and system install. Frontend `m2m-024-release` (c417eb2), desktop `m2m-024-release-desktop` (68616f3) pinning it; ESLint 0/0, tsc 0, Vitest 48/299, Vite OK; built + installed `0.1.10.r73.gc417eb2-1` (desktop hash e432afd5…, MCP b5918d30…); installed app PID 1434395 alive with 684 MP4 + 684 WebP resources. User visually confirmed native parity for reported defects. Report `reports/M2M-024/final-release.md`.
- [x] M2M-025A — Create independent Project 3 canonical baseline; verify M2M-024 Git objects/ancestry and provenance discrepancies; restore dependencies from lockfiles; run full quality suite; document native acceptance gaps. STOP after baseline verification.
- [x] NATIVE-005 code fix — WebKitGTK computed-style capture showed identical correct colors before/after scroll (not a variable-timing bug); applied themed background + `min-height:100%` to `.virtualized-catalog-content` on isolated branch `m2m-025d-remix-first-paint` commit `313d7ac`, with first-paint regression tests. ESLint 0/0, tsc 0, Vitest 48/303, Vite OK, cycles 2/2. Native pixel confirmation BLOCKED. Report `reports/M2M-025/remix-first-paint.md`.
- [x] M2M-025B — Canonical typed AnimationDocument v1, stable rig-derived bone IDs, deterministic validation, pure THREE.AnimationClip adapter, isolated in-memory preview, atomic commit/undo interface, original Agent_Wave_01 fixture, export/reimport and transform-equivalence tests. Isolated branch `m2m-025b-animation-document` commit `55d7e349`; ESLint 0/0, tsc 0, Vitest 54/339, Vite OK, cycles 2/2. Native visual acceptance BLOCKED. Reports `reports/M2M-025/animation-document.md` and `agent-wave-fixture.md`.
# M2M-025C continuation

- [x] Locate/review published AnimationDocument v1 in independent OpenCode checkout.
- [x] Implement Rust typed mirror, validator and bounded draft storage without exposing MCP writes.
- [ ] OpenCode: commit and wire live rig catalog/fingerprint publisher.
- [ ] OpenCode + Codex: agree and implement bounded native IPC preview/confirmation/commit/cancel/undo bridge.
- [ ] Resolve duration lower-bound and validation-field semantics.
- [ ] Add/verify actual MCP JSON-RPC authoring flow, native play/commit/export/reimport, and package provenance before claiming Agent_Wave_01.

- [x] M2M-025B handoff — live rig-to-document adapter (`read_live_rig`, `check_rig_compatibility`, `create_document_from_live_rig`, `draft_document_to_clip`), native-confirmation/v1 contract, and application-owned `ApplicationCommitController` (commit/cancel/undo). Commit `ae48734813fa92788f391a6d1ab4001cae4fb0d7` on `m2m-025b-animation-document`; ESLint 0/0, tsc 0, Vitest 56/358, Vite OK, cycles 2/2. Published `coordination/contracts/animation-document-v1.md` and notified Codex via `coordination/handoffs/M2M-025B-to-Codex.md`. Native GUI tests BLOCKED.
- [ ] NATIVE-006 (Codex-owned) — “Use Your Rigged Model” tab briefly shows a black triangle in the 3D viewport on switch. Evidence `Bildschirmfoto_20261123_032617.png` / `_032531.png`. Not part of Remix NATIVE-005 work.

- [x] M2M-025 integration — merged M2M-025B (`55d7e34`, `ae48734`) and M2M-025D (`313d7ac`) into frontend `m2m-025-integration` (`c5f3cca`, `90ff637`) and pinned desktop `m2m-025-integration` (`7b060d6`). Canonical branches and installed app untouched.

- [x] NATIVE-005 RESOLVED — Remix catalog first-paint background fully themed on native staged build; user confirmed with `Bildschirmfoto_20261123_034321.png`. Final frontend `5f81340`, desktop pin `6b247d2`; quality ESLint 0/0, tsc 0, Vitest 56/364, Vite OK, cycles 2/2. Report `reports/M2M-025/remix-first-paint.md`.
# [COMPLETE] M2M-027 — Merge M2M-025C + M2M-026, verify, package, install release
- Integration branches: not created yet. Verified requested source SHAs exist; canonical branches are clean.
- Installation gate is closed pending final package provenance, staged native checks and user visual verification of five NATIVE-006 first frames.
- Current installed versions are r73; recovery hashes and saved package archives are being inventoried before integration.
- Project 3 `PROJECT.md` and `DECISIONS.md` were not present at the expected paths; no substitute project documents have been assumed.

# M2M-027 — COMPLETE

Integrated, built, and installed the reviewed desktop/frontend pair. Reports: `coordination/reports/M2M-027/`. NATIVE-006 accepted by user on staged native build after five tab switches. Post-install versions, exact binary hashes, and installed JSON-RPC read tools were verified. MCP write tools remain unavailable pending the live frontend authoring bridge.
- [x] M2M-030B integration — merged Rust MCP, Codex transport, and OpenCode parity/determinism/preview fixes on isolated branches. Desktop final evidence commit `8b763f5a5f3f0eb201aa4e9b50ac603cfcbfaf6d` (code/gitlink commit `94518dc5cdd093f66693f4364af3cffd026cfb9e`) pins frontend `c83c210b63a8fee0680ca563d460a32950665997`; frontend 479 tests, TypeScript, ESLint and Vite pass; locked Rust build, prior tests/fmt/Clippy, Ruff/pytest/ty and ast-grep pass. Native JSON-RPC integration probe received six real 512×512 PNGs and canceled its disposable draft. OpenCode vision-guided wave run verified parity, repeated capture, preview, commit, export/reimport, and undo. Remaining: human aesthetic sign-off, legacy staged six-track clip cleanup, native fault injection; Vite large-chunk advisory and bounded Bacon/BugStalker limitations documented; deprecated __dirname warning fixed. No canonical promotion or install. Reports: `reports/M2M-030/`; audit: `audits/M2M-030/mcp-vision-integration-native.ndjson`.

- [ ] M2M-032 — Reproduce and fix user-reported Dragon → Horse rig loading failure (Horse model
  does not load and its skeleton is not exposed); verify repeated rig switches and selected rig /
  skeleton / fingerprint consistency in the native app. See `reports/M2M-032/dragon-horse-rig-loading.md`.
- [ ] Project 4 migration — wait for OpenCode's in-flight frontend fix, integrate its committed
  change, rebuild the exact paired sources, verify clean Git state, and only then install. Do not
  push; GitHub upload is for the user's later action.
