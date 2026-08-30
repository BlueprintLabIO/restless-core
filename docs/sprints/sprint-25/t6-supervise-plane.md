# S25-T6 — Supervise the plane; never auto-wake a cell

Register the account plane with the platform supervisor at install, so it is running before any owner
surface is used — and make waking a cell always deliberate.

**Observed friction:** the owner hit `is restlessd running?` repeatedly across shells because the
plane's lifecycle was manual and its home was carried in an environment variable that does not survive
a new terminal. The fix is not a better error (T2 did that); it is that the question should not arise.

**Layer:** Authority Plane + Runtime (fleet).

**Deletion target:** manual `restlessd` launch as the normal path; and, once test cells are
compose-isolated, `RESTLESS_PORT_OFFSET` — a supervised per-owner plane plus containerised test
installations removes the need to rewrite ports on one host.

## Scope

- launchd agent (macOS) / systemd user service (Linux) registered at install; container supervisor in
  Cloud.
- CLI fallback may start the **plane** when no supervisor is registered — starting it is idempotent,
  spends nothing and sends nothing.
- **No owner surface may auto-wake a cell.** A verb targeting a sleeping company reports that and
  offers to wake it.
- Test isolation moves to compose-per-installation rather than port rewriting on one host.

## Acceptance

A fresh shell runs `restless company list` with no environment setup and no manual daemon start.
`restless chat -c <asleep>` reports the company is asleep and does not spend.

## Closure evidence

- With no plane running, `restless company list` started the plane and answered in **5.5s**; the
  second invocation answered in **5ms**.
- With a plane running on another home, the CLI **refused to start a second** and named the running
  one — silently starting another is how installations multiply by accident.
- `infra/launchd/io.restless.plane.plist` supervises the plane at login with `KeepAlive` and a 30s
  `ThrottleInterval`, so a plane that cannot boot backs off instead of spinning.
- Auto-wake was audited rather than assumed: both `runtime::up` call sites are behind the explicit
  `up` command and `company::recover`. No owner surface wakes a cell implicitly.

## Still open

Test isolation via compose-per-installation, and the consequent deletion of `RESTLESS_PORT_OFFSET`.
