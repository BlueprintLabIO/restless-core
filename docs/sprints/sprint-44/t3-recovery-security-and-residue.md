# Sprint 44 T3 — Prove recovery, isolation, and residue handling

**Layer:** Runtime safety

**Serves:** The governance advantage Restless claims over unmanaged native work.

## Work

- Exercise crash, daemon restart, abandoned takeover, stale lease, conflicting return, and interrupted reconciliation cases.
- Test cross-company process, session, filesystem, browser, and credential isolation.
- Inspect checkpoints, logs, telemetry, process tables, native client state, and published artifacts for secret or session residue.
- Verify cleanup and revocation after completion, cancellation, rollback, and company deletion.
- Compare the recoverability and audit trail of direct native, managed, and hybrid runs.

## Acceptance

- [ ] The owner can recover or safely terminate every injected failure case.
- [ ] No test demonstrates unauthorized cross-company access.
- [ ] Secrets do not appear in stored artifacts, logs, checkpoints, or comparison packets.
- [ ] Abandoned native sessions lose authority and cannot silently continue work.
- [ ] Residual processes and state are either purged or explicitly retained by policy.

## Makes deletable

- Trust in wrapper boundaries that has not survived adversarial runtime cases.
- Manual cleanup as a normal part of native takeover.
