# M2M-025A — Project 3 quality baseline

Source revisions: desktop `68616f3` (branch `m2m-025-dev`, tag `m2m-025-baseline-desktop`), frontend
`c417eb2` (branch `m2m-024-release`, tag `m2m-025-baseline-frontend`). All results below were executed
in the Project 3 clones after `npm ci --offline` and `cargo fetch --locked --offline`.

| Check | Command | Exit / result |
| --- | --- | --- |
| ESLint | `./node_modules/.bin/eslint . --max-warnings 0` | **0**, 0 errors / 0 warnings |
| TypeScript | `./node_modules/.bin/tsc --noEmit -p tsconfig.json` | **0**, 0 errors |
| Vitest | `./node_modules/.bin/vitest run` | **0**, 48 files / 299 tests |
| Vite build | `./node_modules/.bin/vite build` | **0**, existing >500 kB chunk warning; `__dirname` config notice |
| Runtime cycles | `vitest run src/runtime-import-cycles.test.ts` | **0**, 2 tests |
| ast-grep | `ast-grep --lang ts -p 'any' src` | 0 explicit `any` matches |
| Cargo fmt | `cargo fmt --all -- --check` | **0** |
| Cargo test | `cargo test --locked --all-features --offline` | **0**, 3 desktop tests |
| Cargo tests (MCP crate) | `cargo test --locked --offline --manifest-path src-tauri/mcp-server/Cargo.toml` | **0**, 18 library + 3 sidecar tests |
| Clippy | `cargo clippy --locked --all-features --all-targets --offline -- -D warnings` | **0** |
| Ruff | `ruff check mcp_stdio_smoke.py test_mcp_stdio_smoke.py` | **0** (after import-order fix in the Project 3 copy) |
| pytest | `python3 -m pytest -q -p no:cacheprovider test_mcp_stdio_smoke.py` | **0**, 3 passed, 1 pre-existing Dash plugin warning |
| ty | `ty check` | **0** |
| Bacon | `timeout 25s bacon --headless --job check -- --all-features` | 124 (bounded timeout); Cargo check finished at 17.32 s — watcher stopped, not a crash |
| npm audit | `npm audit --offline --omit=dev` | **0**, 0 vulnerabilities (9 moderate notices in full dev audit from earlier reports; no upgrades) |

## Source integrity
Checked all `.ts` files for the literal-backslash-n corruption found in older revisions: **0 affected files**
in Project 3.

## Notes
- Vite's bundle warning is pre-existing (`offline-CWtD4lLb.js` ≈1.69 MB); dependencies were not upgraded.
- npm's blocked `esbuild`/`workerd` postinstall scripts did not affect ESLint, TypeScript, Vitest or Vite.
- No BugStalker/GDB target: no new native crash occurred. BugStalker is not installed.
- Native acceptance is only partial (see `native-ui-acceptance.md`); automated GUI control is unavailable.
