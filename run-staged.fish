#!/usr/bin/env fish
# M2M-030A + M2M-031 staged native launcher (debug build, NOT installed).
# Exact staged pair:
#   desktop  m2m-030-031-integration @ f459500ade8d86051de103689d4d85c05cccbe16
#   frontend bc8b22ea9128cebfa4fa81f7edf46dbe2bb6df5a
set -l root (dirname (status --current-filename))
set -gx M2M_MCP_ENABLE 1
# Prefer the sidecar built from this exact staged source. The installed
# /usr/bin/mesh2motion-mcp is stale (5 read-only tools, no authoring tools).
set -l matching_sidecar "$root/src-tauri/mcp-server/target/debug/mesh2motion-mcp"
if test -x "$matching_sidecar"
    set -gx M2M_MCP_SIDECAR_PATH "$matching_sidecar"
else
    set -gx M2M_MCP_SIDECAR_PATH /usr/bin/mesh2motion-mcp
end
set -gx XDG_RUNTIME_DIR /run/user/(id -u)
exec "$root/src-tauri/target/debug/mesh2motion" $argv
