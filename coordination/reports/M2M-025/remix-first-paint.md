# M2M-025D — Remix catalog first-paint background (NATIVE-005)

**Branch:** `m2m-025d-remix-first-paint` (isolated frontend worktree
`worktrees/m2m025d-remix-catalog/mesh2motion-app`, base `c417eb2`).
**Exact source commit:** `313d7ac500895c37b30e4934b9801a9e5a1c8653`.
Canonical `m2m-025-dev`, the M2M-025B AnimationDocument branch, Rust, MCP, packages and the installed
application were not modified. No package was installed.

## 1. Evidence

### User screenshots (native)
- Before: `/home/burstcachyhtpc/Bilder/Bildschirmfotos/Bildschirmfoto_20261123_020704.png`
- After scroll: `/home/burstcachyhtpc/Bilder/Bildschirmfotos/Bildschirmfoto_20261123_020719.png`

Sampled pixels in the catalog's background gaps:

| Point | Before | After scrolling |
| --- | --- | --- |
| `(1075,700)` | `(0,0,0)` | `(66,96,116)` |
| `(1705,700)` | `(0,0,0)` | `(66,96,116)` |
| `(1075,900)` | `(0,0,0)` | `(66,96,116)` |
| `(1080,1050)` | `(0,0,0)` | `(66,96,116)` |

`(66,96,116)` is exactly `#426074`, the dark-theme `--bg-alternate` defined in `src/styles.css:11`.
Posters and tile text are painted in both states, and the whole scroller rectangle (including its gaps)
is black in the first state.

### Computed-style capture (WebKitGTK webview, diagnostic only — NOT native acceptance)
Probe script: `coordination/audits/M2M-025/remix-theme-probe.py`.
Raw results: `coordination/audits/M2M-025/remix-first-paint-before.json` (unfixed build) and
`.../remix-first-paint-after.json` (fixed build).

**Before the fix**, at first paint, immediately after paint and after scroll:

| Stage | `--bg-alternate` | `#composer-library-items` computed background | `.virtualized-catalog-content` computed background |
| --- | --- | --- | --- |
| First paint | `#426074` | `rgb(66, 96, 116)` | `rgba(0, 0, 0, 0)` |
| Immediately after paint | `#426074` | `rgb(66, 96, 116)` | `rgba(0, 0, 0, 0)` |
| After scroll | `#426074` | `rgb(66, 96, 116)` | `rgba(0, 0, 0, 0)` |

The computed colors are identical and correct at every stage. **This disproves the earlier
unconfirmed CSS-variable-timing hypothesis.** `--bg-alternate` is a static `:root` declaration and
`ThemeManager` applies the theme synchronously before the catalog mounts; no JavaScript writes the
background variables.

## 2. Diagnosis

The black rectangle is a **WebKitGTK repaint / compositing-layer issue**, not a CSS cascade or variable
problem. Evidence chain:

1. Computed background of the scroller is correct before and after the black frame.
2. No rule or inline style sets `black`, `#000` or `rgb(0,0,0)` on these elements.
3. Regression origin is commit `6fdac10` (M2M-021B virtualized Remix library): it changed
   `#composer-library-items` into a `position: relative` scroller with absolutely positioned children
   inside a very tall (≈13,000 px) `.virtualized-catalog-content` wrapper.
4. The content wrapper had **no background** and **no minimum height**. At mount it is rendered with
   `height: 0`, then `set_items()` jumps it to the full catalog height. On the first frame WebKitGTK
   can paint the scroller's own background layer as black; the scroll event forces a repaint that
   shows the correct color.

Because a native window could not be driven or pixel-captured in this session, the compositor mechanism
itself is a strongly supported inference rather than directly observed. The computed-style non-difference
is directly observed.

## 3. Fix

Small CSS-only correction in `src/composer/composer.css`:

```css
#composer-library-items.virtualized-animation-catalog .virtualized-catalog-content {
    min-height: 100%;
    background-color: var(--bg-alternate);
}
```

The visible pixels now come from a normally painted child that always covers the viewport, instead of
depending on the scroller's first-frame layer. The original scroller background rule is kept.
No JavaScript, virtualized mount logic, preview scheduler or animation state was changed.

### After the fix (WebKitGTK computed-style probe)

| Theme / stage | `--bg-alternate` | Viewport background | Content background |
| --- | --- | --- | --- |
| Dark, first paint | `#426074` | `rgb(66,96,116)` | `rgb(66,96,116)` |
| Dark, after paint | `#426074` | `rgb(66,96,116)` | `rgb(66,96,116)` |
| Dark, after scroll | `#426074` | `rgb(66,96,116)` | `rgb(66,96,116)` |
| Light | `#c5dff0` | `rgb(197,223,240)` | `rgb(197,223,240)` |
| Light, after scroll | `#c5dff0` | `rgb(197,223,240)` | `rgb(197,223,240)` |

`min-height: 100%` and `var(--bg-alternate)` are theme- and size-dependent, so tab/page load, window
resize, virtualization and dark/light switching need no extra JavaScript.

## 4. Regression tests

- `src/ui/native-ui-layout.test.ts` — three source-level assertions: themed background on the virtualized
  content wrapper, `min-height: 100%`, and no black literal on the scroller.
- `src/features/animation-catalog/VirtualizedAnimationCatalog.test.ts` — first-frame DOM test: the
  wrapper exists, the scroller has the virtualization class, inline height/width are set, and no inline
  background overrides the CSS rule.

Full quality suite on `313d7ac`:

| Check | Result |
| --- | --- |
| ESLint `--max-warnings 0` | 0 errors / 0 warnings |
| TypeScript `--noEmit` | 0 errors |
| Vitest | **48 files / 303 tests passed** (baseline 48 / 299) |
| Vite build | exit 0 (pre-existing chunk-size warning) |
| Runtime import cycles | 2/2 passed |

Preservation checks: all existing suites pass, including the poster-count and preview-scheduler tests
in `native-ui-layout.test.ts` (684 MP4 / 684 WebP, no per-card play buttons, single `video.play()`,
20-video and 20-fetch ceilings, no `video.load()`). No AnimationDocument, rig, Rust or MCP file was
touched.

## 5. Native staged verification — partial pass, residues found

A staged native build was produced from desktop `68616f3` with the fixed dist and launched (main PID
`1597580`, WebKitWebProcess `1597868`; log `/tmp/opencode/m2m025d/staged-app.log`, details in
`coordination/audits/M2M-025/staged-run.txt`). No package was installed. The user inspected it and
reported:

- **Improvement:** the animation tiles and the area directly behind them now render with the themed
  background on first display, without scrolling.
- **Remaining black residues:** the **scrollbar element/track** and the **box element itself** holding
  the animation previews (its padding strip and the strip below the content wrapper) are still black.
  Evidence: `/home/burstcachyhtpc/Bilder/Bildschirmfotos/Bildschirmfoto_20261123_032423.png`.
  Sampled pixels (original 2560×1600 image, scaled from the 1920-wide view): `(1406,800)`,
  `(1406,933)`, `(2013,800)`, `(2013,933)`, `(1413,1120)`, `(2013,1120)` are all `(0,0,0)`, while
  tile-area points `(1440,746)` and `(1993,746)` are themed.

This matches the diagnosis: the virtualized *content* is now painted correctly, but the scroller's own
background layer (padding box and scrollbar) still fails on the first frame. The fix is therefore
incomplete; the next correction must force the scroller itself to repaint, not just its child.

### New, separate bug discovered during this inspection — NATIVE-006

When switching to the **“Use Your Rigged Model”** page, a **black triangle** is visible in the 3D
viewport for a split second before the preview skeleton appears. Evidence:
`/home/burstcachyhtpc/Bilder/Bildschirmfotos/Bildschirmfoto_20261123_032617.png` (black pyramid) and
`/home/burstcachyhtpc/Bilder/Bildschirmfotos/Bildschirmfoto_20261123_032531.png` (skeleton shortly
after). This is a different page and a different mechanism from NATIVE-005; it has been handed to
Codex and is explicitly out of scope for this Remix fix.

## 6. Historical risk note (superseded)

The risk and first C-owned pseudo-element proposal below were superseded by the integrated M2M-025
padding relocation/repaint fix documented next. The user confirmed the final NATIVE-005 result visually;
the earlier “native re-check pending” statements are historical only.

---

# Integration and residue fix (2026-11-23, later session)

## Merged branches

| Repo | Branch | Commit |
| --- | --- | --- |
| Frontend | `m2m-025-integration` (from `c417eb2`) | `c5f3cca` merge M2M-025B, `90ff637` merge M2M-025D, then **`c25e08d103991591d36af42354d5eceb051185a6`** residue fix |
| Desktop | `m2m-025-integration` (from `m2m-025-dev` `68616f3`) | `7b060d6` integration pin, then **`78e0040ba1504fe8f311c5a446daac11a385b81e`** pin to `c25e08d` |

Canonical `m2m-025-dev` / `m2m-024-release` and the installed application were not changed. No package was
installed. M2M-025B (AnimationDocument) and M2M-025D (Remix) are now on one reviewable branch line.

## Residue fix applied after native inspection

The user confirmed improvement but reported that the **scroller's padding box / catalog box** and the
**scrollbar** were still black on first display (`Bildschirmfoto_20261123_032423.png`). Two additions on
`c25e08d`:

1. `VirtualizedAnimationCatalog.schedule_first_paint_repaint()` — after the first populated render, adds
   the class `m2m-catalog-first-paint-repaint` for one frame and removes it on the next. The CSS rule
   applies a visually harmless `box-shadow: inset 0 0 0 0.5px transparent`, forcing WebKitGTK to
   repaint the scroller's own layer (padding box + scrollbar) instead of waiting for a user scroll.
2. Scoped `::-webkit-scrollbar`, `::-webkit-scrollbar-track` and `::-webkit-scrollbar-thumb` rules for
   `#composer-library-items.virtualized-animation-catalog` so those colors follow the theme.

Quality on `c25e08d`: ESLint 0/0, TypeScript 0, Vitest **56 files / 364 tests**, Vite build OK,
runtime import cycles 2/2. A new jsdom test verifies the two-frame class toggle; CSS source assertions
cover the repaint rule and themed scrollbar.

Native re-inspection of this residue fix is still required; see `coordination/audits/M2M-025/staged-run.txt`
for the staged-run procedure. NATIVE-006 (black triangle on “Use Your Rigged Model”) is Codex-owned and
was intentionally not changed here.

---

# Second residue pass (user: scrollbar fixed, catalog box still black)

User feedback on `c25e08d`: **scrollbar is now correct**, but the catalog box itself (padding strip and
areas without mounted tiles) is still black — evidence
`/home/burstcachyhtpc/Bilder/Bildschirmfotos/Bildschirmfoto_20261123_033922.png`.

Applied on frontend `5f8134081883069abc35f538da212738bb40caa5`, desktop pin
`6b247d2e0d85b2bc57aa3d17d5a1c0806113362c`:

1. Removed `padding: 0.75rem` from `#composer-library-items.virtualized-animation-catalog` (the scroller's
   padding layer was painting black) and moved the same padding onto
   `.virtualized-catalog-content` with `box-sizing: border-box`, so the visible area is now entirely
   inside the themed wrapper.
2. Added `background-image: linear-gradient(var(--bg-alternate), var(--bg-alternate))` to both the
   scroller and the wrapper, in addition to `background-color`, because WebKitGTK paints image
   backgrounds more reliably in this layer.
3. Extended the two-frame invalidation to apply a visually negligible `opacity: 0.9999` to the content
   subtree, forcing a full repaint of the wrapper background rather than only the tiles.

Quality: ESLint 0/0, TypeScript 0, Vitest 56 files / 364 tests, Vite build OK. Native re-inspection needed.


---

# FINAL STATUS — RESOLVED and visually confirmed (2026-11-23)

The user inspected the second residue pass and confirmed **Remix is fixed**.
Evidence: `/home/burstcachyhtpc/Bilder/Bildschirmfotos/Bildschirmfoto_20261123_034321.png`
— the catalog box, gaps between tiles and scrollbar all render with the themed
background on first display; no black residue remains.

## Final commit set

| Component | Branch | Commit |
| --- | --- | --- |
| Frontend foundation | `m2m-025b-animation-document` | `55d7e34`, `ae48734` |
| Frontend Remix fix | `m2m-025d-remix-first-paint` | `313d7ac` |
| Frontend integration merges | `m2m-025-integration` | `c5f3cca`, `90ff637` |
| Frontend scrollbar/padding repaint | `m2m-025-integration` | `c25e08d` |
| Frontend final themed-wrapper fix | `m2m-025-integration` | **`5f8134081883069abc35f538da212738bb40caa5`** |
| Desktop integration | `m2m-025-integration` | `7b060d6`, `78e0040`, **`6b247d2e0d85b2bc57aa3d17d5a1c0806113362c`** |

Quality at final frontend commit: ESLint 0/0, TypeScript 0, Vitest 56 files / 364 tests,
Vite build OK, runtime import cycles 2/2. Both integration worktrees are clean. No package was
installed; the installed application is untouched. NATIVE-006 (black triangle on the rigged-model
page) remains Codex-owned and was not changed.
