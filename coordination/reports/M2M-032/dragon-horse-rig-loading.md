# Open issue: Dragon-to-Horse rig switch does not load the rig

Date reported: 2026-11-24
Status: OPEN — not yet reproduced or fixed.

## User report

After switching the rig selection from Dragon to Horse, the Horse rig does not load and its
skeleton is not exposed. The user supplied a screenshot in chat; no local screenshot path was
available to archive in this report.

## Impact

Horse animation authoring and preview are blocked. The UI may also leave stale Dragon model,
skeleton, catalog, or rig-bound animation state visible after a rig switch.

## Reproduction to verify

1. Open the rig selector with Dragon loaded.
2. Select Horse and wait for loading to settle.
3. Check model render, skeleton/bone catalog, selected rig identity/fingerprint, and animation
   track binding.
4. Switch back to Dragon, then to Horse again to detect stale state or a race.

## Acceptance

- Horse model and its actual skeleton/bone catalog load and become usable.
- The selected rig identity and fingerprint match the rendered model and authoring catalog.
- No Dragon skeleton, pose, or animation binding remains after the switch.
- Repeated Dragon ↔ Horse switching works without reload, stale content, or hidden errors.
- Add regression coverage for the failing loading/state transition and verify in the staged native
  app.

No implementation or passing verification is claimed here. Route frontend/model-loader findings
through the active integration handoff before changing OpenCode-owned frontend files.
