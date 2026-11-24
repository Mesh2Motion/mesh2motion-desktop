# M2M-030A + M2M-031 — Final integration, release and installation record

**Status (2026-11-23 22:25 local):** both features implemented and quality-checked;
the user has now confirmed the authored animation preview in the staged native build
(`Bildschirmfoto_20261123_215020.png`, supplied in chat). The matching in-workspace
visual evidence is `artifacts/M2M-031/screenshots/06-working-preview-verified.png`.
The M2M-031 release refs and r82 packages are present; the installed package pair
already matches r82, and the installed launcher reported `M2M_AUTHORING frontend_ready=true`.

## 1. Final source provenance

| Item | Value |
| --- | --- |
| Frontend branch | `m2m-030-031-observation-animation` (persistent, isolated) |
| Frontend final commit | **`0920f70`** |
| Staged desktop branch | `m2m-030-031-integration` |
| Desktop final commit | **`47962b6`** (frontend gitlink `0920f70`) |
| Frontend release ref | `m2m-031-release` @ **`0920f70`** (same source commit; no additional merge commit needed) |
| Desktop release ref | `m2m-031-release-desktop` @ **`e439eb1`** (r82 packaging commit, descendant of `47962b6`) |
| Safe baseline | `75aae01` (natively verified XYZ helper fix) on `0505109` exact-request trusted MCP approval |
| Staged binary | `worktrees/m2m030-031-observation/desktop/src-tauri/target/debug/mesh2motion` |
| Binary SHA-256 | **`f41baf7c436da408ed4a9e17e8e89673180476614f5a75fae9162635c1d995e2`** |
| Built MCP sidecar | `src-tauri/mcp-server/target/debug/mesh2motion-mcp`, SHA-256 `18b03d41…` (installed `/usr/bin` sidecar is stale, 5 read-only tools) |
| Contract | `coordination/contracts/mcp-pose-observation-v1.md`, publication commit `9683e96` |
| Handoff | `coordination/handoffs/M2M-030-to-Codex.md`, `coordination/handoffs/M2M-030-031-observation-channel.md` |

## 2. Features delivered

### M2M-030A — internal MCP pose observation
- Six rig-relative orthographic cameras (front/back/left/right/top/bottom) with
  explicit top/bottom up vectors; framing from evaluated pose bounds; focus-group
  reframing; no visible controls, overlays or scene objects.
- One pose evaluated at one draft revision/timestamp; shared pose digest across
  all views; stale revisions and rig mismatches rejected.
- Offscreen WebGL2 render-target capture with full GPU state save/restore, joint
  projection/occlusion metadata, bounded retention, cancel/dispose, optional Tauri
  adapter, and an idempotent rig watcher. No extra render loop or WebGL context.
- Developer-only six-camera harness (`src/authoringObservation/inspect.*`), not linked
  from app navigation, used to produce real capture samples.

### M2M-031 — New Animation workspace
- Top-level **New Animation** tab with From Scratch and From Existing modes; resolves
  the rig identity so pre-rigged GLB loads work with the authoring bridge.
- Godot-style clip editor: time ruler with scrubbing, one lane per channel, draggable
  diamond keyframes, playhead, play/pause/stop with real pose playback, local TRS and
  interpolation editing, undo/redo, save/export/import.
- Live-bone mouse editing: searchable bone list + viewport picking using canvas-relative
  coordinates (fixed mirrored left/right selection), transform gizmo (move/rotate/scale,
  local/world) writing keyframes into the shared draft.
- One authoritative AnimationDocument: GUI actions go through the same bridge/store as
  MCP; stale revisions rejected; MCP edits propagated to the GUI.
- **Save clip** commits to the application-owned library; authored documents persist
  locally, rehydrate into Explore after rig load, and get generated PNG thumbnails with
  a hover flipbook preview (2D skeleton fallback). Bundled animations are protected from
  deletion; user/MCP clips require a yes/no confirmation before deletion.

## 3. Verification executed

| Check | Result |
| --- | --- |
| ESLint `--max-warnings 0` | 0 / 0 |
| TypeScript `tsc --noEmit` | 0 errors |
| Vitest | **70 files / 467 tests passed** |
| Vite production build | exit 0; `newAnimation` and `pose-inspect` pages emitted |
| Runtime import cycles | 2 / 2 passed |
| ast-grep TS `any` / Rust `unsafe` | 0 matches |
| Ruff / `ruff format` | clean after fixing one unused import in the demo client |
| pytest (`tools/mcp`) | 6 passed (1 external Dash plugin warning) |
| ty | all checks passed |
| bacon | TTY unavailable; equivalent `cargo check --locked --all-features --offline` exit 0 |
| bugstalker | `/usr/bin/bs` v0.4.9; launched under pseudo-TTY against a compiled MCP test binary; no failing target/core exists |
| Native Tauri build | `cargo build --locked --all-features --offline` exit 0 |
| Staged native launch | MCP host bound, `M2M_AUTHORING frontend_ready=true`, 0 rejected snapshots, clean shutdown in all logs |

## 4. Evidence

- Real six-camera PNG samples: `artifacts/M2M-030/pose-inspection/*.png` (malformed
  T-pose → corrected arms → crossed legs → correct jumping jack open pose).
- User-verified native screenshots: `artifacts/M2M-031/screenshots/*.png`, including
  `06-working-preview-verified.png` showing the authored clip with a working thumbnail
  in the Explore catalog.
- Live MCP transcripts: `coordination/audits/M2M-031/*.ndjson`.
- Authored document: `artifacts/M2M-031/mcp_jumping_jack.animation-document.json` —
  4 s, 10 quaternion tracks, 90 slerp keyframes, committed at revision 12.

## 5. Known limitations

- Observation transport event names are still PROVISIONAL pending Codex confirmation;
  no Rust MCP tool was added by this work.
- Real capture images require the dev harness or Codex tools; the app never exposes a
  camera UI. Document samples are from the harness, not an MCP round-trip.
- Authored clip preview is a generated image flipbook, not an MP4 video; bundled
  animations keep their original MP4 hover previews.
- Long-run GPU telemetry and automated native pixel assertions remain BLOCKED; human
  visual confirmation is recorded in `native-acceptance.md`.
- Persisted authored clips are browser-origin local storage (bounded 20 clips / 4 MiB)
  and are skipped on rig-fingerprint mismatch rather than remapped.

## 6. Promotion and this-turn verification

1. Reports and project state were reconciled with the user's preview confirmation.
2. Promotion target selected as the prepared M2M-031 release refs. Frontend release
   ref `m2m-031-release` points to the feature tip `0920f70`; desktop release ref
   `m2m-031-release-desktop` is `e439eb1`, based on integration commit `47962b6` and
   pins frontend `0920f70`. There is no literal `main` branch in these repositories.
3. r82 packages were already present in the release worktree and already installed
   when checked; this turn did not rerun pacman. Package SHA-256: desktop
   `013533d05072df8093b212efc405974bbc0bc887ca4b7a8138ff40620207b6dd`, MCP
   `9b541c0157c79ee5371932f4b942dcb8fec7c60598c6c8798b411375383697aa`.
   Installed payload hashes match the archives: desktop
   `7c2f9afad347f8d39e482b6d4b0190b0c05343e3523d7bbe63f22dcf05b8d7fb`, MCP
   `db69364b95806bc591da472b6d4b0190b0c05343e3523d7bbe63f22dcf05b8d7fb`.
4. During this turn, installed package versions and payload hashes were checked again;
   the release pair was already installed, so pacman was not rerun. Launching
   `/usr/bin/mesh2motion` reached `M2M_AUTHORING frontend_ready=true`. A `makepkg`
   retry did not rebuild because its cached local source clone had a different origin
   URL; the r82 package archives in the release worktree were verified and matched the
   installed payload hashes. The earlier successful build/install and MCP smoke are
   recorded in §7.

## 7. Release execution result (2026-11-23 22:15 local)

### Branch promotion (user-approved)
- Frontend release branch: **`m2m-031-release`** @ `0920f7025773e68939fb329fc9d8be1f3d8dbf34`.
- Desktop release branch: **`m2m-031-release-desktop`** @ **`e439eb1`**
  (packaging pin commit; source gitlink `47962b6190fe7c2bc76046b83fc962964060bc71`).
- No canonical branch was rewritten; these are clean, persistent release branches.

### Packages built (from pinned local source commits, offline caches)
| Package | SHA-256 |
| --- | --- |
| `mesh2motion-git-0.1.10.r82.m2m031-1-x86_64.pkg.tar.zst` | `013533d05072df8093b212efc405974bbc0bc887ca4b7a8138ff40620207b6dd` |
| `mesh2motion-mcp-git-0.1.10.r82.m2m031-1-x86_64.pkg.tar.zst` | `9b541c0157c79ee5371932f4b942dcb8fec7c60598c6c8798b411375383697aa` |

Desktop package contains 1,368 preview assets (684 MP4 + 684 WebP). Location:
`worktrees/m2m031-release/desktop/packaging/arch/`.

### Installation
- Command: `pkexec pacman -U --noconfirm <both r82 packages>`.
- Result: exit 0. `0.1.10.r79.m2m027-1` → **`0.1.10.r82.m2m031-1`** for both packages.
- Snapper pre/post hooks reported their usual snapshot warnings (`Das Erstellen eines
  Schnappschusses ist fehlgeschlagen` / `Ungültiger Schnappschuss`); pacman itself
  completed successfully. This is a pre-existing environment issue.
- Rollback packages for r73/r63 remain in `artifacts/rollback/m2m-027-preinstall/`.

### Installed verification
| Item | Value |
| --- | --- |
| Installed versions | `mesh2motion-git` and `mesh2motion-mcp-git` `0.1.10.r82.m2m031-1` |
| Installed desktop binary | `/usr/lib/mesh2motion/mesh2motion`, SHA-256 `7c2f9afad347f8d39e482b6d4b0190b0c05343e3523d7bbe63f22dcf05b8d7fb` |
| Installed MCP sidecar | `/usr/bin/mesh2motion-mcp`, SHA-256 `db69364b95806bc591da472a40826c83972b61e618bb109a7e162494a6efcc40` |
| Installed preview assets | 1,368 files in `/usr/lib/Mesh2Motion/animpreviews/` |
| Installed app launch | `M2M_HOST enabled=true socket_bound=true`, `M2M_AUTHORING frontend_ready=true`, no errors |
| Post-install MCP smoke | `initialize` OK; **17 tools** including **11 `animation_*` authoring tools**; live Human catalog with 66 bones and fingerprint `e02f5440…` |
| Transcript | `/tmp/opencode/installed-app-smoke.ndjson`, app log `/tmp/opencode/m2m-installed-app.log` |

**Status: RELEASED AND INSTALLED.** The installed app is the r82 build containing the
internal observation cameras (MCP-side tools still pending Codex transport) and the
complete New Animation workspace. No further action is pending from OpenCode.
