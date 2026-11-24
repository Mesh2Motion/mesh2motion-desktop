# Project 4 migration preparation

Date: 2026-11-24
Status: HISTORICAL — superseded by the integrated and installed state in `project4-integration-install.md`.

## Source and history

- Destination: `/home/burstcachyhtpc/Schreibtisch/Mesh2motion wayland project 4`
- Desktop branch: `m2m-030b-integration`
- Desktop source commit: `2a28c615bb2186c353b24b5861911fc5130c111e`
- Desktop history: 169 commits; all local desktop branches fetched from project 3's repository.
- Frontend gitlink: `117fdaea85e623434e53bb64cfae370e65c89e00`
- Frontend history: 1,070 commits retained in the frontend repository; checkout is detached at the exact gitlink.
- No canonical branch or installed package was changed. No GitHub push was made.

The pending edit in project 3's shared integration worktree (`src-tauri/mcp-server/src/model.rs`)
was not copied into the baseline. OpenCode's separate nested `desktop/` and `frontend/` checkouts
appeared in the project 4 destination during preparation; they were left untouched. The frontend
checkout currently has commit `507192b01bd49c14aec5241b266afba59f245475`, while the parent baseline
still pins `117fdae`. The shared project 3 integration worktree also has uncommitted edits in
`src-tauri/mcp-server/src/model.rs`, `src-tauri/mcp-server/src/server.rs`, and
`coordination/reports/M2M-030/new-animation-source-list-defects.md`. Those are the current
in-flight OpenCode edits and were not copied. Wait for their commit SHA, then review and integrate
that exact change.

## Verification on the pinned baseline

- Frontend `npm run build`: PASS, using existing cached `node_modules` through a temporary symlink;
  symlink removed immediately after the build. Vite retains its >500 kB chunk warning (offline
  bundle ~965 kB).
- `cargo test --locked --workspace` in `src-tauri`: PASS (2 library tests; binary and doc-test
  targets passed with 0 tests).
- `cargo test --locked --manifest-path mcp-server/Cargo.toml`: PASS (41 library + 3 binary tests).
- `cargo build --locked --features mcp-host` in `src-tauri`: PASS.
- `cargo build --locked --manifest-path mcp-server/Cargo.toml`: PASS.
- Built app SHA-256: `74eb72f90d9fd7369235d445a49caae5be465db1945ea5c25b7eff562e1504bd`.
- Built sidecar SHA-256: `bc9574abb486f26fa0cd3bd223794f8daaf7ff95c714baaa1936328c7a72d217`.

These are debug binaries built from the pre-fix baseline, not release packages. The app was not
launched, packaged, or installed. The install gate is after OpenCode's commit is integrated and
paired source verification/build succeeds.

## Open issue added during preparation

User reports that selecting Horse after Dragon does not load the Horse rig or expose its skeleton.
It is recorded OPEN in `KNOWN-ISSUES.md` and `TASKS.md`; the detailed reproduction and acceptance
steps are in `dragon-horse-rig-loading.md`. The screenshot was supplied in chat but no local path
was available to archive.

## Next steps

1. Get OpenCode's exact commit and confirm it is the pending change.
2. Integrate that change into project 4; resolve frontend ancestry and parent gitlink intentionally.
3. Rebuild frontend, Tauri app with `mcp-host`, and MCP sidecar; rerun the focused tests.
4. Reproduce and fix Dragon → Horse loading/skeleton exposure, then run the relevant native check.
5. Preserve OpenCode's nested checkouts until it confirms they are no longer active; verify each
   repository and the destination desktop worktree are clean.
6. Prepare/install the final paired build. Do not push; the user plans to upload to GitHub later.


This preparation report is retained as historical context. OpenCode delivered and the integration branch merged the fix; the final paired source and install evidence is in `project4-integration-install.md`.
