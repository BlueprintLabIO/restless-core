# Sprint 61 — Prove connections hold, in Core and in Cloud

**Status:** draft for founder alignment
**Programme:** company extensibility (connectors, tools, plugins)
**Depends on:** Sprint 57's one gateway (ADR 0014: `connections.rs`, `tool_gateway.rs`,
`owner_tool_connections.rs`), Sprint 55's skills and `scripts/skill-compatibility-smoke`, Sprint 60's
one address, and the `multiplayer-smoke` runner as the precedent for a scripted owner browser

## Outcome

One command tells a founder whether a company can still connect and use tools, and supply itself
with skills and plugins, on the build they are about to ship. It gives the same answer for a local
Core install and for a Cloud plane behind `app.restless.run`.

`scripts/connections-smoke --target core` and `--target cloud` each run against a fresh `_test`
company with no model and no real provider. A scripted owner browser connects a fixture MCP
provider through its real OAuth sign-in. The smoke then grants that provider's tools by class,
calls them through the gateway as an actor would, and drives freeze, disconnect, token refresh,
contract change, restart and container replacement. The result is a pass/fail record per
assertion, written as evidence, and every resource is removed, including after a failure.

This is Tier A of a three-tier ladder. Tier B (nightly live-provider canaries on dedicated test
accounts) and Tier C (agent journeys measuring owner touches) are named next and are out of scope.

**Product hypothesis.** The owner's worry is not missing features, but whether connections work
reliably on both deployments. A deterministic contract smoke, run before every promotion and
release, catches most regressions that would otherwise reach an owner first. It costs no model
spend and needs no third-party account.

## Observed friction

Each item was read on 6 October 2026 from `main` at `30657f5`.

| Friction | Evidence |
|---|---|
| Nothing checks the connection lifecycle end to end | Sprint 57 records unit evidence only: T9 says "The Git clone and Exec's skill step have not run live"; T10, the acceptance runs, is open. No script under `scripts/` exercises `tool_gateway.rs` or `owner_tool_connections.rs` |
| Cloud has never been exercised for connections | Sprint 57's acceptance is Core-only. Sprint 60 moved Cloud to one address after Sprint 57 shipped |
| The OAuth callback may not route under one address (**suspected, unverified**) | `owner_tool_connections.rs` builds `redirect_uri` from the request `Origin` plus the root path `/connections/tools/oauth/callback`. On Cloud the origin is now `https://app.restless.run`, and Sprint 60's router table sends only `/`, `/account/**`, `/auth/**`, `/_fleet/**`, `/<company>/**` and company APIs to planes. A root `/connections/**` path is in none of these. It would match `/<company>/**` with company `connections` |
| How a hosted Runtime reaches the gateway is unproven | `tool_gateway.rs` gives actors `http://host.docker.internal:{port}/tools/{company}`. That holds when the Runtime is a container beside the plane. Cloud ADR 0002 puts Runtimes under a host supervisor and denies planes the Docker socket |
| Self-supply is only partly exercised | `skill-compatibility-smoke` covers `restless skill add` and candidates. Plugin import (`POST /tool-connections/plugins`), the local stdio worker (S57-T5, open) and persistence of user-space CLIs across container replacement have no end-to-end check |
| Every ticket in Sprint 57 still needs its own live run | A shared fixture provider and runner make each later acceptance cheap rather than bespoke |

## Design stance

**Core contract**

- **Test the shipped path, not a test double of it.** The smoke uses the real daemon, real
  Postgres, the released company image, the owner HTTP API and the gateway's MCP endpoint, with a
  real `tool_session` capability. Only the upstream provider is a fixture.
- **The fixture is a real MCP server with real MCP authorization.** It is built on the official MCP
  TypeScript SDK's Streamable HTTP server and its OAuth 2.1 authorization-server support:
  protected-resource metadata, dynamic client registration, PKCE and refresh. Restless never
  special-cases it. If the gateway can connect it, the gateway uses the same code to connect Linear
  or Notion.
- **The owner is a browser, not an API shortcut.** Sign-in begins with a cockpit click in Playwright
  and completes through the provider's redirect to the plane's callback. This is the only way to
  test the callback route that Cloud may have broken.
- **Probe, never guess, applies to the smoke itself.** Each assertion states the input, the expected
  observation and what a broken system would show instead (CLAUDE.md, "A check that happens to pass
  is not evidence"). The token check greps for the exact fixture-issued token strings, never a
  prefix.
- **Same assertions on both targets.** Core and Cloud differ only in how the company is provisioned
  and how the browser enters. If an assertion has to be skipped on one target, the record says so
  and why. It never passes silently.
- **Own every resource.** `_test` company names only. Cleanup runs in a `finally`, and
  `restless-reap --check` must report no new debt at exit.

**Default pattern**

- Run the Core target before `restless-dev promote` and in the release workflow. Run the Cloud
  target after each Cloud release, against a `_test` company on a test plane. Not on every commit:
  building the image is the dominant cost.
- Evidence goes to a new output directory as JSON, one entry per assertion, with the commit, image
  digest and target.

## The fixture provider

`tools/fixture-mcp-provider/` is one small Node service, pinned to the SDK version in its lockfile.
It serves:

| Tool | Annotation | Purpose in the smoke |
|---|---|---|
| `whoami` | `readOnlyHint` | The harmless probe call; returns the signed-in fixture account |
| `list_notes` | `readOnlyHint` | `reads` path and read receipt |
| `send_note {to, body}` | none | `acts` path, with `to` as the party argument; returns a fixture message id |
| `delete_note {id}` | `destructiveHint` | `reserved` path; must wait for owner approval |

It has control endpoints, reachable only by the runner, to:

- change one tool's schema, which drives the contract-digest assertion;
- expire the current access token, which drives refresh;
- revoke the refresh token, which drives reconnect;
- accept one `send_note` and drop its response, which drives the unknown-outcome assertion;
- count how many `send_note` calls actually executed, which checks for duplicates.

The same package runs as a stdio server (`--stdio`, bearer from env) for the local-worker path. It
also ships as a minimal Codex-format plugin bundle under `fixtures/plugin/`: one skill plus an
`.mcp.json` naming the fixture.

## Assertions

The runner records each assertion as pass, fail or skipped-with-reason. **(C)** marks those that
need Cloud-specific provisioning.

1. **Connect.** An owner click on Connect starts sign-in. The provider's consent page approves. The
   callback lands on the plane and the connection becomes *working* with account `fixture-owner`,
   observed by the `whoami` probe. *(Broken would look like: a 404 from the router, a callback page
   error, or a connection stuck in pending.)*
2. **Grant by class.** Granting `list_notes` as `reads`, `send_note` as `acts` (party `to`) and
   `delete_note` as `reserved` is reflected in the connection's grant record with each tool's
   contract digest.
3. **Actor view.** An actor `tool_session` lists exactly `fixture__whoami`, `fixture__list_notes`,
   `fixture__send_note` and `fixture__delete_note`, and no tools from any other connection. This is
   observed from inside the company container, at the address the launch contract gives the actor.
4. **Reads.** `list_notes` returns the fixture's notes and leaves a read receipt.
5. **Acts and first contact.** A `send_note` to a new party creates one owner-approval item and
   does not reach the fixture. Approval lets that send through, and the receipt carries the
   fixture's message id. A second send to the same party needs no approval.
6. **Reserved.** `delete_note` waits for approval of the exact prepared call every time.
7. **Unknown outcome.** With drop-response armed, a `send_note` becomes *unknown*. A retry is
   refused until `restless_reconcile_effect` settles it from a read. The fixture's execution count
   shows exactly one send.
8. **Freeze.** Freeze refuses the next `acts` and `reserved` call at once, with a plain reason.
   Reads keep working.
9. **Contract change.** After the fixture changes `send_note`'s schema, only `send_note` disappears
   from the actor's list. Re-granting restores it.
10. **Refresh.** After the access token is expired, the next call succeeds via host-side refresh
    with no owner action. After the refresh token is revoked, the connection shows that it needs
    sign-in, and one reconnect handoff appears. It does not loop.
11. **No token in the Runtime.** The exact access and refresh token strings the fixture issued
    appear nowhere in the company container's filesystem, environment or process arguments.
12. **Isolation.** A second `_test` company on the same plane sees no connection, grant, tool or
    receipt of the first, and its `tool_session` cannot call the first company's gateway path.
13. **Survives restart.** After a daemon restart and a Runtime container replacement, assertions 3
    and 4 still pass without the owner signing in again.
14. **Disconnect.** Disconnect clears the credential reference and revokes every grant. The actor's
    list is empty, and the fixture sees its token revoked.
15. **Local stdio connection.** The fixture runs as a local command connection on the host-side
    worker, and assertions 2–4 pass for it. *Skipped with reason until S57-T5 lands.*
16. **Plugin import.** Importing `fixtures/plugin` from a Git URL adds its server as a connection
    awaiting grant and asks Exec to add its skill. The runner performs Exec's
    `restless skill add <url>#<path>` itself, because no model runs, and the skill appears as a
    candidate.
17. **Skills.** `skill-compatibility-smoke` runs unchanged as a sub-step. It is reused, not
    re-implemented.
18. **User-space CLI persists.** A pinned CLI installed as the `company` user without root survives
    container replacement and an image upgrade, at the same path and version.
19. **(C) Cloud entry.** Assertions 1–14 and 16–18 run against a `_test` company on a Cloud test
    plane, with the browser entering through `app.restless.run`.
20. **Clean exit.** The `_test` companies, containers, volumes, databases and fixture processes are
    gone. `restless-reap --check` reports no new debt.

## Scope by layer

### Kernel / Authority

No new mechanism. The smoke exercises the grant, effect, approval, freeze and reconcile paths
Sprint 57 built. A fix found by the smoke lands in its owning module.

### Gateway / Engine

- Fix whatever T1 finds about the callback route and Runtime-to-gateway reachability on Cloud. The
  likely shape is a company-scoped callback path (`/<company>/connections/tools/oauth/callback`) so
  the one-address router needs no new rule. T1 decides.
- No test-only branches in `connections.rs` or `tool_gateway.rs`.

### Runtime

- None, beyond confirming the user-space install location that assertion 18 relies on.

### OrgIntel

- None. No model runs in Tier A.

### Owner cockpit

- None. The smoke drives the existing Connections page. A selector the smoke needs becomes an
  accessible name, not a test id.

### Cloud (`restless-cloud`, paired ticket)

- A test plane that the runner can provision a `_test` company on and remove it from, plus a
  public, TLS-terminated address for the fixture provider that the plane can reach.

## Acceptance

1. `scripts/connections-smoke --target core` passes assertions 1–14, 16–18 and 20 on `main`, and
   records 15 as skipped with its reason. Run twice from a cold image build: the second run passes
   without manual cleanup between them.
2. **Each assertion is shown to fail when its guarded behaviour is broken.** For a representative
   set (1, 3, 5, 7, 8, 11, 12), break the behaviour in a throwaway local patch and observe that
   assertion fail. Record which patch broke what. The patches are not committed.
3. `--target cloud` passes the same assertions through `app.restless.run` on a Cloud test plane, or
   T1's finding is fixed first and then it passes.
4. The Core run is a required step in `restless-dev promote` and in the release workflow. It is
   documented in `docs/operations/` with its runtime and cost.
5. Sprint 57's open acceptance items that this fixture can carry (3 for local stdio, 4 for plugin
   import, 7 for deletion) are re-pointed to the smoke rather than run by hand.

## Deletable machinery

- None deleted yet. If the runner needs the daemon, Postgres and company setup that
  `skill-compatibility-smoke` and `multiplayer-smoke` each re-implement, extract it into
  `scripts/lib/` once, and move all three runners onto it. Do not copy it a third time.

## Out of scope (named next)

- **Tier B: live-provider canaries.** Nightly against a `_test` company with dedicated accounts on 4–6
  real services (for example GitHub, Linear, Notion and Stripe in test mode). These verify real
  OAuth, refresh over days and provider drift. A passing canary is dated evidence that the
  Connections page could later show beside a suggestion.
- **Tier C: agent journeys.** A brief that needs a capability the company lacks. Measured: whether
  Exec proposes the connection, owner touches (target: one consent), time, and whether a fresh worker
  reads live data. Three harnesses.
- **CLIs as connections, and a store-like Add flow.** Today CLIs are argv effects and can't be
  added from Connections. Sprint 57 explicitly ruled out a marketplace. Whether one Add flow should
  cover MCP, plugin and CLI (with install, credential and a live probe such as `gh auth status`) is
  a founder decision and the natural home for Sprint 42's application roles. It needs Tier A first,
  so that it lands on a verified substrate.
- **Harness parity under a real model.** Tier A proves what the gateway gives each actor session. It
  does not prove each harness surfaces those tools to its model. That needs Tier C.

## Risks and dispositions

| Risk | Disposition |
|---|---|
| The fixture's OAuth server is more lenient than real providers (for example around redirect URI matching), so Tier A passes where Gmail would fail | **Accepted.** Tier B exists for this. Configure the fixture to require exact redirect matching and PKCE so the common failures still show |
| Cloud runs need a public fixture address and a test plane, both of which cost money and setup | **Accepted.** One small always-off service, started by the runner |
| The smoke grows into an untargeted invariant suite | **Guarded.** New assertions require an observed failure or an owner-visible behaviour, as with the tickets (§16.7) |
| A flaky smoke gets ignored | **Guarded.** A flaky assertion is a bug in the smoke or the product. It is fixed or removed in the same week, never retried into green |
| Making the Core run a required promote step slows promotion | **Accepted.** Image build dominates and is already paid by promote. The smoke itself should take a few minutes |

## Open questions for founders

1. Should the Core run block `restless-dev promote`, or only warn, for the first two weeks while it
   settles?
2. Where should the Cloud test plane live: a permanent `_test` plane, or one provisioned per run?
3. Is the CLI-as-connection / one Add flow question worth a design spike now, in parallel, or only
   after Tier A is green on both targets?

## Tickets

- [ ] T1 ([ticket](sprint-61/s61-t1-cloud-probe.md)) — Probe the two Cloud suspects (callback route
  under one address, Runtime-to-gateway reachability) on a Cloud test plane, and fix or file what
  fails. *Gateway/Cloud*
- [ ] T2 ([ticket](sprint-61/s61-t2-fixture-provider.md)) — The fixture MCP provider: OAuth 2.1 with
  DCR, PKCE and refresh, four tools, control endpoints, stdio mode, plugin bundle. *Tooling*
- [ ] T3 ([ticket](sprint-61/s61-t3-core-smoke.md)) — `scripts/connections-smoke --target core`:
  assertions 1–18 and 20, scripted owner browser, evidence JSON, cleanup, shared setup in
  `scripts/lib/`. *Tooling across all layers*
- [ ] T4 ([ticket](sprint-61/s61-t4-break-each-check.md)) — Show each representative assertion fails
  when broken (acceptance 2). *Verification*
- [ ] T5 ([ticket](sprint-61/s61-t5-cloud-target.md)) — `--target cloud` and the paired
  `restless-cloud` test plane and fixture address. *Cloud*
- [ ] T6 ([ticket](sprint-61/s61-t6-gate.md)) — Wire the Core run into `restless-dev promote` and the
  release workflow, document it, and re-point Sprint 57's carried acceptance items. *Operations*
