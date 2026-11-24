# M2M-030 — Native authored clip save/restart check

## Result

The integrated native app and sidecar successfully committed a new AnimationDocument to the real Human animation library, and the saved clip survived a full application restart.

- Integration desktop branch at test: `d042136fa99b86506eda9d517e56b8a29b67deae`
- Integrated frontend pin: `d106ffe266404e18e7874be029e6af0b68e2bb53`
- Tauri app binary SHA-256: `9a74a35c8c877ff1061c4bba8cbf381ea21bc0c8121159a595eed277240fb344`
- MCP sidecar SHA-256: `69fcc7a6f2dfe70b0286916d891ab1e70dbd10dc3c7092a50ef19e522f4c4e1e`

The actual Tauri app was built with `--features mcp-host`, launched, and reached ready state. Through a real stdio JSON-RPC client connected to its private Unix socket, the check opened the New Animation page, read the live Human rig catalog, created `Codex Save Restart Smoke`, authored two rest-position keyframes against live bone `human:bone:010`, validated it, and committed it. The commit returned library entry `custom-import-1`, revision 3. In Explore, the app reported exactly one matching custom-library entry (one second, 67 tracks, authored id `authored-mwu4ouac-74dkci`).

The app was then terminated and relaunched from the same build. After the restarted app reached `phase=ready` on Explore, `animations_list(includeCustom=true, rig=human)` again returned exactly one `Codex Save Restart Smoke` entry with the same authored id. This confirms persistence across process restart, not merely an in-memory library insertion.

## Scope limit

The native app was visibly opened on New Animation, but this environment does not expose native mouse/keyboard automation. I did not physically click the Save clip button. The actual native MCP commit exercises the same shared commit service and `RealApplicationOwner` persistence path used by that button; the added UI fix removes the button's second, duplicate localStorage write. Its handler behavior is covered by the frontend bridge persistence regression test (17 focused tests passed, including exactly one stored document), and the integrated frontend suite/build passed (484 tests, ESLint, TypeScript, Vite build). A direct button click remains a short human check.

The `Codex Save Restart Smoke` test entry remains in the user's authored library so it can be inspected. It is a rest-pose persistence probe, not an animation-quality sample. Delete it from New Animation's saved-clips list if it is not useful.

## Evidence

- `artifacts/M2M-030/save-restart-check.ndjson`: app page switch, live catalog, draft creation, keyframe edit, validation, native commit, and initial library-read attempt while New Animation was active (`NOT_READY`, as expected for that page).
- `artifacts/M2M-030/save-restart-check-list-2.ndjson`: Explore readiness and pre-restart library listing with the single smoke entry.
- `artifacts/M2M-030/save-restart-check-after-restart.ndjson`: post-restart ready status and matching persisted entry.

No packages were installed. No installer or release package was built or installed.
