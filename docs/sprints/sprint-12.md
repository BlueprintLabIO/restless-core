# Sprint 12 — Recoverable natural-team execution

**Status:** Implementation evidenced on 24 August 2026; owner-surface connected-browser visual
sign-off remains open.

**Date:** 23 August 2026

**Depends on:** Sprint 11's controlled ACP session, closed wake/Attempt recovery and event-driven
waiting substrate. If those contracts are not integrated, this sprint is rebased rather than
duplicating them.

**Spec refs:** `ARCHITECTURE.md` §2.1 / §4.4 / §4.5 / §6 / §9 / §16,
`orgintel` §6.1–§6.3 and §10,
`company-runtime` process lifecycle and recovery,
`owner-cockpit` executive availability and outcome review,
`cross-layer-contract` source ownership,
`evaluation-dogfood` real-company evidence rules

---

## Observed product gap

Restless now has the right organisational principle: every executable owner request goes through the
continuously available Exec to exactly one accountable, non-producing lead; that lead commissions one
end-to-end worker by default and adds only differentiated Staff. Natural accountable teamwork is
already the architectural contract.

The remaining product risk is more ordinary and more damaging:

> Useful Staff work can exist in a file, commit or running process while the organisation loses its
> meaning, waits for ceremonial callback text, restarts it, duplicates it or cannot present it to the
> accountable lead for judgement.

Today the lead can express a natural commission, but assembling the exact repository, role, Attempt,
upstream artifacts and available capabilities still requires avoidable ceremony. Questions and
material interface changes can arrive as ordinary messages without reliably resuming the actor who
must decide. A process may exit after producing a useful artifact but before reporting semantically.
The facts needed for recovery exist, yet the lead may have to reconstruct them manually or rerun the
work.

Adding more team topology, handoff schemas or plan states would make this worse. The product needs a
small factual membrane around free-form collaboration.

## Value decision

> **Keep the Work graph, but keep it sparse. Make one context-aware commission create the real
> cross-actor responsibility and attach the facts Restless already knows. Let actors communicate in
> ordinary language. Observe processes and artifacts directly, wake only the actor whose judgement is
> required, and preserve an uncertain result for review instead of guessing or rerunning it.**

This sprint does not automate team judgement. It makes the lead's judgement cheap to enact and makes
the resulting work difficult to lose.

## Relationship to EXP-02

Sprint 12 commits to the product facts: attributable commission, addressed delivery, process/artifact
observation, uncertain-outcome recovery and accountable native review. It does **not** preselect an
optional context capsule, shared-history fork, event-summary format or recovery presentation.

EXP-02 may identify a cheaper lead-facing implementation shape before the affected Sprint 12 tickets
are frozen. A winning shape can be used; a losing or inconclusive experiment leaves Sprint 12 on the
smallest ordinary Work/message/artifact interface that satisfies this contract. Experimental results
may simplify the implementation but cannot weaken its attribution or recovery invariants.

## Outcome

An owner can send two independent requests close together. Exec frames each request, appoints one
accountable lead and ends each dispatch wake without becoming a departmental producer. Both leads
continue independently while Exec remains available.

One lead deliberately works alone. The other uses a natural free-form commission to give one Staff
member a stable, independently useful responsibility. Restless automatically binds the exact actor,
role, workspace, Work/Attempt, upstream artifact versions, capability hints and expected proof that
already exist in company state. Staff can ask a consequential question or announce an interface
change; that event wakes the accountable lead without polling or routing through Exec.

Staff produces an inspectable artifact. In the recovery case, the Staff process exits before its
semantic completion report. Restless does not manufacture success or failure and does not launch a
duplicate Attempt. It surfaces the observed process and artifact evidence to the lead, who inspects
the same candidate, accepts it, requests revision or abandons it. The lead integrates the whole
outcome and prepares the best native ReviewTarget. The owner sees the result and evidence rather than
the team machinery.

## Success contract

Sprint 12 passes only when all of the following are observed:

1. **Exec remains the dispatcher.** Every executable owner request receives exactly one accountable
   lead before productive work starts. After framing and dispatch, Exec quiesces and can accept the
   next request while departments continue.
2. **One lead owns one outcome.** Delegation never creates a second integrator or leaves Exec as the
   hidden production owner. The lead owns scope, staffing, integration, native review and completion
   judgement.
3. **Team size remains intelligent judgement.** In the same product path, a lead may choose zero
   Staff or commission Staff. No rules engine infers team size from file count, task label, estimated
   duration or token budget.
4. **A commission is natural but attributable.** The lead communicates purpose, current
   understanding, material unknowns, a stable responsibility and observable proof in ordinary
   language. One commission creates the corresponding Work; the scheduler atomically claims it and
   records all initial Attempt inputs before Staff starts.
5. **Known context is attached, not retyped.** Actor role, company doctrine, repository/base/worktree,
   upstream artifact versions, Work-linked feedback, relevant capability/skill locations and exact
   acceptance evidence come from authoritative state. The lead may override or add context without
   completing a form.
6. **Messages carry changed information.** A material question, blocker, interface change, review
   request or result is linked to the relevant Work and wakes the actor able to act. Narration and
   presence do not create mandatory events or wake cycles.
7. **Exec is not a message bus.** Lead–Staff communication is direct. Exec wakes only for portfolio
   priority, cross-department conflict, company-wide resource allocation or a genuine escalation.
8. **Runtime facts are observed.** The supervising layer records process start, liveness and terminal
   observation, and can probe a declared file, Git commit, URL or native target without importing it
   into a custody system.
9. **Missing semantic completion stays uncertain.** A terminal process with incomplete semantic
   reporting does not imply success or failure. Existing artifacts and logs remain attached to the
   Attempt and are presented for accountable review.
10. **Useful work is never discarded because coordination is stale.** Restart or repair preserves the
    workspace and artifact. A stale message, Work status or callback cannot invalidate the productive
    result.
11. **Recovery does not duplicate work.** Daemon, Runtime or actor restart reconciles the existing
    Attempt and process/artifact observations before another Attempt may start. At-least-once internal
    delivery is tolerated without duplicate execution.
12. **Review is outcome-native.** The lead opens or runs the exact candidate, integrates or rejects it
    on observed evidence and prepares a live site, playable build, rendered document or other native
    target where possible. Status text is supporting evidence.
13. **Attribution is truthful.** The owner and systematic learning distinguish solo lead work,
    accepted Staff contribution, revised Staff contribution and rejected contribution using real
    Work/Attempt/artifact evidence. Prose cannot invent a team contribution.
14. **The owner surface stays calm.** The owner sees accountable outcome, current risk, decisions and
    prepared last mile. Role prompts, message traffic, process details and Work internals remain
    inspectable on request rather than becoming a team-management dashboard.
15. **The path survives a real run.** After deterministic `_test` failure/restart scenarios, one
    founder-selected, low-consequence real company outcome completes with a real model, real Runtime
    tools and an inspectable native result. Simulated capabilities do not enter that company's
    evidence.

## Product path

```text
owner request
  → Exec frames and appoints accountable lead
  → Exec quiesces
  → lead forms causal understanding
      ├─ works alone
      └─ naturally commissions a stable Staff responsibility
           → Work + Attempt bind known factual context
           → direct material messages wake the right actor
           → Runtime observes process and artifact facts
           → missing callback remains recoverable, not failed
  → lead inspects, integrates and judges the whole outcome
  → native ReviewTarget reaches the owner
```

The branch is a lead decision, not a deterministic workflow branch stored by OrgIntel.

## Layer slices and ownership

| Concern | Authoritative owner | Sprint 12 responsibility |
| --- | --- | --- |
| Lead responsibility, sparse Work/Attempt, immutable inputs, messages, artifact references and wakes | OrgIntel | Make a natural commission attributable and recoverable without modelling the lead's plan |
| Process liveness, worktree, files, Git and native target probes | Company Runtime | Observe productive reality and preserve it across process/restart failure |
| Actor launch and completion envelope | Runtime bridge / ACP adapter | Deliver bound context and terminal observations without making ACP the company ontology |
| Consequential external effects | Authority Plane | Unchanged; receives a request only if the completed outcome needs an effect |
| Executive availability and native owner review | Owner gateway/cockpit | Show department ownership and outcome without exposing an agent-admin dashboard |
| Staffing, communication, recovery and acceptance judgement | Accountable lead intelligence | Decide freely within the factual substrate above |

## Problem classification

**Deterministic and enumerable:** exactly one accountable lead, Work/Attempt creation order, actor and
workspace coordinates, input fingerprints, process observation, artifact existence, message
delivery/deduplication, wake closure, restart reconciliation and effect receipts.

**Judgement and open-ended:** causal understanding, whether a colleague can add value, how to phrase a
commission, whether information is material, whether an artifact is good, when to revise and which
native review target best represents the outcome.

The sprint must not use prompting to repair missing process facts or static policy to replace lead
judgement.

## Acceptance scenarios

### A. Two departments, one available Exec

In an isolated `_test` company, send two executable owner requests while the first lead is still
working. Observe two distinct accountable leads, no Exec production Attempt and a bounded dispatch
turn for each request. A third owner question is accepted while both leads remain active.

### B. Correct solo choice

Give one lead a tightly coupled outcome below its credible saturation point. The lead chooses zero
Staff, completes the native artifact and incurs no synthetic Work nodes merely to satisfy a team
shape.

### C. Natural one-Staff commission

Give another lead a broad outcome with one stable, independently useful seam. The commission text is
free-form; Restless binds the known factual context. Staff produces a real artifact, the lead performs
complementary work and the final result shows whether the contribution was accepted, revised or
rejected.

### D. Material mid-work communication

Introduce one interface fact that invalidates part of the original commission. Staff sends a linked
question or changed-interface message. The lead wakes once, answers or revises the responsibility and
returns to work. No timer, fixed stand-up, Exec relay or full transcript replay is required.

### E. Artifact survives a missing callback

Terminate the Staff cognitive process after it writes the declared artifact/commit but before a
semantic result. Restart OrgIntel and the Runtime in separate runs. Observe:

- the original Attempt remains the attribution boundary;
- the workspace and exact artifact are preserved;
- outcome is not inferred from elapsed time or process exit alone;
- the lead receives the observed evidence and can review the same candidate; and
- no duplicate Staff process or consequential effect begins during reconciliation.

### F. Real outcome

Repeat the winning path on one low-consequence real company outcome selected during ticket alignment.
The pass artifact is the native result plus exact Work/Attempt/process/artifact evidence, elapsed time,
model usage, owner interventions and unresolved risk.

## Measurements

- Exec dispatch duration and time available while leads work;
- time to first useful artifact and accepted native outcome;
- lead/Staff active time, overlap and blocked time;
- owner interventions and owner waiting;
- number of messages that changed a brief, interface, decision or artifact;
- duplicate discovery, duplicate implementation and duplicate Attempts;
- Staff artifact accepted unchanged, revised or rejected;
- recovery time from missing callback and from each restart;
- token/tool/runtime cost, including recovery overhead;
- native outcome quality and regressions.

Counts diagnose behaviour; none becomes a synthetic success score. Accepted outcome quality and
truthful recovery are gates.

## Risks and dispositions

| Risk | Disposition | Why |
| --- | --- | --- |
| A lead chooses a poor team size | **Accepted** | It is recoverable judgement; EXP-02 will develop advisory evidence |
| Free-form commissions vary in quality | **Accepted** | Variation is part of intelligent work; outcome evidence exposes it |
| Initial context becomes too large | **Guarded** | Attach authoritative coordinates and retrieve depth on demand; measure replay and unused context |
| Material-message classification becomes a hidden workflow | **Guarded** | Actors choose meaning; the substrate only delivers explicitly addressed facts |
| A process exits after producing useful work | **Guarded** | Preserve the Attempt, workspace and observations for lead review |
| Process death is mistaken for semantic failure | **Invariant** | Terminal process observation and outcome judgement remain separate facts |
| Restart launches duplicate work | **Invariant** | Reconcile the existing Attempt before claim/retry |
| A message manufactures a contribution | **Invariant** | Staff credit requires its Work, Attempt and observed artifact/terminal result |
| Owner cockpit becomes agent administration | **Guarded** | Outcome-first acceptance and progressive disclosure remain required |
| Sprint 12 rebuilds Sprint 11 substrate | **Invariant** | Reuse or rebase; one concept retains one owner |

## Non-goals

- a deterministic team-size router, task classifier or topology recommender;
- a handoff form, universal brief schema or fixed communication cadence;
- a shared blackboard, common-room protocol or second plan database;
- a capability ontology, employee score, automated performance ranking or hiring market;
- persistent full shared transcripts for every actor;
- semantic timeouts that decide Work outcome;
- exact-once internal messaging;
- a new Work state merely to encode every shade of model uncertainty;
- a bespoke durable workflow engine, artifact custody system or replacement for ACP;
- implementing EXP-02 mechanisms before their evidence warrants promotion.

## Tickets

Ticket status lives only in this checklist.

| Status | Ticket | Slice | Observed friction served | Prior machinery made deletable |
| --- | --- | --- | --- |
| [x] | [**S12-T0 · Freeze the natural-team recovery contract**](sprint-12/t00-natural-team-contract.md) | Cross-layer | Natural behaviour exists in prose but process/artifact/recovery ownership can still overlap | Duplicate completion semantics and callback-as-truth assumptions |
| [x] | [**S12-T1 · Bind authoritative commission facts**](sprint-12/t01-commission-context.md) | OrgIntel + Runtime | Leads retype or omit facts the company already knows | Handoff templates and ad hoc kickoff assembly |
| [x] | [**S12-T2 · Deliver consequential team messages**](sprint-12/t02-consequential-messages.md) | OrgIntel + Runtime bridge | Questions and interface changes can wait or route through Exec | Polling wakes, fixed rendezvous and Exec relay |
| [x] | [**S12-T3 · Reconcile process and artifact evidence**](sprint-12/t03-artifact-process-recovery.md) | Runtime + OrgIntel | Useful work can become lost or duplicated when semantic completion is missing | Callback-only completion and restart-by-default repair |
| [ ] | [**S12-T4 · Present one accountable outcome**](sprint-12/t04-owner-outcome.md) | Owner gateway/cockpit | Owner-visible truth can fragment across status, messages and artifacts | Team-admin projections and duplicate status surfaces |
| [x] | [**S12-T5 · Dogfood, measure and purge**](sprint-12/t05-dogfood-purge.md) | Full slice | Contracts remain hypotheses until one real outcome survives failure | Losing adapters, compatibility paths and unused coordination fields |

Existing code or an isolated green test does not check a ticket. Each ticket closes only on its named
behavioural evidence.

## Verification and evidence package

The completed sprint must contain:

1. frozen inputs and expected observations for acceptance scenarios A–F;
2. service-level traces showing actor, Work, Attempt, process and artifact identity without secrets or
   hidden chain-of-thought;
3. a negative control proving missing callback cannot silently pass;
4. a duplicate-delivery/restart control proving the same responsibility does not execute twice;
5. the native solo artifact and the native team artifact;
6. the real-company ReviewTarget and its exact observed result;
7. measurement tables with invalid runs identified rather than averaged in; and
8. a deletion note naming every abandoned field, adapter, event or UI projection.

Automated tests cover attribution, idempotent claim/recovery and boundary ownership. Real-tool probes
and dogfood prove productive behaviour. A green unit suite alone does not pass the sprint.

## Salvage

No legacy orchestration, universal command, ledger or asset-custody machinery is authorised for lift.
Existing Sprint 11 ACP, wake-recovery and event-delivery work may be reused only after its own stated
verification passes. Mature process supervision, Git and ordinary Runtime files remain the preferred
substrate.

## Entry, stop and exit gates

**Entry:** founders approve this spec; Sprint 11 dependencies are evidenced or the ticket cut states
the exact rebase; the real-company dogfood outcome is named; unrelated worktrees are preserved.

**Stop:** pause for founder judgement if the implementation requires a new durable entity, a second
source of truth, a fixed teamwork protocol, owner-facing agent administration or a consequential
effect outside the selected dogfood outcome.

**Exit:** acceptance scenarios pass; the real native outcome is observed; false paths are deleted;
the sprint report records counterevidence and remaining accepted risks; and only then is the relevant
product spec wording updated to reflect what the build actually proved.

## Proposed start resolutions

If the product sprint is started later, the 23 August alignment discussion suggests these reversible
working resolutions:

1. Reuse every verified Sprint 11 dependency, but absorb the minimum missing wake/Attempt dependency
   rather than waiting for unrelated publication work.
2. Use a bounded Cosmon repository outcome as scenario F. It must use the real Runtime and models but
   requires no consequential external publication.
3. Keep the first owner projection to accountable lead, outcome, risk and native target. Staff detail
   remains available through inspection.

Any finding that makes one resolution consequential or misleading returns for founder judgement.
