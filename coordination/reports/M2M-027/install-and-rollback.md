# M2M-027 Install and Rollback

## Pre-install recovery record

Before install, pacman reported both packages at `0.1.10.r73.gc417eb2-1`. Installed desktop SHA-256 was `75b9ee801826097988d6be89a98ff5f1217f07476c8d8306606a34d49e51a24b`; installed sidecar SHA-256 was `5756529cd56fa9d58cd574cb25b315105080db716f0f5cdf15e61e122fbb7311`; launcher SHA-256 was `9e0ac374a936bd27e912ab53f58acf17ed34c03131abd4fd5c8511e1a439e06f`.

Rollback archives retained at `artifacts/rollback/m2m-027-preinstall/`: r73 desktop `e432afd5ddb25e2864de9e8281dc49dd6e59867701de38220bde3f14e3d26779`; r73 MCP `b5918d303f93bdf56d7f5e6fce410d62636e925cbca712a0c3abd89655b1631a`; r63 desktop `36a942a036504e65d2891b8c68257dafc31f6f03562695de1b673a7513ff5794`; r63 MCP `d62764af0d417636c873367f223cbfc74976d8b89723fc3cafbc94db8b897292`. Recovery tags preserve canonical, MCP and LAN source tips in both repos.

## Installation

The user confirmed the staged native acceptance and closed the staged app cleanly; its private socket directory was absent before install. Installation completed as a paired `pacman -U` transaction through an interactive graphical terminal. Both installed package versions are `0.1.10.r79.m2m027-1`.

Installed payload verification:

- `/usr/lib/mesh2motion/mesh2motion`: `bd82227e28da1216b4478bbd01b9c8b69090f1240eb073a61e9276d6e4d3d5b3`, matching extracted package and stripped build output.
- `/usr/bin/mesh2motion-mcp`: `079bab85e86e5a082a803ad1d5e2c7df7f64b49ec536c924589d6666aad505af`, matching extracted package and stripped build output.
- Launcher `/usr/bin/mesh2motion` started the installed native app successfully.
- Normal launch did not create an MCP socket, confirming default disabled. Relaunch with explicit `M2M_MCP_ENABLE=1` logged `enabled=true socket_bound=true` at `/run/user/1000/mesh2motion/mcp.sock`.
- The installed sidecar and app passed `tools/mcp/mcp_stdio_smoke.py`: initialize, tools/list, and all five read-only JSON-RPC calls. The exact five tools were present; no authoring tools were exposed. Exchange log: `artifacts/M2M-027/install-smoke.ndjson`.

The application remains open with explicit MCP opt-in after the smoke check. Close it normally when finished.

## Rollback

Use the preserved pair together:

```sh
sudo pacman -U artifacts/rollback/m2m-027-preinstall/mesh2motion-git-0.1.10.r73.gc417eb2-1-x86_64.pkg.tar.zst artifacts/rollback/m2m-027-preinstall/mesh2motion-mcp-git-0.1.10.r73.gc417eb2-1-x86_64.pkg.tar.zst
```

Do not remove rollback artifacts. Animation authoring tools remain absent; no live bridge is available in this release.
