# S37-T5 — Recover, observe ceilings and prove terminal absence

**Layer:** Authority reconciliation + provider mechanics + OrgIntel evidence

## Observed friction served

Provider acknowledgement did not prove real reachability, public UDP failed independently of HTTP,
and deletion acknowledgement did not immediately prove that routes had disappeared. Restarts and
unknown results can otherwise create duplicate endpoints or residue.

## Outcome

Reconcile every publication by stable identity across Runtime, account-plane, gateway, workload and
provider failure. Record provider and external-client observations separately, enforce frozen limits,
and make teardown terminal only after re-observed absence.

## Acceptance

- Runtime replacement leaves an exact ready publication usable without exposing the replacement.
- Account-plane, gateway and workload restarts follow the frozen connection/admission behaviour.
- An ambiguous create or delete result is reconciled before another operation is attempted.
- At most one route, port allocation, gateway, workload and internal network exist per publication.
- Connection/resource ceilings refuse excess load without affecting another publication.
- Unknown bandwidth, spend, connection or cleanup values remain unknown.
- Expiry, revoke and stop re-observe absence of route, allocation, workload, network, invitation,
  service identity, lease and scoped temporary artifacts.
- One publication's crash or cleanup failure does not interrupt another company or publication.

## Makes deletable

Timeout-as-success checks, provider-acknowledgement readiness and broad host/container cleanup.
