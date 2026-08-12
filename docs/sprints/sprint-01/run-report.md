# Sprint 01 run report (DRAFT — runs pending)

Ticket: [t15-run-report.md](./t15-run-report.md). This document is being written incrementally as
evidence arrives. Sections marked **PENDING** await the T11–T13 company runs and the T14 harness,
which are blocked on model credit (friction item F1). Everything not so marked is machinery-level
evidence gathered during the build — stated inputs, observed output, no live company run yet.

## Recorded per company

| Company | Elapsed | Dollar cost | Owner interventions |
|---|---|---|---|
| Cosmon | PENDING | PENDING | PENDING |
| Aris | PENDING | PENDING | PENDING |
| Thymelake | PENDING | PENDING | PENDING |

## The questions

- **Did the §4.4 ontology survive three company shapes?** PENDING for companies; machinery note: no
  company-specific vocabulary has entered the schema so far. Companies 2 and 3 were added as
  directories of persona files plus a DB schema — the "cost of adding a company" so far is a schema
  migration, a seed goal, and persona files. The run will price the rest.
- **Did event-driven wakeups fire?** Machinery-level yes: NOTIFY triggers + T6 reconcile were
  observed waking the exec on owner mail (`tell` → wake with `event: mail from owner`) and on
  daemon mail (`event: unread mail waiting (reconciled)`). Exec behaviour over a full run: PENDING.
- **Where did owner attention get pulled in — judgement or missing machinery?** PENDING, with one
  data point already: the owner was pulled in for an OpenRouter credit top-up — genuine external
  authority boundary, correctly escalated, and the system failed honestly (zero fake receipts) while
  blocked. That is the boundary working as designed, not friction — except that the *classification*
  of the failure was wrong (F1).
- **Did file + Git work survive crashes without custody machinery?** Machinery-level partial:
  the T9 orphan sweep was observed blocking a crashed staffer's commitment while preserving its
  worktree (staged crash, real worktree, verified on disk). Mid-turn kill on a live run: PENDING (T14).
- **What did companies 2 and 3 cost to add?** PENDING for the runs; platform cost now *measured*,
  not estimated: **55 seconds of wall time each, zero code** — one `company.toml`, one persona
  directory, `restless up`. Schema, container, volume, and in-container CLI identity all provisioned
  by the existing path and verified live (`docker exec restless-co-aris restless status` →
  `aris: Running`). The only gap found: Thymelake's `email.send` persona was missing against its
  ticket's capability list — a docs-level omission, fixed with one file. Cosmon run-state was also
  verified resume-ready (mid-run commit `62b2ccb` intact on its volume; Playwright acceptance
  harness smoke-tested against the current build: page loads, canvas renders, zero JS errors).
- **Strongest ongoing dogfood?** PENDING.
- **How often did agents fail to report coordination state via the CLI?** PENDING — needs the runs.
- **Did the Exec terminate on its own judgement?** PENDING.

## Negative-claim assessment

Did we have to reinvent the legacy machinery? **So far, no.** Concrete observations:

- Internal coordination (tasks, teams, messages, schedules) lives as ordinary recoverable OrgIntel
  state; the kernel was not given a command algebra. The effect surface (`restless effect`) is one
  function with a receipt, not a universal mutation envelope.
- No append-everything ledger: OrgIntel events are an operational stream; only effect receipts and
  governance-relevant events are treated as durable truth. Nothing needed compaction or repair yet.
- Work is ordinary files in ordinary git worktrees. The orphan-sweep recovery path points the exec
  at the preserved worktree in plain language — no export/import/materialise state machine appeared.
- Supervision is a process registry + deterministic liveness checks. No Work/Attempt/retry
  state machine was needed; resume/reassign is the Exec's judgement by design.
- One watch item for the runs: the `spawn` envelope in the termination output is the closest thing
  to a "universal command" smell. It is deliberately untyped at the exec boundary and validated at
  claim time. If it grows a second verb, that is the moment to re-examine (§12).

## Deletion pass

PENDING the runs (which paths no company exercised is a run-data lookup). Candidates already visible:

- `restless message --from` (agent-to-agent send) — exercised only in channel tests so far.
- **Deleted during the review sweep:** `gateway_dir` helper (`restlessd/src/gateway.rs`) — written
  for "later slices", T4 never used it, zero callers. Deletion is product progress, not backlog.
- T16's `judge!` helper was never built: all three call sites named in its ticket resolved to the
  model judging directly (the agent *is* the judge for termination and staff state; personas *are*
  model calls). The heuristic smell-family grep over the sprint diff comes back clean — every
  `contains`/`starts_with` hit is set membership, input validation, or gateway allowlist matching,
  all genuinely deterministic. **Disposition: keep T16 as the standing rule, not as built code; the
  next daemon-internal judgement call builds it. Building it now would be speculative generality.**

## Friction backlog

Concrete failures observed while doing the sprint's work, in build order. **Fixed** items are
recorded for the diff's archaeology; **open** items are the sprint-02 platform candidates.

- **F1 (partly fixed, remainder open): provider quota/auth failures are misclassified.** When the
  OpenRouter key hit its limit, every exec turn ended as "unparseable termination" — the provider
  402/403 body never reached the exec or the owner as what it was. Worse, the loop: blocked
  outcomes never latched the milestone Blocked, so the 15-minute tick re-woke the company forever,
  re-mailing the owner an identical block each time — **20 identical mails in 3h observed live**.
  Fixed: Blocked now latches the milestone (tick skips it; owner mail still event-wakes it) and
  Continue unlatches (commit `exec.rs` this session). Open remainder, sprint-02 S02-T5: classify
  the provider *error channel* deterministically (HTTP status → quota/auth/rate/unknown) as a
  first-class wake outcome, not a parse failure — frame 2 says that is not a model judgement call.
- **F2 (open): host resource exhaustion is invisible until it breaks the run.** The Mac's disk
  filled mid-sprint (build cache + Docker images): cargo failed, then the harness itself, then
  Docker Desktop hung and needed a full restart. The daemon and Postgres survived — but nothing in
  the product notices "the company computer is out of disk" as a first-class condition. A company
  that cannot write is blocked in exactly the way the exec should hear about plainly. Candidate:
  pre-flight disk/space probe in `restless status` and a daemon-side guard that turns ENOSPC into a
  blocked-with-reason, not a silent stall. **Filled a second time 2026-08-13** (3.4Gi free, 100%),
  caught before it broke anything by a manual `df` probe while the runs are credit-blocked; ~50GiB
  reclaimed from Docker build cache (44.7GiB) and unused images (4.2GiB). The recovery being manual
  is the friction: the cron's auto-resume would have started a company run straight into ENOSPC.
- **F3 (open): company containers do not come back on their own.** Docker Desktop's restart left
  the company containers stopped; nothing reconciles desired-company vs actual-container state at
  daemon boot. The "persistent company computer" currently persists only until a Docker or host
  restart. Candidate: boot-time reconciliation in `restlessd` (it already sweeps orphans there;
  starting stopped companies is the same shape). *Observed twice now: the disk-full incident
  (2026-08-13) stopped `restless-co-cosmon`; it came back only by manual `docker start`. Docker
  Desktop itself needed a hard kill — its VM went catatonic (process alive, zero console output)
  after the first clean relaunch attempt.*
- **F4 (fixed): daemon→exec mail silently dropped.** `messages.from_actor` has an FK to `actors`
  and no "daemon" actor existed; staged-orphan recovery blocked the commitment but the notification
  vanished. Fixed: `mail_exec` ensures the daemon actor and logs failures loudly.
- **F5 (fixed): inspecting the exec's inbox consumed it.** `inbox --as exec` marked the exec's
  owner mail read. Fixed: only the actor's own inbox fetch marks read; inspection is read-only.
  Found because my own test masked a real notification — inspection paths must never mutate.
- **F6 (open, observe in runs): exec mail is never marked read by its recipient.** The exec's
  unread set only grows; reconcile wakes stay bounded by `last_wake`, but the context assembler will
  re-surface ancient mail forever. No platform fix yet — first observe whether the exec handles
  staleness gracefully via the playbook (it can mark read through the CLI itself). If not, this is
  missing machinery, not agent error.
- **F7 (accepted risk, named): company identity on the TCP coordination channel is trusted
  as-sent.** Bind-mounting the host unix socket hangs Docker Desktop containers (probed: container
  I/O freezes), so coordination moved to TCP 7791 with `RESTLESS_COMPANY`/`RESTLESS_ACTOR` env
  identity. Any local process can claim any actor. Expiry: before any real external effect is
  reachable through this channel.
- **F8 (open, small): company image entrypoint swallows args.** `docker run <image> <cmd>` ignores
  `<cmd>` (`exec tini sleep infinity`), so probes need `--entrypoint sh`. Every operator's first
  probe hits this.
- **F9 (open, small): build-time friction — sqlx migration macro staleness.** Adding a migration
  file did not reliably retrigger the compile-time check; stale "no such migration" errors survived
  until a clean. Cost: one confused debugging session. (May be moot after the disk event forced a
  clean build — reconfirm on next migration.)
- **F10 (open, characterize in T11): codex-sandbox permission friction in-container.** The agent's
  in-container sandbox blocked some ordinary tool uses in early probes; worked around ad hoc. Needs
  one real run to characterize before choosing a fix.
- **F12 (open): daemon boot blocks on a hung Docker.** During the disk incident, Docker Desktop
  hung; `restlessd`'s boot-time orphan sweep wedged on a docker call *before binding its socket*, so
  every CLI command hung with no error until Docker was hard-killed. A company computer's runtime
  being down must not take the coordination plane with it. Folds into the S02 boot-reconciliation
  ticket: sweep/reconcile under a bounded timeout, serve requests with runtime state "unknown".
- **F11 (fixed): clap optional-positional panic.** An optional positional before a required one
  panics at parse (debug_asserts); all company args became `-c/--company` with env fallback. Fixed;
  recorded because env-identity (`RESTLESS_COMPANY`) is what makes in-container agent CLI use
  zero-arg, which T10's acceptance depends on.
- **F13 (fixed): boot orphan-sweep detection was vacuous.** The sweep ran `pgrep -f codex-acp`
  inside `sh -c`, whose own cmdline contains the pattern — every boot "found orphans" in every
  running company (**28 false warns observed in one day**) and ran the kill unconditionally.
  Same self-match family as the host-side pkill lesson. Fixed with bracket patterns; verified both
  directions live: a staged fake orphan in cosmon was detected and killed (warn fired for cosmon
  only), and the next boot was the first silent one on record.

## What sprint 02 should take from this

See [../sprint-02.md](../sprint-02.md) (draft). The spine: finish the three company runs and the
crash harness with the same machinery, plus F1/F2/F3 as the platform slice. Sprint 02's spec should
be finalized only after T11–T15 complete — this report is the evidence base it finalizes from.
