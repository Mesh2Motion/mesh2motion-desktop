# M2M-025A — Source provenance and discrepancy investigation

## 1. Verified Git facts (checked, not inferred from reports)

| Check | Result |
| --- | --- |
| Desktop `68616f3` exists in Project 2 desktop repo | commit |
| Frontend `c417eb2` exists in Project 2 frontend repo | commit |
| Prior desktop `d53041b` | commit; ancestor of `68616f3` (exit 0) |
| Prior frontend `e06d6bb` | commit; ancestor of `c417eb2` (exit 0) |
| `68616f3` pins frontend | `160000 c417eb2555921b1a90cdcc669efb2f55af9b0690` |
| Release branches | desktop `m2m-024-release-desktop` = `68616f3`; frontend `m2m-024-release` = `c417eb2` |

## 2. Fix verification at frontend `c417eb2` (Git blobs, not working tree)

| Requirement | Evidence |
| --- | --- |
| 684 persistent WebP posters | `git ls-tree -r --name-only HEAD | grep -c .webp` = **684**; MP4 = 684 |
| No separate per-card play buttons | `anim-preview-control` only appears in a **test assertion** that forbids it; zero markup/handler/CSS in catalog sources |
| Hover-driven playback | `pointerenter` present in `AnimationPreviewVideos.ts` and its tests |
| Props viewport anchoring | `PropsPanel.ts` has zero `dock.style.right`; `SceneEnvironmentManager.ts:70-72` reparents `#props-dock` into `#m2m-viewport`; `props.css` uses `position:absolute; top:.5rem; right:.5rem` |
| Settings stacking | `styles.css` has `nav { z-index: 30 }`; `#tool-panel` z-index 2 |
| Settings text layout | `settings-dropdown.css` uses `minmax(0, 1fr)` twice, `min-width:0`, wrapping |
| Responsive rigging/import panel | `body.tool-panel-page { --responsive-right-width: clamp(32rem, 46vw, 54rem) }`; `.bone-map-grid { flex-wrap: nowrap }`, `.bone-map-section { flex:1 1 0; min-width:0 }` |
| Source integrity | zero single-line `.ts` files containing literal `\n` in Project 3 |

## 3. MCP sidecar SHA discrepancy — finding

Installed `mesh2motion-mcp-git 0.1.10.r73.gc417eb2-1` contains sidecar SHA-256
`5756529cd56fa9d58cd574cb25b315105080db716f0f5cdf15e61e122fbb7311`, **identical to r72 and r63**.
Payload hashes extracted from all packages:

| Package | Sidecar payload SHA-256 |
| --- | --- |
| r63 / r72 / **installed r73** | `5756529c…` |
| r75 / r77 / r78 candidate | `24883bc6beac69a52e39e85c14ea9f935c3dbe7b964e92ec99ff046965e7c9ef` |

String inspection: r78 sidecar contains `M2M_MCP_AUTOSTART` messages; installed r73 sidecar contains none.
Desktop `68616f3` source has the opt-in autostart implementation in `src-tauri/mcp-server/src/main.rs`
(commit `1798a17`) and host-exit cleanup in `src-tauri/src/lib.rs` (`bd2e4e5`).

**Root cause:** the final r73 `PKGBUILD` copied the MCP sidecar from the older `artifacts/stage/usr/bin/`
directory (r72 artifact) instead of rebuilding it from the release Rust source. The r73 version string
therefore overstates the sidecar provenance; the frontend and desktop binary are from the release, but the
sidecar is stale. No package was installed during this migration; the installed working app remains
untouched. Remediation for a future approved release: build the sidecar from `68616f3` and verify its
payload hash equals `24883bc6…` before packaging.

## 4. r78-versus-r73 version discrepancy — finding

- `r78.gd53041b` was Codex's **candidate** built from desktop `d53041b` / frontend `e06d6bb`; it was not
  installed (M2M-024 integration/package reports).
- `r73.gc417eb2` is the **installed** package built from desktop `68616f3` / frontend `c417eb2`; its
  version number came from the frontend commit count, not a shared global revision counter.
Both are valid, differently-derived versions from two parallel package lines. The discrepancy is a
labelling/provenance hazard, not a source-tree conflict. A future release should use one monotonic
package revision derived from the final desktop commit.

## 5. Package integrity notes

- Installed desktop binary SHA-256 `75b9ee801826097988d6be89a98ff5f1217f07476c8d8306606a34d49e51a24b`.
- Installed resources: 684 MP4 + 684 WebP under `/usr/lib/Mesh2Motion/animpreviews`.
- Tauri config: `withGlobalTauri:true`, `assetProtocol` scope `$RESOURCE/animpreviews/**/*`, bundle
  resources map `../mesh2motion-app/static/animpreviews/` → `animpreviews/`.
- Capabilities: `core:default`, `opener:default` only; no broad filesystem permission.
- `generate_handler!` remains in `src-tauri/src/lib.rs`; five read-only MCP tools unchanged.
- Reproducibility caveat: the installed binary embeds one absolute Project 2 build path string.
