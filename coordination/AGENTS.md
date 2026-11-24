# Shared AI Agent Instructions

Mission: Make Mesh2Motion work natively on Linux Wayland.
Preserve Windows, macOS, and X11 compatibility.
Use modular, upstream-friendly changes.

Before starting any task:
1. Read PROJECT.md, STATE.md, TASKS.md and DECISIONS.md.
2. Check Git status and identify the active branch.
3. Claim a task before editing files.
4. Never automatically update the frontend submodule.
5. Never make unrelated changes to upstream code.

Mandatory tools: Ruff, pytest, ty, ast-grep, bacon, bugstalker.
Also run relevant TypeScript, Rust, and application tests.

After every task:
1. Document changes, commands, results and unresolved errors.
2. Update STATE.md and TASKS.md.
3. Commit changes to the appropriate Git repository.
4. Record commit hashes and provide a handoff summary.

Do not claim tests passed unless they were executed.
Do not push, merge or update upstream without approval.

## Project 3 canonical branch policy (M2M-025)

Project 3 is the canonical development baseline, cloned independently from the verified M2M-024 release.
- Canonical desktop development branch: **`m2m-025-dev`** (tag `m2m-025-baseline-desktop` @ `68616f3`).
- Canonical frontend revision: **`c417eb2`** (tag `m2m-025-baseline-frontend`).
- **Forbid concurrent direct edits to `m2m-025-dev`.** All work must happen on a reviewable branch created
  from it; integrate only after tests and review.
- Do not start animation authoring, graphics-backend migration, package installs or upstream pushes from
  this baseline without explicit approval.
- Keep the installed working application untouched until a new release is approved.
