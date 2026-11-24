# M2M-030B — integrated feature status (2026-11-24)

- Integrated branches: desktop `m2m-030b-integration` final report commit `8b763f5a5f3f0eb201aa4e9b50ac603cfcbfaf6d` (code/gitlink commit `94518dc5cdd093f66693f4364af3cffd026cfb9e`) (Rust feature `622ebe804ca3b25cc20e764a54ffa3a542e1ecf8`); frontend `m2m-030b-integration-frontend` @ `c83c210b63a8fee0680ca563d460a32950665997` (OpenCode `98fcf60c6e882a05a435f1a7df50ae22b6846357`, Codex transport `63b8b95ce9f9c3e125cc8e18ce75de904325d949`). Desktop gitlink matches the integration frontend. Canonical `m2m-025-dev`, release branches, packages and installed app remain unchanged.
- Resolved engineering issues: renderer/joint parity, stale rig binding and repeated capture, deterministic bind-pose evaluation, stable authoring transport on rig swaps, undo sequence, sparse-clip playback, authored previews. Native image-guided workflow and undo passed in OpenCode staged app.
- Verification: frontend 71 files / 479 tests, TypeScript, ESLint and Vite build pass; locked Rust build pass; previously reported locked Cargo tests, fmt, Clippy, Ruff, pytest 4, ty and ast-grep pass. Final native MCP probe against the freshly built integration app returned six real 512×512 PNGs and matching rig/projection/joint metadata, then canceled its disposable draft. Audit and PNGs in `audits/M2M-030/mcp-vision-integration-native.ndjson` and `artifacts/M2M-030/integration-native/`.
- Remaining acceptance/maintenance notes: no human aesthetic sign-off; the previous six-track `Agent_Wave_01_vision` entry remains in the staged library, with `Agent Wave 01 clean` (66 tracks, preview frames) as the corrected clip. Native fault-injection cases were not rerun. Vite retains `__dirname` and large-chunk advisories; external Dash deprecation is external. Bacon watcher timeout and BugStalker no active process are recorded without claiming success. No install or promotion to `main`. Detailed report: `reports/M2M-030/`.

# Project State

## M2M-030A + M2M-031 — RELEASED AND INSTALLED (2026-11-23 22:20)

- Final frontend `m2m-030-031-observation-animation` @ **`0920f70`**; final staged desktop
  `m2m-030-031-integration` @ **`47962b6`** (gitlink `0920f70`); staged binary SHA-256
  **`f41baf7c436da408ed4a9e17e8e89673180476614f5a75fae9162635c1d995e2`**.
- Both features implemented and quality-checked. The user visually confirmed the
  authored animation preview on the staged native build (`Bildschirmfoto_20261123_215020.png`,
  supplied in chat; retained workspace evidence `artifacts/M2M-031/screenshots/06-working-preview-verified.png`).
  Six hidden observation cameras also produced real PNG samples in
  `artifacts/M2M-030/pose-inspection/`; their MCP image transport remains provisional.
  New Animation includes gizmo bone editing, timeline playback, saved-clip persistence,
  Explore inspection, generated thumbnail/flipbook previews and protected deletion.
- Contract `mcp-pose-observation/v1` published at `9683e96`; Codex handoff files updated.
- Quality: ESLint 0/0, TypeScript 0, Vitest **70 files / 467 tests**, Vite OK, cycles 2/2,
  ast-grep 0, Ruff/pytest/ty clean, cargo check/build pass, bacon/bugstalker used as
  documented. See consolidated report `reports/M2M-031/final-release.md`.
- Promoted to clean release branches: frontend `m2m-031-release` @ `0920f70`, desktop
  `m2m-031-release-desktop` @ `e439eb1` (source gitlink `47962b6`). Built and installed
  both `mesh2motion-git` and `mesh2motion-mcp-git` `0.1.10.r82.m2m031-1` via pkexec/pacman
  (exit 0; snapper hook warnings only). Installed desktop binary SHA-256 `7c2f9afad3…`,
  sidecar `db69364b95…`. Post-install smoke: host bound, `frontend_ready=true`, 17 MCP
  tools including 11 authoring tools, live 66-bone Human catalog. Full record
  `reports/M2M-031/final-release.md` §7.

## M2M-030A + M2M-031 — historical development log (2026-11-23)

### M2M-031 code cleanup (complete)

- Frontend branch `m2m-031-cleanup` @ `f254e01e5355ece09adec1f75eacee6a1088d271`;
  fixed JSDOM anchor navigation warning, TypeScript generated-output race, deprecated
  Vite `__dirname` usage, and the shared `node_modules` symlink status. BugStalker ran
  the MCP unit binary to completion. Detailed warning audit is in
  `reports/M2M-031/cleanup.md`.
- Ruff, pytest, ty, ast-grep, Bacon, ESLint, TypeScript, Vitest, Vite build, Rust fmt,
  Clippy, and Rust tests run. See `reports/M2M-031/cleanup.md` for results and the
  remaining external warnings. Release refs and installed r82 packages unchanged.

- Isolated frontend branch `m2m-030-031-observation-animation` (worktree `worktrees/m2m030-031-observation/frontend`), based on natively verified safe pair `75aae01` (`0505109` + view-helper fix). Final SHA `32e4db9`; runtime fix for the user-reported white-on-white rig select is `badb511`. Canonical branches, installed app and Rust unchanged.
- Published `coordination/contracts/mcp-pose-observation-v1.md` (commit `9683e96`) and exact SHA to Codex in `coordination/handoffs/M2M-030-031-observation-channel.md`. Transport event names remain PROVISIONAL pending Codex confirmation.
- Implemented `src/authoringObservation/`: rig-relative orthographic front/back/left/right/top/bottom cameras with explicit top/bottom up vectors, one-pose evaluation at draft revision/timestamp, offscreen WebGL2 render-target capture with full GPU state restore, joint projection/occlusion metadata, bounded retention/cancel/dispose and an optional Tauri adapter. No visible camera UI; orbit camera and XYZ helper untouched.
- Implemented New Animation tab (`src/newAnimation/`) with From Scratch and From Existing modes, keyframe timeline/editor, interpolation, undo/redo, preview, save/import/export, commit/cancel. `SharedDraftController` routes GUI edits through the existing authoring bridge store — one authoritative AnimationDocument for GUI and MCP; stale revisions rejected, MCP changes propagated to GUI.
- Native staged pair pinned on `m2m-030-031-integration` @ `83ecbd2` (frontend `32e4db9`); Tauri build exit 0, binary SHA-256 `cc7c3556…`. Bounded staged launch: MCP host bound, `M2M_AUTHORING frontend_ready=true`, 0 rejected snapshots. User screenshot showed a white-on-white rig `<select>`; fixed with themed WebKitGTK-compatible CSS + regression test. Visual native acceptance remains BLOCKED without GUI/screenshot control and Codex's MCP observation tools.
- Quality: ESLint 0/0, TypeScript 0, Vitest **67 files / 454 tests**, Vite build OK (new page emitted), runtime cycles 2/2, ast-grep 0 `any`. Reports: `reports/M2M-030/frontend-observation-cameras.md`, `reports/M2M-031/{new-animation-workspace,native-acceptance}.md`.
- Live authoring on the staged New UI: `mcp_jumping jack` created from an empty draft (4 s, 8 quaternion channels / 72 slerp keys, rest-composed), previewed and committed into the real library at revision 10 via `tools/mcp/jumping_jack_mcp_demo.py`; transcript `audits/M2M-031/jumping-jack-transcript.ndjson`, document `artifacts/M2M-031/mcp_jumping_jack.animation-document.json`. Staged launcher now prefers the matching built sidecar (installed `/usr/bin` sidecar is stale, read-only tools only). Desktop head `e06351e`; staged app left running. Follow-up fixes: themed select, live bone gizmo editing, searchable bone list + smaller gizmo, canvas-relative picking (mirrored left/right selection bug). Godot-style clip editor (time ruler, per-channel keyframe lanes, playhead, play/stop with real pose playback) and readable bone list added in `4e30909`. Saved clips now persist locally, rehydrate into Explore after rig load, and can be deleted only when user/MCP authored (bundled animations protected). Final frontend `6227d98`, desktop `9650667`, staged binary `c0d13855…`; internal six-camera capture samples in `artifacts/M2M-030/pose-inspection/`.

## M2M-028B — MCP permission transition verified; live animation acceptance pending

- Isolated desktop branch `m2m-028b-keyframe-authoring` @ `15b0f3a`, based on M2M-027 desktop `0f39a06`; desktop gitlink exactly pins frontend `a96dff2`. Canonical branches and installed M2M-027 remain untouched.
- OpenCode contract source includes `393bd67`, `0fe12356`, `39414908`, and Rust/frontend reconciliation `380cb70`. Rust mirrors the final 13 operations, 256 KiB UTF-8 frame, ID-byte limits, JSON-safe sequence, unknown-field rejection and 19 error variants.
- Native Tauri event request/ack readiness is verified. The user-directed permission amendment removes the native enable toggle; a valid authoring MCP operation enables in-memory permission for the app session. Live Human catalog (66 bones), all five read-only calls, authoring-enabled `animation_session_status`, empty draft create/cancel all passed through the real JSON-RPC client. Transcript: `audits/M2M-028/mcp-permission-transition.ndjson`.
- Frontend quality: ESLint/TypeScript/Vite pass, Vitest 61 files / 411 tests. Rust: fmt, MCP 51 library + 3 sidecar tests, workspace 3 Tauri tests, and denied-warning Clippy pass. Maintained MCP Python Ruff/ty pass and pytest 4 passed (one system Dash deprecation warning). ast-grep TypeScript and Rust scans found no matches.
- Remaining gate: run Agent_Wave_01 through the native viewport, request digest-bound confirmation, and verify commit, export/reimport, cancel and undo on the actual application. The user previously clicked the removed control; no additional permission click is needed. No package built or installed.

## M2M-025C — Resume status (2026-11-23)

- Authoritative frontend source found in the independent checkout at commit `55d7e349181c452ae0dccfe1d785782d1c5127db`; canonical desktop submodule remains `c417eb2`.
- Rust typed mirror, validation and bounded session draft store implemented on `m2m-025c-mcp-authoring`. Source commit is recorded in the M2M-025C report.
- Locked MCP tests passed (22 library + 3 sidecar); Clippy with denied warnings passed; rustfmt check passed. Workspace Tauri tests are blocked by the empty isolated frontend submodule missing `static/animpreviews`.
- MCP write tools remain unexposed. OpenCode live rig adapter is uncommitted and not wired to publisher; preview IPC, native confirmation, library commit and application undo remain missing. Duration minimum (0 vs TypeScript 0.001) and validation-state ownership remain open questions.
- No package installed; r73 remains stale. Agent_Wave_01 actual native end-to-end was not run.

## M2M-025C — Authoring backend blocked on shared contract

- Canonical baseline verified: desktop `m2m-025-dev` at `68616f344134706889ab330eb7b475777d8baedc`, frontend gitlink/HEAD `c417eb2555921b1a90cdcc669efb2f55af9b0690`.
- Isolated Rust branch/worktree created: `m2m-025c-mcp-authoring`, `worktrees/m2m025c-mcp-authoring`, based at `68616f3`; no source changes or feature commits. Canonical branch remains clean.
- AnimationDocument v1 schema and OpenCode adapter are not present in Project 3 reports, source branches, or available frontend worktree. Stable bone IDs and transform conventions are also absent. No speculative write contract or authoring operation was implemented.
- Confirmed `scene_inspect` bone names disappear because the frontend snapshot publisher sends count/root only; Rust `bone_names` defaults to empty. Canonical bone catalog needs the coordinated frontend publisher.
- Installed sidecar is `mesh2motion-mcp-git 0.1.10.r73.gc417eb2-1`, SHA `5756529c…`, identical to the r72 package payload. The r78 candidate payload is `24883bc6…` and contains `M2M_MCP_AUTOSTART_APP`; Project 3 source includes exact-value opt-in and Tauri exit cleanup. No package was installed or changed.
- Agent_Wave_01 end-to-end was not run: no fake draft/snapshot was used. See `reports/M2M-025/{mcp-authoring,agent-wave-end-to-end}.md`.
- Next gate: OpenCode publishes the shared AnimationDocument v1 schema/adapter and confirms supported rig set and preview/commit integration. Then implement and test against that contract on the isolated branch.

Project 2 is an independent local workspace at /home/burstcachyhtpc/Schreibtisch/Mesh2motion wayland project 2. Its root is not a Git repository.

## M2M-024A — Integrated native UI candidate; acceptance still open

- Desktop integration branch `m2m-024a-integration` at `d53041b92af8ba22faa275ced6a540c17aea53f2`; frontend branch `m2m-024a-native-integration` at `e06d6bb41e0de3bdc1bbb6f86a2ed50275c2dd42`; desktop submodule gitlink matches frontend HEAD. Baseline tags and feature branches remain preserved; both worktrees are clean.
- Integrated OpenCode fixes `25b4f3f`, `3fac9db`, final poster correction `e06d6bb` (including removal of extra preview buttons), protected preview lifecycle `7cd06bfa`, and Codex Rust/MCP commits `bd2e4e5` and `1798a17`. See `reports/M2M-024/integration.md`.
- Final gates: ESLint 0/0, TypeScript pass, Vitest 47 files/294 tests, Vite build pass with existing config/chunk warnings; Cargo fmt/tests/Clippy pass; MCP 21 tests pass; Ruff/ty pass; pytest 3 pass with a plugin deprecation warning. Bounded Bacon Cargo check succeeded before its watcher time limit.
- Native package-extracted run confirmed visible Human posters (100×120), zero per-card preview buttons, and Props 7 px from the actual viewport top/right at the observed wide size. Fox poster URLs decoded in an earlier run on the same frontend source.
- Native acceptance remains incomplete: resize/tiling/fullscreen/DPR, Settings layout, final-package Horse switch, hover loop/pause/virtualization ceilings, and MCP end-to-end remain unverified. Historical WebKit SIGABRT cause remains unresolved. Do not mark release ready.
- r78 packages prepared but not installed. Installed versions remain `mesh2motion-git` and `mesh2motion-mcp-git` `0.1.10.r72.gade2ee2-1`. r72 and r63 rollback packages are preserved. Package has an embedded absolute build path; reproducibility caveat recorded.
- See `reports/M2M-024/{integration,native-acceptance,package-manifest}.md` for hashes, evidence and manual steps.

The earlier M2M-023 sections below are historical snapshots and are superseded by this M2M-024 status for current branches, installed package, and native acceptance.

## M2M-023B — MCP read path verified; authoring contract awaits Agent 3

- Desktop candidate branch m2m-022-integrated-candidate, commit ade2ee2c1c1c440ceb2fc6da40b954b3efac5c8f.
- Frontend candidate branch m2m-022-architecture-integration, commit b96f528254c4dc6190d94120dcc5e2c45d883ad1; desktop gitlink matches.
- Clean RC refs retained: desktop ce06c9ff61bd8b6e0b91b6a24e5c866eafe95d12, frontend 3909f18a6c4ff4a4adb6d8ee2a0e7fe6aded16d7.
- Protected preview 7cd06bfa and Codex parser 839495b are ancestors. Architecture ce073a5 merged. Competing parser 5be8671 archived, not merged.
- ESLint 0/0; TypeScript clean; Vitest 46 files / 280; Vite build passes. Rust fmt/tests/Clippy pass; MCP server 18 tests pass. Ruff/pytest/ty pass.
- Candidate package pair `0.1.10.r72.gade2ee2-1` was installed at the user's request; exact r63-3 rollback packages are preserved in artifacts/rollback.
- Installed native package was not changed. Native acceptance is PARTIAL based on the user's screenshots/spot check; full native acceptance remains incomplete and the candidate is not release-approved.
- User native spot check: Human-to-Horse switch succeeded without a crash. Open findings NATIVE-001 (P1 preview thumbnails), NATIVE-002 (P1 Props viewport anchoring), NATIVE-003 (P2 Settings typography/layout), and passed initial NATIVE-004 rig switch are specified in `coordination/reports/M2M-023/native-acceptance-findings.md`, with screenshots referenced there. Full native acceptance and release approval remain open.
- Installed MCP sidecar completed real `initialize`, `tools/list`, and all five read-tool calls. After Horse selection, live `animations_list` and `scene_inspect` reported Horse, 57 tracks, and 56 bones. Current publication audit: 379 accepted, 0 rejected; live socket modes 0700/0600 and same-UID peer accepted.
- Persistent branch `m2m-023b-mcp-backend` is at `1798a17628f34c8fcfa996bc99363ae98cc619c8`, after `bd2e4e5512bce32fbc451f8f19fa35b93d770a23` added the Tauri exit cleanup hook and `1798a176` added opt-in MCP-driven app startup. Staged normal-close verification confirmed socket removal; opt-in staged launch connected all five read tools and remained running after the client exited. Installed packages remain unchanged.
- Staged MCP app is currently visible/running as PID 1265721 (WebKit child 1266010); socket `/run/user/1000/mesh2motion/mcp.sock` is 0600 in a 0700 directory. Staged artifacts are preserved at `artifacts/stage-023b/`.
- No AnimationDocument spec or stable bone-ID catalog is present. Authoring operations are not implemented; proposed contract and Agent 3 dependencies are documented in `coordination/reports/M2M-023/codex-mcp.md`.
- Reports: coordination/reports/M2M-022/{migration,integration,validation}.md.

Next: agree the AnimationDocument and stable bone-ID contract with Agent 3 before any MCP write-tool implementation. Resolve M2M-023 NATIVE-001 through NATIVE-003 and complete the remaining native checklist. Snapper hooks emitted snapshot errors during installation; pacman itself completed successfully.

## M2M-023A — UI fixes implemented (native acceptance still open)

- Isolated frontend branch `m2m-023a-native-ui` from `b96f528`: commits `25b4f3f` (repair three literally-newline-corrupted sources so gates can run) and `3fac9db` (UI fixes). Canonical desktop branch and installed package untouched.
- NATIVE-001: 684 offline WebP posters generated by `tools/generate-preview-posters.mjs`; Explore/Remix idle tiles show real frames or an intentional "No preview" placeholder. Scheduler, limits and media transport unchanged.
- NATIVE-002: `#props-dock` anchored via shared `--viewport-*` CSS variables; JS `ResizeObserver`/pixel `right` and `!important` override removed; pointer events/ARIA preserved.
- NATIVE-003: MCP grid uses `minmax(0,1fr)`, `min-width:0`, wrapping, smaller type and bounded dropdown with vertical scroll.
- Gates: ESLint `--max-warnings 0` 0/0, tsc 0, Vitest 47 files/288 tests (8 new), Vite build OK, ast-grep structural checks clean, runtime cycle tests 2/2.
- Staged Tauri 2 candidate built (`cargo build --locked --release --features mcp-host` exit 0) at `worktrees/m2m023-native-ui/stage`, binary SHA-256 `e22bbe7011550f857afc4b1c3458e2e779ff56570489ede8e7770f1467cb14b5`, with 684 MP4 + 684 WebP resources and `run-staged.fish`. Native GUI observation remains BLOCKED; fish UAT steps are in `reports/M2M-023/opencode-native-ui.md`. Not installed.

## M2M-023C — urgent correction (Codex M2M-024A base)

- Codex's `m2m-024a-native-integration` (`303cfbd`) added per-tile ▶ preview buttons and loaded posters with a `.webp` asset key that the Rust allowlist rejects, so native cards were empty.
- OpenCode fix branch `m2m-023c-always-on-posters` at `e06d6bb` (based on `303cfbd`, Codex branch untouched): removed buttons/handlers/CSS; posters now resolved via the allowlisted `.mp4` key with sibling `.webp` derived locally for `convertFileSrc` (no Rust/MCP change).
- Gates: ESLint 0/0, tsc 0, Vitest 47 files/294 tests, Vite build OK. Staged native candidate at `worktrees/m2m023c-poster-fix/stage`, binary SHA-256 `5bfec101ce70fae3110e8fb0760d55ff671c76e7b3ea5d6b2ae230f0ced13527`, launcher `run-staged.fish`. Native GUI verification remains BLOCKED.

## M2M-023D — settings/right-rail fix

- Root cause: SettingsDropdownManager constructed before nav markup existed, cached null refs, never bound click. Now public idempotent initialize() called after UI ref refresh; regression test covers late markup and open/close.
- body.tool-panel-page declares --viewport-right/left/top at all widths so right-side controls cannot be occluded by the fixed tool panel.
- Branch m2m-023d-settings-and-layout at c417eb2, based on Codex e06d6bb. Gates: ESLint 0/0, tsc 0, Vitest 48 files/299 tests, Vite build OK. Staged binary SHA-256 6efca568005bb853705d60b7db64d34e6e4bf72d5cd97d62d6697a51d75e28b0. Native GUI still BLOCKED.

## M2M-024 — merged, packaged and installed

- Frontend release branch `m2m-024-release` at `c417eb2`; desktop release branch `m2m-024-release-desktop` at `68616f3` with frontend gitlink `c417eb2`. Codex `e06d6bb` and all M2M-023C/D fixes are ancestors; no history rewritten or upstream push.
- Gates: ESLint 0/0, tsc 0, Vitest 48 files/299 tests, Vite build OK; ast-grep structural checks clean; runtime import-cycle tests pass.
- Built `mesh2motion-git` and `mesh2motion-mcp-git` `0.1.10.r73.gc417eb2-1` and installed via pkexec/pacman. Installed desktop binary SHA-256 `75b9ee801826097988d6be89a98ff5f1217f07476c8d8306606a34d49e51a24b`; resources 684 MP4 + 684 WebP. Installed app launched: PID 1434395, WebKit child 1434683.
- User confirmed native thumbnails, settings menu and rigging layout now match the upstream web app for the reported defects. Fullscreen/tiling/telemetry and the historical WebKit SIGABRT remain unverified.
- Rollback packages r63-3 preserved in `artifacts/rollback/`; M2M-022 packages in `artifacts/pkgbuild/`. Full record: `coordination/reports/M2M-024/final-release.md`.

## M2M-025 — Project 3 canonical baseline (created 2026-11-23)

- Independent clones without hardlinks/alternates: desktop `sources/mesh2motion-desktop` @ `68616f3` (branch `m2m-025-dev`), frontend submodule @ `c417eb2` (branch `m2m-024-release`). Gitlink matches; both clean. Tags `m2m-025-baseline-desktop` / `m2m-025-baseline-frontend`.
- Dependencies restored fresh from lockfiles (`npm ci --offline`, `cargo fetch --locked --offline`); no node_modules/target/dist/caches copied.
- Quality baseline all green: ESLint 0/0, tsc 0, Vitest 48/299, Vite build 0, cycles 2/2, ast-grep 0 any, cargo fmt/test/clippy 0, MCP 18+3 tests 0, Ruff/pytest/ty 0, npm audit 0 vulnerabilities. Bacon bounded check finished cargo check at 17.32s before its 25s timeout.
- Known provenance defects: installed r73 MCP sidecar is stale (hash equals r72/r63, lacks autostart; correct r78 sidecar hash `24883bc6…`); r78 vs r73 came from parallel package lines. Installed binary embeds one Project 2 build path. No new install performed during migration.
- Native acceptance partial: user-confirmed posters/no-play-buttons/settings/rigging width; hover/fullscreen/tiling/DPR/telemetry blocked by lack of native GUI control. Installed app relaunched PID 1466532 (WebKit child 1466821).
- NATIVE-005 code fix committed on isolated branch `m2m-025d-remix-first-paint` @ `313d7ac` (themed background + min-height on virtualized content wrapper). WebKitGTK computed-style probe disproved CSS-variable timing; native pixel confirmation still pending. See `coordination/reports/M2M-025/remix-first-paint.md`.
- Reports: `coordination/reports/M2M-025/{project3-migration,source-provenance,native-ui-acceptance,quality-baseline}.md`.

## M2M-025B — AnimationDocument domain (frontend branch, not integrated)

- Isolated branch `m2m-025b-animation-document` @ `55d7e349` (base `c417eb2`); clean status. Canonical `m2m-025-dev` and installed app untouched.
- Delivered schema v1, rig identity/fingerprint, conventions, validation, pure Three.js conversion, `DraftPreviewSession` + engine adapter, atomic commit/undo store, and original `Agent_Wave_01` two-second Human wave fixture (5 quaternion channels / 35 keys).
- Contract published for Codex in `coordination/reports/M2M-025/animation-document-contract.md`.
- Quality: ESLint 0/0, tsc 0, Vitest 54 files / 339 tests, Vite OK, runtime cycles 2/2, ast-grep 0 `any`. Native visual acceptance BLOCKED (no GUI automation).
- Slerp weight bug found by the equivalence test and corrected (cos→sin weights). Known limitation: `step` channels use duplicate times that `ExportAnimationCleanupService` drops before export; fixture uses slerp only.
- Commit/undo store is an interface + reference implementation only; it is NOT wired into `StepAnimationsListing`. Full MCP authoring workflow not claimed.

## M2M-025B — authoritative handoff published

- Branch `m2m-025b-animation-document` now has two commits: foundation `55d7e34` and handoff `ae48734813fa92788f391a6d1ab4001cae4fb0d7`. Canonical `m2m-025-dev`, installed app, Rust/MCP untouched.
- Frozen `animation-document/v1` contract at `coordination/contracts/animation-document-v1.md`; Codex notified via `coordination/handoffs/M2M-025B-to-Codex.md` plus pointers in its two blocked reports.
- Added live rig adapter (real Skeleton.bones/parents/rest transforms/fingerprint, rename/reorder/reparent guards), native-confirmation/v1, and application-owned commit/cancel/undo controller. Draft authoring stays separate from Remix.
- Quality: ESLint 0/0, tsc 0, Vitest 56 files / 358 tests, Vite OK, cycles 2/2. All JSDOM; native GUI acceptance still BLOCKED.
- Provisional pending Codex: confirmation transport, application owner wiring, binding prefix, step export encoding, catalog tool shape, MCP write op names. No frozen limit or convention changed.

## M2M-025 integration + NATIVE-005 residue fix

- Integrated branches: frontend `m2m-025-integration` @ `c25e08d103991591d36af42354d5eceb051185a6` (merges `c5f3cca`, `90ff637`; contains M2M-025B and M2M-025D), desktop `m2m-025-integration` @ `78e0040ba1504fe8f311c5a446daac11a385b81e`. Canonical `m2m-025-dev` / `m2m-024-release` and installed app untouched.
- Native inspection found residual black scroller padding box + scrollbar after the first M2M-025D fix (`Bildschirmfoto_20261123_032423.png`). Added two-frame harmless repaint invalidation and scoped themed scrollbar rules. Quality: ESLint 0/0, tsc 0, Vitest 56/364, Vite OK, cycles 2/2.
- New NATIVE-006 (black triangle on “Use Your Rigged Model” page switch, screenshots `_032617.png` / `_032531.png`) is Codex-owned; not modified here.

## NATIVE-005 RESOLVED (2026-11-23)

- User visually confirmed the staged native Remix catalog has no black background residue on first display; evidence `Bildschirmfoto_20261123_034321.png`. Final integrated frontend `5f8134081883069abc35f538da212738bb40caa5`, desktop pin `6b247d2e0d85b2bc57aa3d17d5a1c0806113362c`; both worktrees clean. No package installed. NATIVE-006 remains Codex-owned.
# M2M-027 release status

M2M-027 integration, quality gates, native acceptance, paired package installation, and post-install MCP smoke completed. Desktop source: `0f39a0688f974d3d23fa3c3f576d5403eb5d9702`; its exact frontend gitlink: `921b1dbbdb375dfe392d4d35f99059e4f43e48a1`. Installed package pair: `0.1.10.r79.m2m027-1`. Animation authoring MCP tools remain disabled; this release is the M2M-028 foundation.

## M2M-030B — initial Rust compatibility review (2026-11-23)

- Created persistent desktop feature worktree `sources/mesh2motion-desktop/worktrees/m2m030b-mcp-vision/desktop`, branch `m2m-030b-mcp-vision`, at release base `e439eb1a638c136df594888bd25071776083c24d`; pinned frontend gitlink is `0920f7025773e68939fb329fc9d8be1f3d8dbf34`.
- Compatibility review appended to `coordination/handoffs/M2M-030-031-observation-channel.md`. Request/response/cancel names and bounded capture limits accepted. Image retrieval unresolved: contract refers to `pose.get_image_bytes`, omitted from the closed operation set, and no Tauri byte route exists. No competing wire protocol introduced; no Rust code changed.
- Ruff, pytest (6), ty, ast-grep unsafe-block scan, MCP Bacon check, cargo fmt, 33+3 MCP tests, and MCP clippy passed. Whole Tauri workspace build/tests and frontend tests blocked by empty frontend submodule / absent node_modules; no dependencies installed. BugStalker availability recorded, no controlled debug target.
- M2M-030B remains open. No JSON-RPC image transcript, native acceptance, or Agent_Wave_01 visual correction. See `coordination/reports/M2M-030/` and `coordination/audits/M2M-030/mcp-vision-transcript.ndjson`.

## M2M-030B — Rust transport implementation (2026-11-23)

- Continued only in persistent feature worktree `sources/mesh2motion-desktop/worktrees/m2m030b-mcp-vision/desktop`, branch `m2m-030b-mcp-vision`, based on `e439eb1a638c136df594888bd25071776083c24d`; pinned frontend remains `0920f7025773e68939fb329fc9d8be1f3d8dbf34`.
- Implemented Rust observation envelopes, authenticated session-bound pose requests, image chunk event/command binding, correlation and byte/MIME/image validation, bounded capture references, cancellation/disconnect/shutdown cleanup, two MCP tools, standard MCP image output, and a Python stdio client. No frontend-owned files, canonical refs or installed packages were changed.
- Locked MCP tests: 61 library + 3 binary PASS. Tauri app-host tests: 4 PASS. MCP and Tauri Clippy with `-D warnings` PASS; Rust fmt PASS. Ruff PASS; pytest 3 PASS (external Dash deprecation warning); ty PASS; ast-grep unsafe block scan PASS. Real stdio JSON-RPC `initialize`/`tools/list` lists 19 existing+new tools; transcript saved. No native application was connected and no camera image was returned.
- Bacon `timeout 20s bacon check --headless` did not exit (BLOCKED). BugStalker 0.4.9 availability passes; bounded TUI smoke shows `Process: not running`, so no fault trace exists. Frontend tests blocked because the exact checkout has no `node_modules`; packages were not installed. Temporary build-only `dist/index.html` used for Tauri compile/test was removed after checks.
- Exact frontend still lacks the image-chunk listener/command. The handoff now contains exact payloads and implementation checklist. Native visual workflow and Agent_Wave_01 refinement remain blocked. Rust feature commit: `15017056d9f9467b3b81dfb0d19640443628a85d`; feature worktree clean. Reports: `reports/M2M-030/`.

## M2M-030B — native image transport and pose-parity finding (2026-11-24)

- Separate Codex feature commits: Rust `622ebe804ca3b25cc20e764a54ffa3a542e1ecf8` on `m2m-030b-mcp-vision`; frontend transport `63b8b95ce9f9c3e125cc8e18ce75de904325d949` on `m2m-030b-codex-image-transport`. OpenCode owns its separate frontend skin-matrix fix branch. No canonical branch, frontend gitlink, installed package, or installed app changed.
- Real JSON-RPC client connected to the rebuilt native Tauri app through the private Unix socket. MCP returned six actual PNG ImageContent blocks (512×512) with live Human rig fingerprint, draft revision/time, camera metadata and joints; cancel succeeded. Transcript: `audits/M2M-030/mcp-vision-native-test.ndjson`.
- Native image/geometry parity remains **BLOCKED**. After OpenCode's `skeleton.update()` change, a fresh app build and MCP draft captured with preview inactive still rendered a crouched mesh while the metadata placed `hand_l` above the head. Reproduction transcript: `audits/M2M-030/vision-agent-wave-post-skin-matrix.ndjson`; image and diagnosis are in `reports/M2M-030/vision-agent-wave-demo.md` and the append-only handoff.
- OpenCode's staged native app committed `Agent_Wave_01_vision` at revision 14 (`custom-import-2`, six tracks); export/reimport preserved six tracks / 54 keys. Undo to revision 13 returned `BAD_REQUEST`, so the staged library currently contains the clip. The wave is not visually accepted because image pixels still disagree with joint metadata; a second observation returned `RIG_UNAVAILABLE` after four seconds. M2M-030B remains open for parity, lifecycle, rollback and race/privacy fixes. Do not merge, install, or update the canonical gitlink until acceptance is complete.

## Project 4 migration prep (2026-11-24)

- Fresh destination `/home/burstcachyhtpc/Schreibtisch/Mesh2motion wayland project 4` is cloned
  from desktop integration baseline `2a28c615bb2186c353b24b5861911fc5130c111e` on
  `m2m-030b-integration`; its frontend gitlink is `117fdaea85e623434e53bb64cfae370e65c89e00`.
  Both complete Git histories were retained (desktop: 169 commits; frontend: 1,070 commits).
- Frontend production build, locked desktop workspace tests (2), locked MCP tests (41 library +
  3 binary), and locked native desktop build passed. Vite emitted its existing >500 kB chunk
  advisory. No dependencies were installed and no package was installed.
- OpenCode's nested `desktop/` and `frontend/` checkouts remain untouched and each is individually
  clean. They are separate repositories, excluded locally via `.git/info/exclude` so the desktop
  baseline can remain clean without staging or altering those checkouts. The shared project 3
  integration worktree separately has in-flight OpenCode changes in `model.rs`, `server.rs`, and
  `coordination/reports/M2M-030/new-animation-source-list-defects.md`; none were copied.
- Do not install this pre-fix build. Next: integrate the OpenCode commit, rerun the final build and
  relevant checks from project 4, then verify clean status before the user-requested install.
- Newly reported open blocker: Dragon → Horse rig selection fails to load the Horse model or expose
  its skeleton. Issue record: `KNOWN-ISSUES.md` and `reports/M2M-032/dragon-horse-rig-loading.md`.
