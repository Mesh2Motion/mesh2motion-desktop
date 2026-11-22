# Mesh2Motion MCP sidecar

This optional Rust MCP sidecar exposes five read-only tools over stdio. An MCP
client launches `mesh2motion-mcp`; it is not started by the Tauri application.
The Tauri app separately enables its private snapshot host only after explicit
user opt-in.

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

The Tauri application publishes validated snapshots through a private local
IPC endpoint. If the app is unavailable, disabled, or has not published a
snapshot, application-dependent tools return `APP_UNAVAILABLE`; the sidecar
never substitutes test data. Mock snapshots exist only in `#[cfg(test)]`
tests.

On Linux the host uses a private per-user runtime directory, a `0700`
directory, a `0600` Unix socket and same-UID peer verification. On Windows the
host uses the platform named pipe implementation. The app removes the socket
when the host is disabled or the app exits. Both IPC peers close each
one-request connection. Stdio remains the only MCP transport; there is no TCP
listener.

To opt into sidecar-started application launch, set
`M2M_MCP_AUTOSTART_APP=1` in the local MCP client configuration. The sidecar
then runs the configured Mesh2Motion executable directly, enables the existing
read-only host for that app process, and waits up to 25 seconds for a fresh
snapshot. The default Linux executable is `/usr/lib/mesh2motion/mesh2motion`;
`M2M_MCP_APP_PATH` is an optional absolute path override for trusted local
configuration only. No MCP tool accepts an executable path or command.

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
