# ADR 0016: Restless as a personal MCP server

Status: accepted (2026-10-05)

## Decision

The owner gateway serves `/mcp`, a Streamable HTTP MCP server for one person. A person creates a
personal token under Account settings → AI apps and adds it to Claude Code, Claude Desktop or Codex.
The tools are `list_companies`, `inbox`, `decide`, `message` and `conversation`.

A token speaks for exactly the person who made it:

- Only a SHA-256 of the token is stored, in the Authority store (`mcp_access_tokens`).
- Every call rebuilds that person's principal. In network mode the membership is re-checked with the
  same `network_session_is_current` the browser session uses, and the request passes the same
  `membership_boundary_violation` rule. A removed or changed member's token stops at once.
- Each tool is served by the existing owner API handler through the API router, so role checks live
  in one place. A member can talk to Exec but cannot read the Inbox or decide approvals.
- A local-owner token works only in local mode, and a member token only in network mode.
- Requests carrying `Origin` are refused, so a browser page cannot spend a token.

## Rejected

- A second set of MCP-specific authorization rules: it would drift from the cockpit's.
- OAuth for MCP clients: worth adding when a client needs it; a personal bearer covers Claude Code,
  Claude Desktop and Codex today.

## Risks

- Accepted: a leaked token acts as its holder until revoked. Tokens are listed with last use and
  revocable in one click.
