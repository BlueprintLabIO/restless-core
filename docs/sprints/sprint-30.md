# Sprint 30: accountable supervision without routine coordination tax

**Status:** Complete — all seven slices and the integrated adversarial fixture passed

**Date:** 31 August 2026

**Depends on:** Sprint 26 accepted exact-execution substrate and EXP-17 terminal findings.

**Unblocks:** EXP-18 counted execution and the Dogfood 4 v0.6 autonomous repair campaign.

## Outcome

Restless retains the company hierarchy the founder chose: Exec delegates, a lead remains accountable
and non-producing, and a worker produces. On a nominal bounded task, however, exact policy can commission
the worker and accept passing mechanical evidence without paying for lead narration. The lead remains
hot and receives one model wake when a material event requires judgement.

This is not removal of supervision. It separates supervisory authority from compulsory supervisory
speech.

## Non-negotiable contracts

1. **Exec delegates once.** Exec never claims staff production and returns to availability as soon as
   the accountable delegation is durably assigned.
2. **The lead owns but does not produce.** It may change scope, partitioning, authority or recovery,
   but may not modify candidate artifact bytes.
3. **One coherent owner.** The nominal route assigns one end-to-end worker and one Attempt workspace.
4. **Material wakes only.** Failed or conflicting gates, stalled or lost Attempts, contract ambiguity,
   authority/effect requests, cross-worker conflicts and new owner judgement wake the lead. Progress,
   cached passes and ordinary terminal bookkeeping do not.
5. **Exact settlement identity.** Every charged request has a unique request and Attempt coordinate;
   concurrent turns cannot poison or merge one another's accounting.
6. **Reviewable custody.** Immutable review artifacts are read-only, content-addressed and demonstrably
   readable by the reviewer identity before review spend.
7. **Terminal cleanup.** Every controller-owned process, lease, marker and transient artifact has a
   terminal owner and a verified deletion receipt.

## Work slices

### S30-T1: nominal single-worker route

Add an explicit producing-topology decision to accountable Work. For the accepted bounded route, the
substrate creates one worker commission from the exact owner/lead mandate without a paid lead
paraphrase. The lead can inspect, interrupt or supersede it and remains responsible for its terminal
account.

Acceptance:

- one owner request produces one Exec delegation, one accountable lead and one worker Attempt;
- Exec has no production claim and the lead changes no artifact bytes;
- a passing exact gate reaches a reviewable terminal outcome with zero paid lead turns; and
- the terminal record truthfully names both the worker producer and accountable lead; the nominal path
  invents no lead-authored narrative when no lead turn occurred.

### S30-T2: material-event wake policy

Represent the small set of events that require judgement. Coalesce causally related events while a
lead wake is queued or active, and preserve late material feedback as one successor obligation.

Acceptance:

- 100 nonterminal progress events cause zero lead model wakes;
- two failures from one causal gate episode cause one wake;
- ambiguity, effect authority, owner correction and cross-worker conflict each cause one prompt wake;
- repeated delivery is idempotent; and
- no event disappears merely because the lead or daemon was temporarily offline.

### S30-T3: concurrent request accounting

Move spend settlement from actor-wide inference to an atomic request/Attempt identity. Retain the
company envelope and model admission checks.

Acceptance:

- four concurrent charged turns by one actor settle independently and exactly once;
- one missing or corrupt terminal usage poisons only its own request;
- the other three requests remain attributable and spend-correct;
- cancellation, retry and resumed sessions cannot double charge; and
- per-request totals reconcile exactly to actor, Work and company totals.

### S30-T4: review custody primitive

Publish an immutable candidate with normalized traversal/read permissions and perform a reviewer-
identity access probe before paid review begins.

Acceptance:

- directories are immutable and traversable; files are immutable and readable;
- the producer cannot change the published bytes;
- the reviewer can read every declared file and no undeclared source or arm label;
- a digest mismatch or unreadable file stops before model spend; and
- reuse of one human-readable label for different content is refused.

### S30-T5: decision telemetry

Export elapsed and active time, tokens, cached tokens, spend, tool failures, gate executions/cache hits,
supervisor wakes/interventions, duplicated work and process replacements directly from Runtime facts.
Missing telemetry remains `unknown`, never zero.

### S30-T6: terminal residue contract

Give all evaluator and productive subprocesses a scoped owner. Reap on completion, cancellation,
daemon restart and company destruction. Audit named temp directories, PID markers, leases and ports.

### S30-T7: integrated adversarial acceptance

Run the preceding slices together against one bounded coherent task and one synthetic parallel task.
Inject a passing path, gate failure, ambiguous contract, concurrent metering corruption, unreadable
candidate, worker process death and daemon restart.

The integrated fixture passes only if the right lead wakes occur, unaffected work continues, evidence
remains exact and terminal cleanup reports zero residue.

## Deletion targets

- mandatory lead commissioning and acceptance narration for nominal bounded work;
- actor-wide settlement identity for concurrent charged turns;
- review bundles whose filesystem ownership is trusted without a reviewer probe;
- experiment-specific PID cleanup and fixed gateway-port assumptions; and
- progress polling used to simulate supervision.

## Evidence

Retain source tests, one compact integrated receipt and the accepted architecture text. Do not retain
raw model homes, transcripts containing credentials, screenshots, temporary review bundles or runtime
process state.

## Activation record — 31 August 2026

The founder approved the sprint and the first vertical slice is implemented and source-verified:

- accountable Work now records its producing topology, exact producer, commissioner and accountable
  lead;
- Exec may take the zero-paraphrase route only for `coherent_single_worker` Work when the accountable
  team has exactly one active non-lead worker; ambiguous or parallel choices still require the lead;
- clean gated completion remains observable without a paid lead wake, while blocked, failed and
  abandoned Attempts coalesce into one material Work-scoped supervisor obligation;
- direct owner correction creates one lead control notice without changing the worker's exact
  production input; and
- source fixtures prove one unambiguous Exec-routed worker, zero wakes across 100 routine progress
  events plus a clean completion, one coalesced wake for related failures, and idempotent delivery.

This activation record is retained as history. The completion record below supersedes its open-work
statement.

Verification at activation: `cargo check --workspace`, `cargo test --workspace` (233 daemon tests
passed; eight live/external tests intentionally ignored), TypeScript binding drift, formatting and
diff hygiene.

## Terminal decision

- **Pass:** all seven slices and the integrated fixture pass from one source state, with zero nominal
  lead turns and correct material wakes.
- **Revise once:** a bounded substrate defect invalidates the fixture and is repaired without changing
  the role or evidence contracts.
- **Stop negative:** the nominal zero-turn route cannot retain lead accountability or exact terminal
  truth. Record the contradiction before changing the organisational principle.

## Completion record — 31 August 2026

All seven slices passed from the current source state. The compact evidence receipt is
[`sprint-30/final-evidence.md`](./sprint-30/final-evidence.md).

- The counted nominal route preserved one Exec commission, one accountable non-producing lead and one
  exact producer while recording zero lead requests or ceremonial wake facts.
- The material-event matrix proved silence across 100 routine progress facts, one coalesced obligation
  for related failures, and one durable lead obligation for ambiguity, effect authority, cross-worker
  conflict and owner correction, including reopen/restart delivery.
- Four same-actor model requests retained request, session, responsibility, Work and Attempt identity;
  one missing terminal remained request-local while three exact siblings reconciled and exact replay
  was idempotent.
- Review evidence is a root-owned, content-addressed detached Git worktree. The reviewer-identity probe
  verifies its exact commit/tree, alias, traversal, readability and non-writability before model
  dispatch. A deliberately unreadable declared file was refused.
- `restless telemetry` now projects Attempts, gates/cache hits, exact model settlement, lead wakes and
  interventions, duplicate Work and process replacements. Active model time, tool-call failure facts
  and provider-absent token splits remain explicit `null`/unknown rather than false zeroes.
- Attempt, gate and agent-session cleanup now verifies path/process absence. The live fixture killed an
  owned session, reaped an orphaned evaluator after a simulated restart and reported no Attempt paths
  or live resource leases.
- The integrated coherent and parallel fixture passed its positive and adversarial arms with zero
  terminal residue.

EXP-18 and Dogfood 4 v0.6 are deliberately not hidden inside this implementation sprint. Sprint 30 makes
their counted execution trustworthy; they are the next empirical tests.
