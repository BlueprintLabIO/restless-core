# S09-T0 · Freeze the Company projection and doctor contract

**Layer:** Cross-layer contracts and the owner BFF.

**Observed friction:** The current cockpit projection returns provider-shaped fields and collapses
some source failures into empty arrays. The first Sprint 09 draft also promised versioned mandate and
snapshot writes that the current sources do not implement.

## Outcome

One source-aware Company read presents charter, limits, resources, external actions and doctor state
without becoming a writer. Unknown, stale, unavailable, absent and empty remain distinguishable.

## Acceptance

- The read names the source and observation time of every live claim.
- Generic effect rows distinguish self-attested, provider-confirmed, unknown, reconciled and legacy
  evidence where the sources support those facts.
- The doctor composes the existing Runtime doctor with Authority and OrgIntel availability.
- Only start, restart and reconcile are exposed as recovery actions; each records an Authority
  lifecycle receipt and is followed by a fresh doctor observation.
- No Company database, resource registry, mandate lifecycle or snapshot protocol is added.

## Deletion

Makes the provider-shaped `CockpitView.authority` projection and raw receipt interpretation on the
old Authority page deletable.
