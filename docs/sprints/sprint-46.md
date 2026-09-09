# Sprint 46 — Authoritative shared-state spine

**Status:** In progress — T1 (Rooms outbox/cursor) already done; T0/T2/T3/T4/T5 partial, each proven
for Rooms and/or Documents but not yet generalized to Work/Attention/Authority/company-header (see
ticket outline)
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

Audited against the actual codebase (not the stale "Draft" label below) — the same evidence-first
pass Sprint 45 got. Bottom line: the hard infrastructure (monotonic cursor, compaction, resync, SSE
snapshot+stream, scoped cache patch) is proven and working, but **only for Rooms**, and separately
(without a shared stream) for **Documents**. The real remaining work is generalizing those two proven
patterns to Work, Attention/Handoffs, Authority and the company header — not inventing the mechanism
from scratch.

- [~] C46-T0 — canonical mutation and conflict contract — `correlation_id` does not exist anywhere in
      the workspace. `expected_version` is real but Documents-only (`documents.rs`, `owner_documents.rs`).
      Idempotency (`client_command_id`) is real for Room messages (unique index,
      `rooms.rs:140-147,515-532`, 409 on reuse) and separately for Publication/Finance/Authority
      effects, but Work and Attention/Handoffs have no idempotency key or version field at all. No
      uniform envelope struct exists — every `owner.rs` `*Input` type invents its own subset ad hoc.
- [x] C46-T1 — transactional outbox/cursor migration — done for the Room/global event stream:
      `events.rs`'s monotonic append-only table, locked snapshot cursor, bounded replay with
      `resync_required`, and transactional compaction are all real (`events.rs:79,98-172,178`), backed
      by migration `0045_room_event_stream_watermarks.sql`. Not yet extended to non-event-table
      mutations (Work/Attention/Authority never emit into `events` at all).
- [~] C46-T2 — replay, gap and deduplication service — solid and wired to a real SSE endpoint, but
      Rooms-only (`owner.rs:4462-4483,3910-3959`: `resync` SSE event + `Last-Event-ID` honored on
      reconnect). No equivalent stream exists for Work, Attention, company-header or Authority.
- [~] C46-T3 — bootstrap projection and generated client types — `cockpit_view` (owner.rs:3302)
      aggregates company header + people + teams + goals + spend + authority + effect receipts in one
      `tokio::try_join!`, but omits personal Attention, unread Rooms, top Work and a sync cursor (those
      stay separate endpoints; no combined `bootstrap`/`sync` route exists). ts-rs generation is real
      and enforced by a binding-match test for `cockpit`/`conversation`/`orgintel`
      (`web/src/lib/model/generated/`, `owner.rs:9897`), but `documents.ts`, `rooms.ts`, `company.ts`
      and part of `attention.ts` remain hand-written duplicate DTOs with no generated counterpart —
      exactly Sprint 45's newest, most complex domains.
- [~] C46-T4 — scoped client reconciliation — real for Rooms (`room-queries.svelte.ts` patches
      `setQueryData` surgically per affected room, driven by the room SSE stream). Everywhere else,
      `queries.svelte.ts` still blanket-polls every 10s (`REFRESH_MS`) and `invalidateCompany` busts
      attention + cockpit + collaboration together on any change — the exact full-surface invalidation
      this ticket wants deleted, still the default path for everything except Rooms.
- [~] C46-T5 — two-client failure-injection proof — Documents has explicit two-writer 409 conflict
      tests (`owner_documents.rs:3268,3435,4047`); cross-company isolation is spot-checked per endpoint
      (~14 cases in `owner.rs`, ~8 in `owner_documents.rs`). No systematic publisher-failure-after-
      commit -> replay-convergence harness exists generically across entities.

## Acceptance — status against the audit

1. Conflict-on-stale-write: PARTIAL, Documents + Room-message-edit only; Work/Attention/Authority have
   no version field so remain silent last-write-wins.
2. Retry creates one mutation: PARTIAL, Message only (unique `client_command_id`); Work's
   `add_work`/`add_work_inner` (`goals_work.rs:140-296`) has no idempotency key, and it is called
   internally by Staff/Exec/publication code with no client-facing HTTP creation route at all today —
   closing this needs the route to exist before it needs an idempotency key.
3. Publisher-failure convergence: PARTIAL — the event table is durable and delivery-independent, but no
   test proves a crashed SSE publisher recovers purely from replay.
4. Snapshot+stream, no gaps: DONE for Rooms, not extended elsewhere.
5. Expired cursor -> `resync_required`: DONE for Rooms, not extended elsewhere.
6. Scoped invalidation, no cross-surface reload: PARTIAL, Rooms only; `invalidateCompany` still couples
   attention + cockpit + collaboration everywhere else.
7. Isolation under a manipulated identifier: likely already generic — `network_boundary_violation`'s
   `identity.scope.permits(company)` check runs once for every `/api/companies/{company}/*` route
   before any handler, not per-entity — but this has not been directly confirmed for Work/Attention
   specifically with a dedicated test the way Documents has.

## Deletion and exclusions

Delete blanket ten-second polling, full-surface invalidation, writing-client SSE dependence and any
parallel event store. Do not create a universal internal command enum, event-sourced company model,
exactly-once delivery promise, global ordering, Kafka/NATS/Redis requirement or offline company replica.
