# S15-T1 — Authenticate the Runtime coordination channel

**Layer:** Authority plus Runtime Bridge.

**Observed friction served:** `0.0.0.0:7791` accepts a JSON `principal`; a reachable process can claim
`owner` before the daemon decides whether a command is consequential.

## Outcome

Local owner access derives from the Unix listener. TCP Runtime access requires a signed, expiring
capability whose company, principal and session/actor identity are verified before dispatch.

## Acceptance

- Runtime creation receives a company-scoped bridge capability; supervised actor processes receive a
  narrower session capability.
- TCP rejects absent, malformed, expired, tampered, cross-company and `owner`-claiming requests.
- The daemon, not `RESTLESS_ACTOR` or JSON `principal`, chooses Runtime principal/company/actor.
- The existing CLI remains functional for local owner and Runtime coordination paths.
- A focused raw-socket test proves rejection and valid bounded coordination.

## Non-goals

Accounts, mTLS infrastructure, a capability database, per-worker isolation or a generic permission DSL.

## Deletion target

Caller-supplied principal as authority evidence.
