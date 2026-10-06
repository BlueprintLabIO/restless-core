# S61-T2 — Fixture MCP provider

**Layer:** Tooling. **Serves:** every assertion. A real provider is needed for the shipped path
to be tested, and no third-party account is acceptable in Tier A.

`tools/fixture-mcp-provider/`: one Node service on the official MCP TypeScript SDK (Streamable HTTP
server and its OAuth authorization-server router), pinned by lockfile. Build it from the SDK, not
a hand-written OAuth server.

- OAuth 2.1: protected-resource metadata, dynamic client registration, PKCE required, exact
  redirect URI matching, short-lived access tokens and refresh tokens. The consent page has one
  accessible **Approve** button and signs in the fixed account `fixture-owner`.
- Tools: `whoami`, `list_notes` (read-only), `send_note {to, body}` (no annotation),
  `delete_note {id}` (destructive), as in the sprint spec.
- Control endpoints, bound to a separate port and token known only to the runner: mutate one tool's
  schema, expire the access token, revoke the refresh token, drop the next `send_note` response
  after executing it, and report execution counts and every token it has issued (for assertion 11).
- `--stdio` mode with a bearer from env, for the local-worker path.
- `fixtures/plugin/`: a minimal Codex-format bundle with one `SKILL.md` and an `.mcp.json` naming
  the fixture via a `${FIXTURE_TOKEN}` placeholder.

**Evidence:** connecting the fixture with the official MCP Inspector (or the SDK's client) through
the full OAuth flow, then calling each tool. No Restless code involved.

**Deletes:** nothing.
