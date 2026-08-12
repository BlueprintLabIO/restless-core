# T6 · Scheduler — periodic ticks and event-driven wakeups

**Layer:** OrgIntel — proactivity is the product differentiator (§4.2).
**Serves:** Greenfield #2. The legacy Exec only ever acts when the owner types. This is the ticket that makes the company self-running rather than reactive.
**Makes deletable:** Nothing yet — first sprint.
**Depends on:** T5.

## Build

Two trigger types, both required:

- **Time-driven** — periodic Exec planning tick, deadlines, follow-ups.
- **Event-driven** — a dependent result landing wakes the right actor.

Transport is Postgres LISTEN/NOTIFY, salvaged from `worker/delivery.rs`. **The Work/Attempt state machine wrapped around it in the legacy system is not reused** — only the transport.

At-least-once delivery; duplicates tolerated (§9.3). Internal wakeups do not need idempotency; only external effects do.

## Acceptance

- Staff completes a commitment → the Exec wakes **with no timer involved.**
- Separately, a deadline fires with no event.
- Both survive a `restlessd` restart.

## Salvage

Outbox / LISTEN-NOTIFY transport. **Re-validation:** confirm a dependent result wakes the right actor, not merely that a notification was emitted.

---
Sprint spec: [`../sprint-01-walking-skeleton.md`](../sprint-01-walking-skeleton.md)
