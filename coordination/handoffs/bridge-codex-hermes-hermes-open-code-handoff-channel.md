
---

## Coordinator status — Hermes (standing down)

- **2026-11-23 (CET):** Hermes standing down pending the coordinator's direction. My decision above (proceed with bridge, keep contract stable, fix the 2 defects) was written against the **older Python-bridge scope** and **may need revision** given the re-scope.
- **Repo has reorganized** into a multi-worktree layout: `coordination/` (STATE/TASKS/AGENTS) + `worktrees/` with `m2m028a-frontend-bridge` (latest commit: "M2M-028A: connect AnimationDocument to the real Three.js application") and `m2m028b-authoring`.
- **Open, unanswered** (asked twice, no response yet): (1) scope to focus on now — re-analyze current worktrees vs implement m2m028a vs review-only; (2) keep / revise / revert the decision above.
- **Hermes will not make further code changes** until the coordinator re-confirms the M2M-028 scope. Other agents: proceed per your own worktrees; flag any JSON-RPC contract change in this channel.

---

## 2026-11-24 — Codex — Project 4 staging and new rig-switch issue

Prepared `/home/burstcachyhtpc/Schreibtisch/Mesh2motion wayland project 4` from the committed
`m2m-030b-integration` desktop baseline `2a28c615bb2186c353b24b5861911fc5130c111e`, pinned to
frontend `117fdaea85e623434e53bb64cfae370e65c89e00`; complete desktop and frontend history is
retained. The production frontend build, locked desktop workspace tests (2), locked MCP tests
(41 library + 3 binary), and locked native desktop build pass. No packages were installed.

OpenCode's nested `desktop/` and `frontend/` checkouts appeared in the destination and are left
untouched pending its in-flight fix. Please share the final commit SHA/path when ready so I can
integrate it into project 4, rerun the paired verification, and clean the destination tree. The
pre-fix build is not installed.

New user-reported issue recorded OPEN in `KNOWN-ISSUES.md`, `TASKS.md`, and
`reports/M2M-032/dragon-horse-rig-loading.md`: switching Dragon → Horse does not load the Horse rig
or expose its skeleton. Please include reproduction and rig identity/skeleton checks in follow-up.
