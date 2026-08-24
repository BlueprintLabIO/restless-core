# S15-T5 — Check the owner cockpit projection contract

**Layer:** Owner cockpit.

**Observed friction served:** `owner.rs` constructs cockpit JSON dynamically while the Svelte client
maintains an unrelated handwritten `CockpitView` model.

## Outcome

The cockpit's high-value read model has a serializable Rust DTO and an explicitly checked Svelte
contract, with one router-level response scenario.

## Acceptance

- Replace the primary dynamic cockpit projection with named Rust response types.
- Generate or validate the matching TypeScript contract from the same source of truth.
- Exercise the router response shape for populated and degraded/error-relevant data.
- Preserve the owner surface's calm existing design; this is a contract change, not a redesign.

## Deletion target

Unchecked dynamic BFF shape at the owner boundary.
