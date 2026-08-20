# Sprint 08 run report

**Run date:** 19–20 August 2026  
**Synthetic companies:** `sprint08_test` for boundary probes; `sprint08_graph_test` for the clean
external-sourcing Work run; `sprint08_ui_test` for the representative People/Work owner surface;
`sprint08_bootstrap_test` for a fresh-company lifecycle probe (destroyed after the probe)  
**Current status:** Implementation and local adversarial verification are substantially complete.
T8 is complete. T0 and the real-provider portions of T1–T6 remain open because no Airwallex account, sandbox
credential, onboarding authority, operating-wallet exposure or live transfer was supplied or
authorised. T9/T10 retain a visual-review gap because the configured in-app browser exposed no
browser to the verification client. This report does not turn public documentation, fake-provider
responses or `_test` artifacts into a live-provider claim.

## Architecture and retained boundaries

- `ARCHITECTURE.md` now names the hosted target as cell-based SaaS: a narrow shared cloud control
  plane provisions one strongly isolated cell per company. `docs/CELL_ARCHITECTURE.md` records the
  Core/Cloud operating split without creating two product architectures.
- Missing capability acquisition remains judgement expressed through ordinary Work, Attempts,
  `requires`/`revises` edges, decisions and artifacts. The implemented postures are guidance, not an
  enum or workflow: reuse, do internally, build/automate, buy an input, rent a bounded tool/resource,
  commission a deliverable, delegate a function, partner, hire/internalise now, or internalise later.
- Providers are not OrgIntel Actors and own no Work. One internal actor remains accountable. Spend,
  account installation, credentials, terms and consequential effects still cross Authority or a
  prepared owner handoff.
- Money credentials terminate in the host Authority adapter. The generic Runtime effect path refuses
  finance classes and finance credential bindings.
- Legal/company-safe facts, money envelopes, payment intents and provider observations each have one
  source owner. The cockpit composes them; it does not become another writer.
- A final primary-documentation audit corrected the candidate adapter from Connected Accounts'
  `api-demo` origin to the Business Account sandbox at `https://api.sandbox.airwallex.com`.
  Connection setup now requires the exact account/date API version observed by T0; there is no
  source-code `latest` version. The host reuses an unexpired 30-minute bearer token only in memory,
  refreshes before expiry, and retries exactly once after a known 401. It does not retry transport,
  rate-limit, server or transfer-response ambiguity. Provider calls have a bounded 30-second timeout.
  These assertions were checked against Airwallex's current
  [authentication](https://www.airwallex.com/docs/api/authentication/api_access_token),
  [sandbox](https://www.airwallex.com/docs/developer-tools/sandbox-environment),
  [versioning](https://www.airwallex.com/docs/api/versioning) and
  [Transfers API](https://www.airwallex.com/docs/api/payouts/transfers/api) documentation; they remain
  candidate-contract evidence until T0 observes the selected account.

## External-sourcing run

The first attempted `_test` graph exposed a real scheduling defect. A reviewer was created with a
`revises` edge before its `requires` edge, so it was runnable against no producer artifact. The
repair is now structural:

- initial reviewer `requires` and `revises` edges commit atomically;
- revision power is rejected unless the same producer is already a hard prerequisite;
- graph repair cannot add any dependency after a Work has an Attempt.

The same run exposed a second scheduler defect: the original free-form Exec wake remained live
while the scheduler claimed Exec-owned Work, because conversation wakes and Work Attempts consulted
different busy-actor sets. The scheduler now excludes `exec` from Work claims while its free-form
wake is live. A regression test proves that a ready Exec-owned node remains unclaimed in that state.
This preserves the stronger invariant—one durable actor, one live cognitive process—without adding a
second Work lifecycle. The affected run is retained as defect-discovery evidence; it is not presented
as a clean concurrency proof.

The trial's owner-step recovery exposed a third, narrower race. Creating an attached handoff marked
the Attempt `blocked` even though its supervised ACP process remained live. When that process later
returned `outcome_met`, the terminal Attempt could not accept the result; its self-addressed Exec note
then raced a new Work claim and launched two Exec processes. The repaired contract blocks Work and
successor release while leaving the attached Attempt `running` until the process actually returns.
Actor reservation now happens before the first asynchronous dispatch setup, and an Exec note to
itself remains transcript-only. A database scenario proves one attached Attempt stays running across
the handoff and can complete after the observed response; a scheduler regression proves the self-note
does not wake Exec.

`sprint08_graph_test` then created one clean six-node graph:

```text
frame retained outcome
  ├─ compare ten sourcing postures ─┐
  └─ research Airwallex claims ─────┴─ bounded sourcing decision
                                        → non-live trial
                                        → evaluation (requires + revises trial)
```

All Work is internally owned by `exec`; there is no provider Actor, sourcing status, engagement
table, provider registry or marketplace. The first Runtime Attempt also exposed a repository
precondition: a newly initialised repository had no valid base commit for a Work worktree. Exec
created the initial commit and resumed the same Work rather than adding another workspace path.

The graph completed all six nodes. Each node remained owned by `exec`; the final evaluator had both
the trial artifact as a hard prerequisite and the declared `revises` return edge. Its exact output,
`evaluation.md`, was committed at `ffb4896`, linked as artifact `939415f6` with SHA-256
`cf26afe2…653d`, and integrated into the company repository at `e880746`. The verdict is **RETAIN**:
keep the combination of reused Authority controls, internally built runbook/harness/reconciliation,
and a rented bounded provider candidate, but do not activate it. AC1 and AC4–AC6 were evidenced for
the non-live path; the company half of AC2 was evidenced; provider-native approval and live
observation remain explicitly open. The evaluator independently reproduced both missing-credential
refusals, both fail-closed reservation attempts, the 13/13 fake-provider suite and byte-identical
Authority state before accepting the trial.

The run consumed $5.2702 against its nominal $5 model-spend ceiling. The final Attempt began with
$0.4129 remaining and completed $0.2702 beyond the ceiling; no later turn could start. This is the
documented per-turn metering trade-off in the current spend fuse, not evidence of a hard per-request
financial cap. The company was not poisoned, and no provider or payment spend occurred.

## Legal and finance boundary probes

In `sprint08_test`:

- a deliberately fake company profile was owner-attributed and survived a subsequent read;
- ABN Lookup without its credential recorded `registry_observation.status = unavailable` and
  `detail = "ABN Lookup credential unavailable"`; it did not claim the fake ABN was absent;
- an Airwallex connection carrying `Beneficiaries:Write` was rejected;
- a valid-shaped but explicitly unobserved sandbox connection retained
  `approval_workflow_observed = false`;
- provider probe without a configured read credential failed by naming the missing scoped reference;
- `finance.transfer` through the generic Runtime effect command was rejected;
- a generic effect naming `finance.airwallex.submit` was rejected before a child process ran;
- exact finance sentinels were absent from the test Runtime environment and company volume.

The first envelope command was intentionally sent through the still-running pre-fix daemon and
preserved lowercase `aud`; that output is not accepted as current evidence. After rebuilding and
restarting the daemon, the same lowercase CLI input persisted as canonical `AUD`, survived a read and
froze the exact uppercase envelope. The one synthetic lowercase row produced by the discarded binary
was inspected and removed by its exact company/account/currency/timestamp key. With the envelope
temporarily unfrozen in `sprint08_test`, a 501-minor reservation against the 500-minor limit failed
with `payment exceeds the per-payment envelope`; `finance show` returned no payment row, and the
owner immediately refroze the test envelope.

Two synthetic payment-precondition handoffs also exercised the continuation projection. Before
resolution each appeared as `orgintel:handoff:<uuid>` in Attention. After owner-attributed withdrawal,
the same source reference appeared under `continuations` with the recorded decision, exact Work,
responsible actor and observed current Attempt/Work state. One continuation observed the resumed
trial's Attempt 2 as running; no successor was described as released merely because a handoff ended.
The rebuilt CLI also accepted the documented kebab-case `payment-confirmation` spelling in a third,
immediately withdrawn parser-only `_test` handoff; no provider or payment row was created.

## Automated verification

The actual database-backed command was:

```text
RESTLESS_TEST_DATABASE_URL=postgresql:///restless cargo test --workspace -- --nocapture
```

Observed result: 111 Rust tests passed. This includes live local Postgres scenarios for actor/team
integrity, owner escalation, payment-handoff isolation, graph atomicity and repair, concurrent money
reservation, idempotent replay, unknown-outcome persistence, freeze, current provider-state
correction, handoff/Attempt continuity and daemon-store restart. A prior plain
`cargo test --workspace` pass is not counted as database evidence because those scenarios correctly
reported that the test URL was absent.

The Airwallex fake-provider subset contains seven tests. In addition to scoped authentication,
version, request identity, webhook HMAC and reflected-secret boundaries, it proves that an expired
token is refreshed then reused from process memory and that a provider-rejected bearer triggers
exactly one re-authentication/request replay. No broader failure class is automatically replayed.

The actor/team database scenario now carries one `centre-critique` identity through assigned Work,
revision 1 and revision 2 Attempts, promotion to team lead, replacement back to member, a subsequent
team move and a Work-attributed message. Both Attempts and the retained message still name the exact
same actor id. The same scenario rejects all assignment/retry-shaped ids before partial writes and
proves creation reasons for two genuinely distinct domain/craft identities.

Additional checks:

- `cargo clippy --workspace --all-targets -- -D warnings` — passed;
- `npm ci` — reproduced the cockpit dependency tree from the retained npm lockfile;
- `npm run check` — zero Svelte errors and warnings; raw font-size guard passed;
- `npm run lint` — all matched files formatted;
- `npm run build` — passed, 202 SSR and 227 client modules transformed;
- `npm audit --omit=dev` — zero production vulnerabilities;
- full `npm audit` — three low-severity development-tooling findings in SvelteKit's `cookie`
  dependency; the offered automated fix is a breaking downgrade and was not applied.

## Owner surfaces and purge

- Actor `kind` (`owner`, `exec`, `staff`, `system`) is now separate from durable organisational
  `role`. Historical ids remain unchanged. `world` and `daemon` retain message provenance as
  `system` actors but are filtered from People by kind.
- New Staff ids require exactly `{domain}-{craft}` and reject class, team position, environment,
  stage, revision, retry and implementation suffixes. `staff-copy-critic` and
  `centre-critic-live` were rejected; `finance-analyst` was accepted once and a duplicate creation
  pointed back to reuse. Live Aris history was read without renaming legacy actors: six active Staff
  ids still retain their historical `staff-`/`-live`/assignment-shaped provenance.
- Standing Owner and Exec lifecycle is now established at company creation and daemon startup rather
  than opportunistically by the first `tell`. A brand-new `sprint08_bootstrap_test` returned Owner
  and Exec—including the configured Exec model—before any message or wake, then successfully created
  Staff with Exec attribution. The exact throwaway company was destroyed after the probe.
- People gives the singleton Exec its own first contact treatment, team leads contact weight, and
  members compact inspection rows routed to their lead. The old UI actor-id list, `Not people`, idle
  `READY`, duplicate role subtitles and instruction-shaped focus copy are deleted.
- Attention now shows a payment only while authenticated provider state is `in_approval`. After an
  observed owner step, the same source reference becomes a compact causal continuation backed by
  the actual handoff, Work/Attempt and provider state. It never calls a successor released until the
  predecessor Work is actually complete.
- Work Map and Board consume one `WorkGraphSnapshot`, one Work/Attempt/evidence mapping and one detail
  route. No live/current-step treatment is emitted because Sprint 08 has no trustworthy
  Work-associated process observation with time and source health. The SvelteFlow candidate and its
  package are deleted; Dagre remains the sole layout engine.
  The Work client node fell from approximately 213.73 kB / 70.10 kB gzip to 60.48 kB / 21.13 kB
  gzip in the production build.
- The representative `_test` projection contains seven Work rows: a linear handover, branch,
  fan-in, `revises` return, blocker, disconnected path and two evidence-backed completions. It carries
  seven edges (six `requires`, one `revises`), four Attempts and two artifacts through the same-origin
  API. A direct run of the retained Dagre layout produced seven unique nodes and paths in a
  1198×502 extent with zero node overlaps. Daemon recovery changed the deliberately stale running
  Attempt to `failed` and its Work to `blocked`; neither owner lens presents synthetic live activity.
- The old full-weight People row CSS, unused `talk-closed` path, large-avatar/subtitle rules and the
  invented goal-progress bar are deleted. The cockpit now has one npm command path and lockfile;
  the stale pnpm instructions and lockfile were removed. Root and package documentation no longer
  describe the runnable daemon, CLI or owner surface as empty/unwired stubs.

## Local smoke and external blockers

After the final Attempt exited, `restless up -c sprint08_graph_test --reconcile` rebuilt the isolated
cell onto the current image without losing its company volume. The first immediate doctor saw the
five supervised desktop/browser services in `starting`; ten seconds of observed uptime later all
five were `running`. A second `scripts/restless-dev sprint08_graph_test` reported the complete local
stack `live`: coordinator, OrgIntel, owner gateway, cockpit shell/API, Runtime supervisor, Chromium,
browser broker, desktop and web transport were available; source and image digests matched and no
repair action remained.

Through Vite's same-origin proxy, `/cockpit` and `/attention` both returned HTTP 200. The projections
showed OrgIntel and Authority available, Runtime running, browser available, only `owner` and `exec`
in People (no `world`/`daemon`), the owner-asserted synthetic legal profile with registry status
`unavailable`, one frozen canonical `AUD` envelope, no payments, and all six sourcing nodes completed
with the expected seven edges. Attention had zero outstanding items and two causal continuations;
both retained their original `orgintel:handoff:<uuid>` identity, exact recorded withdrawal, Exec as
responsible actor, and the now-completed evaluation outcome.

Visual browser verification could not be performed in this run because the configured in-app
browser service reported no available browser on both allowed connection attempts. The production
build and complete browser-to-runtime doctor remain headless evidence; they are not represented as
a visual capture or substituted with another automation stack.

The separate `sprint08_ui_test` cell was then reconciled against the final source. Its direct
`restless doctor` result was `live`: coordinator, OrgIntel, owner gateway, cockpit shell/API,
Runtime supervisor, Chromium, browser broker, desktop and web transport were all available; source
and image digests matched and no repair action remained. Same-origin `/cockpit` excluded the two
`system` actors while Postgres retained both rows and three exact `world`/`daemon` provenance
messages. `/attention` returned the seven-node representative graph described above. The visual
browser client remained independently unavailable, so this is API/layout/transport evidence rather
than a claimed visual sign-off.

Real-provider completion is blocked on explicit owner inputs, not code execution:

1. the legal entity and safe display/address fields for the selected live company;
2. permission to create/use the Airwallex sandbox and accept any applicable terms;
3. separate read, submit and webhook credentials with the proved scopes;
4. an observed account-level API-created-transfer approval configuration;
5. owner-set operating-wallet and per-payment exposure;
6. a genuine low-value transfer between owner-controlled accounts or against a real obligation;
7. provider-native owner approval.

Until those exist, T0 cannot freeze the real contract and T6 cannot truthfully supply sandbox IDs,
a live receipt, restart reconciliation against Airwallex, or the provider-native approval capture.
