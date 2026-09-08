# ADR 0011 — Transactional outbox for client delivery

**Status:** Accepted

**Date:** 8 September 2026

**Parent:** [`ARCHITECTURE.md`](../../ARCHITECTURE.md) §4.4 and §9.3

## Context

The current Postgres notifications are useful wake signals but are not replayable. Mobile clients,
Rooms, mentions and document metadata need to recover after backgrounding or a broken stream without
turning realtime delivery into a second source of truth. A new event bus or event-sourced company
model would duplicate the existing operational event substrate.

## Decision

Extend the existing recoverable operational-event store into a transactional outbox with monotonic
cursors for client-visible state changes. A domain mutation and its small invalidation event commit in
one transaction. SSE publishes identifiers, versions and affected projections; clients apply the
canonical command response, consume cursor events and refetch targeted state. On an unrecoverable gap,
database state wins and the client performs a scoped refetch.

Idempotency is required at retryable network/mobile mutation boundaries. It is not a universal
internal command algebra, and no event becomes constitutional truth merely because clients consume
it.

## Consequences

- Evolve the existing events/publisher path rather than add Kafka, NATS, Redis or a parallel ledger.
- Retention is bounded and observable; projections and clients tolerate duplicate delivery.
- Full sensitive bodies do not appear in general SSE events or telemetry.
- Blanket polling becomes deletable as each surface adopts the cursor contract.
