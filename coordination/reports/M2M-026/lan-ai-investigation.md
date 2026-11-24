# M2M-026 — Native rendering defects and complete error hunt

## 1. Baseline provenance (verified, not assumed)

| Item | Verified value |
| --- | --- |
| Historical desktop baseline | `68616f344134706889ab330eb7b475777d8baedc` |
| Historical frontend baseline | `c417eb2555921b1a90cdcc669efb2f55af9b0690` |
| Canonical desktop branch | `m2m-025-dev` @ `68616f3`, clean, gitlink `c417eb2` |
| Integrated branch before this task | frontend `m2m-025-integration` @ `5f8134081883069abc35f538da212738bb40caa5`; desktop `m2m-025-integration` @ `6b247d2e0d85b2bc57aa3d17d5a1c0806113362c`, gitlink `5f81340` |
| Other active branches | `m2m-025b-animation-document` @ `ae48734` (OpenCode), `m2m-025c-frontend-integration` @ `a33015c` (Codex) |
| Installed packages | `mesh2motion-git 0.1.10.r73.gc417eb2-1`, `mesh2motion-mcp-git 0.1.10.r73.gc417eb2-1` |
| Installed binary SHA-256 | `75b9ee801826097988d6be89a98ff5f1217f07476c8d8306606a34d49e51a24b` |
| Installed sidecar SHA-256 | `5756529cd56fa9d58cd574cb25b315105080db716f0f5cdf15e61e122fbb7311` (known stale r72 artifact, see `M2M-025/source-provenance.md`) |
| Native process at baseline | none |

Isolated worktrees created for this task (canonical branches untouched):
- frontend `worktrees/m2m026-native-fixes/frontend`, branch `m2m-026-native-fixes`, base `5f81340`
- desktop `worktrees/m2m026-native-fixes/desktop`, branch `m2m-026-native-fixes`, base `6b247d2`

## 2. Concurrent-agent ownership

| Area | Owner | Action |
| --- | --- | --- |
| `src/animation-document/**`, frozen contract | OpenCode | READ-ONLY; not edited |
| Rust MCP write operations / schema | Codex | READ-ONLY; not edited |
| Coordination shared docs (`KNOWN-ISSUES.md`, `STATE.md`, `TASKS.md`) | integration agent | Proposed entries supplied in `regression-results.md`; not edited directly |
| Renderer lifecycle / `CustomSkeletonHelper` | no prior active owner | Fixed |
| Remix catalog CSS / virtualization | M2M-025D complete, no active owner | Hardened |

**Ownership conflict recorded:** NATIVE-006 was previously annotated Codex-owned in
`native-ui-acceptance.md`, `remix-first-paint.md`, `STATE.md`, `TASKS.md` and `KNOWN-ISSUES.md`. The
M2M-026 assignment explicitly directs this agent to repair it. Codex had *already* produced the correct
frontend fix in commit `03cef900bcef7e0bda273a5415ef52753d4ba1ca` on `m2m-025c-frontend-integration`.
Rather than create a competing implementation, this task **adopted only the two relevant files** from
that commit (`CustomSkeletonHelper.ts` and its new test). Codex's concurrent AnimationDocument changes
(`5f6d010`) and its NATIVE-005 CSS variant were deliberately **not** taken.

## 3. NATIVE-006 — confirmed root cause

The black pyramid was **Three.js scene geometry**, not CSS, not a WebKit compositor artifact and not an
empty framebuffer.

- `src/lib/geometry/OctahedralBoneGeometry.ts:24-63` builds an octahedral bone template: head at origin,
  tip at `y=1`, ring radius `0.5` at `y=0.1`, with baked grayscale vertex colors.
- `src/lib/CustomSkeletonHelper.ts:141` constructs an `InstancedMesh` from that template with
  `MeshBasicMaterial({ color: 0xffffff, vertexColors: true, opacity: 0.4, depthTest: false, depthWrite: false })`.
- `src/lib/CustomSkeletonHelper.ts:194` allocates `instanceColor` as a zero-filled `Float32Array`.
- Three.js `InstancedMesh` initializes every instance matrix to **identity**. Until
  `update_bone_instances()` (called from `onBeforeRender`, lines 221-223) writes live transforms and
  category colors, one full-size template bipyramid is drawn at the origin with zero instance color:
  `0 × white × baked shade = (1,1,1)` in the screenshot.
- Why only this page: `src/retarget/steps/StepLoadSourceSkeleton.ts:153-169` constructs the helper inside
  the async GLB load callback and immediately `scene.add(helper)` while the render loop is already
  running. Other pages add helpers later or keep them hidden until a checkbox is enabled.
- Pixel evidence: `Bildschirmfoto_20261123_032617.png` has a single connected near-black component,
  x=359..666 / y=653..914, expanding linearly from an apex to a ~300 px base; `032531.png` shows the
  thin colored live skeleton shortly after.
- `static/assets/human/skeleton.glb` contains 66 bones and 1 skin but **zero meshes/materials**, so the
  pyramid is not inside the asset.
- CSS search found no `clip-path`/`polygon()` triangle; the small axis gizmo in both screenshots is
  `CustomViewHelper`, rendered in its own 128×128 corner viewport.
- Renderer lifecycle audit: `Mesh2MotionEngine.animate()` (`730-790`) is the only WebGL rAF chain
  (`lifecycle.schedule_next()`); no duplicate render loop exists. `SceneEnvironmentManager` sets
  `autoClear = false`, which can make an uninitialized buffer black, but does not create a pyramid.

## 4. NATIVE-005 — status and residual-surface audit

The user's later screenshot `Bildschirmfoto_20261123_034321.png` visually confirmed the full Remix
catalog (tiles, gaps, scrollbar) is themed on first display at frontend `5f81340`. This task's spec
describes the older residual symptoms (`032423.png`, `033922.png`); those were already repaired by
`c25e08d` and `5f81340`. A targeted audit of the acceptance list found two surfaces that the previous
fix did not explicitly paint:

- `::-webkit-scrollbar-corner` — separate WebKitGTK paint surface.
- `.virtualized-catalog-footer` — the absolute spacer below the content, used when filtering reduces
  the item count.

Both now use `var(--bg-alternate)` so dark/light themes work without hardcoded colors. This is
hardening of a confirmed-first-paint class of bug, not a claim that the earlier screenshots still fail.

## 5. Broader error hunt — confirmed vs suspected

### Confirmed
- NATIVE-006 uninitialized `InstancedMesh` template (fixed as above).
- Two unpainted Remix surfaces (corner + footer spacer) identified by static ownership analysis and
  fixed; no native screenshot exists for these specific sub-surfaces yet, so their visual effect is
  **unverified** rather than claimed.

### Suspected / ruled out (no change made)
- `ModelVariationSwitcher.update_loading_progress()` schedules a one-shot rAF without cancellation;
  callbacks only mutate DOM text/transform, so no leak or exception was demonstrated. Left unchanged.
- Absolute links in built HTML (`/create.html`, `/retarget/index.html`, `/composer/index.html`) look
  "missing" to a `file://` checker but resolve correctly under Tauri's custom origin; the user's
  successful native tab switching rules this out.
- `SceneEnvironmentManager.autoClear = false` with no explicit `renderer.clear()` is a generic
  first-frame risk, but changing renderer lifecycle is broader than the observed defects and could
  affect the view helper; no change made.
- 684 MP4 + 684 WebP assets present in the built dist; no missing poster/mp4 found.
- No WebKit child-process errors appeared in the staged app's stderr during this session.

## 6. Native verification status

A staged native build was launched from `worktrees/m2m025d-stage` using this branch's built dist,
main PID **1695278** (WebKitWebProcess `1695564`), log `/tmp/opencode/m2m026/native-run.log`.
No package was installed; the installed r73 application was untouched.

The session cannot drive the KDE Wayland window or capture native frames. All native acceptance items
are therefore recorded as **BLOCKED** pending the user's inspection; the provided screenshots remain
the only native visual evidence. See `native-visual-fixes.md` for the repeatable manual procedure.
