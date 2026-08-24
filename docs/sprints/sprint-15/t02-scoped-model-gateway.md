# S15-T2 — Scope and meter host model access

**Layer:** Authority plus Runtime.

**Observed friction served:** all Runtime processes receive one reusable OMP gateway bearer while the
daemon's spend ledger records only supervised ACP summaries.

## Outcome

The OMP root bearer stays loopback-only. A narrow host relay accepts only a signed, expiring session
grant for one company/actor/provider, admits against the current ceiling, and records terminal
pi-native charged usage at the host boundary.

## Acceptance

- OMP's imported gateway moves to loopback; the Runtime-facing relay does not expose its root bearer.
- A model capability encodes company, actor, session, provider, expiry and billing policy.
- The relay rejects other providers, expired/tampered grants and exhausted ceilings before forwarding.
- Metered terminal stream usage writes one attributed host record; unknown charged completion poisons
  the company fail-closed. Subscription usage stays zero charged dollars.
- ACP reports remain useful turn telemetry but do not double-charge the canonical ledger.

## Non-goals

A provider registry, custom model implementation, request queue, durable reservation lifecycle or
per-completion approval.

## Deletion target

Global Runtime gateway bearer and ACP-only canonical charged accounting.
