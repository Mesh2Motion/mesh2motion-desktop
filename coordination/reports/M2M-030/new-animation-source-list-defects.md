# M2M-030 — New Animation source-library defects (user report, 2026-11-24)

User screenshot: `Bilder/Bildschirmfotos/Bildschirmfoto_20261124_040005.png`
Branch state at report: frontend `9b5a659`, desktop integration `20acb743`.

## Reported issues (documented before any code change)

1. **Selecting an existing animation does not import its keyframes.** In "From Existing Animation"
   mode the status says `Source selected: Attack_Ground_Pound. Create the draft to copy its tracks.`,
   but after Create clip the editor does not show reusable keyframes for the chosen source.
2. **Search does not work and user-authored clips cannot be selected.** Typing in
   `Filter animations` does not find clips such as `Wave Final 4`; those clips never appear in the
   source list, so they cannot be selected as a source.
3. **The source list stops early.** It shows roughly the first 100 animations (Angry, Attack…,
   Backflip, Bow…, Burpee) and does not expose the rest of the ~238-entry library.

## Code evidence (initial diagnosis, to be verified)

- `src/newAnimation/newAnimationWorkspace.ts` `refresh_library()` calls
  `this.animation_library.reset(this.rig_type, 1)` — only page 1 is loaded.
- `render_library(filter)` filters `this.animation_library.all()` and then applies
  `entries.slice(0, 100)`, so both the data set and the rendered list are capped.
- Because authored clips are added to the library after reset and are not part of the loaded page,
  they are absent from `.all()` used by the filter, which explains issues 2 and 3.
- `create_draft()` existing mode loads the source via `this.animation_library.load(id)` and converts
  `entry.clip_pair?.original_animation_clip` with `import_clip_as_document`. Need to verify that the
  returned clip actually has tracks and that the converted document is installed as the active draft
  (issue 1).

## Additional user bug (2026-11-24 06:19)

Screenshot `Bilder/Bildschirmfotos/Bildschirmfoto_20261124_061929.png`:
4. Rigs from **Dragon through Horse** in the New Animation rig selector do not load and do not expose
   a skeleton. Selecting Horse shows `Rig change failed: rig Horse has no rigged model`; the viewport
   still shows the previously loaded model. This appeared after the rig selector was expanded from 3
   to 9 entries, so the selector now advertises rigs that `RigConfig` cannot load on this page.

This must be fixed together with the source-list/phase issues.

## Phase / rig-loading resolution (2026-11-24 06:25)

- `scene_inspect`/`animations_list` returned `NOT_READY` because the New Animation page loads meshes
  directly into the engine, leaving `load_model_step.vertex_count = 0`; `derive_phase()` therefore
  never saw a loaded model. `EngineControlHost.observe()` now also counts
  `engine.active_skinned_meshes()` (the provider-backed accessor). Verified after rebuild: on the
  New Animation page `phase = ready`, `scene_inspect` and `animations_list` both return `ok:true`.
- Additive `phaseInputs` diagnostics were exposed in the snapshot and `application_get_status`
  (`first_frame_rendered`, `has_loaded_model`, `engine_attached`, `active_meshes`, `vertex_count`),
  which made the root cause measurable.
- Dragon/Spider/Snake/Horse have `model_variations = []` in the asset manifest but do ship
  `model.glb`. `load_rig()` now falls back to `RigConfigEntry.model_file`, so all nine rigs can load
  instead of reporting "has no rigged model".
- All mandatory tools pass: Ruff, pytest (6), ty, ast-grep, bacon, BugStalker; tsc, 71 files / 484
  Vitest tests, ESLint, Vite; cargo fmt, 63+3 MCP and 4 Tauri tests, Clippy -D warnings.

## Resolution (after documentation)

- First attempted fix (reverted): using `StepAnimationsListing.library_entries()`. It made the list
  empty because `StepAnimationsListing.begin()` calls `reset_step_data()`, which **clears** the library
  and only repopulates it through the AnimationsListing loader callback — never active on this page.
- Final fix: keep the rig-manifest `AnimationLibrary` for default clips, **add authored documents from
  `load_authored_documents()`** to the source list, and remove the `slice(0, 100)` cap. Selecting an
  authored entry now calls `create_from_existing(stored.document, name)` directly, so its real tracks
  become an editable draft. Selecting a manifest clip uses `animation_library.load()` as before.
- Frontend gates: 71 files / 481 tests, tsc, eslint, Vite all pass. All mandatory tools were re-run
  on the integration pair (see handoff).
