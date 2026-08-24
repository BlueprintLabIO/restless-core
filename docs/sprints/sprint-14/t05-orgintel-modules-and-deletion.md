# S14-T5 — Split OrgIntel internally and delete settled compatibility paths

**Layer:** OrgIntel.

**Observed friction served:** The 4,583-line OrgIntel façade holds actors/teams, Work/Attempts,
messages/feedback, schedules, recovery, types and 224 raw SQL calls together. The implementation is
hard to navigate even though its ownership model is now clearer after Sprint 12.

## Outcome

The Rust OrgIntel crate keeps one public façade and one authoritative Postgres owner, but its internal
implementation is organised around stable current domains. Obsolete compatibility code is deleted when
the S12 behavioural suite proves it unused.

## Acceptance

- Proposed module boundaries follow actual facts: shared types/schema, actors/teams, Work/Attempts,
  messages/feedback, schedules, recovery/review and compactable event helpers.
- Public callers keep their current facade or receive a deliberate, minimal source-compatible move;
  no repository trait, ORM layer or second database client is added.
- SQL transactions stay co-located with the operation that owns their invariant.
- Migration ordering and existing database schema remain unchanged unless an independently observed
  defect requires a separate migration.
- Live-Postgres scenarios prove atomic claim, feedback cursor/successor, direct delivery, recovery
  and review linkage after moves.
- Remove unused types/helpers or compatibility paths revealed by the move; retain only evidence that
  still serves a live scenario.

## Non-goals

- a TypeScript OrgIntel module;
- a universal event ledger;
- an event-sourcing rewrite, generic persistence repository or query builder abstraction;
- reformatting every query just for style.

## Deletion target

The monolithic implementation layout and stale S12 compatibility code that no longer protects a
current outcome.

## Evidence

- The public façade remains at `lib.rs`; its internals now separate actors, goals/work, attempts,
  artifacts, review, messages, schedules, events and shared types. The old `work.rs` monolith was
  deleted without changing migrations or adding another persistence layer.
- `scripts/verify-orgintel-live-db` passed 17 live local-Postgres unit/integration scenarios after the
  move, including atomic claim, feedback, direct delivery, recovery and review-linkage coverage.
- Strict OrgIntel Clippy and the full 149-test Rust workspace passed after the split.
