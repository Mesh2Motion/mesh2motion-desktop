# M2M-025A — Native UI acceptance status (Project 3 baseline)

> **Current status update (2026-11-23):** NATIVE-005 is user-confirmed resolved at integrated frontend `5f8134081883069abc35f538da212738bb40caa5`, including the scrollbar and the preview-holder box/padding residue. Evidence: `Bildschirmfoto_20261123_034321.png`. NATIVE-006 source fix is integrated at frontend `a33015c082aa753f04b6685e94b57297cf375207`; regression tests pass, but native visual confirmation is still pending. The installed r73 application was not updated.

Native target: installed Tauri 2 / WebKitGTK app, package `mesh2motion-git 0.1.10.r73.gc417eb2-1`.
Relaunched during this task: main PID **1466532**, WebKitWebProcess child **1466821**, both alive.
No browser tab was used as evidence.

Visual regression references supplied by the user:
`Bildschirmfoto_20261122_230632.png`, `Bildschirmfoto_20261122_231114.png`,
`Bildschirmfoto_20261123_003734.png`, `Bildschirmfoto_20261123_011619.png` (plus the upstream
mesh2motion.org rigging-page reference screenshot).

| # | Case | Status | Evidence |
| --- | --- | --- | --- |
| 1 | Every bundled tile shows its WebP poster when idle | **PASS (user-visible)** | User confirmed thumbnails fixed after M2M-023C/D; source has 684/.webp pairs and posters load from embedded dist |
| 2 | No separate per-card play buttons | **PASS (user-visible + source)** | User confirmed removal; Git blob has zero `anim-preview-control` markup |
| 3 | Hovered visible tile plays preview | **BLOCKED (automation)** | No native window control; needs manual hover check |
| 4 | Mouse exit returns to persistent poster | **BLOCKED (automation)** | needs manual hover-out check |
| 5 | Offscreen playback count zero | **BLOCKED (telemetry)** | no native debug HUD capture in this session |
| 6 | 20-video / 20-fetch ceilings | **SOURCE VERIFIED / native BLOCKED** | constants unchanged (`MAX_ACTIVE_PREVIEW_VIDEOS=20`, `MAX_CONCURRENT_PREVIEW_FETCHES=20`); no runtime telemetry |
| 7 | Props anchored during resize/tiling/fullscreen | **PARTIAL** | user-confirmed at wide size; resize/tiling/fullscreen not observed here |
| 8 | Settings opens; long MCP text wraps | **PASS (user-visible)** | user confirmed menu works again; CSS `minmax(0,1fr)` present |
| 9 | Rigging/import panels fit available width | **PASS (user-visible)** | user confirmed after upstream-width rail change |
| 10 | Human/Fox/Horse switching stable | **PARTIAL** | Human→Horse spot check passed earlier; Fox/Horse on final build not re-observed here |

## Reproducible fish procedure for the remaining blocked cases

```fish
set -l p2 '/home/burstcachyhtpc/Schreibtisch/Mesh2motion wayland project 3'
mesh2motion   # installed r73 candidate; launched PID recorded above
```

1. Hover one visible preview tile; confirm only that tile starts playing. Move the pointer away; confirm
   the poster returns. Scroll the tile offscreen; confirm it pauses.
2. With `?m2m-preview-debug=1`, fast-scroll the 234-animation catalog and confirm playing videos ≤20 and
   in-flight fetches ≤20, with offscreen playback 0.
3. Resize the window, half-screen tile it, enter/exit fullscreen, change DPR and toggle sidebars; confirm
   Props stays at the 3D viewport upper-right and the rigging/import panels are not clipped.
4. Open Settings at normal and narrow widths; verify long endpoint/value text wraps without overlap and
   MCP buttons still work.
5. Switch Human → Fox → Horse → Human; confirm no crash and posters update per rig.

Record frame times, WebKit child exits and any crash identifiers. A browser tab is not a substitute.
The historical WebKit SIGABRT is not claimed fixed; this baseline produced no new crash.

## New user-observed defect — NATIVE-005 (Remix catalog background)

**Status: RESOLVED and visually confirmed on native staged build (final frontend commit `5f81340`).** The unconfirmed hypothesis below was later tested with a WebKitGTK
computed-style probe and **disproved**: the scroller's computed background is already correct at first
paint. The full before/after evidence and fix are in `remix-first-paint.md`. This section is retained as
the original record.

- **Symptom:** in the **Remix** tab, the container background behind the animation preview tiles can be
  **black on startup**. After the user scrolls the animation list, it flips back to the normal themed
  background. The poster tiles themselves are visible in both states.
- **Evidence (user screenshots):**
  - `/home/burstcachyhtpc/Bilder/Bildschirmfotos/Bildschirmfoto_20261123_020704.png` — black catalog background.
  - `/home/burstcachyhtpc/Bilder/Bildschirmfotos/Bildschirmfoto_20261123_020719.png` — same tab after scrolling, normal background.
- **Affected surface:** `#composer-library-items` when it is a `VirtualizedAnimationCatalog`
  (`src/composer/composer.css` sets `background: var(--bg-alternate)`, and
  `.virtualized-catalog-content` is inserted by the virtualized catalog).
- **Unconfirmed hypothesis:** the first paint can occur before theme CSS custom properties are fully
  applied or before the virtualized content gets its layout/repaint, so `var(--bg-alternate)` resolves to
  the initial dark/transparent value; a scroll-induced relayout repaints with the resolved theme colour.
  This is a hypothesis only — no computed-style capture was taken from the native window.
- **Required verification before fixing:** on first paint of the Remix tab, capture the computed
  `background-color` of `#composer-library-items` and `.virtualized-catalog-content`, then capture the
  same values after one scroll event; confirm whether the theme variables are unresolved initially.
- **Required fix direction (once confirmed):** make the catalog wait for theme initialisation before
  mounting, or set an explicit themed background on the virtualized content wrapper, and add a regression
  test that asserts the computed background is the theme value at first render rather than only after a
  scroll-induced repaint.
- **Not claimed:** this is not evidence of a WebKit crash, and it does not affect the Explore thumbnails,
  Props anchoring or Settings fixes already accepted.

## New user-observed defect — NATIVE-006 (black triangle on rigged-model page)

**Status: OPEN, handed to Codex.** While switching to the **“Use Your Rigged Model”** tab, the 3D
viewport briefly shows a solid black triangle/pyramid before the normal preview skeleton appears.
This is a transient first-frame artifact on a different page from NATIVE-005 and is not covered by the
Remix catalog fix.

- Evidence: `Bildschirmfoto_20261123_032617.png` (black triangle), `Bildschirmfoto_20261123_032531.png`
  (skeleton shortly after).
- Owner: Codex (M2M-025C/next task), per user direction.
- Scope boundary: do not change this in the Remix first-paint workstream.

## NATIVE-005 update after staged native inspection

The staged M2M-025D build improved the Remix catalog: tiles and the area behind them are themed on
first display. Residues remain — the scrollbar and the scroller box/padding are still black
(`Bildschirmfoto_20261123_032423.png`). See `remix-first-paint.md` section 5.

## NATIVE-005 final verification

Native staged re-inspection by the user confirmed the Remix catalog background is fully themed on
first display, including the box, gaps and scrollbar. Evidence:
`Bildschirmfoto_20261123_034321.png`. Earlier residues (`032423.png`, `033922.png`) are resolved.
This is a native visual pass; the WebKitGTK computed-style probes remain diagnostic evidence only.

## NATIVE-006 C-branch source fix

At frontend branch `m2m-025c-frontend-integration`, the `CustomSkeletonHelper` constructor now initializes
each instanced bone transform to zero scale before its first `onBeforeRender` update. This prevents the
initial identity matrices combined with an empty per-instance color buffer from drawing a black,
full-size stack of bone geometry at the world origin. Regression test:
`src/lib/CustomSkeletonHelper.test.ts`. The source-level mechanism and test are verified; native
confirmation that the black pyramid is gone remains pending.

Integrated frontend tip at report time: `a33015c082aa753f04b6685e94b57297cf375207`. Vitest 57 files /
367 tests, ESLint, TypeScript and Vite build pass. No package installed.
