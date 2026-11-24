# M2M-031 code cleanup and quality audit

## Scope

Audited the released frontend (`0920f70`), desktop integration source (`47962b6`),
and maintained MCP Python helpers in `tools/mcp`. Changes are isolated on frontend
branch `m2m-031-cleanup`; the release refs and installed r82 packages were not changed.

## Fixes

- Updated `src/lib/BlobDownload.test.ts` to mock anchor clicks. This prevents JSDOM's
  unsupported navigation warning while preserving the download-helper assertions.
- Excluded generated `dist` files and dependencies from TypeScript's default file
  discovery. Concurrent `tsc` and Vite builds no longer race over files that Vite is
  replacing.
- Replaced deprecated `__dirname` references with `import.meta.dirname` in
  `vite.config.js`, compatible with the repository's Node 24.11 pin.

## Results

| Tool/check | Result |
| --- | --- |
| Ruff `check tools/mcp` | Pass |
| Ruff `format --check tools/mcp` | Pass; 6 files already formatted |
| pytest `tools/mcp -q` | 6 passed; default run shows one external Dash plugin deprecation warning |
| pytest `-p no:dash tools/mcp -q` | 6 passed without the globally installed Dash plugin |
| ty `check tools/mcp` | Pass |
| ast-grep TypeScript explicit `any` scan | 0 matches |
| ast-grep Rust `unsafe` scan | 0 matches |
| ESLint `--max-warnings 0` | Pass |
| TypeScript `tsc --noEmit` | Pass, including concurrent run with Vite build |
| Vitest | 70 files / 467 tests passed; focused BlobDownload test also passes without the prior JSDOM navigation message |
| Vite production build | Pass; `__dirname` warning removed |
| Bacon Tauri `check --all-features --offline` | Pass on desktop integration worktree at `47962b6` with pinned frontend initialized |
| Bacon MCP `check` | Pass |
| Cargo formatting | Pass |
| Cargo Clippy with denied warnings | Pass |
| Cargo Tauri tests | 4 passed |
| Cargo MCP tests | 33 library + 3 sidecar tests passed |
| BugStalker | Launched the compiled MCP test binary using the TUI Run command; process reached `running` then `finished`, with no crash reported. |

## Environment notes

- The separate release worktree has an empty frontend submodule directory, so its first
  Bacon Tauri check stopped because `mesh2motion-app/static/animpreviews` was absent.
  The check passed in the matching integration worktree, which contains the exact
  pinned frontend commit. The submodule was not updated.
- Vite's `poseEvaluation` chunk is 784 kB and contains Three.js/GLTF loading; `offline`
  is 960 kB and contains the FBX import/export path, JSZip, and related dependencies.
  These are actual payload-size warnings; reducing them needs a larger lazy-loading or
  dependency-splitting change. No threshold was raised to hide the warning.
- The Dash deprecation is emitted by the globally installed `pytest11` entry point
  named `dash`, not project code. Disabling it for this helper-only run removes the
  warning while keeping all six tests.
- `node_modules/` was a symlink to the shared source checkout dependencies. Added an
  ignore rule for the symlink; its target was left intact, and the cleanup worktree is
  now clean.
- No Python or Rust source changes were needed. The desktop cleanup branch is isolated
  and has no source changes; installed packages remain unchanged.

## Commit

Frontend cleanup commits: **`b0c9cf669a8574d82442ff1d3627a2af5aad3c86`**
(`M2M-031: clean frontend validation warnings`) and **`f254e01e5355ece09adec1f75eacee6a1088d271`**
(`M2M-031: ignore shared node modules symlink`). The cleanup branch is clean.
