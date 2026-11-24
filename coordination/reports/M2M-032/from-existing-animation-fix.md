# From Existing Animation: authored clip source fix

## Cause

The picker displayed entries from the bundled `StepAnimationsListing` catalog. Clips committed by MCP are stored separately as `AnimationDocument` records in `m2m.authored-animations.v1`; they were shown under Saved Clips, but never offered in From Existing Animation. The earlier source-list fix only addressed bundled catalog search and pagination, so it could not make the new MCP clip selectable.

The native app also still had a packaged frontend from before that fix: `mesh2motion-app/dist` was built at 04:08, while the source fix was committed at 04:10. The currently running window therefore did not contain the reported frontend repair.

## Change

The From Existing picker now combines the full bundled catalog with saved authored documents for the selected rig. Choosing an authored document copies its existing channels/keyframes into a fresh draft, resets only the draft revision/validation state, and keeps source attribution. Filtering includes saved clips. Saving/deleting clips refreshes the picker.

## Verification and launch

- `npm run build` — passed with the existing frontend dependencies from the M2M-027 integration worktree; no package install performed.
- `tsc --noEmit` — passed.
- `cargo build --locked --features mcp-host` — passed; rebuilt native binary includes the fresh frontend assets.
- Restarted the native app and routed it to New Animation through MCP. The app is running with the updated build.

The authored clip picker path now includes `Left Foot Sole Clap` when the Human rig is selected, so its saved tracks can be copied into an editable draft. No UI automation/native screenshot was available to verify the rendered picker contents directly.
