# Project 4 integration, verification, and installation

Date: 2026-11-24 (Europe/Berlin)

## Integrated source

- Project: `/home/burstcachyhtpc/Schreibtisch/Mesh2motion wayland project 4`
- Desktop branch: `m2m-030b-integration`
- Integrated desktop merge: `854be4018aa91894e82aa8fa7f37c14cd64dba90`
- OpenCode desktop fix: `f5dafaa28e377b254e55b253306ecb391b8ffe05`
- Frontend gitlink: `490db8939d3cb99b956dbce267d235976cabf3c5`
- Prior integration implementation/evidence commit recorded in task history: `8b763f5a5f3f0eb201aa4e9b50ac603cfcbfaf6d` (code/gitlink commit `94518dc5cdd093f66693f4364af3cffd026cfb9e`).
- No push, GitHub upload, canonical branch promotion, or canonical frontend gitlink update was performed.

The New Animation phase fix uses provider-backed `engine.active_skinned_meshes()` in the readiness
calculation, because the page can load meshes directly into the engine while the old load-step
vertex count remains zero. The additive `phaseInputs` snapshot/status diagnostics expose the
readiness inputs. The rig loader falls back to `RigConfigEntry.model_file` for shipped rigs with no
manifest variations. OpenCode reports native verification that New Animation reaches `ready`,
`scene_inspect` and `animations_list` return successfully, and all nine advertised rigs load and
expose skeletons. The detailed OpenCode record is
`coordination/reports/M2M-030/new-animation-source-list-defects.md`.

## Verification

Project 4 paired-source checks recorded for this integration:

- Frontend: 71 test files / 484 tests passed; ESLint, TypeScript, and Vite production build passed.
- Python MCP client/harness: Ruff passed; pytest 6 passed; ty passed.
- Rust: formatting passed; locked Tauri workspace tests (4) passed; locked MCP tests (63 library +
  3 binary) passed; denied-warning Clippy passed for both the Tauri workspace and MCP workspace.
- ast-grep Rust unsafe-block scan found no `unsafe {}` blocks; AST inspection also covered Drop,
  cancellation, deadline, and disconnect cleanup paths.
- Bacon ran headless with a 20-second bound: Cargo check completed successfully, but its watch
  process did not exit before timeout (exit 124). This is a successful check with a non-terminating
  watcher, not a clean natural exit.
- BugStalker 0.4.9 was present, but its debugger smoke was blocked by `ENOTTY`; no runtime fault
  debugging or trace is claimed.
- Vite emitted the existing >500 kB chunk advisory. The external Dash plugin deprecation warning
  appeared in pytest; no test failed.

OpenCode additionally reports its required tool set (Ruff, pytest 6, ty, ast-grep, Bacon,
BugStalker 0.4.9) and frontend/Rust checks in its implementation report. Its native checks are the
basis for the nine-rig and New Animation readiness claims above; this Codex run did not repeat
interactive rig switching after package installation.

## Installed packages

Installed together with pacman and verified via `pacman -Q`:

- `mesh2motion-git 0.1.10.r83.m2m032-2`
- `mesh2motion-mcp-git 0.1.10.r83.m2m032-2`

Package archive SHA-256 values before relocating the archives from the source tree:

- Desktop: `1e584472f17760064b55f029d5e8e40aee60b72753037a635a4be66ba4d1e9f4`
- MCP sidecar: `0d39cd2a5bddffc76e19b0292a1f54b534d6457118410187e5f3cc727c2232da`

Installed binary SHA-256 values:

- `/usr/lib/mesh2motion/mesh2motion`: `993a8bf42db0da6dfbf9586c4dc6e24bd28257401d555dd19ad7989ccbf47df0`
- `/usr/bin/mesh2motion-mcp`: `31c916fcf5ae7c2d868e26050c281f700fc552e192c4de51616ec407cb59c310`

A real stdio JSON-RPC smoke invoked `initialize`, `notifications/initialized`, and `tools/list` on
the installed sidecar. It advertised 20 tools, including `application_get_status`,
`animation_observe_pose`, `animation_capture_view`, keyframe mutation, and commit. Transcript:
`coordination/audits/M2M-032/installed-sidecar-tools-list.ndjson`. This smoke did not connect to a
running native app or capture images.

Pacman completed the install. The system's Snapper pre/post hooks logged snapshot errors
(`Creating snapshot failed` and `Invalid snapshot '--type'`); these did not fail the package
transaction. The app window was not launched after installation, to avoid taking the user's focus
or pointer.

## Artifact cleanup and remaining checks

The generated archives are retained outside the repository in the sibling installer output folder
`/home/burstcachyhtpc/Schreibtisch/Mesh2motion-installer-r83-m2m032/`. Build caches, generated Vite
output, package VCS caches, temporary dependency symlinks, and Python bytecode are removed from the
source tree. The Git tree is committed clean after this report is recorded.

Remaining human acceptance: open the installed app and interactively switch Dragon ↔ Horse more
than once, confirming the model and skeleton change together. OpenCode reports all nine rigs loaded
in its rebuilt native app, but Codex did not repeat that UI check post-install. The app is installed;
no claim is made that the remaining interactive check was performed.
