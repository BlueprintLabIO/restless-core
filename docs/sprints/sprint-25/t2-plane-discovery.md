# S25-T2 — Publish and discover account planes

Every plane registers itself in one well-known directory that does not depend on `RESTLESS_HOME`, so
any CLI invocation can enumerate the live planes and say exactly how to reach one.

**Observed friction:** `restless company list` reported `connect /Users/yao/.restless/restlessd.sock —
is restlessd running?` while **three** daemons were running, each on its own `RESTLESS_HOME` and
`RESTLESS_PORT_OFFSET`. The error was not merely unhelpful, it was false, and the only way to find the
right plane was `lsof -nP -iTCP` plus `ps -E`.

**Layer:** Authority Plane (account plane) + CLI. The plane knows where it lives; the CLI needs to ask.

**Deletion target:** `lsof`/`ps` archaeology and `RESTLESS_HOME` guesswork as the discovery mechanism.

## Scope

- `plane::register` writes `~/.restless/planes/<root>.json` — root, socket, pid, port offset,
  configured companies, start time — and removes it on exit via a `Drop` guard.
- The record name is derived from the root path, so a restart replaces its record rather than leaving
  one stale entry per boot.
- A record is a claim, not proof: the CLI treats a record whose pid is dead as stale.
- Registration is best-effort; a plane that cannot write its record still serves.
- The CLI's connect failure enumerates live planes with their companies, or says plainly that none is
  running.

## Deliberately not in scope

A `restless installation use <name>` context subsystem. `RESTLESS_HOME` remains the selector: for a
population of two founders running experiments, a stored default is state that goes stale, and the
plurality of planes is a test-harness affordance that should stay out of the owner's vocabulary. T6
removes most of the need for it.

## Closure evidence

- `~/.restless/planes/_Users_yao__restless.json` written at boot with all seven companies; removed on
  exit.
- `RESTLESS_HOME=/tmp/restless-nonexistent restless company list` →
  `no account plane at /tmp/restless-nonexistent/restlessd.sock, but one is running elsewhere:
  RESTLESS_HOME=/Users/yao/.restless (pid 92738, companies: …)`
- `plane::tests::record_name_is_stable_and_path_safe`, `different_homes_do_not_collide`.
