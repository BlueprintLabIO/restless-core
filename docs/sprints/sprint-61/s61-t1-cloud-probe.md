# S61-T1 — Probe the two Cloud suspects

**Layer:** Gateway/Cloud. **Serves:** acceptance 3; the "callback may not route" and "hosted
Runtime reaches the gateway" frictions. Done first because a positive finding changes T3 and T5.

On a Cloud test plane with a `_test` company, behind `app.restless.run`:

1. Start a connection sign-in from the cockpit and read the `redirect_uri` it sends to the provider.
   Request that exact URL with a dummy `code` and `state`. Record which service answered (router,
   Fleet, plane) and the status. Expected if the suspicion holds: not the plane's callback handler.
2. From inside the company Runtime, as the `company` user, request the gateway address the launch
   contract hands an actor (`tool_gateway.rs`, `host.docker.internal:{port}/tools/{company}`).
   Record whether it reaches the plane's gateway.

For each that fails, choose the smallest fix, preferring one that adds no router rule. The likely
fix for (1) is a company-scoped callback path. Land it in Core, or file the paired
`restless-cloud` change. Record both observations, before and after, in this ticket.

**Deletes:** nothing.

## Findings (6 October 2026)

Read from Core `main` at `10b9920` and `restless-cloud` `origin/main` at `f816af7`. No signed-in
Cloud session was used. The full sign-in and gateway call on a Cloud plane are acceptance 3 (T5).

**1. Callback route: refuted, no fix needed.** `restless-cloud` `ae1b997` (6 October) added
`/connections/tools/oauth/callback` to the edge router as a `home-plane` route
(`services/edge-router/router.mjs`, with a test in `router.test.mjs`). The worker resolves the plane
from the `restless_home_plane` hint. Fleet sets that hint on every `/account` page with
`SameSite=Lax`, so a provider's top-level GET redirect carries it, and `/` redirects to `/account`
after sign-in. Live, against `https://app.restless.run` with no cookies:

| Request | Observed | Meaning |
|---|---|---|
| `GET /connections/tools/oauth/callback?code=probe&state=probe` | `401 {"error":"no_account_plane",...}` | The router took the home-plane branch, and found no hint |
| `GET /connections/not-a-route` | `404 Not found` (router) | The control: an unrouted reserved path |

Accepted residual: a browser whose 30-day hint expired, and that never passes `/account` before
connecting, gets the 401. Sign-in passes through `/account`, so this needs a stale tab.

**2. Runtime-to-gateway reachability on Cloud: refuted, no fix needed.** Cloud ADR 0006 (4 October)
superseded ADR 0002's supervisor. Each owner's VM runs the plane with `network_mode: host`, the Docker
socket and `RESTLESS_RUNTIME_MODE=local` (`services/vm-provider/plane.mjs`). This is the Linux Core
appliance topology. The tool gateway is merged into the model relay's listener (`model_gateway.rs`
`start_runtime_relay`, bound to `0.0.0.0:7790`). Runtimes reach it as `host.docker.internal`
through `--add-host host.docker.internal:host-gateway` (`runtime.rs`). Any Cloud company whose agents
have made a model call has therefore already reached the same listener.

**3. New: Hosted Runtime mode has no tool gateway, but still hands actors one.** `restlessd`
mounts the gateway only when `!runtime_bridges.is_hosted()` (`crates/restlessd/src/main.rs`). Yet
`exec.rs` and `staff/execution.rs` call `tool_gateway::session_servers` unconditionally, which points
actors at `http://host.docker.internal:7790/tools/{company}`. In the `network` + `hosted` row of
`docs/self-hosted-network-entry.md`, which is still the default when network entry is on and
`RESTLESS_RUNTIME_MODE` is unset, the owner can connect and grant tools, and every actor session is
then given an MCP server that is not served. Cloud is unaffected (it sets `local`).

Disposition: **pending founder decision.** Either the Connections page and `session_servers` should
say plainly that hosted Runtimes cannot use connected tools yet, or, if nothing runs hosted mode since
ADR 0006, the mode itself is a deletion candidate. The smoke covers only `local` placement until this
is decided.

**Disposition of finding 3 (6 October 2026): fixed by refusal.** Hosted mode now gives actor
sessions no tool gateway and their brief no connected tools, and the owner API refuses to add an
app with the reason (`refuse_when_hosted`). Deleting hosted mode was not chosen: it is still the
documented default for network entry without `RESTLESS_RUNTIME_MODE`, and removing it needs its own
look at who runs it. The hosted branch compiles and the local branch is covered by the smoke; the
hosted branch itself has not run, because it needs a network-entry plane with an identity issuer.
