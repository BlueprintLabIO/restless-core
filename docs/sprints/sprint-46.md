# Sprint 46 — Authoritative shared-state spine

**Status:** Draft for founder alignment
**Programme:** [Core company collaboration](company-collaboration-programme.md)
**Paired Cloud sprint:** Cloud 17
**Depends on:** Sprint 45

## Outcome

Two authorized clients can read and change one company through narrow authoritative contracts, see
their own write immediately, receive the other client's committed change without blanket polling, and
recover correctly after a dropped or expired realtime cursor.

## Scope

- Establish a versioned network command envelope only at HTTP boundaries: idempotency key, verified
  company/principal, expected version where needed, correlation ID and domain payload.
- Return canonical entities/projections, new versions, server time and resulting event cursor from
  successful mutations; the initiating client never waits for its own SSE echo.
- Extend the existing operational-event store into a transactional outbox with monotonic company
  cursor, bounded replay, deduplication and explicit `resync_required`.
- Add race-free snapshot-plus-stream semantics to state-bearing projections.
- Build a compact bootstrap projection for current membership/Actor, company header, personal
  Attention, unread Rooms, top Work, active Actors, urgent Authority state and sync cursor.
- Define generated Core client contracts and remove hand-maintained duplicate DTOs.
- Partition query/cache identities by user, company and schema/release version. Add targeted patch or
  scoped invalidation rules; full refetch is a recovery path, not a normal event response.
- Add stage-level timing for optimistic paint, command, commit, outbox, SSE and render.

## Acceptance

1. Two clients update one versioned shared record: one succeeds and a stale editor receives current
   state plus a legible conflict, never silent last-write-wins.
2. Retrying the same command creates one Message/Work/Attention mutation and returns the same canonical
   outcome.
3. A publisher failure after database commit loses no state; later replay or bounded refetch converges.
4. A snapshot at cursor N followed by the stream from N observes every later committed change once or
   tolerates duplicates by event ID.
5. An expired cursor returns `resync_required`; the client refetches only affected projections.
6. An event for one entity does not reload unrelated Work, People, Attention and company header data.
7. Every query, command and event proves company/principal isolation with a manipulated identifier.

## Ticket outline

- [ ] C46-T0 — canonical mutation and conflict contract
- [ ] C46-T1 — transactional outbox/cursor migration
- [ ] C46-T2 — replay, gap and deduplication service
- [ ] C46-T3 — bootstrap projection and generated client types
- [ ] C46-T4 — scoped client reconciliation
- [ ] C46-T5 — two-client failure-injection proof

## Deletion and exclusions

Delete blanket ten-second polling, full-surface invalidation, writing-client SSE dependence and any
parallel event store. Do not create a universal internal command enum, event-sourced company model,
exactly-once delivery promise, global ordering, Kafka/NATS/Redis requirement or offline company replica.
