# S15-T6 — Require live-Postgres evidence at checkpoint exit

**Layer:** Evaluation plus OrgIntel.

**Observed friction served:** direct OrgIntel scenarios intentionally skip when
`RESTLESS_TEST_DATABASE_URL` is absent, so a green workspace test is not live-Postgres evidence.

## Outcome

The documented sprint/checkpoint exit command invokes the existing guarded verifier and calls missing
scratch database configuration a failure, while ordinary fast test loops remain usable.

## Acceptance

- One documented checkpoint/release command invokes `scripts/verify-orgintel-live-db` before claiming
  live-Postgres evidence.
- Missing/invalid scratch URL fails the command before cargo tests run.
- Fast `cargo test` documentation says exactly what it does and does not prove.
- A self-test covers script argument/preflight behavior without touching a live company.

## Deletion target

Implicit inference from a skipping test suite to live database proof.
