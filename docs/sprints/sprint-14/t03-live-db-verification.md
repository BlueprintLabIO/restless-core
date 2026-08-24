# S14-T3 — Make live-Postgres evidence an explicit command

**Layer:** Evaluation plus OrgIntel.

**Observed friction served:** Core OrgIntel behavioural scenarios intentionally return early when
`RESTLESS_TEST_DATABASE_URL` is absent. That is useful for a fast local unit loop, but not for any
claim about migrations, atomic claims, feedback or recovery.

## Outcome

One repository-owned command preflights a deliberate scratch Postgres URL and then runs the OrgIntel
behavioural suite. It reports absence/misconfiguration as a failure before test execution.

## Acceptance

- The command requires `RESTLESS_TEST_DATABASE_URL` and rejects missing, blank, non-local or
  non-scratch-looking targets.
- It names its exact database evidence; fast `cargo test` remains separately usable.
- It runs the current OrgIntel integration scenarios against a disposable/scratch company scope and
  does not seed a live company.
- Tests cover the preflight’s reject path and a known-good local URL path without requiring a live
  production database.
- The sprint report includes one observed missing-URL failure and one successful local run.

## Non-goals

- general CI orchestration;
- changing every fast test to require Postgres;
- a new test database service or fake persistence layer.

## Deletion target

Silent live-scenario skips being mistaken for completed verification.
