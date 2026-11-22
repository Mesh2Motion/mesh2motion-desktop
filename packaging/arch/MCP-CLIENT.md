# Mesh2Motion MCP sidecar

The optional sidecar exposes five read-only MCP tools over stdio. It is not
started automatically. Install `mesh2motion-mcp-git`, launch Mesh2Motion, and
enable **Settings → Integrations / MCP** in the desktop app. The host is
disabled by default. The app reports its socket path and provides a **Copy
config** button with the paths for the current user.

Configure your MCP client with that copied JSON. The shape is:

```json
{
  "mcpServers": {
    "mesh2motion": {
      "command": "/usr/bin/mesh2motion-mcp",
      "args": [],
      "env": {
        "M2M_MCP_IPC_ENDPOINT": "/run/user/1000/mesh2motion/mcp.sock"
      }
    }
  }
}
```

Replace `1000` with the UID shown by `id -u`, or use the app's **Copy config**
button so the socket and executable paths match this installation. The socket
is created only while the host is enabled, under the per-user runtime directory
with a private directory and socket mode. If the app is closed, disabled, or
has stopped publishing fresh state, the tools return an explicit unavailable
or stale response.

The server exposes only `application_get_status`, `renderer_get_state`,
`rigs_list`, `animations_list`, and `scene_inspect`. It opens no network
listener and provides no write, arbitrary filesystem, or shell tools. Unix
socket permissions limit access to the same user account; this is not
per-client authentication between processes running as that same user.
