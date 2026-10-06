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

## As built (6 October 2026)

- The owner's browser leg is an HTTP client that follows the provider's consent page and redirect to
  the plane callback, the same requests a browser makes. Sign-in starts through the owner API rather
  than a cockpit click, because Sprint 63 replaces the Connections page; Sprint 63's acceptance
  covers the page.
- Tool-session and actor-session capabilities are minted with the installation key in the daemon's
  own `r1.` encoding, as the skills smoke already did; the gateway's verification path is the real
  one.
- `scripts/lib/smoke_plane.py` carries the shared plane, and `skill-compatibility-smoke` now uses it
  (it no longer needs the shared `restless-local-postgres` container). `multiplayer-smoke` is Node and
  keeps its own runner.
- The skills smoke's interval-schedule CLI check was stale on `main`: recurring wakes became
  responsibility-bound in `4423d51`, and `restless-orgintel/tests/schedule_controls.rs` covers them.
  It was removed.
