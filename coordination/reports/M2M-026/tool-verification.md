# M2M-026 — Mandatory tool verification

All commands were executed in this session unless marked. Version outputs are from the installed
executables. No tool was installed and no dependency was upgraded.

## 1. Ruff

- Version: `ruff 0.17.6` (`/usr/bin/ruff`)
- Command: `cd .../project 3/tools/mcp && PYTHONDONTWRITEBYTECODE=1 ruff check --no-cache mcp_stdio_smoke.py test_mcp_stdio_smoke.py`
- Target: maintained Python MCP smoke harness
- **Exit 0** — `All checks passed!`
- Findings: none now. A tool-survey subagent observed an `I001` import-order finding earlier in the
  session; re-running showed the file already has the blank line between `import pytest` and
  `from mcp_stdio_smoke import …`. Current state is clean.
- Fixes applied: none needed.

## 2. pytest

- Version: `pytest 9.2.0` (Python 3.14.10)
- Command: `cd .../project 3/tools/mcp && PYTHONDONTWRITEBYTECODE=1 python3 -m pytest -q -p no:cacheprovider test_mcp_stdio_smoke.py`
- **Exit 0** — `3 passed, 1 warning in 0.00s`
- Warning: pre-existing `PytestRemovedIn10Warning` from the installed Dash plugin, not product code.
- Fixes applied: none.

## 3. ty

- Version: `ty 0.0.107 (85b1fd052 2026-11-18)` (`/usr/bin/ty`)
- Command: `cd .../project 3/tools/mcp && PYTHONDONTWRITEBYTECODE=1 ty check`
- **Exit 0** — `All checks passed!`
- Caveat: `ty check .` from the Project 3 root reports 10 `missing-module-member` diagnostics for
  `gi.repository` (`Gtk`, `WebKit2`, `GLib`, `Gdk`) in the PyGObject audit probes; those are diagnostic
  scripts, not maintained product code.
- Fixes applied: none.

## 4. ast-grep

- Version: `ast-grep 0.51.1` (`/usr/bin/ast-grep`; `sg` alias absent)
- Executed structural queries in the M2M-026 frontend worktree:
  - `ast-grep --lang ts -p 'any' src` → 0 matches (exit 1 = no matches)
  - `ast-grep --lang ts -p 'requestAnimationFrame($$$ARGS)' src` → 11 call sites in 6 files
  - `ast-grep --lang ts -p 'cancelAnimationFrame($$$ARGS)' src` → 5 call sites
  - `ast-grep --lang ts -p 'video.play()' src` → 1 call site (protected scheduler)
  - `ast-grep --lang ts -p 'video.load()' src` → 0 matches
  - `ast-grep run --lang rust --pattern 'unsafe { $$$BODY }' src mcp-server/src` in `src-tauri` → 0 matches (exit 1)
- Findings used: the rAF inventory led to a lifecycle review of `RendererLifecycle`,
  `VirtualizedAnimationCatalog`, `Retargeter`, `ModelVariationSwitcher` and `preview-generator`.
  Only `ModelVariationSwitcher.update_loading_progress()` schedules an uncancelled one-shot rAF; it
  updates DOM text/transform only, so it is classified **suspected, not a confirmed runtime bug**, and
  no patch was made (no ownership conflict, but no evidence of failure).
- Pattern matches were treated as leads, never as proof.

## 5. bacon

- Version: `bacon 3.26.0` (`~/.cargo/bin/bacon`)
- Command: `cd .../worktrees/m2m026-native-fixes/desktop/src-tauri && timeout 30s bacon --headless --job check -- --all-features`
- **Exit 124 (expected watcher timeout)**; the underlying Cargo check finished successfully:
  `Finished dev profile ... target(s) in 17.54s`.
- No uncontrolled watcher was left running.
- Fixes applied: none.

## 6. BugStalker

- Version: `bugstalker 0.4.9`, installed only as `bs` (`~/.cargo/bin/bs`). `bugstalker` / `bug-stalker`
  commands do not exist. The older claim "BugStalker is not installed" in `quality-baseline.md` is stale.
- `bs --help` confirms it is an interactive Rust debugger (TUI/DAP), not a static checker:
  `bs [OPTIONS] [DEBUGEE] [-- <ARGS>...]`, modes `--tui`, `--dap-local`, `--dap-remote`,
  `--dap-oneshot`, `-p/--pid`.
- Direct invocation `bs --oracle tokio <rust-test-binary> -- --list` failed with `ENOTTY` because no TTY
  was available — expected in a non-interactive shell.
- Substantive smoke test performed: sent a DAP `initialize` request over stdin to
  `bs --dap-local --dap-oneshot <target/debug/deps/mesh2motion-1314b092cff138f0> -- --list`.
  BugStalker replied with a full capabilities response (`supportsAttachRequest`, breakpoints, stepping,
  variables, etc.) and an `initialized` event. It then exited when the test session supplied no
  further DAP commands (`DAP connection closed`).
- No native crash or core dump occurred in this session, so no actual crash debugging was performed.
  BugStalker is not applicable to WebKitGTK C/C++ crashes; GDB/coredumpctl would be used for those.

## Native project checks (executed)

| Check | Target | Exit | Result |
| --- | --- | --- | --- |
| `eslint . --max-warnings 0` | M2M-026 frontend worktree | 0 | 0 errors / 0 warnings |
| `tsc --noEmit -p tsconfig.json` | same | 0 | 0 errors |
| `vitest run` | same | 0 | 57 files / 366 tests |
| `vite build` | same | 0 | pre-existing chunk-size warning only |
| `vitest run src/runtime-import-cycles.test.ts` | same | 0 | 2/2 |
| `npm audit --offline --omit=dev` | same | 0 | 0 vulnerabilities |
| `cargo fmt --all -- --check` | M2M-026 desktop worktree | 0 | clean |
| `cargo test --locked --all-features --offline` | same | 0 | 3 desktop tests + 0/0/0 |
| `cargo clippy --locked --all-features --all-targets --offline -- -D warnings` | same | 0 | clean |
| `cargo test --locked --offline --manifest-path mcp-server/Cargo.toml` | same | 0 | 18 library + 3 sidecar tests |
