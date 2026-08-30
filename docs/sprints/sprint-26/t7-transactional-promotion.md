# S26-T7 — Promote evidence transactionally

Once mechanical preconditions are true, custody moves atomically without asking a model to perform
bookkeeping.

**Observed friction:** valid files existed but their artifact references were not linked, causing
extra actor turns solely to repair promotion. A supposedly immutable review directory was later reused
for another candidate while retaining its old human-readable identity.

**Layer:** OrgIntel + Runtime + Authority Plane.

**Deletion target:** model-mediated artifact linking/promotion, partial promotion and mutable review
directories presented as evidence.

## Scope

- Define promotion policy as exact candidate tree, required artifacts and required conclusive gates.
- Verify and commit candidate/artifact/gate links in one transaction.
- Publish review targets at a content-addressed, write-once path with a manifest containing lineage and
  gate receipts.
- Maintain optional mutable aliases separately and show their resolved immutable identity.
- Refuse overwrite, missing artifacts and stale gate results with typed facts.

## Acceptance

- Satisfying the final precondition promotes without another model wake.
- Killing the process between verification and commit publishes neither a partial candidate nor alias.
- Reusing a review identity for changed content fails.
- A founder opening an alias can see the exact commit/tree and immutable target it currently resolves.

