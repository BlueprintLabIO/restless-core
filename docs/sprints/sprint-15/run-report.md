# Sprint 15 — Boundary run report

**Recorded:** 24 August 2026
**State:** complete — implementation, boundary evidence and final shared-worktree checkpoint recorded.

## Checkpoints

The owner authorised checkpoint pushes for this sprint. The independently verified slices were pushed
to `dev` as:

- `90a6a18` — scoped Runtime coordination and model access;
- `d7282a1` — command-domain field rejection;
- `17c39db` — required live-Postgres checkpoint preflight;
- `1b746da` — cockpit router response-shape evidence;
- `0ca0660` — close the Docker bridge-install stdin before waiting;
- `7fa8e1f` — probe authenticated Runtime coordination and renew the bridge during recovery; and
- `60fcaab` — cancel a timed-out Docker health probe rather than leaving its `docker exec` process
  behind.

The working agreement’s checkpoint/push rule was saved earlier in `aa4eb2b`. Each listed checkpoint
was pushed without sweeping unrelated shared-worktree changes.

## Boundary evidence

### TCP cannot claim owner authority

Input:

```text
printf '%s\n' '{"cmd":"approve","company":"sprint08_test","principal":"owner"}' \
  | nc -w 2 127.0.0.1 7791
```

Observed response:

```json
{"ok":false,"error":{"kind":"authority","message":"TCP Runtime traffic may not claim owner authority"}}
```

### Authenticated Runtime coordination works

The first `sprint08_ui_test` reconciliation exposed a real installer bug: the host flushed the
`docker exec -i` stdin but retained its pipe handle, so the container-side `cat` never received EOF.
The capability file was therefore never atomically moved into `/company/run`.

After `0ca0660`, the following real-Docker, opt-in test refreshed only that `_test` company’s bounded
bridge grant and then had the Runtime make its ordinary authenticated status request:

```text
RESTLESS_RUNTIME_BRIDGE_TEST_COMPANY=sprint08_ui_test \
  cargo test -p restlessd bridge_capability_reaches_a_named_test_runtime_when_requested -- --nocapture
```

Observed:

```text
running 1 test
test runtime::tests::bridge_capability_reaches_a_named_test_runtime_when_requested ... ok
test result: ok. 1 passed; 0 failed
```

The direct bounded Runtime command then returned:

```text
docker exec -u company restless-co-sprint08_ui_test restless status
sprint08_ui_test: Running
```

The test refuses any target whose name does not end in `_test`. It neither touches a live company nor
uses a simulated coordination result.

### Doctor fails closed across daemon-version skew

The active daemon predated `7fa8e1f`, so it did not emit the new `coordination` observation. A freshly
built CLI correctly reported the stack as `degraded` rather than calling process/container health
evidence sufficient. Its actions included both:

- `restart restlessd with the current build (stop the old stack, then run restless-dev sprint08_ui_test)`
- `restless up -c sprint08_ui_test --reconcile`

The active stack was not restarted during this run: it is an independently running shared process.
The remaining positive doctor proof is to restart it with the current daemon, reconcile the `_test`
Runtime, and rerun `restless-dev doctor sprint08_ui_test`.

## Automated verification

The required database-backed checkpoint command was run successfully after the final Runtime repair:

```text
RESTLESS_TEST_DATABASE_URL=postgresql:///restless scripts/verify-sprint-checkpoint
```

It observed 130 GiB host headroom, accepted the guarded local Postgres target, completed 19 OrgIntel
live-DB tests, strict workspace Clippy, workspace Rust tests (3 `restless`, 12 model-gateway, 19
OrgIntel and 133 `restlessd`), and Svelte checks with zero diagnostics.

After `7fa8e1f`, focused checks passed:

```text
cargo fmt --all -- --check
cargo clippy -p restlessd --all-targets -- -D warnings
cargo clippy -p restless --all-targets -- -D warnings
cargo test -p restlessd company::tests::doctor_ -- --nocapture
cargo test -p restless runtime_health_fails_closed_when_required_runtime_evidence_is_missing -- --nocapture
cd web && npm run check && npm run lint
```

The checkpoint ran against the current shared worktree, including a concurrent owner-review slice
that had already passed formatting, bindings and tests. That slice remains outside Sprint 15’s narrow
commits; its presence does not replace any of the recorded Sprint 15 boundary evidence.

## Scope decisions and deletion

- The health change adds one ordinary Runtime Bridge observation; it does not add a new lifecycle,
  state owner, workflow engine, token catalogue, or policy language.
- Recovery now materialises the bridge through one shared helper, deleting the duplicate
  issue-and-install sequences from the two `up` paths.
- A running container, browser, and supervisor are no longer sufficient for a `live` claim: the
  bounded authenticated coordination path itself must answer.
