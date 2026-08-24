# S16-T2 — Make research source freshness a Runtime evidence artifact

**Layer:** Company Runtime + owner projection.

**Observed friction served:** Dogfood 1 had useful raw sources, but the review did not compactly show
source freshness or route health; a later public quote re-probe returned HTTP 429.

## Outcome

A research ReviewTarget links one ordinary per-run source evidence manifest that makes every material
claim's source, time and observed access condition inspectable.

## Acceptance

- Store the manifest with the research output in the Runtime, linked through the existing artifact
  reference; do not create an asset-custody or provider-state database.
- For each material source record locator, source type, claim supported, observed time, as-of time when
  available, freshness expectation and exact observed route/probe state.
- Keep unverified, unavailable, rate-limited and unknown distinct from a successful live authenticated
  observation. A configured credential or owner return click is not a probe result.
- Exercise at least a normal available source and a controlled unavailable/rate-limited observation in
  a test context; ensure both appear honestly in the review evidence rather than being flattened to a
  boolean connection flag.
- Preserve raw source files as ordinary work; the manifest is an index to claims, not a replacement for
  evidence.

## Deletion target

Unstructured duplicated freshness narrative and implied source health in owner-review prose.
