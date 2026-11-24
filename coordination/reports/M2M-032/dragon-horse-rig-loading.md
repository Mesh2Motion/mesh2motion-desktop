# Dragon-to-Horse rig loading — resolved in integration

Date reported: 2026-11-24
Status: FIXED in source and integrated; native interactive switching not independently repeated by Codex.

## Root cause and fix

Dragon, Spider, Snake, and Horse ship a `model.glb` but have zero manifest variations. `load_rig()`
only searched variations and therefore reported no rigged model. OpenCode added a fallback to
`RigConfigEntry.model_file`, which loads the shipped model and exposes its skeleton. OpenCode's
rebuilt native verification reports all nine advertised rigs load.

A related New Animation page readiness defect was fixed at the same time. Models loaded directly
into the engine left `load_model_step.vertex_count` at zero, so `derive_phase()` kept the app in
`loading`. `EngineControlHost.observe()` now also counts provider-backed
`engine.active_skinned_meshes()`. Additive `phaseInputs` diagnostics make the decision inspectable.
OpenCode verified phase `ready` and successful `scene_inspect` and `animations_list` calls on the
New Animation page.

## Integration and regression evidence

- Desktop integration merge: `854be4018aa91894e82aa8fa7f37c14cd64dba90`.
- Frontend pin: `490db8939d3cb99b956dbce267d235976cabf3c5`.
- OpenCode implementation and verification report:
  `coordination/reports/M2M-030/new-animation-source-list-defects.md`.
- Project 4 full integration, test, package, and install record:
  `coordination/reports/M2M-032/project4-integration-install.md`.
- OpenCode reports Ruff, pytest (6), ty, ast-grep, Bacon, BugStalker 0.4.9, TypeScript, 484 frontend
'tests, ESLint, Vite, Rust format, MCP and Tauri tests, and denied-warning Clippy passing. Codex's
  independent paired-tree checks and any bounded-tool limitations are detailed in the integration report.

## Remaining verification

The source defect is fixed and the resulting packages are installed. Codex did not launch the
application window after installation, to avoid stealing the user's focus or pointer. OpenCode's
native verification covers all nine rig loads, but repeated Dragon ↔ Horse interaction in the
installed build remains a manual acceptance check.
