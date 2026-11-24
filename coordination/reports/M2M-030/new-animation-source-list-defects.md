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

- Root cause of issues 2 and 3: `NewAnimationWorkspace` owned a **second, private `AnimationLibrary`**
  reset from the rig manifest, while authored clips live in `StepAnimationsListing`'s authoritative
  library (the one MCP `animations_list` reports). The source list also hard-capped rendering at
  `entries.slice(0, 100)`. Fixed by exposing `library_entries()` / `load_library_entry(id)` on
  `StepAnimationsListing` and using that library for the source list (no 100-entry cap).
- Issue 1: with the authoritative library, `load_library_entry()` returns the loaded entry with its
  `AnimationClip`, so `import_clip_as_document()` receives real tracks and installs them as the active
  draft via `create_from_existing()`.
- Frontend gates: 71 files / 481 tests, tsc, eslint, Vite all pass. All mandatory tools were re-run
  on the integration pair (see handoff).
