#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export M2M_MCP_ENABLE=1
export M2M_MCP_SIDECAR_PATH="${M2M_MCP_SIDECAR_PATH:-$root/src-tauri/mcp-server/target/debug/mesh2motion-mcp}"
if [[ -z "${XDG_RUNTIME_DIR:-}" ]]; then
  XDG_RUNTIME_DIR="/run/user/$(id -u)"
  export XDG_RUNTIME_DIR
fi
exec "$root/src-tauri/target/debug/mesh2motion" "$@"
