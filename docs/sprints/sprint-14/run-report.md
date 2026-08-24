# Sprint 14 — Run report

**Recorded:** 24 August 2026

Sprint 14's implementation tickets are complete. Sprint 12's connected desktop/mobile owner review
remains an explicit, separate release gate and is not represented as complete here.

## S14-T0 — Contract, branch purge and baseline

- Sprint 13's unstarted TypeScript/OIDC feasibility package was removed. Its short archive record
  remains at `docs/sprints/sprint-13.md`; no TypeScript control-plane dependency or service landed.
- The Sprint 14 contract explicitly preserves Rust as the control-plane canon and limits TypeScript
  to the cockpit or a Runtime tool with observed ecosystem value.
- The prior full-suite baseline exposed one stale Staff wording assertion after the S12 operating-rule
  wording wrapped across a line. The assertion was repaired to check the still-current handoff
  contract; no runtime behaviour changed.
- Strict Clippy then exposed two mechanical warnings in the active daemon code (a needless borrow and
  a simplifiable optional-actor predicate). Both were reduced without changing their result.

Current baseline, run against the local scratch database after the completed S14 changes:

```sh
cargo fmt --all -- --check
cargo clippy -p restlessd --bin restlessd -- -D warnings
cargo clippy -p restless-orgintel --all-targets -- -D warnings
RESTLESS_TEST_DATABASE_URL='postgresql:///restless' cargo test --workspace --no-fail-fast -- --nocapture
npm run check --prefix web
git diff --check
```

Observed result: formatting, strict daemon and OrgIntel Clippy, the 149-test Rust workspace, the
Svelte/type check and diff hygiene all passed. Postgres accepted connections first; the host had
143 GiB free.

## S14-T1 — Company metered-turn lane

`SpendLedger` now keeps one in-process semaphore per company for metered API sessions. Exec and Staff
acquire it after provider authentication and before opening ACP; it is released only after final
usage is recorded. The next charged turn therefore reads the current ledger total. Subscription
sessions do not take the lane.

The narrow test below passed, then the complete daemon suite passed:

```sh
cargo test -p restlessd --bin restlessd \
  spend::tests::charged_turns_share_one_company_lane_without_blocking_other_companies \
  -- --exact --nocapture
```

The test holds one company permit, proves a same-company metered turn waits, proves another company's
turn proceeds, releases the first permit and observes the waiting turn proceed. It also proves that a
subscription route does not acquire a charged lane. This is deliberately not a durable reservation or
scheduler. One active provider session may still overshoot before its next usage report.

## S14-T2 — Completed-source review integrity

Owner-work-linked Staff conversations now identify a completed Work's current `Produced` Attempt and
prepare an ordinary detached Git worktree at the Attempt's terminal source commit. Review-only output
goes there, never to the completed source checkout. The prepared location is linked through an
attempt-bound `review_copy` artifact reference labelled supporting evidence rather than a candidate
replacement; preparation failure is reported honestly and leaves the Staff conversation in `/company`.

The ACP Docker command now uses the same requested workdir as its ACP session. The Staff review prompt
also limits that detached location to bounded executable inspection and review-only support output:
no candidate edits, commits, publishing or external effects.

The focused local-Git scenario passed:

```sh
cargo test -p restlessd --bin restlessd \
  staff::workspace::tests::detached_review_copy_keeps_completed_source_commit_and_status_unchanged \
  -- --exact --nocapture
```

It creates a source worktree at a terminal commit, prepares the detached review copy, writes a
review-only file there, and proves the source worktree's commit and Git status are unchanged. The
live-Postgres OrgIntel smoke suite also passed with an attempt-bound `review_copy` reference.

## S14-T3 — Explicit live-Postgres evidence

Added `scripts/verify-orgintel-live-db`. It accepts only the existing local `postgresql:///restless`
convention or local TCP scratch targets, refuses a missing, blank or remote URL before Cargo starts,
and distinguishes this proof from ordinary fast tests.

Observed preflight evidence:

```text
live OrgIntel verification refused: RESTLESS_TEST_DATABASE_URL is required; a fast test run is not live-Postgres evidence
live OrgIntel verification target accepted: postgresql:///restless
```

`scripts/verify-orgintel-live-db --self-test` passed its missing, remote and known-good URL paths
without a database. Its complete local command then passed all 17 OrgIntel unit/integration tests,
including atomic gates, direct/late feedback, recovery, review semantics and handoff scenarios.

## S14-T4 — Daemon transport and Staff supervision modules

The daemon keeps one explicit command dispatcher and the existing flat JSON protocol, but its input
envelope now decodes into `CommonInput`, `LifecycleInput`, `AuthorityInput`, `OrgIntelInput` and
`OwnerInput` groups in `wire.rs`. This removes the 57-field all-optional request shape without
introducing a universal command protocol or a new writer.

`staff.rs` is now the public façade and dispatch/orphan-sweep entrypoint; its implementation is split
into context, conversation, execution, recovery and workspace modules. `dispatch_claimed_work`,
`dispatch_actor_conversation`, `ConversationRuntime` and `sweep_orphans` remain the public entry
points. Direct-message, feedback, terminal observation, orphan recovery and budget-fuse behaviour
were preserved by the existing daemon suite, which passed with 117 daemon tests.

## S14-T5 — OrgIntel ownership modules and deletion

The OrgIntel crate remains one public façade and one Postgres owner, but the former monolithic
implementation is now split into actors, goals/work, attempts, artifacts, review, messages,
schedules, events and shared types. `lib.rs` is 335 lines and re-exports the existing public types;
the old `work.rs` monolith was deleted. No migration or database-schema behaviour changed, and no
repository trait, ORM or second client was added.

The explicit live-Postgres suite above passed atomic claims, feedback cursor/successor, direct
delivery, recovery and review linkage after the move. The unstarted Sprint 13 TypeScript/OIDC
scaffold was also purged: Rust remains the control-plane canon, with TypeScript constrained to the
cockpit or an ordinary Runtime tool where a concrete ecosystem advantage is observed.

## Runtime evidence

`restless up -c sprint10b_office_test --reconcile` rebuilt only the disposable `_test` company's
Runtime image and retained its volume. A subsequent `restless doctor -c sprint10b_office_test`
reported `status: live`: cockpit API and shell returned 200, OrgIntel was available, browser and
desktop were available and unclaimed, desktop services were running, reconciliation was current, and
the host had 142 GiB free.

The read-only `restless doctor -c cosmon` probe found its cockpit/runtime paths available but returned
`status: degraded` because the live company's Runtime image needs reconciliation. It recommended
`restless up -c cosmon --reconcile`; that mutating command was intentionally not run against Cosmon.

## Still open

- **S12-T4 — connected desktop/mobile owner-cockpit review.** This is a release gate from Sprint 12,
  not unclaimed Sprint 14 implementation work. No visual review was substituted with a headless check.
