# M2M-026 — Regression results and handoff

## 1. Automated results at final commits

Frontend `83f0439c2712ba9d65f4643d3666320e5fc92c20`, desktop `a981b88bd4031d62d28bc409f55b3db1d7528137`.

| Check | Exit | Result |
| --- | --- | --- |
| ESLint `. --max-warnings 0` | 0 | 0 errors / 0 warnings |
| TypeScript `tsc --noEmit` | 0 | 0 errors |
| Vitest full | 0 | **57 files / 366 tests passed** (baseline 56 / 364) |
| Vite build | 0 | success; pre-existing chunk-size warning |
| Runtime import cycles | 0 | 2/2 |
| `npm audit --offline --omit=dev` | 0 | 0 vulnerabilities |
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo test --locked --all-features --offline` | 0 | 3 desktop tests |
| `cargo clippy --locked --all-features --all-targets --offline -- -D warnings` | 0 | clean |
| MCP crate tests | 0 | 18 library + 3 sidecar tests |
| Ruff (maintained Python harness) | 0 | all checks passed |
| pytest (maintained Python harness) | 0 | 3 passed, 1 pre-existing Dash warning |
| ty (maintained Python harness) | 0 | all checks passed |
| ast-grep | 1 (0 matches) | 0 `any`, 0 `video.load()`, 0 Rust `unsafe`; 11 rAF call sites inventoried |
| bacon (bounded 30 s) | 124 (expected) | Cargo check finished in 17.54 s |
| BugStalker `bs` 0.4.9 | DAP smoke | initialize request answered with full capabilities; no crash existed to debug |

Full command lines, versions and caveats: `tool-verification.md`.

## 2. Preservation checks (executed against final frontend tree)

| Requirement | Result |
| --- | --- |
| 684 WebP posters | PASS (`git ls-tree` count 684) |
| 684 MP4 previews | PASS (count 684) |
| Built dist asset counts | PASS (684 WebP, 684 MP4) |
| No `video.load()` | PASS (0 matches) |
| Single protected `video.play()` | PASS (1 call site) |
| No per-card play button markup | PASS (0 non-test occurrences) |
| 20 active videos / 20 concurrent fetches ceilings | PASS (`MAX_ACTIVE_PREVIEW_VIDEOS = 20`, `MAX_CONCURRENT_PREVIEW_FETCHES = 20`) |
| File-level ownership boundaries | PASS (no AnimationDocument, Rust/MCP or package file changed) |
| Runtime import cycles | PASS (2/2) |

Human/Fox/Horse switching, Props anchoring, settings, rigging panel fit and weight painting were not
re-observed natively in this session; their existing automated tests still pass. They remain
**BLOCKED** for fresh native confirmation.

## 3. Native testing status

**BLOCKED for direct observation.** A real Tauri 2 / WebKitGTK staged executable was built and launched
(main PID `1695278`, WebKitWebProcess `1695564`, no stderr output), but this session cannot drive the
KDE Wayland window or capture frames. A standalone WebKitGTK webview was used in earlier M2M-025 work
only for computed-style diagnosis and is explicitly **not** counted as native acceptance. The manual
procedure is in `native-visual-fixes.md`.

No package was installed; installed `0.1.10.r73.gc417eb2-1` remains the running system candidate.

## 4. Proposed coordination entries (ownership respected)

The shared `KNOWN-ISSUES.md`, `STATE.md` and `TASKS.md` are owned by the integration agent, so this
report submits proposed text rather than editing them:

- **NATIVE-006 — fix available, native confirmation pending:** initial full-size black `CustomSkeletonHelper`
  instance eliminated by adopting Codex's `03cef90` hunk; frontend `83f0439`, desktop `a981b88`.
  Repeated native tab-switch check still required.
- **NATIVE-005 — main surfaces already confirmed fixed at `5f81340`;** M2M-026 additionally themes the
  WebKit scrollbar corner and virtualized footer spacer; visual confirmation of those two surfaces
  pending.
- **Tooling correction:** BugStalker is installed as `bs` 0.4.9; the earlier report saying it is not
  installed is stale. Current maintained-Python Ruff result is clean.

## 5. Remaining issues

1. Native acceptance of both defects is not yet recorded; no new user screenshots were produced in this
   session.
2. `ModelVariationSwitcher.update_loading_progress()` uses an uncancelled one-shot rAF — suspected only,
   no runtime failure evidence, left unchanged.
3. `SceneEnvironmentManager.autoClear = false` with no explicit clear remains a generic first-frame risk;
   changing it is broader than the observed defects and was not done.
4. The known stale installed MCP sidecar (`5756529c…`, lacking autostart) is unchanged; packaging is
   explicitly out of scope.
5. Vite chunk-size warning and npm dev advisories are pre-existing; no dependency upgrade was made.

## 6. Proposed integration steps

1. Integration agent reviews `m2m-026-native-fixes` frontend `83f0439` and desktop `a981b88`; confirms
   no overlap with OpenCode's `m2m-025b-animation-document` or Codex's `m2m-025c-frontend-integration`.
2. If approved, merge/cherry-pick the two-file NATIVE-006 hunk and the small CSS/test hardening into the
   shared integration line; do **not** take Codex's `5f6d010` AnimationDocument changes in this task.
3. Rebuild a staged executable from the integrated commit and have the user perform the native procedure.
4. Only after a native pass should packaging/install be considered in a separately approved release task.
