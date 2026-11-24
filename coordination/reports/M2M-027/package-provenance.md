# M2M-027 Package Provenance

## Source and version

- Desktop recipe/source revision: `0f39a0688f974d3d23fa3c3f576d5403eb5d9702`; functional Rust integration merge: `28f8da03000c9027c94b4761b1aebe10f0ab406d`.
- Frontend source and desktop gitlink: `921b1dbbdb375dfe392d4d35f99059e4f43e48a1`.
- Both package versions: `0.1.10.r79.m2m027-1`, monotonic over r73/r78 under `vercmp`.
- Artifacts: `artifacts/M2M-027/packages-final/mesh2motion-git-0.1.10.r79.m2m027-1-x86_64.pkg.tar.zst` and matching `mesh2motion-mcp-git` package.
- Package SHA-256: desktop `87614b26bde1be820c2b1d85bec71f7ec016d6cb49f320a338e43cd3a9378d11`; MCP `f5f849b342e290f3f102eaeb333bd12a66c303c2caa9215155d2126a76c8ece7`.

## Build and payload verification

The package recipe fetched the exact local Git revisions and compiled both executables from those sources with locked dependencies: Tauri `cargo build --locked --release --features mcp-host,custom-protocol`; sidecar `cargo build --locked --release` from `mcp-server/Cargo.toml`. Package extraction verified the staged launcher, sidecar path, capabilities, resources, and frontend dist. The package contains exactly 684 MP4 files and 684 WebP posters.

After accounting for package stripping, extracted payload hashes match freshly stripped build outputs byte-for-byte: desktop `bd82227e28da1216b4478bbd01b9c8b69090f1240eb073a61e9276d6e4d3d5b3`; sidecar `079bab85e86e5a082a803ad1d5e2c7df7f64b49ec536c924589d6666aad505af`. The final makepkg run had no absolute `$srcdir` warning; cargo remap flags cover build and cargo-home paths. This establishes source/payload provenance, not a claim of independent reproducible builds.

Older failed/obsolete package attempts and rollback archives remain preserved under `artifacts/M2M-027/` and `artifacts/rollback/m2m-027-preinstall/` respectively.
