# S61-T3 — Core connections smoke

**Layer:** Tooling across Kernel, Gateway and Cockpit. **Serves:** acceptance 1 and 5.

`scripts/connections-smoke --target core`, following `multiplayer-smoke`'s shape: an isolated
daemon and Postgres, the installed company image (`repository@sha256:<digest>`), and Playwright as
the owner. It runs assertions 1–18 and 20 from the sprint spec in order. Each assertion writes
`{id, status, input, observed, expected, broken_would_show}` to an evidence JSON in a new output
directory.

- The actor view (3, 9, 12–14) is observed from inside the company container with a real
  `tool_session` capability. Mint it through the same path the launch contract uses, not by
  forging a token.
- Assertion 11 greps the container for each exact token the fixture reports having issued.
- Assertion 15 records `skipped: S57-T5 open` until the local worker lands, then runs.
- Assertion 17 invokes `skill-compatibility-smoke` as a sub-step.
- Assertion 18 installs one pinned CLI as `company` without root (pick the location the Runtime
  spec's user-space rule implies, and record it), replaces the container, and checks path and
  version.
- Cleanup runs in a `finally`. It ends with `restless-reap --check` and fails on new debt.

First extract the daemon, database and company setup that the two existing smokes duplicate into
`scripts/lib/`. Then move both of them onto it in the same change.

**Deletes:** the duplicated setup code in `skill-compatibility-smoke` and `multiplayer-smoke`.
