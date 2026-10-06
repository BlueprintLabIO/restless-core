# Fixture MCP provider

A real MCP server behind real MCP authorization, used only by `scripts/connections-smoke`
(Sprint 61). It is built from the official MCP TypeScript SDK, not a hand-written OAuth server,
so Restless connects it exactly as it connects any provider.

- `server.mjs`: Streamable HTTP MCP at `/mcp`, with protected-resource metadata, dynamic client
  registration, PKCE, exact redirect matching and refresh tokens. One consent page with
  **Approve**. Four tools: `whoami` and `list_notes` (read-only), `send_note {to, body}` (reaches a
  person, unannotated), `delete_note {id}` (destructive).
- A loopback control port, guarded by `FIXTURE_CONTROL_TOKEN`, changes the world under the
  connection: `POST /mutate-send-schema`, `/expire-access`, `/revoke-refresh`, `/drop-next-send`,
  and `GET /state` (what executed, and every token issued).
- `stdio.cjs`: the same tools as a dependency-free stdio server, run by Restless's host-side
  local worker inside the company image.
- `fixtures/plugin/`: a minimal Codex-format plugin bundle (one MCP server, one skill). The runner
  publishes it as a dumb-HTTP Git repository on the optional static https port.

Check it without Restless:

```bash
FIXTURE_CONTROL_TOKEN=$(openssl rand -hex 16) node server.mjs --port 9100 --control-port 9101
npx @modelcontextprotocol/inspector   # connect to http://127.0.0.1:9100/mcp and sign in
```
