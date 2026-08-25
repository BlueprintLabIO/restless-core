# Experiment Sprint 04 — Local closure and sparse convergence

**Status:** Completed 25 August 2026. Founder approval was supplied by the instruction to finish the
sprint; execution followed the sparse activation and stop gates below.

**Decision owner:** Founder.

**Date:** 24 August 2026

**Primary model envelope:** matched live `zai/glm-5.3` calls through the first-party Pi/OMP Company
Runtime and Restless model gateway. Candidate cheaper workers are admitted only in the final economic
wave and only after a live capability and concurrency proof.

**Depends on:** EXP-03 final synthesis and supervisor operating guide; the factual
Actor/Work/Attempt/artifact path; the owner → available Exec → non-producing lead → worker invariant.

---

## 1. Decisions already made

Restless keeps this organisation in every counted arm:

```text
owner
  └── Exec — dispatches every executable request and immediately returns to portfolio availability
       └── accountable lead — preserves mission, supervises and judges; performs no planned production
            └── worker(s) — own production and close useful outcomes or independent units
```

EXP-03 established a strong default for one shared outcome: one end-to-end worker under one
non-producing lead. It did **not** establish that parallel sales or research is intrinsically weak.
Its sales team sent eight independent account units through a model assembler, and its research cell
gave ten already-supplied sources to evidence workers before another model rebuilt one memo. Those
are small, convergence-heavy shapes. They do not represent account ownership to local closure,
continuous monitoring, or broad evidence discovery that can saturate one analyst.

EXP-04 therefore tests a corrected boundary:

> Parallel work earns its keep when workers close independently valuable units. Fan-in is a sparse
> operation requested for one named decision, conflict or integrated artifact—not the default ending
> of parallel work.

When parallel work returns, use the cheapest operation capable of preserving value:

1. retain independent outcomes as independent outcomes;
2. aggregate facts deterministically;
3. integrate mechanically when interfaces are disjoint and exact;
4. commission one bounded synthesis worker only for a named integrated artifact;
5. escalate one consequential decision to the accountable lead, Exec or owner at the correct level.

The lead may sample, inspect, calibrate, redirect, reassign, reject and accept. It may not become the
assembler, rewrite normal units or inspect every ordinary unit merely to make supervision visible.

## 2. Decision this sprint must produce

Find the first empirically defensible operating envelope for a parallel autonomous company:

1. At what unit volume and work shape does Q2/Q4/Q8 beat one worker on accepted throughput, tail
   quality and cost per accepted unit?
2. How much exception volume can one non-producing lead supervise before lead attention becomes the
   bottleneck?
3. Does a persistent, cache-eligible actor session reduce reorientation cost without corrupting
   separation between arms or actors?
4. Can reasoning effort be allocated by role and consequence rather than spent uniformly?
5. Does reserving capacity to verify, repair and close prevent late-run failure better than spending
   the same envelope opportunistically?
6. When does broad research benefit from parallel discovery, and when is one synthesis pass worth its
   compression loss?
7. After topology wins with one model, can a strong lead supervise cheaper workers without losing the
   quality tail or operational reliability?
8. Can several independent departments operate concurrently while Exec remains available for the
   next owner request?

The deliverable is a measured span/crossover guide, an effort and closure policy, a fan-in guide and
a short list of production-worthy harness fixes. It is not a fixed org chart, deterministic task
router, universal batch engine or catalogue of department workflows.

## 3. What counts as fan-in

A fan-in is any step where several cognitive contributions must be interpreted and recomposed into a
new cognitive artifact or decision. A database view, sorted index, arithmetic roll-up, concatenation
of explicitly disjoint files or merge of non-conflicting commits is mechanical composition, not
cognitive fan-in.

### Real fan-ins

- **Decision fan-in:** evidence from several regions is needed for one exact go/no-go, allocation or
  prioritisation decision.
- **Exception fan-in:** independent units expose the same unexpected policy or product conflict and
  need one ruling.
- **Adjudication fan-in:** independent evidence or reviews materially disagree.
- **Contract fan-in:** several components must satisfy one shared interface, narrative or release
  contract.
- **Campaign fan-in:** channel-native assets must express one positioning and claims system.
- **Incident fan-in:** several observations need one commander and one current response.
- **Pattern fan-in:** a sufficiently large local-output set is explicitly analysed to change the
  playbook, product or portfolio.

### Anti-pattern fan-ins

- a universal assembler after every parallel batch;
- a summary of summaries when exact sources or local outcomes remain available;
- reconstructing account, case or alert detail into one prose batch report;
- consensus or review cycles with no named disputed decision;
- status fan-in used to simulate supervision;
- premature convergence during discovery;
- nested or unbounded synthesis chains;
- a shadow integration artifact that becomes a second source of truth;
- using fan-in as memory because the runtime cannot preserve actor or artifact context;
- requiring every worker to finish on the same cadence before any useful unit may close.

Synthesis is worker production. The lead frames and judges it; the lead does not write it.

## 4. Confirmed harness gaps and conformance gates

No organisational result is counted until the gate relevant to that cell passes. These are repairs to
experimental validity and ordinary process control, not new coordination abstractions.

### H1 — Persistent actor sessions

**Observed gap:** the runner preserves company files and process evidence, but launches a new model
process and ACP session for each wake. The actor is reconstructed rather than conversationally hot.

**Required behaviour:** one supervised actor process and model session persists across its wakes and
tool calls. Every trace names actor, Attempt, provider and exact session identity. A process loss may
reconstruct from factual state, but the resulting turn is marked as a cold restart. Sessions never
share private history across different actors or experimental arms.

**Gate:** a worker receives three event-driven wakes in one session, retains an exact earlier factual
commitment without replaying the full transcript, and then survives a forced process loss through an
honestly marked cold reconstruction. Observed provider cache usage is recorded if exposed; absence is
`unknown`, never inferred.

### H2 — Real cancel and redirect

**Observed gap:** redirect records cancellation intent, but the active model call can continue until
it naturally returns. The later bookkeeping is not an interrupt.

**Required behaviour:** Attempt identity maps to the exact live controller or process. Cancel stops
that sampling/tool loop, preserves its workspace and terminal evidence, records whether termination
was observed, and only then starts redirected work. Cancellation is idempotent.

**Gate:** a deliberately long tool/model turn is cancelled while active; no post-cancel production is
accepted; its workspace and latest usage remain inspectable; one redirected Attempt resumes without
duplicate ownership.

### H3 — Locally complete units without a model assembler

**Observed gap:** the run protocol expects one exact candidate artifact, which pressured EXP-03's
sales queue into a batch assembler even though account units were independent.

**Required behaviour:** workers return disjoint, locally complete unit artifacts or commits. The
harness produces a deterministic projection/index and can mechanically compose explicitly disjoint
outputs. Duplicate ownership, missing units and content conflicts fail closed. The lead judges the
batch from anchors, exceptions, a predeclared sample and aggregate evidence, then accepts the exact
projection. This introduces no new production entity or artifact lifecycle: Work, Attempt, ordinary
files/Git and the domain projection remain authoritative.

**Gate:** Q4 closes a synthetic eight-unit queue with randomized finish order, zero model assembler,
zero duplicate/missing units and byte-identical projections across two compositions.

### H4 — Honest time, usage, effort and phase telemetry

**Observed gaps:** operator pauses can contaminate run wall time; usage at forced cancellation has not
been proved; the trace cannot distinguish orientation, production, verification, handoff and repair;
configured effort is not always distinguishable from observed provider behaviour.

**Required behaviour:** record separately request wall time, active actor segments, provider queue or
wait, explicit founder/operator pause, tool time and overlap. Preserve the latest cumulative usage at
terminal and cancellation when the provider supplies it. Record configured effort and observed effort
metadata separately. Add a low-cost one-way phase signal—`orient | produce | verify | handoff |
repair`—without waking the lead or storing private chain-of-thought. Tool and artifact evidence can
corroborate phase claims.

**Gate:** a deterministic trace fixture and one live conformance run account for every segment without
double counting, include a forced cancellation, and never turn missing provider fields into zero.

### H5 — Sustained provider concurrency

**Observed gap:** one successful model admission does not prove Q4 or Q8 can run without provider
queueing, rate limits or route instability.

**Required behaviour:** before every scale step, make an n-way sustained capability probe with the
same provider/model/tool envelope. Attribute admission wait, rate limits, provider failures and model
work separately. Infrastructure-invalid runs do not become topology losses.

**Gate:** the intended concurrency completes useful tool work with exact attribution. A failed gate
stops that scale; it does not trigger route roulette during a matched comparison.

### H6 — Evaluator separation

**Observed gap:** deterministic checks have sometimes used brittle text patterns for semantic claims,
while same-family model review can share the producers' blind spots.

**Required behaviour:** exact schemas, IDs, source existence, duplicates, policy invariants and
arithmetic are checked deterministically. Meaning, usefulness and whole-outcome coherence are judged
from blinded native artifacts by a fresh evaluator. Open-language assertions are never promoted to
facts by regex. Evaluator model/provider correlation is reported.

**Gate:** planted positive, negated and omitted cases produce the intended exact-check result, while a
blind review packet exposes no topology, worker count, producer trace or spend.

### H7 — Material wakes must be causally useful

**Observed gap:** EXP-03's material progress message was delivered as a next-wake event and did not
alter active work. That is valid mailbox delivery, not evidence of live supervisory guidance.

**Required behaviour:** a material event either reaches an actor at a safe observable boundary in its
persistent session or invokes H2 to redirect it. Routine progress remains reply-free and does not wake
the lead.

**Gate:** a frozen policy change injected after observable production begins prevents acceptance of
stale post-change units, with exact event and cancellation/acknowledgement timing.

## 5. Sparse method

The search space is too large for a full factorial. EXP-04 uses staged elimination:

1. **Make the measurement honest.** Pass H1–H7 only where required.
2. **Hold model and effort constant.** Establish whether the corrected local-closure topology wins.
3. **Find the crossover, then stop.** Scale Q1 → Q2 → Q4 → Q8 only through positive marginal gates.
4. **Vary effort on a proved workload.** Do not confuse weak effort allocation with topology.
5. **Vary worker model/price last.** Do not use a cheaper provider to explain a coordination loss.
6. **Replicate only decision-changing wins.** One loss stops the next doubling; one provisional win is
   replicated once in randomized order before a wider claim.

The main explanatory dimensions recorded for every work shape are unit independence, local acceptance
surface, shared mutable state, tool density, evidence-search breadth, volatility, consequence and tail
risk, cognitive convergence required, useful overlap, supervisor sampling cost and provider capacity.

## 6. Wave 1 — Effort and closure controls

They include what EXP-03 originally left unresolved. E1 runs after H1, H3, H4, H5 and H6; C1
requires H1, H4 and H6.

### E1 — Consequence-weighted effort

Use a frozen queue containing routine units plus a small predeclared set of subtle exceptions. Compare:

- **E1-A — uniform:** lead and every worker use `high` effort;
- **E1-B — allocated:** lead uses `high`; routine-unit workers use the lowest live-admitted effort;
  exceptions or low-confidence units receive a fresh high-effort worker Attempt after explicit
  escalation.

Both arms have the same total spend ceiling, inputs, tools and supervisor. No raw reasoning trace is
collected. Compare accepted throughput, worst-decile correctness, exception recall, unnecessary
escalation, lead load and spend. The allocated arm advances only if its quality tail is non-inferior
and it reduces cost or latency materially; a cheap average with missed high-consequence exceptions is
a loss.

### C1 — Closure reserve

Repeat one coherent outcome with a known risk of spending the envelope before verification and
handoff. Compare:

- **C1-A — open envelope:** ordinary actor budgets draw from the matched total ceiling;
- **C1-B — reserved closure:** admissions preserve 25% of the same ceiling for worker verification,
  lead native review and at most one worker repair. Unused reserve is not spent for its own sake.

Completion remains event/callback based; the envelope is a resource limit, never timeout semantics.
The reserve advances only if it increases exact native closure or reduces late repair without merely
starving useful production. If 25% is wrong, report the observed allocation pressure rather than tune
repeatedly on the same fixture.

## 7. Wave 2 — Parallel work that should not fan in

Q-SALES and Q-MONITOR are the must-run local-closure cells. Q-SUPPORT is the first activation-only
extension because it adds volatility and asymmetric harm. In every cell, each worker owns a disjoint
partition to observable unit completion. A deterministic domain projection shows portfolio state; no
model rewrites the units into one report.

### Q-SALES — Account ownership

Use 48 frozen fictional prospect/account dossiers in an isolated `_test` company. Every account owner
must produce an evidence-backed qualification, personalised next-action package, disposition,
follow-up state and exact policy/claim checks. Nothing is sent.

- Scale Q1 → Q2 → Q4 → Q8 through the gate in §11.
- Partition by account, not by research/copy/review stage.
- The lead reviews fixed calibration anchors, every exception and a predeclared stratified sample; it
  does not reread every normal account.
- Hidden evaluation covers every unit, including worst decile, duplicate/missed accounts and
  cross-account contamination.

This is the primary sales correction to EXP-03. The product is a set of closed account states, not a
batch memo.

### Q-SUPPORT — Case ownership under sparse exceptions

Use 48 frozen fictional support/success cases with shared policy and six planted exception classes.
Every case owner closes a resolution package, customer-safe response draft, system action plan and
observable next state. Nothing is sent or applied.

Compare Q1, the Q-SALES crossover size and at most one larger step. The lead receives only genuine
questions, policy conflicts, harmful-risk cases and the frozen sample. H7 injects one material policy
change after work has begun. Measure accepted cases/hour, stale cases, harmful tail defects, exception
recall, lead interventions and recovery time.

This distinguishes a high-volume queue with volatility and asymmetric harm from routine sales work.

### Q-MONITOR — Competitive monitoring alerts

Use a dated, frozen but search-requiring corpus of at least 80 documents across 12 fictional entities,
including irrelevant, contradictory, duplicated and late-arriving material. Workers own disjoint
entities or evidence regions and return traceable, locally complete alerts with severity, source,
uncertainty and exact follow-up trigger.

Compare Q1, Q2 and Q4. The output is a deterministic alert feed/index. There is no summary memo. The
lead samples alerts, handles policy ambiguity and commissions deeper work only when a frozen trigger
fires. Measure relevant-event recall, precision, source entailment, time to first useful alert,
coverage, cost per accepted alert and worst-entity quality.

This tests parallel research as continuous discovery without compulsory convergence.

## 8. Wave 3 — Broad research with exactly one named fan-in

### R-DECIDE — Discovery to one decision

Use a fresh order-randomized instance of the broad corpus. Freeze one consequential question whose
answer genuinely needs several independent evidence regions. Compare:

- **R1:** one analyst discovers evidence and produces the decision artifact end to end;
- **R4-S:** four evidence workers return exact source maps and one synthesis worker produces the one
  named decision artifact.

The synthesis worker receives exact sources and structured evidence maps, not summaries of summaries.
There is one cognitive fan-in and no later rewrite chain. The non-producing lead frames the decision,
adjudicates only material conflict and judges the native artifact.

Advance only if parallel discovery materially improves hidden evidence coverage, contradiction
handling, decision usefulness or latency enough to repay synthesis cost and compression loss. If the
locally complete Q-MONITOR feed wins but R4-S loses, the conclusion is not “research does not
parallelise”; it is “parallel discovery works until this convergence step.”

## 9. Wave 4 — Model economics and the parallel company

### M1 — Strong lead, cheaper workers

Activate only on the best replicated Wave-2 local queue and only after the topology has won with
matched GLM-5.3 actors. Keep the GLM-5.3 supervisor and compare:

- GLM-5.3 workers at the winning team size;
- the strongest live-admitted cheaper/free worker route at the same team size.

Candidate routes include `stealth/ox-alpha` and `z-ai/glm-5.2:free`, but names are not capability
claims. Each must first pass tools, exact artifact handoff, session continuity and sustained n-way
admission. If neither passes, M1 is `not-activated`; it does not fall back to a different experiment.

Compare accepted cost/unit, tail quality, repair and escalation rate, provider failure and supervisor
load. A lower token price that creates correlated cleanup or unstable capacity is not an economic win.

### P1 — Concurrent departments and available Exec

Run three independent outcomes at once under three accountable non-producing leads:

```text
Exec
├── sales lead → account owners
├── customer-operations lead → case owners
└── intelligence lead → monitoring workers
```

While all three are active, inject a fourth bounded owner request. Exec must dispatch it to a distinct
lead and return without becoming a status relay or joining production. Leads communicate directly only
when a real resource, policy, customer or strategy dependency crosses outcomes. Routine outputs do not
fan into Exec.

Compare each department with its isolated baseline. Measure owner-request dispatch latency, Exec active
time, cross-department isolation, provider contention, supervisor saturation, accepted throughput and
whether any status/summary meeting was invented. This is the first company-level capacity test; it is
not permission to manufacture a universal global work graph.

## 10. Economically valuable reserve search space

The must-run cells cover the most decision-relevant shapes, not the whole economy. These locally
closing workloads remain activation-only reserves:

| Workload | Independent unit | Normal aggregation | Legitimate sparse escalation |
| --- | --- | --- | --- |
| Recruiting | candidate or role pipeline | candidate states and funnel counts | policy conflict, shortlist or hiring decision |
| Accounts receivable / collections | invoice or debtor case | ledger projection and exception list | disputed amount, cash allocation or consequential outreach |
| Procurement | vendor/quote package | comparable fact table | trade-off decision, incompatible terms or authority boundary |
| Catalogue / localisation | SKU × locale | catalogue projection | shared brand/claim change or cross-SKU inconsistency |
| QA and browser journeys | scenario × environment | pass/fail matrix with exact evidence | systemic defect, release decision or ambiguous contract |
| Inbox / lead operations | message or record | routed queue and structured fields | unclear authority, high consequence or changed policy |
| Moderation / compliance triage | case | disposition queue and audit sample | novel policy class or regulated final judgement |
| Per-target diligence | company, asset or counterparty dossier | portfolio index | one investment/vendor decision requiring comparison |
| Marketplace operations | listing, return or seller case | operational state projection | fraud pattern, policy conflict or portfolio intervention |
| Content variants | channel, locale or bounded asset | asset register | campaign coherence or shared factual claim |

Recruiting, finance operations and procurement are the highest-priority reserves because they are
frequent in-house operating work and add consequence/structured-system dimensions absent from the
must-run portfolio. Activate one only if Wave 1–3 leaves a specific work-shape ambiguity; do not run a
department tour.

Legal, tax, audit, medical and other licensed final opinions remain external-accountability
boundaries. Restless may parallelise evidence preparation and prepare the last mile, but this sprint
does not pretend model workers supply the professional sign-off. Physical fulfilment is likewise
coordinated, not simulated.

## 11. Controls and decision gates

### Frozen controls

- Same fixture, prompt bytes, tools, native acceptance surface, model, effort and total envelope inside
  each topology comparison.
- Fresh actor/session identities between arms; persistent sessions only within an arm. No history or
  provider cache is intentionally shared between arms.
- Arm order randomized; producer identity, topology, trace and spend hidden from qualitative review.
- All fictional operational work runs only in `_test` companies. No message, publication, payment,
  account mutation or consequential external effect is authorised.
- Domain-unit ownership is frozen before work begins. Duplicate or unowned units fail the arm.
- Lead sampling policy and hidden full-population evaluator are frozen before outputs exist.
- The Work graph represents outcome/partition responsibility and true blocking dependencies, not every
  account, case, alert, plan step or tool event.
- No raw chain-of-thought is requested, retained or scored. Observable phase, effort configuration,
  tools, artifacts, messages and outcomes are sufficient.
- A hard wall envelope is only an operator safety stop. Completion, blockage and cancellation come
  from observed callbacks, process state and explicit control.

### Queue scaling gate

Advance from Qn to Q2n when the completed step, compared with the immediately smaller arm:

1. improves accepted units per active hour by at least 25%;
2. keeps worst-decile blind/mechanical quality within 0.5/10 and introduces no uncorrected invariant or
   high-consequence breach;
3. keeps cost per accepted unit within 25% unless a predeclared quality gain justifies it;
4. produces no duplicate/missing ownership and no model assembler;
5. does not saturate the lead or provider enough to erase the next plausible overlap.

Replicate the first apparent crossover once with randomized arm order before doubling again. Stop at
the first clear marginal loss. One diagnostic jump from a losing Q2 to Q4 is permitted only when the
trace already proves useful concurrent work and shows that a one-off fixed orientation or launch cost,
rather than growing coordination or quality loss, dominated Q2. Record that trigger before Q4 and do
not tune the fixture or threshold after seeing results.

### Supervisor span gate

A team size is outside the lead's span when exception/question latency grows, the frozen sample is not
completed, a systemic error survives review, or lead active time becomes the throughput bottleneck.
The response is not automatically another coordination layer. First distinguish worker quality,
exception rate, provider capacity, poor partitioning and missing deterministic tooling.

### Fan-in gate

A cognitive fan-in is permitted only when its launch record names:

- the exact decision, conflict or integrated artifact it must produce;
- why independent outputs or deterministic aggregation are insufficient;
- the bounded inputs and authoritative sources;
- one accountable synthesis worker and one terminal acceptance surface.

If these cannot be named, retain the local outcomes. No nested fan-in is activated in EXP-04.

### Effort and model gates

Effort allocation advances on tail-safe economics, not average fluency. Cheaper workers advance only
after a same-model topology win and must preserve operational reliability as well as artifact quality.
Provider failure is reported separately from cognitive failure.

## 12. Measurements

### Outcome and tail

- accepted units and locally observable completion state;
- hidden full-population correctness, source entailment and policy adherence;
- worst-decile and worst-entity quality, high-consequence breaches and systemic correlated errors;
- duplicates, omissions, contamination, stale post-change work and unnecessary fan-in;
- blind decision/artifact usefulness where one integrated outcome is actually required.

### Capacity and economics

- time to first useful unit, median unit, 90th-percentile unit and full queue;
- accepted units per active hour and request wall hour;
- lead, worker, provider-wait, tool, repair and operator-pause time;
- useful overlap, concurrency actually achieved and admission/rate-limit failures;
- input/cache/output/tool usage and USD by actor, phase and accepted unit;
- cost of briefing, orientation, verification, handoff, repair and synthesis.

### Supervision and control

- lead active time, wake cause, sample size and response latency;
- genuine questions/exceptions versus status narration;
- interventions that changed an outcome, unnecessary interventions and missed systemic errors;
- cancellation latency, post-cancel work, redirect success and preserved evidence;
- Exec dispatch latency and active time while several departments operate;
- cold restarts, session continuity, context replay and observed cache use.

## 13. Priority bottlenecks this sprint can expose

EXP-04 must attribute a loss before proposing machinery. The current priority candidates are:

1. **provider capacity:** nominal parallelism queues or fails upstream;
2. **session reorientation:** disposable model sessions repeatedly rebuild actor state;
3. **lead inspection:** sampling and exceptions consume the saved worker time;
4. **tail correlation:** more similar workers replicate one subtle error faster;
5. **demand starvation:** the fixture is too small to keep parallel workers useful;
6. **partition leakage:** supposedly local units share policy, mutable state or customer context;
7. **tool bottlenecks:** browser, repository, CRM-like state or evaluator serialises the queue;
8. **closure failure:** production consumes the envelope before verification and handoff;
9. **communication latency:** material corrections cannot affect in-flight work safely;
10. **convergence loss:** a synthesis pass discards exact evidence or creates a serial critical path;
11. **evaluator cost/correlation:** proving quality becomes more expensive or less independent than
    production;
12. **portfolio contention:** concurrent departments compete for provider, runtime, budget or Exec
    attention despite independent missions.

The sprint may fix H1–H7 and minimal faults discovered while proving them. A new coordination
primitive requires a repeated observed failure that existing Actor/Work/Attempt/message/artifact,
ordinary process supervision and deterministic domain tooling cannot express. “It might help” is not
an implementation ticket.

## 14. Deliverables and stop conditions

The sprint is complete when it publishes:

1. H1–H7 conformance evidence and an exact list of repairs retained or purged;
2. E1 and C1 matched results;
3. the Q-SALES crossover curve and Q-MONITOR result, plus Q-SUPPORT if H7 and the volatility question
   activate it;
4. R-DECIDE or an explicit `not-activated`/infrastructure-invalid disposition;
5. M1 and P1 only if their dependencies activate;
6. a supervisor span guide by work shape;
7. a real-pattern/anti-pattern fan-in guide;
8. model/effort/provider economics with confounds named;
9. updated `CANON.md`, `EVIDENCE.md`, `PROGRAM.md`, `REGISTRY.md` and any architecture/spec text whose
   enduring claim actually changed;
10. deletion of experiment-only adapters that lost or never activated.

Stop the sprint early when:

- a harness gate cannot make the intended comparison valid;
- same-model local closure fails before a model-economics arm;
- Q2 loses without the predeclared fixed-cost diagnostic trigger for one Q4 jump;
- provider capacity invalidates the intended concurrency;
- the spend ceiling would be exceeded;
- a third materially identical retry would be required;
- outputs would contaminate a live company or require an unauthorised external effect.

## 15. Spend and execution boundary

- Draft sprint ceiling: **US$100**.
- Harness/conformance ceiling: **US$8** total.
- Any matched counted cell: **US$12** maximum unless the founder amends this spec before execution.
- A provider route is frozen within a matched set. No silent fallback or mid-cell route substitution.
- Infrastructure-invalid work preserves artifacts and usage but does not count as an organisational
  result.
- Create `experiment/coordination/experiments/EXP-04/` only when execution begins after approval.
- No production promotion, live-company effect, deploy, purchase, send, publish, payment, hiring or
  professional opinion is authorised by this document.

Founder approval freezes the sprint questions, first-wave fixtures, thresholds, model controls and
spend boundary. It authorises the smallest H1–H7 fixes needed to make the comparisons honest and then
the sparse activation sequence above—not every reserve workload.

## 16. Execution close — 25 August 2026

The frozen search stopped sparsely:

- H1 passed with a resumable hot model session across replaceable ACP processes. The draft's demand
  for one immortal process was an over-specified mechanism; continuity, actor isolation and explicit
  cold reconstruction were proved directly.
- H2, H3, H4 and H6 passed. H5 passed at Q2 and failed as infrastructure-invalid at Q4. H7 proved
  real redirect and ordinary batch-event coalescing but did not run the frozen policy-change cell.
- Sales Q2 did not cross the 25% throughput gate. Monitoring Q2 was flat. Q4/Q8 stopped.
- E1 consequence allocation lost: routine `low` preserved exact tail quality but was slower and
  dearer than uniform `high`.
- C1 was non-discriminating; observed closure cost exceeded a fixed 25% bucket in both inspected
  runs, so dynamic closure headroom is retained without a universal percentage.
- Q-SUPPORT, R4-S, M1, P1 and reserve workloads were not activated by their dependencies.

The measured result, retained fixes and exact branch dispositions are in
[`../coordination/experiments/EXP-04/t02-final-results.md`](../coordination/experiments/EXP-04/t02-final-results.md).
