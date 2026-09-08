# ADR 0008 — Activate bounded company collaboration

**Status:** Accepted

**Date:** 8 September 2026

**Parent:** [`ARCHITECTURE.md`](../../ARCHITECTURE.md) §2.8, §4.4 and §7.4

## Context

Core deliberately deferred multiplayer until real use showed that another human and managed access
improved the outcome. That trigger is now present: the founder needs to operate one canonical company
from phone and desktop, invite a small development team, and keep shared company state available while
the heavy Runtime sleeps. Cloud already treats human multiplayer as part of its accepted target, so
leaving Core's deferral in place creates two incompatible architectures.

## Decision

Build small-company collaboration as a Core capability. Durable human and agent Actors share one
authoritative company through Rooms, Messages, recipient-relative Attention, the existing Work and
Decision concepts, and a bounded native-Docs subsystem. The Core Svelte cockpit remains the sole
company client and grows mobile/PWA behaviour. Cloud adapts hosted identity, routes entry, delivers
notifications and operates released cells; it does not own or proxy company semantics.

The programme is deliberately bounded. It does not include a Slack replacement, social feed,
presence as correctness, arbitrary ACL graphs, a general office suite, shared realtime filesystem or
shared mutable company tenancy.

## Consequences

- The old "multiplayer deferred" wording is superseded.
- Existing owner-shaped handler shortcuts must be replaced by a verified request principal before an
  invited-member surface ships.
- Chat remains ordinary append-only Postgres state. Discussion changes Work, Decisions or Authority
  only through an explicit source-owned operation.
- Collaboration services remain readable while Runtime is suspended and join company backup,
  restore, relocation, release and deletion contracts.
- Success is a real two-human company outcome with lower owner friction, not schema or UI completion.

## Risk dispositions

| Risk | Disposition | Reason |
| --- | --- | --- |
| Collaboration expands into a general suite | **Guarded** | The programme and acceptance scenarios name the small-company boundary and deletion targets. |
| Membership becomes company Authority | **Invariant** | Entry, organisational responsibility and consequential capability remain separately owned. |
| Cloud becomes a second company brain | **Invariant** | Fleet leaves the company data path after entry and stores no company bodies or mutable semantics. |
| New coordination concepts add ceremony | **Accepted** | Dogfood may delete or simplify any Room/Doc mechanism that does not improve a real outcome. |
