# ADR 0014 — One gateway for connected tools, governed by class

**Status:** Accepted for Sprint 57; the Gmail acceptance run remains open

**Date:** 5 October 2026

## Decision

Every external tool a company uses reaches its actors through one host-side MCP server,
`restless-tools`, mounted beside the model relay at `/tools/{company}`. A connection is one row in
`restless_authority.connections`: a remote MCP URL (no auth, a bearer credential reference, or MCP
OAuth) or a local stdio command run as a disposable host-side container. The upstream session and
its credential never enter the Runtime; the actor holds only an expiring `tool_session` capability
bound to its company, actor and, when productive, its exact Work and Attempt.

The owner grants each observed tool one class:

| Class | Owner sees | Governance |
| --- | --- | --- |
| `reads` | Reads freely | Straight through, with a read receipt |
| `acts` | Acts with a receipt | Effect runner: first contact with each declared party needs the owner; one intent per execution; receipt |
| `reserved` | Asks you first | Effect runner plus owner approval of the exact prepared call |

Annotations only propose a class. A grant pins each tool's contract digest, so a tool whose
definition changes upstream disappears from the actor's list until it is granted again. Freeze
refuses `acts` and `reserved` calls at once and leaves reads working. Disconnect clears the token,
revokes every grant and starts a new revision.

## Why

Effects execute where the credential lives. The prior shape had four install paths and per-provider
read profiles, and remote OAuth tokens lived in the Runtime through `mcp-remote`. One connection
model, one gateway and the existing first-contact rule cover arbitrary MCP servers without a
provider catalogue.

## Consequences

- A tool-call effect is the second execution kind beside argv. It reuses the idempotency rule
  (`prior_execution`): a successful receipt replays, a different command under a key is refused,
  and an intent newer than its receipt is an unknown outcome. MCP has no idempotency key, so a lost
  response stays unknown until `restless_reconcile_effect` settles it with a read receipt on the
  same connection observed after the intent.
- Test companies are governed like live ones for tool effects, because their credentials are
  dedicated test accounts held host-side.
- `acts` and `reserved` tools gain a required `_restless {purpose, key?}` argument that the gateway
  strips before forwarding.
- The legacy profiles in `connected_tool.rs` and `mcp_gateway.rs` remain until the Clapping Hands
  connection migrates (T6), then are deleted.
