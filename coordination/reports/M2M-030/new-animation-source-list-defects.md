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
