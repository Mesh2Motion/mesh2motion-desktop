# Mesh2Motion MCP sidecar

The sidecar is optional and is not started by the desktop application or desktop entry. Install `mesh2motion-mcp-git` and explicitly configure your MCP client to launch `/usr/bin/mesh2motion-mcp` over stdio, for example:

```json
{
  "mcpServers": {
    "mesh2motion": {
      "command": "/usr/bin/mesh2motion-mcp",
      "args": []
    }
  }
}
```

The server exposes read-only tools. The Tauri application's snapshot publisher/local IPC bridge is not implemented yet, so application queries currently return `APP_UNAVAILABLE`; this package does not provide live application control. It opens no network listener and has no filesystem or shell tools.
