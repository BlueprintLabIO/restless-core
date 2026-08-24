# Experiment Sprint 03 — The supervised company

**Status:** Approved and running.

**Decision owner:** Founder.

**Date:** 24 August 2026

**Model envelope:** live GLM-5.3 calls (`zai/glm-5.3`) through the first-party Pi/OMP Company Runtime and
Restless model gateway. No simulated cognition counts.

**Depends on:** EXP-01 and EXP-02 evidence; the factual Actor/Work/Attempt/artifact path; the accepted
Exec → accountable-lead boundary.

---

## 1. Decision already made

Restless is a supervised company:

```text
owner
  └── Exec — portfolio judgement; always dispatches and returns to owner availability
       └── accountable lead — mission keeper and supervisor; does no planned production
            └── one or more workers — produce, operate and return observable outcomes
```

The lead/worker separation is a product invariant, not an experiment arm that must justify its
existence through lower local latency. A supervisor supplies a distinct function the company needs:
preserving the wider mission while workers enter narrow production contexts, observing evidence,
guiding, redirecting, stopping churn, repairing failure and making the final judgement.

EXP-03 may measure the cost of this decision. It may not promote a direct producer or player-coach
lead as the Restless architecture merely because one finishes a bounded task faster.

The lead may:

- understand and frame the complete outcome;
- choose, brief, commission, replace and release workers;
- inspect work, tools, sources, artifacts and native outcomes;
- communicate material corrections, answer questions and change direction;
- redirect or reassign blocked, drifting or weak work;
- run or request checks without editing the produced content;
- accept, reject or return an outcome and prepare it for review.

The lead may not perform planned production, silently repair a worker's artifact, or keep a private
parallel implementation. Applying an exact accepted worker artifact is supervisory promotion, not
production. A conflicting or content-changing integration must go back to a worker. Any emergency
takeover is recorded as a supervision failure and requires fresh independent review; it never becomes
normal behaviour.

## 2. Decision this sprint must produce

Find the smallest powerful operating design by which a non-producing GLM-5.3 lead can hire and
coordinate GLM-5.3 workers across real economic work.

The sprint must answer:

1. What context, tools and events does a supervisor actually need?
2. When should a supervisor wake and intervene without polling?
3. How many workers can one lead supervise for different work shapes?
4. When should one worker own the whole outcome, when should work split across specialists, and when
   should many same-role workers process independent units?
5. Which failures come from the model, the brief, the boundary, the runtime or the coordination code?
6. What is the measurable supervisory premium relative to direct and player-coach counterfactuals?
7. Which company functions should Restless prove first, and which can be prepared internally then
   handed to an external professional?

The result is a compact supervisor operating guide, empirical span curves and at most one justified
implementation recommendation. It is not a deterministic task router, universal workflow engine or
fixed organisation chart.

## 3. First-principles design imported from research

The research prior is consistent but conditional:

- central supervision is better at containing error propagation than unverified peer or independent
  agent networks;
- parallel agents help when work decomposes into independent evidence, alternatives or units;
- they often hurt sequential, tool-heavy or tightly shared-state work;
- good delegation names a narrow objective, observable completion, available tools and sources,
  boundaries and relevant context;
- artifact references preserve more truth than long prose relay;
- elastic worker allocation is preferable to a fixed team;
- supervisors should plan, observe, replan and recover, but a second exhaustive plan/progress ledger
  is not required to obtain those behaviours;
- external checks and fresh review matter because agreement among correlated agents is not evidence.

Primary sources and exact imported priors are recorded in
[`exp03-supervisor-systems.md`](../coordination/research/exp03-supervisor-systems.md). They guide cell
selection and do not count as Restless product evidence.

## 4. Proposed minimal supervisor operating system

EXP-03 tests this design first:

```text
Supervisor context
  mission + exact outcome + consequential constraints
  current native evidence + material changes
  available workers and observed capabilities
  sparse responsibilities + returned artifact references

Supervisor actions
  commission | inspect | message | redirect | reassign | stop | accept/reject

Wake causes
  owner/Exec change | worker question | material progress | blocked | artifact | terminal

Worker return
  exact artifact/observation + checks + unresolved uncertainty
```

Operating rules:

1. The lead first builds the causal model of the whole outcome.
2. It selects the smallest worker set that can close the outcome or useful independent units.
3. Each worker receives purpose, responsibility, observable success, exact inputs and tools, important
   constraints, authority boundary and the intended return surface.
4. Workers own production end to end within their boundary. They communicate only when new information
   can change another actor's work.
5. The lead quiesces while useful work proceeds. It wakes on events, never periodic status checks.
6. The lead inspects the real artifact or native outcome, not the worker's confidence.
7. Correction stays with a worker. The lead preserves mission context and judgement separation.
8. The team expands and contracts with useful independent work; idle roles are not kept hot merely to
   resemble a human department.

The Work graph remains a sparse factual substrate: who accepted which responsibility, the current
Attempt, dependencies that truly block another outcome, and returned artifact references. It does not
mirror every plan step, message, customer record, tool call or thought. Full shared transcripts,
meetings, status polling, a semantic blackboard and dual task/progress ledgers are excluded unless a
specific observed failure later activates them.

## 5. Which economic work comes first

“Traditionally in-house” is a useful prior, not the underlying law. A function is more important to
prove internally when it is continuous, frequent, mission-coupled, feedback-rich, strategically
differentiating or difficult to specify completely at an external boundary. A function can be
deferred when it is episodic, credential-gated, heavily professionalised, independently reviewable
and naturally bought as a specialist service.

| Priority | Function | Why | EXP-03 disposition |
| --- | --- | --- | --- |
| **A — core loop** | Product and engineering | Creates and changes the offering; tightly coupled to mission and customer evidence | Must-run coupled-work sentinel |
| **A — core loop** | Marketing and demand generation | Continuous market learning plus coherent multi-channel production | Must-run mixed-work sentinel |
| **A — core loop** | Sales and pipeline | Revenue-critical repeated units with natural parallel capacity | Must-run replicated-queue sentinel |
| **A — core loop** | Customer support and success | Continuous event flow, exception handling, retention and product feedback | Must-run volatile-supervision sentinel |
| **A — company mind** | Research, strategy and competitive intelligence | Broad independent evidence must become one accountable decision | Must-run breadth/synthesis sentinel |
| **B — operating core** | Finance operations | Reconciliation, invoicing, collections and forecasting are frequent; consequential exceptions need oversight | Reserve `_test` sentinel after the A wave |
| **B — operating core** | Internal operations and procurement | Repeated workflows and exception queues can consume real company capacity | Reserve; activate only if A cells leave a work-shape gap |
| **B — people loop** | Recruiting and people operations | Sourcing/coordination parallelise, while consequential judgement remains human-governed | Later queue/decision sentinel |
| **C — specialist boundary** | Legal, tax, audit opinions and regulated compliance advice | Episodic professional judgement, credential and liability boundary | Defer production; Restless prepares a complete evidence brief and review surface for an external professional |
| **C — specialist boundary** | Medical, licensed engineering and other certified opinions | Independent professional accountability is irreducible | Defer opinion; prepare inputs and last mile only |
| **C — physical boundary** | Physical fulfilment and field work | Requires people, robotics or service providers outside the model runtime | Coordinate providers; do not pretend model calls perform it |

This sprint measures representative **work shapes**, not every department separately. A result
generalises to a new function only when its coupling, unit independence, volatility, consequence and
acceptance surface are materially similar.

## 6. Frozen sparse portfolio

### T1 — Coupled product change

Produce one small but real Cosmon improvement that requires a coherent design/code/native-proof causal
model and has no credible independent coauthoring seam.

- Primary organisation: one non-producing supervisor plus one end-to-end worker.
- Diagnostic counterfactual: one GLM-5.3 actor works directly.
- Purpose: price the supervisory invariant on work where multi-producer decomposition should lose.
- Native acceptance: playable/browser outcome plus frozen repository gates.
- Expected design lesson: keep a single producer, not a team of coauthors; supervision can still
  preserve mission and independent judgement.

### T2 — Marketing campaign system

From a frozen fictional company, customer evidence and product truth, produce a decision-ready
campaign strategy and native execution pack: positioning, channel plan, two channel-native assets,
measurement plan and claims/evidence register. Nothing is published.

- Compare one end-to-end worker with two specialist workers under the same supervisor.
- The lead must preserve one strategy and reject inconsistent claims; it cannot write copy itself.
- Native acceptance: rendered campaign pack and exact source/claim checks, plus blind whole-pack
  judgement.
- Purpose: test mixed work where independent production exists but coherence must close at the lead.

### T3 — Sales pipeline batch

In an isolated `_test` company, freeze independent fictional prospect dossiers. Every unit requires
qualification evidence, a personalised next-step draft, a disposition and an observable acceptance
result. No message is sent.

- Run Q1 and Q2 first; activate Q4 and Q8 only through positive marginal gates.
- Same model, role, playbook and tools; disjoint units.
- The lead calibrates, samples, handles exceptions and judges the batch. It does not rewrite units.
- Native acceptance: every unit scored, with duplicate/missed-unit checks and tail-quality reporting.
- Purpose: measure elastic staffing where value aggregates rather than requiring artifact merging.

### T4 — Customer-operations change and recovery

From a frozen `_test` customer queue, produce response/resolution packages for several cases. After
production begins, inject one material mission or policy change and one worker-local obstacle. No
response is sent.

- Compare terminal-only supervision with material-event supervision.
- Freeze event bytes and delivery condition before either arm.
- Lead may redirect/reassign, but never repair the packages itself.
- Native acceptance: policy adherence, case resolution, stale work after the change, time to redirect,
  repair loops and worst-case customer harm proxy.
- Purpose: isolate the value of preserved supervisory attention under drift and blockage.

### T5 — Research and strategy breadth

From a frozen source-complete corpus with independent evidence regions, produce one decision memo
whose recommendation, uncertainties and next action are traceable to exact sources.

- Compare one end-to-end worker with two independent evidence workers plus one synthesis worker under
  the same supervisor.
- The supervisor frames and judges; synthesis remains worker production.
- Native acceptance: citation entailment, coverage, contradiction handling, decision usefulness and
  hidden source checks.
- Purpose: retest the strongest externally supported multi-agent region without the source-transfer
  defect that invalidated earlier Restless runs.

### Activation-only reserve — finance operations

Use fictional `_test` statements, invoices and policies to prepare a reconciled close pack and an
exception list. Activate only if T1–T5 leave uncertainty about deterministic-volume plus consequential
exception work. No real account, transaction or financial advice is involved.

## 7. Experimental organisations

All cognitive actors use `zai/glm-5.3` through the same gateway and tool envelope. Separate sessions,
roles and workspaces are real actor boundaries.

| Cell | Organisation | Purpose |
| --- | --- | --- |
| **C0** | One direct GLM-5.3 producer, no supervisor | Diagnostic counterfactual; measures the cost and benefit of separation but can never become product canon |
| **P1** | Player-coach lead plus one worker | Activate only where prior evidence is insufficient; diagnostic counterfactual for attention contamination |
| **S1-T** | Non-producing supervisor plus one worker; wakes at terminal/block only | Minimum canonical supervision |
| **S1-E** | Non-producing supervisor plus one worker; material questions/changes/progress may wake it | Tests useful mid-course guidance without polling |
| **S2/S3** | Non-producing supervisor plus two or three complementary workers | Tests specialisation only on T2/T5 |
| **Q1/Q2/Q4/Q8** | Non-producing supervisor plus same-role workers on disjoint units | Measures span and throughput on T3 |

The supervisor is not required to spend tokens while workers run. “Available” means event-addressable
and context-preserving, not an always-sampling process.

## 8. GLM-5.3 controls and spend

- Exact runtime selector: `zai/glm-5.3`, the credentialed Z.ai first-party route exposed by the
  Restless broker. OpenRouter's catalogue identifies the corresponding model as `z-ai/glm-5.3` but
  the current broker has no OpenRouter credential, so that route is not used.
- Lead, worker and qualitative evaluator model family: identical.
- Reasoning effort: `high` for all actors in matched cells; a fresh artifact-only evaluator may use
  `max`, but it cannot see producer reasoning or topology labels.
- Tool availability, starting state, prompt bytes, source corpus and evaluator are frozen per matched
  set.
- Every run records provider identity, newly processed/cached input, output, tool calls, wall time,
  actor time and USD.
- Public catalogue claims do not count as runtime proof. Before counted work, GLM-5.3 must execute an
  exact file tool operation through the Company Runtime and Restless gateway.
- Sprint spend ceiling: **US$75**. A counted cell ceiling is **US$8**; capability/conformance work is
  capped at **US$5** total. Stop rather than silently exceed either bound.
- Provider unavailability, rate limiting or stale registration before useful cognition is
  infrastructure-invalid and does not count as an organisational loss.

OpenRouter currently lists a 1,048,576-token context, always-on reasoning with `low`, `high` and `max`
efforts, tool calling, and prices of $1.40/M input and $4.40/M output. These are dated environment facts
and must be live-probed again if execution resumes later.

## 9. Completion without timeout choreography

Actor completion is callback or process exit, not “wait N seconds then guess.” Work is event-driven:

```text
commission → durable Attempt + supervised process
process/tool events → progress or material wake when useful
artifact + terminal report → lead wake
process loss → observed unknown + preserved workspace → lead recovery judgement
```

A wall-clock envelope is a budget and operator safety stop, not an inference that work failed or
completed. Attempt leases are renewable liveness evidence. A stopped envelope preserves process,
workspace and truth for later inspection; it never converts silence into success or retries blindly.

## 10. Measurements

### Outcome

- frozen native success contract and hidden evaluator result;
- blind whole-outcome preference where judgement matters;
- factual/citation/claim correctness;
- regressions, omissions and worst-decile unit quality;
- whether worker artifacts were accepted, returned, rejected or recreated.

### Supervisory value

- mission or policy changes noticed and correctly propagated;
- stale work produced after a material change;
- useful interventions, unnecessary interventions and time to redirect;
- blocked work recovered, reassigned or allowed to churn;
- final whole-outcome defects caught before review;
- lead takeover or hidden production violations;
- lead active time, event wakes and owner/Exec attention.

### Team economics

- request-to-first-useful-artifact and accepted-outcome latency;
- summed actor time, useful overlap and provider saturation;
- briefing/orientation, communication, inspection, rework and integration cost;
- newly processed/cached input, output, tools and USD;
- accepted units per hour and marginal cost per accepted unit for replicated work.

### Organisational truth

- exact actor/model identity for every cognitive contribution;
- Work → Attempt → artifact attribution;
- one unambiguous outcome owner;
- no lead-authored production diff;
- duplicate work, missing callbacks, orphaned processes and unobserved artifacts.

## 11. Decision gates

### Supervisor system

Supervisor separation stays regardless of results. A design variant advances only if it improves the
accepted whole outcome, recovery or lead efficiency without hidden production or polling. A slower
result records the **supervisory premium** and focuses optimisation on briefing, events, model/runtime
speed or task boundary—not removal of the supervisor.

### Event policy

Prefer terminal-only supervision on stable work unless a material-event wake prevents measurable
stale work, damage or repair. Never infer a periodic check cadence from a win. Events should be tied to
new information, not elapsed time.

### Specialised team

S2/S3 advances only when multiple worker artifacts are genuinely used and the complete outcome gains
quality, evidence or latency value that repays briefing and synthesis. A worker producing an unused
artifact is not team value. Do not test a larger complementary team after a clear +1 loss.

### Replicated queue and span

Each Q scale step advances only while marginal accepted throughput remains positive after quality,
lead attention, provider pressure and cost. The answer is a curve scoped to the queue shape, not a
universal headcount rule. Stop at the first bottleneck and name it.

### Failure attribution

Every failed cell receives one primary class before any retry:

```text
model capability | task/boundary | brief/context | coordination behaviour
runtime/provider | harness/code | evaluator/contract | external dependency
```

Retry only when new evidence changes the hypothesis. No third identical attempt.

## 12. Execution sequence

### Wave 0 — research, freeze and conformance

- record primary-source design priors separately from Restless evidence;
- live-prove GLM-5.3 identity, price, tool use, exact artifact production and accounting;
- add a supervisor-only conformance mode without changing production architecture;
- prove the supervisor cannot produce and the candidate derives from worker artifacts;
- freeze T1–T5 task bytes, starting states, native evaluators and randomised arm order.

### Wave 1 — one supervisor, one worker

- run T1 C0/S1-T to measure the fixed supervisory premium on coupled product work;
- run T4 S1-T/S1-E with identical injected changes;
- stop and repair the harness if lead production, polling or missing attribution appears.

### Wave 2 — complementary specialists

- run T2 S1-E/S2;
- run T5 S1-E/S3 only if source delivery and artifact lineage conformance pass;
- stop each branch at its first clear larger-team loss.

### Wave 3 — replicated capacity

- run T3 Q1/Q2 in randomised order;
- activate Q4 then Q8 only through consecutive positive marginal gates;
- identify the first supervision, quality, demand, provider or cost bottleneck.

### Wave 4 — synthesis and purge

- update `CANON.md`, `EVIDENCE.md`, `REGISTRY.md` and `docs/COORDINATION_THEORY.md`;
- publish the supervisor operating guide, event policy and scoped span curves;
- distinguish owner decisions, external priors and live Restless evidence;
- delete losing experiment-only mechanisms and recommend at most one bounded implementation change.

## 13. Validity and stop rules

- No simulated cognitive result counts; all actors use live GLM-5.3.
- All business fixtures are frozen and fictional or `_test`; no external effect is authorised.
- The direct and player-coach cells are diagnostic counterfactuals, not candidate architectures.
- A model report is a claim until its native artifact and frozen gate are inspected.
- Same-model agreement is correlated evidence, not independent truth.
- Lead file edits or content-changing integration invalidate a supervisor cell.
- Polling, scheduled status meetings or timeout-inferred completion invalidate the event manipulation.
- Do not change a prompt, task, evaluator or cell after seeing its matched result.
- One infrastructure retry is allowed only after the fault is identified and corrected.
- One clear larger-team loss stops that scaling branch.
- Do not add a production schema, Work kind, workflow engine, shared transcript or owner UI during
  the experiment.
- Controlled `_test` work validates coordination and artifacts, never real demand or market response.

## 14. Exit contract

EXP-03 is complete when:

1. GLM-5.3 has live gateway, tool, artifact and accounting proof;
2. supervisor-only conformance is mechanically checked, including absence of lead production;
3. T1–T5 have truthful dispositions or an explicit evidence-based stop reason;
4. at least marketing, sales, customer operations, research/strategy and product/build have been
   represented by native outcomes;
5. the stable versus material-event wake decision is evidence-backed;
6. a scoped span curve exists for replicated work and complementary teams stop at first loss;
7. costs are decomposed into production, supervision, handoff, recovery and infrastructure;
8. the knowledge base distinguishes enduring theory, owner decisions, external priors and contingent
   model/runtime findings; and
9. losing experimental machinery is removed.

Execution is authorised by the founder's instruction on 24 August 2026. No further confirmation gate
precedes Wave 0; consequential external effects remain separately governed and out of scope.
