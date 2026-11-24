# M2M-025A — Project 3 canonical baseline migration

Project 3 was empty before this work. No Project 1/2 file, installed package or running application was
modified during the migration.

## 1. Clones and revisions

| Item | Result |
| --- | --- |
| Desktop clone | `sources/mesh2motion-desktop`, branch `m2m-024-release-desktop` → HEAD **`68616f344134706889ab330eb7b475777d8baedc`** |
| Frontend clone | `sources/mesh2motion-desktop/mesh2motion-app`, branch `m2m-024-release` → HEAD **`c417eb2555921b1a90cdcc669efb2f55af9b0690`** |
| Desktop gitlink | `160000 commit c417eb2555921b1a90cdcc669efb2f55af9b0690 mesh2motion-app` |
| Baseline tags | desktop `m2m-025-baseline-desktop` @ `68616f3`; frontend `m2m-025-baseline-frontend` @ `c417eb2` |
| Development branch | desktop `m2m-025-dev` @ `68616f3` — **canonical branch; no concurrent direct edits** |

Both clones used `git clone --no-hardlinks` from local paths. Verified:
- no `.git/objects/info/alternates` in either clone;
- sample commit object link count 1 and different inode from Project 2 (independent object stores);
- object stores 373 MB (desktop) and 154 MB (frontend);
- `.git` for the frontend is a real directory (no stale pointer into old worktrees);
- no tracked symlinks in either repository;
- `git status --short --branch` clean for both;
- no tracked text referencing “wayland project 1/2”.

## 2. What was deliberately not copied
worktrees, `node_modules`, Cargo `target`, `dist`, package build trees, historical session logs, caches
(`__pycache__`, `.pytest_cache`, `.ruff_cache`) — all absent in Project 3.

The maintained Python harness `tools/mcp/{mcp_stdio_smoke.py,test_mcp_stdio_smoke.py}` was copied from
Project 2 (not a session log) so the baseline can run its own tools; its pre-existing Ruff import-order
finding was corrected in the Project 3 copy only.

## 3. Fresh dependency restoration
- `npm ci --offline --no-audit --no-fund` in the frontend → exit 0 (npm blocked `esbuild`/`workerd`
  postinstall scripts; ESLint/tsc/Vitest/Vite all work from the restored tree).
- `cargo fetch --locked --offline` in `src-tauri` → exit 0.
No system packages were installed and no dependency versions were changed; `package-lock.json` and
`Cargo.lock` are unchanged.

## 4. Canonical branch policy
`m2m-025-dev` is the single integration branch for Project 3. Agents must create their own branches from
it and submit changes for review; nobody may edit `m2m-025-dev` directly or concurrently. Project 3 must
not be used for a parallel rewrite, graphics-backend migration, animation authoring or upstream push.
