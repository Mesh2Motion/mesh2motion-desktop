# M2M-028B integration status

**Status: permission flow amended and verified through the real native MCP path; Agent_Wave_01 live acceptance remains incomplete.** No package/install and no Agent_Wave_01 success claim.

- Desktop feature base: `m2m-028b-keyframe-authoring` @ `15b0f3a`; its current uncommitted permission change removes the enable toggle and grants in-memory permission from a valid authoring MCP request. Frontend gitlink was `a96dff2` at the base.
- Frontend base: `m2m-028b-frontend-integrated` @ `a96dff2`; includes OpenCode's final contract reconciliation and Codex's Tauri event target / stable bind-pose fingerprint fixes.
- Permission amendment follows user direction: live catalog stays read-only; the first valid non-catalog authoring operation grants permission for the app session. Browser CustomEvents cannot grant it. Commit and undo still require native digest-bound approval.
- Successful staged native JSON-RPC permission probe: all five read-only tools and live Human catalog (66 bones); `animation_session_status` returned `authoring_enabled: true`; a bounded empty probe draft was created then cancelled; no animation-library clip was added. Transcript: `coordination/audits/M2M-028/mcp-permission-transition.ndjson`.
- Older read/catalog transcript: `coordination/audits/M2M-028/mcp-transcript.ndjson`. No Agent_Wave_01 keyframes, preview, commit, export/reimport or undo claim yet.
- Frontend checks: ESLint 0/0, TypeScript pass, Vitest 61 files / 411 tests, Vite build pass. Rust checks: fmt, workspace 3 Tauri tests, MCP 51 library + 3 sidecar tests, Clippy denied warnings pass. Python checks: Ruff pass, pytest 4 pass (one system Dash deprecation warning), ty pass. ast-grep TypeScript `any` and Rust `unsafe` scans found zero matches.
- `npm ci` restored 241 lockfile packages without a lockfile change; npm audit reported 9 moderate advisories and blocked esbuild/workerd install scripts. The production Vite build still completed.
- Permission-amended isolated debug binaries: desktop SHA-256 `e8d86a817561ed28f03fb295c33b71ebd1e8bf8a8648f042b2468fbca3b24136`; sidecar SHA-256 `bf94c62c62a3b22ed071ba8900f8710990fa662077105fda33b1b27566b2e647`.
- Staged app launched with `M2M_MCP_ENABLE=1`, the matching sidecar path and isolated XDG runtime `/tmp/m2m028b-runtime`. Runtime root/subdirectory modes are 0700; socket is 0600. An earlier probe hit a stale socket/session; that process was cleared before the successful run.
- Bounded Bacon check completed Cargo check in 6.44 seconds; its persistent watcher ended at the 30-second bound.
- OpenCode was asked via the shared handoff to create its integration worktree from the updated final feature commits. No package was built or installed; canonical branches and installed M2M-027 remain untouched.
