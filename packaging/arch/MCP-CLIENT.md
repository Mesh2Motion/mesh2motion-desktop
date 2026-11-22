# Mesh2Motion MCP sidecar

The optional sidecar exposes five read-only MCP tools over stdio. The MCP
client starts the sidecar process. By default, Mesh2Motion itself must already
be open and its host enabled in **Settings → Integrations / MCP**. The host is
disabled by default.

To let the MCP client start Mesh2Motion when it launches the sidecar, add
`"M2M_MCP_AUTOSTART_APP": "1"` to the server environment shown below. This is
an explicit local opt-in: the sidecar starts the fixed Mesh2Motion executable
directly, sets the existing read-only MCP-host opt-in for that app process, and
waits up to 25 seconds for a fresh snapshot before it starts serving MCP. It
does not invoke a shell, accept executable paths in MCP tool calls, or expose a
process-launch tool. If Mesh2Motion is already running but its MCP host is not
available, the sidecar reports that MCP must be enabled in the app instead of
opening a duplicate window.

On Linux the default application binary is
`/usr/lib/mesh2motion/mesh2motion`. For a trusted development or custom local
installation, `M2M_MCP_APP_PATH` may specify another absolute executable path
in this local MCP client configuration. It is ignored unless autostart is
explicitly enabled. On Linux the configured file must exist and have an
executable bit.

Without `M2M_MCP_AUTOSTART_APP=1`, sidecar startup behavior is unchanged. The
app reports its socket path and provides a **Copy config** button with the
paths for the current user.

Configure your MCP client with that copied JSON. The shape is:

```json
{
  "mcpServers": {
    "mesh2motion": {
      "command": "/usr/bin/mesh2motion-mcp",
      "args": [],
      "env": {
        "M2M_MCP_IPC_ENDPOINT": "/run/user/1000/mesh2motion/mcp.sock",
        "M2M_MCP_AUTOSTART_APP": "1"
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
