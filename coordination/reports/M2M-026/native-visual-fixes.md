# M2M-026 — Native visual fixes, exact changes and before/after

## Branches and commits

| Repo | Branch | Commit |
| --- | --- | --- |
| Frontend | `m2m-026-native-fixes` (isolated from `m2m-025-integration`/`5f81340`) | **`83f0439c2712ba9d65f4643d3666320e5fc92c20`** |
| Desktop | `m2m-026-native-fixes` (isolated from `m2m-025-integration`/`6b247d2`) | **`a981b88bd4031d62d28bc409f55b3db1d7528137`**, gitlink `83f0439` |

Neither canonical branch, any other agent's worktree, the installed package nor the Rust/MCP sources
were modified. No merge, push, install, reset or clean was performed on another agent's worktree.

## Change 1 — NATIVE-006: initial rig helper geometry

**Provenance:** adopted verbatim from Codex's `03cef90` (branch `m2m-025c-frontend-integration`),
limited to two files, because that agent had already diagnosed and implemented the same fix. No
competing implementation was created. Codex's concurrent AnimationDocument changes were excluded.

`src/lib/CustomSkeletonHelper.ts` constructor, after `this.instanceMatrix.setUsage(DynamicDrawUsage)`:

```ts
// InstancedMesh initializes every instance matrix to identity. Until the
// first onBeforeRender pass writes live bone transforms and instance
// colors, that exposes a full-size, black stack of template bones at the
// origin. Start hidden so loading a rig never flashes this artifact.
const hidden_instance = new Matrix4().makeScale(0, 0, 0)
for (let index = 0; index < this.count; index++) {
  this.setMatrixAt(index, hidden_instance)
}
this.instanceMatrix.needsUpdate = true
```

`src/lib/CustomSkeletonHelper.test.ts` (new): asserts the first instance matrix yields scale `[0,0,0]`
before any render callback runs.

**Before:** `Bildschirmfoto_20261123_032617.png` — full-size black pyramid at the origin for a fraction
of a second after entering “Use Your Rigged Model”.
**After (expected):** no black geometry on any entry; live colored bones appear directly. Native
confirmation is BLOCKED in this session (see procedure below).

## Change 2 — NATIVE-005 residual surfaces

`src/composer/composer.css`, scoped only to the virtualized Remix catalog:

```css
#composer-library-items.virtualized-animation-catalog::-webkit-scrollbar-corner {
    background-color: var(--bg-alternate);
}
#composer-library-items.virtualized-animation-catalog .virtualized-catalog-footer {
    background-color: var(--bg-alternate);
}
```

No global CSS was changed and no color was hardcoded. The earlier confirmed fix
(`background-image` gradient, themed wrapper padding, one-shot two-frame repaint, themed scrollbar
track/thumb) remains intact.

`src/ui/native-ui-layout.test.ts` gained a source-level regression assertion for both new rules.

**Before:** `Bildschirmfoto_20261123_034321.png` already shows the main Remix catalog fixed at `5f81340`;
the two newly painted sub-surfaces were not explicitly covered by that screenshot.
**After:** expected consistent theming including corner and footer spacer. Native confirmation pending.

## Repeatable native acceptance procedure (not executable by this session)

1. Launch the staged executable (no install): `worktrees/m2m025d-stage/src-tauri/target/debug/mesh2motion`.
2. Enter **Remix**; capture the catalog on first display, after scrolling to top/middle/bottom, after
   switching to Explore and back, and after a window resize. Confirm no black anywhere, in dark and
   light themes.
3. Switch to **Use Your Rigged Model** at least five times in quick succession. Capture the first
   animation frame after each switch; confirm no black triangle/pyramid appears at any point.
4. Confirm 684 posters, hover previews, no play buttons, Props anchoring, rigging panel fit and
   settings dropdown remain unchanged.
5. Record WebKit child-process errors/coredumps if any.

Current staged run: main PID **1695278**, WebKitWebProcess **1695564**, log
`/tmp/opencode/m2m026/native-run.log`. Stop with `kill 1695278`.
