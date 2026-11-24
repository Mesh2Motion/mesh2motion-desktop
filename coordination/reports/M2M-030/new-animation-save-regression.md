# New Animation save regression — double persistence

## Finding and fix

The New Animation `Save clip` handler called `SharedDraftController.commit()`, whose trusted application owner already inserts the clip into the animation library and persists the AnimationDocument via `save_authored_document_once()`. The UI then called `save_authored_document()` a second time for the same document. This created duplicate entries and could throw on local-storage quota after the actual native commit had already succeeded, leaving the user with a misleading “Save clip failed” status.

The UI now treats the commit owner as the sole persistence authority and only refreshes its saved-clips list after a successful commit. Failed commits continue through the error status path. A regression assertion in the real frontend bridge commit test verifies that a successful commit writes exactly one authored document.

## Verification

- `vitest run src/authoringBridge/service.test.ts src/animation-document/authoredLibrary.test.ts`: 17 passed.
- ESLint on the changed UI and test files: passed with zero warnings.
- TypeScript `tsc --noEmit`: passed.
- `git diff --check`: passed.

Checks used an existing shared `node_modules` directory via a temporary symlink; it was removed after the checks. No package was installed.

## Integration

Cherry-pick frontend commit `91b0f75a9f0b1c4f00d27e5eebf8de0f089864cb` into the integration frontend branch and this report commit into its desktop branch. Rebuild the matching app and manually verify: create/edit a New Animation clip, click Save clip once, confirm the success status, confirm one entry appears in the saved list and Explore library, then restart and confirm it remains available.
