# S40-T2 — Attach the genuine Codex terminal securely

**Layer:** Runtime Bridge + machine host  
**Serves:** Low-level access must reach the real owned session rather than start an unrelated agent.

## Work

- Expose only the exact owned Codex App Server over localhost, authenticated forwarding or an owned
  Unix transport compatible with the pinned real Codex terminal client.
- Exchange a short-lived opaque local launch handle for endpoint and authentication material without
  placing reusable tokens in URLs, argv, logs or shell history.
- Launch `codex --remote` in the exact attempt/conversation cwd and verify returned thread/session,
  model and service identity before granting owner control.
- Prevent ambient Codex config, MCP, skills and remembered approvals from widening the certified
  session contract.
- Supervise detach, client death, App Server death, expiry and exact transport cleanup.

## Acceptance

The native client observes the prior thread and can continue it; a newly created or mismatched thread
fails the exact-session case. Cross-scope handles fail before connection, and cleanup leaves no token,
listener, child or addressable stale session.

## Makes deletable

Copied App Server ports, raw remote tokens and new-session terminal workarounds.
