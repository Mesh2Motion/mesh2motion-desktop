# Mesh2Motion MCP sidecar

This is a standalone, optional Rust MCP server. It is not linked into or
automatically started by the Tauri application. An MCP host must explicitly
launch `mesh2motion-mcp` as a stdio server.

Build/test from this directory with Cargo. MCP frames use stdin/stdout; do not
write diagnostics to stdout.

## Application IPC

The sidecar accepts `M2M_MCP_IPC_ENDPOINT` only when explicitly configured.
The endpoint is an OS local socket (Unix domain socket on Unix, named pipe on
Windows); there is no TCP listener or fallback. Each tool request opens one
connection and exchanges exactly one length-prefixed JSON request and response:

```
u32 big-endian payload length | JSON payload (maximum 256 KiB)
```

Version 1 has one request method, `snapshot_get`. Request and response objects
are strict (unknown fields rejected), carry a protocol version and request ID,
and the server rejects mismatched IDs/versions, malformed frames and
contradictory response fields. A response contains either one typed snapshot or
an allow-listed `app_unavailable` / `not_ready` code. Snapshots carry both an
ISO-8601 capture time and a Unix epoch millisecond value; the sidecar computes
age from the numeric timestamp on receipt and does not trust app-provided age.
Tool error text never
includes an endpoint, path, raw exception, or IPC payload.

The Tauri snapshot publisher/listener is intentionally not implemented in this
foundation. Without it, or when the endpoint cannot be reached, every
application-dependent tool returns `APP_UNAVAILABLE`; the sidecar never
substitutes test data. Mock snapshots exist only in `#[cfg(test)]` tests.

The future Tauri host must create the socket only after an explicit opt-in,
place Unix sockets in a private per-user runtime directory with restrictive
permissions, use a same-user ACL on Windows named pipes, validate and cap the
snapshot before publishing it, and unlink/close the endpoint on app shutdown.
Both peers must close the connection after the single response. Stdio remains
the only MCP transport.

## Exposed read-only tools

- `application_get_status`
- `renderer_get_state`
- `rigs_list`
- `animations_list`
- `scene_inspect`

Inputs use strict camelCase fields. No tool accepts a filesystem path, returns
geometry/texture data, or mutates the application. GL vendor/renderer/version
fields are omitted unless the app actually measured them. Absolute rig asset
paths are always denied in this initial foundation.
