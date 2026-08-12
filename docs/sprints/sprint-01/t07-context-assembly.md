# T7 · Context assembly on wake

**Layer:** OrgIntel — context assembly on wake (§4.4).
**Serves:** An Exec that rehydrates from nothing produces a cold-start turn. This is what makes T4's continuity actually load-bearing.
**Makes deletable:** Nothing yet — first sprint.
**Depends on:** T5.

## Build

A **pure function** from a read-only state snapshot to a `ContextPackage` with a digest. No writes, no side effects.

Assembled from: mission and current priorities, active goals and commitments, recent messages, linked files and repositories, relevant decisions, current blockers, role guidance, selected memory.

**Sources are labelled by trust** (§9.5): owner directive / internal decision / working hypothesis / historical memory / untrusted external content. The owner mandate is a read-only authoritative input, kept separate from editable strategy.

## Acceptance

Two wakes against the same snapshot produce the same digest. A wake after new messages produces a different one, containing them.

## Salvage

`context.rs` (~358 LOC). **Re-validation:** lift the deterministic-snapshot + digest idea against the new OrgIntel read model; drop the kernel aggregate-version pinning, which does not exist here.

---
Sprint spec: [`../sprint-01-walking-skeleton.md`](../sprint-01-walking-skeleton.md)
