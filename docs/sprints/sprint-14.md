# Sprint 14 — Pre-release Rust consolidation

**Status:** Code complete — S14-T0 through T5 are verified. Sprint 12's separate connected
desktop/mobile owner-surface review remains the only release gate.

**Date:** 24 August 2026

**Depends on:** Sprint 12's green implementation baseline and real Cosmon outcome. Its one remaining
owner-surface desktop/mobile visual sign-off stays a separate closing gate; this sprint must not
rewrite that surface before it is reviewed.

**Spec refs:** ARCHITECTURE.md §2.1–§2.7, §3.2–§3.4, §4.4–§4.7, §5, §9 and §16;
authority-plane §1–§2, §6, §12–§13 and §16; orgintel §4.5, §6–§8 and §11;
company-runtime §2–§5 and §9; cross-layer-contract §3–§4 and §8; and
evaluation-dogfood §2, §7–§10 and §25.

---

## Founders’ decision

Rust is Restless’s canonical control-plane language:

- `restlessd`, OrgIntel, Authority, model metering, credential handling, recovery and the owner CLI
  remain Rust;
- TypeScript remains the cockpit implementation and may be used for ordinary, replaceable Runtime
  tools where a concrete ecosystem advantage is observed;
- the Company Runtime stays polyglot for productive work.

Sprint 14 does **not** compare languages or migrate a durable plane. It uses the pre-release window to
break internal Rust interfaces, delete stale paths, and establish clean module boundaries while the
real S12 behaviour is still fresh and executable.

## Observed friction

Sprint 12 produced a real bounded Cosmon outcome and a full 145-test local workspace verification.
It also exposed four facts that are now strong enough to act on:

1. A metered Staff session now has an in-turn fuse, but simultaneous metered sessions each calculate
   their remaining budget from the same pre-turn total. They can oversubscribe one company ceiling.
2. A post-completion factual wake regenerated tracked screenshots inside an already-clean completed
   worktree. The linked candidate remains valid, but review activity must not silently change its
   evidence surface.
3. Live-Postgres OrgIntel scenarios return successfully when `RESTLESS_TEST_DATABASE_URL` is absent.
   An ordinary green test command can therefore omit atomic-claim and recovery evidence.
4. The S12 slice increased the cost of safe changes at the exact boundaries we now understand:
   `staff.rs` is 2,520 lines, OrgIntel’s one façade is 4,583 lines with 224 raw queries, and the
   daemon wire `Request` has 57 optional fields inside a 2,958-line `main.rs`.

These are not a rationale for an abstraction programme. They are observed safety, evidence, and
changeability failures.

## Outcome

Restless has a safer, smaller-to-change Rust core:

~~~text
metered Exec/Staff turn
  → one company-level metered-turn lane
  → existing per-session dollar fuse
  → final durable spend record

completed Attempt
  → exact source remains preserved
  → review runs in a prepared review copy or out-of-worktree evidence location
  → ReviewTarget remains linked to the original Attempt

developer verification
  → explicit live-Postgres command refuses to claim DB evidence without its URL
  → focused behavioural checks protect move-only refactors
~~~

The sprint then moves, rather than redesigns, the daemon transport, Staff supervision and OrgIntel
implementation into ownership-sized Rust modules. Their externally observed contracts stay the same.

## Value decision

> **Be aggressive about code shape and deletion before release; remain conservative about behaviour,
> authority and evidence.**

A clean module boundary is useful only if a real product outcome still runs. A newly invented
repository layer, generic command algebra, workflow engine, provider registry or cross-language
service is not consolidation.

## Success contract

1. **One metered envelope at a time.** For a company, a second metered ACP session cannot begin its
   provider turn while another holds the company metered-turn lane. Exec and Staff share that lane.
   Subscription sessions remain outside it because their authoritative charged cost is zero.
2. **Known bounded overshoot.** The existing cumulative in-turn fuse still applies. The lane reduces
   concurrent overshoot to the one active metered session rather than multiplying it by concurrent
   turns. It is not misrepresented as a durable reservation system.
3. **No post-completion source mutation.** A coordination/review wake that needs executable checks uses
   a prepared review copy or an explicit external evidence directory. Its source Attempt worktree and
   recorded commit remain unchanged; unavailable preparation is an honest blocked review, not a hidden
   repair.
4. **Live database evidence is explicit.** One checked-in verification entry point requires a scratch
   `RESTLESS_TEST_DATABASE_URL` and fails before tests if it is missing or not a scratch/local target.
   Fast unit runs remain available but are never described as live-Postgres verification.
5. **No coordination semantics change during refactors.** Atomic claims, direct messages, late feedback,
   recovery, output linking, process observation and owner handoffs retain their current behavioural
   scenarios. Refactors add no new Work state, message kind, durable workflow, repository abstraction
   or second writer.
6. **Transport is decomposed by domain.** Lifecycle, Authority, OrgIntel and owner inputs decode through
   small domain handlers/types instead of a monolithic all-optional `Request`. This is not a universal
   `Command` protocol.
7. **Staff and OrgIntel become navigable.** Move-only modules separate their observed ownership seams
   while preserving their current public Rust façade and migrations. A call-site must not need to know
   a new persistence abstraction.
8. **Dead S12 paths are deleted.** Compatibility instructions, helpers or duplicate fields that no
   longer serve a current scenario are removed in the same ticket that supersedes them.
9. **Verification is real.** Each completed ticket runs its named headless check. Sprint exit reruns
   formatting, strict Clippy for touched targets, the required live-Postgres suite, the full workspace
   suite, web checks, and a real `restless doctor` probe. The S12 desktop/mobile visual review remains
   separately honest if no connected browser is available.

## Layer slices and ownership

| Concern | Owner | Sprint 14 work |
| --- | --- | --- |
| Company spend ceiling and metered-model concurrency | Authority / model metering | Add one shared company metered-turn lane; no new policy language or durable reservation lifecycle |
| Work, Attempt, messages, recovery and schema | Rust OrgIntel | Preserve behavioural semantics; split the implementation by ownership |
| Review copy, worktree and native evidence preparation | Company Runtime / Runtime Bridge | Keep completed source immutable while preparing executable review evidence |
| Socket/CLI request decoding | `restlessd` transport | Split request parsing and dispatch by domain without creating a universal command |
| Owner projection | Owner gateway/cockpit | No redesign; retain S12-T4 visual review as a separate gate |
| Runtime languages | Company Runtime | No platform language migration; ordinary tools remain language-appropriate |

## Problem classification

**Deterministic and enumerable:** company-level metered session admission, in-turn cost fuse,
source-vs-review-copy coordinates, scratch-database preflight, wire input decoding and module
ownership boundaries.

**Judgement and open-ended:** whether a review target represents the outcome, whether a model session
is worth waiting for, the best repair or staffing decision, and whether a native result is good enough.

The sprint enforces the former and preserves the lead/owner’s role in the latter.

## Risks and dispositions

| Risk | Disposition | Why |
| --- | --- | --- |
| Refactor reintroduces a S12 recovery regression | **Guarded** | Preserve/refire live behavioural scenarios before and after each move |
| A concurrency lane becomes a workflow scheduler | **Invariant** | It gates only charged provider turns, has no Work state, queue or retry policy |
| A review copy becomes asset custody | **Invariant** | It is an ordinary Git/worktree or output path chosen for one Attempt; no import/export lifecycle |
| A hard ceiling is claimed perfect despite provider reporting delay | **Accepted** | One active turn can still overshoot between usage snapshots; report the bound honestly |
| The verification wrapper becomes generic CI machinery | **Invariant** | One shell entry point for the live OrgIntel suite only |
| Broad moves hide behaviour change | **Guarded** | Move-only tickets; no new data model and focused behavioural evidence |
| S12 owner review is buried under backend cleanup | **Invariant** | T4 stays listed as an explicit remaining release gate |
| TypeScript scaffolding quietly becomes a platform alternative | **Invariant** | Purge the unstarted S13 protocol spike; no TS control-plane dependency lands |

## Non-goals

- rewriting OrgIntel, Authority, model metering or the daemon in TypeScript;
- a generic provider adapter, OAuth framework, plugin system, workflow engine, queue, policy language
  or repository layer;
- a persistent spend reservation table, new budget category, allocation market or model scheduler;
- a new custody, artifact or review lifecycle;
- changing the owner cockpit visual identity or treating a headless check as S12-T4 visual sign-off;
- broad schema redesign or historical-data rewrite; and
- retaining duplicate pre- and post-refactor paths for theoretical flexibility.

## Tickets

Ticket status lives only in this checklist.

| Status | Ticket | Slice | Observed friction served | Prior machinery made deletable |
| --- | --- | --- | --- | --- |
| [x] | [**S14-T0 · Freeze hardening contract and verification baseline**](sprint-14/t00-contract-and-baseline.md) | Cross-layer + evaluation | S12 evidence must constrain aggressive changes rather than become prose | Unstarted S13 TypeScript/OIDC scaffold and ambiguous “green” claims |
| [x] | [**S14-T1 · Serialize charged model turns per company**](sprint-14/t01-metered-turn-lane.md) | Authority / model metering | Concurrent metered ACP sessions oversubscribe one ceiling | Independent per-session remaining-budget snapshots |
| [x] | [**S14-T2 · Preserve completed source while preparing review evidence**](sprint-14/t02-review-evidence-integrity.md) | Runtime + OrgIntel | A post-completion factual wake changed tracked review screenshots | Uncontrolled review work in a completed Attempt’s source worktree |
| [x] | [**S14-T3 · Make live-Postgres evidence an explicit command**](sprint-14/t03-live-db-verification.md) | Evaluation + OrgIntel | Scenarios silently skip when the database URL is absent | Accidental use of a fast unit run as database/recovery proof |
| [x] | [**S14-T4 · Split daemon transport and Staff supervision by ownership**](sprint-14/t04-daemon-and-staff-modules.md) | Daemon + Runtime Bridge | 57 optional fields and 2,520 Staff lines make S12 behaviour expensive to change | God-shaped socket input and mixed supervision concerns |
| [x] | [**S14-T5 · Split OrgIntel internally and delete settled compatibility paths**](sprint-14/t05-orgintel-modules-and-deletion.md) | OrgIntel | One 4,583-line façade obscures recovery and atomic-claim ownership | Monolithic implementation layout and redundant S12 helpers |
| [ ] | **S12-T4 release gate · Connected desktop/mobile cockpit review** | Owner cockpit | S12’s outcome projection remains unreviewed in a connected browser | No code change is authorised here; this is preserved release evidence |

Existing code, a successful module move or a green unit suite does not close a ticket. Each ticket
closes on its named behavioural evidence.

## Verification and evidence package

The completed sprint contains:

1. a before/after statement of S12 behavioural commands and observed output;
2. a concurrency control proving one charged company turn waits while another holds the lane, then
   proceeds after release;
3. a source/review-copy control proving a completed source commit/worktree remains unchanged after
   review preparation;
4. the explicit live-Postgres preflight failure and passing scratch-database run;
5. focused daemon/Staff/OrgIntel scenario results after each module move;
6. the full workspace, format, strict-Clippy and web-check output;
7. a real Runtime/doctor probe with the exact company and result; and
8. a deletion record naming purged scaffolding, compatibility helpers and no-longer-used code.

## Entry, stop and exit gates

**Entry:** S12’s current source/image behaviour is documented; the worktree remains preserved; the
sprint does not claim the unconnected cockpit visual review is complete.

**Stop:** pause for founder direction if a slice requires a new durable budget/reservation entity,
direct database access from a Runtime tool, a new review/custody state machine, model-scheduling
policy, owner-facing administrator UI, or a TypeScript control-plane service.

**Exit:** the core modules are smaller and behaviourally verified; the three observed hardening gaps
are closed or explicitly evidenced as remaining; S13’s unstarted TypeScript/OIDC spike is purged; and
S12-T4 remains visibly open until a connected browser checks it.
