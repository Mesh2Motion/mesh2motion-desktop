# M2M-027 Native Verification

## Staged native build

Verified the extracted Tauri 2/WebKitGTK application (not a browser surrogate), built with the matching integrated frontend dist and bundled resources. Human, Fox, and Horse catalog entries/posters were visible; hover-driven preview behavior and inactive/offscreen stopping were checked. No preview play buttons were added. Remix background, scrollbar, corner and padding theme; Props resize anchoring; Settings wrapping; and rigging/import panel fit were checked.

The user confirmed **“all good”** after being asked to switch to “Use Your Rigged Model” at least five times and check the visual acceptance list. This is the visual acceptance for the transient black triangle NATIVE-006 on the staged native candidate. The zero-scale initialization fix and regression test are present in the integrated source.

## MCP

The final staged app and extracted sidecar were exercised through the actual JSON-RPC MCP stdio client: initialize, tools/list, and all five read-only calls succeeded. Available tools: `animations_list`, `application_get_status`, `renderer_get_state`, `rigs_list`, `scene_inspect`. No authoring tools were exposed. Private socket permissions were directory 0700 and socket 0600; staging used explicit `M2M_MCP_ENABLE=1`.

## Limits

No animation-authoring end-to-end claim is made. The live frontend IPC bridge is a remaining M2M-028 dependency. The installed package was relaunched after installation for key UI/read-only MCP smoke; user visual acceptance applies to the staged build.
