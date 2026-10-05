# S57-T4 — One gateway path

**Layer:** Gateway. **Serves:** four install paths and in-Runtime OAuth tokens.

`tool_gateway.rs` serves `restless-tools` at `/tools/{company}` behind the Docker-bridge peer check,
a `tool_session` capability and, for productive sessions, the running-Attempt check. Remote
connections use rmcp's MCP authorization client (discovery, pre-registered client, CIMD or DCR)
with refresh on the host; tokens live in Infisical when configured, otherwise in a 0600 file under
the plane root. The owner API is `/api/companies/{company}/tool-connections…` and the OAuth callback
is `/connections/tools/oauth/callback`.

**Evidence:** the same test against an in-process MCP server. The Gmail run (T10) is the live proof.
