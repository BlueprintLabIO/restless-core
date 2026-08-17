# Evaluation and Dogfood Specification

**Version:** 0.1  
**Status:** Working product and engineering contract  
**Scope:** How Restless proves that OrgIntel improves real company outcomes, how dogfoods are structured, how external reality is tested or observed, and how evidence feeds exploration, repair, and organisational evolution.

---

## 0. Document contract

This document defines Restless's empirical grounding system.

It does not define a generic benchmark platform or a permanent scoring ontology. It defines the minimum discipline required to answer:

> Does Restless produce better accepted economic outcomes with less owner attention than strong agents using the same models, tools, resources, and Linux environment without OrgIntel?

Related specifications:

- **OrgIntel Core Specification** defines the adaptive organisational loop.
- **Authority Plane Specification** defines real and mock effect/resource providers.
- **Company Runtime Specification v0.1** defines the productive work environment.
- **Owner Cockpit Specification v0.1** defines how evidence and attention are presented.
- **Company Lifecycle and Cross-Layer Contract v0.1** defines shared semantics and composition.

### Classification

- **Core contract:** must hold in the evaluation system.
- **Product hypothesis:** must be tested, not assumed.
- **Scenario default:** useful starting point, manually adjustable.
- **Example:** illustrative, not mandatory implementation scope.

---

# 1. Product claim and falsification

## 1.1 Core claim

Restless's differentiated value is not persistence, Linux access, multiple agents, or tool use by themselves.

The claim is:

> An Exec-led, evidence-seeking, self-exploring, self-repairing, and self-evolving organisation can sustain useful economic execution with less owner attention than unstructured agents.

## 1.2 North-star outcome

```text
accepted economic output
────────────────────────
owner attention + cost + time + risk
```

Do not collapse this into one opaque score in the product. It is a conceptual optimisation target.

## 1.3 Falsification

OrgIntel is not yet valuable if, under reasonably matched conditions:

- one strong agent consistently produces better accepted results;
- a few agents sharing Linux and a good prompt perform equally well with less overhead;
- owner intervention remains constant or increases;
- agents spend most of their time coordinating rather than creating;
- repeated runs do not improve from evidence;
- the system optimises task completion while missing real-world success.

A failed hypothesis is useful. It should reduce product scope or change OrgIntel design rather than be explained away by adding more machinery.

---

# 2. Evaluation principles

## 2.1 Outcomes over activity

The following are weak evidence by themselves:

- messages sent;
- tasks marked complete;
- agents spawned;
- plans generated;
- tokens consumed;
- meetings/reviews performed;
- files changed.

Strong evidence is a working, inspected, or externally validated result.

Examples:

- Cosmon has a playable browser build with the required loop.
- Aris receives real payment or qualified buying intent.
- Thymelake launches a restaurant and processes correct live orders.

## 2.2 External reality over self-report

Agent narratives are useful diagnostics, not final truth.

Prefer:

```text
real-world outcomes/provider records
→ executable tests and measured telemetry
→ customer or expert review
→ inspected artifacts
→ agent report/self-assessment
```

When strong evidence is unavailable, act using explicit assumptions, judgment, and bounded tests rather than pretending certainty.

## 2.3 Compare against credible baselines

Every major OrgIntel claim should be tested against simpler alternatives.

A system that works in isolation may still add no value.

## 2.4 Practical evidence, not laboratory theatre

Evaluation should be scientifically minded without blocking work until perfect data exists.

Use:

- clear hypotheses;
- predictions where possible;
- small informative tests;
- comparable baselines;
- explicit uncertainty;
- real artifacts and outcomes;
- repeated trials for unstable results.

Do not demand statistical significance for a one-off strategic decision where only one real attempt is feasible. Record the uncertainty and act proportionally.

## 2.5 Manual alignment before automation

New dogfood scenarios begin as manually reviewed operating stories.

The team aligns on:

- what genuine success looks like;
- what is deliberately flexible;
- what external evidence matters;
- which failures should be injected;
- what owner involvement is acceptable.

Automate stable mechanics only after the team understands the scenario.

## 2.6 Evals must change behaviour

Evaluation is useful only when it feeds:

- current plan changes;
- hypothesis updates;
- local repair;
- actor development;
- process/template evolution;
- tool or model selection;
- internal versus external delegation;
- stop, pivot, or expansion decisions.

---

# 3. Success contracts

## 3.1 Purpose

Every meaningful goal, milestone, department dogfood, and company dogfood has a lightweight success contract.

The contract defines the desired reality without prescribing every execution step.

## 3.2 Minimal success contract

```text
Title
Desired outcome
Accountable owner
Scope and exclusions
Operating phase
Time envelope
Cost/resource envelope
Owner-attention envelope
Required evidence
Sources of trust
Acceptance criteria
Important assumptions and unknowns
What would falsify the current approach
Continue / branch / pivot / stop criteria
```

## 3.3 Stable contract, flexible execution

The contract is relatively stable. The plan may change repeatedly.

Example:

```text
Outcome:
A playable Cosmon browser build proving exploration, encounter,
capture, and battle.

Flexible:
Engine, architecture, art method, team shape, exact mechanics,
and implementation sequence.

Not success:
A design document, backend architecture, or task board without a build.
```

## 3.4 Owner-attention envelope

Specify what the company should handle autonomously and when escalation is justified.

Example:

```text
Expected owner involvement:
- initial mandate;
- one mid-run product judgment if requested;
- final outcome review;
- authority expansion only if needed.

Failure condition:
Owner must repeatedly decompose tasks, chase agents, or resolve ordinary implementation choices.
```

## 3.5 Acceptance ownership

Acceptance has one named accountable evaluator:

- owner;
- Exec or domain lead;
- independent expert;
- customer;
- executable test suite;
- external provider record.

Several sources may contribute evidence, but responsibility for the final decision remains explicit.

---

# 4. Evidence model

## 4.1 Epistemic semantics

Evaluation uses OrgIntel's shared language:

| Type | Meaning |
|---|---|
| Observation | Directly measured, recorded, or witnessed information |
| Claim | Assertion about reality |
| Hypothesis | Testable claim about what is true or will work |
| Assumption | Untested premise temporarily used for action |
| Judgment | Interpretation/evaluation under incomplete evidence |
| Principle | Normative value or operating preference |
| Decision | Chosen course of action |
| Unknown | Explicit unresolved question |

A task report should not silently convert a hypothesis into a fact.

## 4.2 Evidence record

For consequential evidence, retain:

```text
evidence ID
type/source
claim or success criterion supported
locator/provider reference
observed_at
scope
freshness/review date where relevant
actor/session that recorded it
confidence or limitations
```

Most evidence remains a referenced file, test result, provider record, customer response, or artifact. Do not copy all content into OrgIntel.

## 4.3 Source-of-trust hierarchy

The hierarchy is contextual, but the default is:

### Tier 1: external outcomes

- payment settled;
- customer retained;
- restaurant live orders;
- email reply from a real prospect;
- public deployment reachable;
- provider receipt.

### Tier 2: executable/measured evidence

- tests pass;
- build runs;
- benchmark result;
- telemetry;
- reproducible experiment output.

### Tier 3: independent human/expert evidence

- customer interview;
- expert review;
- playtest observation;
- code/design review.

### Tier 4: inspected artifact

- report checked against sources;
- source code reviewed;
- UI manually exercised;
- question paper manually validated.

### Tier 5: agent narrative

- progress summary;
- self-rating;
- claim that work is complete.

Tier 5 can trigger inspection but rarely closes a high-value success contract alone.

## 4.4 Conflicting evidence

Do not average conflicting evidence into false certainty.

Record:

- which claims conflict;
- source quality and scope;
- plausible explanations;
- the next informative test;
- the decision made under remaining uncertainty.

## 4.5 Freshness

Evidence is time-scoped.

Examples:

- a customer reply remains historical truth;
- a market price may become stale quickly;
- a build link may stop working;
- an actor competence estimate should update over time.

Freshness should influence context assembly and decision review, not automatically erase evidence.

---

# 5. Evaluation levels

## 5.1 Artifact level

Question:

> Does the concrete output meet its acceptance criteria?

Examples:

- browser game loads and core loop functions;
- sales copy is accurate and usable;
- QR menu/order flow works;
- generated exam questions pass quality checks.

## 5.2 Work/process level

Question:

> Was the work performed effectively, or was it stalled, duplicated, wasteful, or repeatedly repaired?

Signals:

- elapsed time;
- blocker duration;
- rework;
- duplicated work;
- handoff failures;
- review loops;
- resource consumption;
- artifact progress.

## 5.3 Team/department level

Question:

> Did the team produce the intended operational or economic result?

Examples:

- Game Product Team ships playable build.
- Aris Sales & Marketing closes sales.
- Thymelake Restaurant Launch Team activates a venue.

## 5.4 Company level

Question:

> Is the organisation creating increasing accepted value relative to owner attention, cost, time, and risk?

## 5.5 OrgIntel/system level

Question:

> Did OrgIntel improve performance over simpler agent arrangements, and did it adapt from evidence?

This includes:

- decomposition quality;
- team-shape quality;
- context relevance;
- communication overhead;
- recovery quality;
- exploration quality;
- organisational learning reuse.

---

# 6. Metrics

## 6.1 Primary metrics

### Accepted outcome

Binary or graded assessment of whether the success contract was met.

### Owner attention

Capture:

- active owner minutes;
- number of interventions;
- type of intervention;
- whether the intervention was genuinely high-level or harness-caused.

### Total economic cost

Include where material:

- model cost;
- external service spend;
- compute;
- contractor/vendor spend;
- human review time.

### Time to useful output

Measure from accepted mission to first useful artifact and to accepted final outcome.

## 6.2 Supporting metrics

- outcome quality;
- external conversion/usage/retention;
- recovery success after failure;
- time blocked;
- duplicate/abandoned work;
- rework rate;
- number and cost of exploration branches;
- communication/coordination overhead;
- percentage of work requiring owner rescue;
- harness-caused failures;
- resource utilisation where relevant;
- improvement on repeated similar runs.

## 6.3 Owner intervention taxonomy

Classify interventions:

```text
mandate/strategy judgment
capital or authority decision
product/taste judgment
external relationship/human-only action
ordinary task decomposition
agent chasing/status request
repairing harness failure
correcting false or unsupported claim
```

The first four may be legitimate owner work. The latter four are stronger signals of product failure.

## 6.4 Avoid one magic score

Do not hide trade-offs in a single number.

A run may be:

- higher quality but too expensive;
- faster but require more owner attention;
- cheaper but fail external validation;
- successful once but not repeatable.

Show the dimensions and explain the judgment.

---

# 7. Baseline design

## 7.1 Required baselines

### A. Strong single agent

One capable model/agent receives:

- the same mission;
- the same Linux environment and tools;
- comparable model/cost budget;
- direct owner interaction;
- no OrgIntel team machinery.

### B. Minimally coordinated multi-agent

Several agents share the environment with only basic prompting or manual task assignment.

No persistent Exec-led organisational adaptation beyond what the harness naturally provides.

### C. Restless OrgIntel

One persistent Exec, focused actors, adaptive planning, explicit Work nodes, artifact-centred handoffs, evidence-based evaluation, repair, and evolution.

## 7.2 Fairness controls

Where feasible, match:

- underlying model families;
- tool access;
- starting artifacts;
- time and spend envelope;
- owner availability;
- external-world conditions;
- task difficulty.

Do not give Restless more budget and then attribute performance solely to organisational intelligence.

## 7.3 Best-member test

A multi-agent system should be compared not only to average single-agent performance but to its strongest individual contributor.

A team that dilutes the best available answer through consensus is not intelligent merely because it collaborated.

## 7.4 Repeated trials

Use repeated trials when:

- model variance is high;
- controlled scenario inputs are stochastic;
- outcomes are sensitive to initial branch choices;
- a result would influence a major architecture decision.

One real customer sale may still be decisive evidence of feasibility even without repeated statistical trials. Interpret results in context.

## 7.5 Baseline modes in one harness

Prefer the same scenario runner, starting files, installed tools and external limits with configurable organisation mode:

```text
single_agent
minimal_team
orgintel
```

This reduces accidental differences between evaluations.

---

# 8. Experiment design

## 8.1 Lightweight experimental contract

For an OrgIntel hypothesis:

```text
Question
Hypothesis
Prediction
Baseline/comparison
Smallest informative scenario
Controlled variables where practical
Outcome metrics
Failure/stop criteria
Result
Decision
Limitations
```

## 8.2 Test one important claim at a time

Examples:

- Exec-led decomposition reduces duplicate work.
- Focused local context outperforms full transcript context.
- Producer–critic improves artifact quality enough to justify cost.
- Local repair avoids owner escalation.
- Persistent actor identity improves style consistency across sessions.

Avoid changing team structure, model, prompt, context system, and toolset simultaneously when the goal is to learn which mechanism helped.

## 8.3 Smallest informative test

Before running a week-long company scenario, ask whether a shorter bounded experiment can reject the hypothesis.

Examples:

- Compare two context packets on one code-review task.
- Inject one stalled worker and test recovery.
- Run one sales follow-up loop before building a complete CRM integration.

## 8.4 Decision under limited evidence

When resources are constrained:

1. State the assumption.
2. Estimate consequence and reversibility.
3. Choose the cheapest informative action.
4. Preserve evidence.
5. Revisit when new information arrives.

This is scientific thinking adapted to company operation, not academic perfectionism.

---

# 9. Throwaway-company effect and behavioural tests

## 9.1 Purpose

The `_test` company allows end-to-end testing of:

- organisational behaviour;
- Authority Plane policy and receipts;
- installed-tool delays and failures;
- handling of explicitly controlled customer/market input;
- recovery and reconciliation;
- owner approvals.

Restless uses the same generic governed-process envelope and receipt for a fake CLI and an installed
real CLI. It does not implement matching fake and real provider interfaces.

## 9.2 Architecture

```text
Company Runtime / Exec
        ↓
Authority Plane generic effect runner
        ↓
fake CLI in `_test` or installed real CLI
        ↓
tool outcome and external status observation
```

OrgIntel reasons from labelled receipts and observations; `_test` evidence never enters a live company.

## 9.3 Two test-input types

### Deterministic fake CLI

Used for correctness and recovery:

- success;
- confirmed failure;
- staging or launch failure before execution;
- daemon interruption after durable intent but before receipt;
- duplicate request;
- delayed approval;
- budget exhaustion;
- separate status observation and reconciliation to confirmed success/failure.

Runs are reproducible from the CLI fixture, argv and idempotency key.

### Controlled behavioural input

Used to test strategy and adaptation:

- customer personas;
- differing demand by segment;
- replies and objections;
- conversion probabilities;
- noisy feedback;
- changing requirements;
- vendor/contractor performance.

It may use seeded files, messages, scripted humans, or explicitly labelled LLM-generated examples.

Its output is mechanism input, not proof of real market demand.

## 9.4 World secrecy

Agents should see only the world through the same channels available in real operation.

They should not read hidden scenario rules, future events, conversion probabilities, or expected solution paths.

## 9.5 Initial tool probes

- generic fake CLI success/failure/retry/replay;
- interrupted unknown outcome plus separate status receipt and reconciliation;
- installed email CLI help and dry-run with a clearly marked local attachment;
- installed deployment and Git tool help/status/dry-run where supported;
- deterministic owner approval and budget denials.

Do not build a Restless adapter for these tools.

## 9.6 Simulated versus real progression

```text
deterministic fake CLI in `_test`
→ real tool help/dry-run/status probe
→ controlled real dogfood
→ wider real operation
```

Do not remain in controlled inputs once real external validation is safe and affordable.

### 9.6.1 Simulation manufactures beliefs, not just missing evidence

**Core contract.** Added 15 August 2026 from three escalating incidents in one
sprint. The earlier framing — simulation tests operating behaviour, only reality
validates demand — is true and **far too weak**. It describes an absence. What
actually happens is production of false fact:

> A simulated capability emits outcomes. A company records those outcomes as
> evidence, reasons from them correctly, and builds durable strategy on them.
> Nothing internal can distinguish a simulated fact from a real one, because the
> whole point of the interface is that they are identical.

Observed, in ascending severity:

1. A synthetic webhook injected to test an ingress became *"the strongest single
   demand signal so far"* in a real hypothesis file.
2. A simulated `web.deploy` produced a 404 that a company chased for **three
   wakes** as a real blocker.
3. Six wakes of well-cited, self-correcting commercial work — segment analysis,
   an offer, price points, a channel plan, a sample artifact — turned out to be
   about **the wrong country, the wrong exam and the wrong business model.** The
   company's reasoning was sound throughout. Only contact with the real
   repository revealed it.

The company is not at fault in any of these, and better prompting would not have
helped: **the failure is epistemic, not cognitive.**

Three rules follow.

1. **Never exercise a fake capability inside a live company.** Fake tools and controlled behavioural
   input belong in `_test` companies, whose purpose is therefore not safety but
   *keeping fiction out of a live company's evidence base*.
2. **Test outcomes must be distinguishable in the record.** A receipt says
   which world produced it; reconciliation and any owner-facing summary must
   carry that distinction rather than flattening it.
3. **Give a company something real to check itself against, early and
   continuously.** A repository, a live URL, a provider's own record. One wake
   with the real repository corrected more false belief than six wakes of
   simulated selling produced true belief.

The progression above is still right about *order*. It is wrong if read as
"a controlled scenario is a safe place to accumulate knowledge." It is a safe place to
exercise **mechanism**, and an unsafe place to accumulate **facts**.

---

# 10. Dogfood scenario package

## 10.1 Required contents

Each scenario package contains:

```text
scenario identity/version
company/department context
starting mission
starting files and tools
initial actors or actor-creation constraints
authority/resource envelope
success contract
external observation setup and installed/fake tool inventory
hidden controlled-input state where applicable
possible failure injections
manual review instructions
termination conditions
expected evidence bundle
```

## 10.2 Non-prescriptive design

A scenario defines:

- desired reality;
- constraints;
- available resources;
- world behaviour;
- evidence requirements.

It does not hard-code the exact plan, team, or workflow the company must use.

## 10.3 Scenario versions

Version scenarios so results remain interpretable.

Changing customer behaviour, starting assets, budget, or acceptance criteria creates a new version rather than silently altering historical comparisons.

## 10.4 Difficulty profiles

A scenario may have:

- **smoke:** fast, deterministic, mechanical;
- **standard:** realistic ambiguity and moderate failure;
- **stress:** adversarial timing, incomplete information, multiple failures;
- **real:** live external environment and real consequences within limits.

The smoke scenario proves integration, not economic viability.

---

# 11. Cosmon dogfood — building

## 11.1 First department

**Game Product Team**

Possible accountable roles:

- Exec/studio lead;
- game/design lead;
- gameplay engineer;
- 3D technical artist;
- browser/platform engineer;
- independent reviewer only when useful.

## 11.2 Core mission

> Produce a working browser game that proves the exploration, creature encounter, capture, and battle loop.

## 11.3 Success contract

Required evidence:

- playable browser URL or reproducible local build;
- exploration movement works;
- at least one creature encounter;
- capture mechanic;
- basic battle mechanic;
- run/build instructions;
- source and meaningful Git checkpoint;
- concise account of unresolved risks.

Not required initially:

- MMO-scale backend;
- large content library;
- polished monetisation;
- production live operations;
- full player-research programme before a game exists.

## 11.4 What this tests

- ambiguous creative/technical decomposition;
- cross-disciplinary team formation;
- parallel Git/worktree use;
- actual code and asset integration;
- changing requirements;
- build/test evidence;
- recovery from agent failure;
- avoiding premature infrastructure.

## 11.5 Example failure injections

- gameplay worker process crashes with uncommitted work;
- game engine/library choice fails in browser;
- art asset format blocks implementation;
- owner changes one core design constraint mid-run;
- two workers modify the same system;
- a GPU request is delayed or denied;
- the first playable build fails acceptance despite completed Work nodes.

## 11.6 External validation progression

1. Build runs under executable checks.
2. Independent internal reviewer plays it.
3. Small human playtest after the core loop exists.
4. Real retention/interest experiments only after sufficient product exists.

---

# 12. Aris dogfood — selling

## 12.1 First department

**Sales & Marketing**

Aris already has the ability to generate selective-exam practice questions/papers. The initial bottleneck is distribution and willingness to pay.

## 12.2 Core mission

> Sell selective-exam practice papers to real parents, students, tutors, or coaching centres, and learn which segment, offer, and channel converts.

## 12.3 Success contract

Possible first contract:

- select one or more credible segments;
- create a truthful offer and sales assets;
- conduct bounded outreach/campaigns;
- achieve first paid sale, repeat purchase, or clearly qualified buying pipeline;
- deliver the promised paper/product;
- capture objections, use, trust, and quality feedback;
- recommend next segment/offer/product improvement.

## 12.4 Required sources of trust

Prefer:

- payment/provider record;
- real reply/demo booking;
- product usage/download;
- repeat purchase;
- customer interview;
- human-reviewed paper quality.

Email volume or landing-page creation alone is not success.

## 12.5 What this tests

- segment hypothesis branching;
- prospect research;
- messaging and offer iteration;
- CRM/follow-up continuity;
- email/payment effects;
- bounded marketing spend;
- external evidence changing the product;
- repeated workflow promotion into skills/tools.

## 12.6 Example failure injections

- one segment does not reply;
- reply indicates low trust in question quality;
- campaign execution returns unknown outcome;
- payment succeeds but runtime later restores;
- follow-up owner becomes unavailable;
- initial offer generates interest but no payment;
- budget ceiling prevents further ads.

## 12.7 Controlled-input caution

Controlled sales inputs prove organisational behaviour and effect correctness, not real demand. Progress to controlled real outreach early.

---

# 13. Thymelake dogfood — B2B launch and operation

## 13.1 First department

**Restaurant Launch Team**

This team crosses sales, onboarding, product, and operations.

## 13.2 Core mission

> Acquire a restaurant, configure its real menu, launch QR ordering at real tables, process orders reliably, and prove that the venue wants to continue using or paying for the system.

## 13.3 Proof stages

### Pilot viability

- restaurant agrees to pilot;
- menu and venue are configured;
- QR flow is understandable;
- orders arrive correctly and promptly;
- staff can use the system;
- common failures can be handled;
- venue perceives enough value to continue.

### Repeatability

- second/third venue can launch without bespoke engineering;
- menu import/setup time falls;
- common issues become product fixes or playbooks;
- support burden is understood.

### Economic value

Evidence of one or more:

- reduced staff workload;
- improved order accuracy;
- increased basket size;
- faster ordering/table turnover;
- paid continuation;
- retention/active usage.

## 13.4 What this tests

- cross-functional handoffs;
- real customer relationship continuity;
- product changes driven by operations;
- deployment and account capabilities;
- support/recovery loops;
- external partner coordination;
- live outcome measurement;
- internal versus outsourced work decisions.

## 13.5 Example failure injections

- menu import is inaccurate;
- deployment succeeds but response is lost;
- restaurant changes requirements late;
- staff training is missed;
- orders fail during service;
- customer asks for unsupported integration;
- owner must decide whether to fund bespoke work;
- Runtime is restored after the venue is already live.

---

# 14. Cross-company test portfolio

The three companies intentionally stress different value chains:

| Company | First proof | OrgIntel stress |
|---|---|---|
| Cosmon | Build an integrated playable product | Creative/technical collaboration and delivery |
| Aris | Sell an existing product | External demand, pipeline, follow-up, and learning |
| Thymelake | Sell, deploy, and operate a B2B product | Cross-functional continuity and live operations |

Together they test:

- building;
- selling;
- onboarding;
- operating;
- recovering;
- adapting from external evidence.

A later cross-company or cross-department scenario is useful only after individual operating loops work.

---

# 15. Failure-injection catalogue

## 15.1 Runtime and process

- worker crash;
- Exec crash;
- bridge disconnect;
- runtime restart;
- runtime restore to older snapshot;
- disk/resource pressure;
- tool/package breakage;
- missing artifact.

## 15.2 OrgIntel

- missed wakeup;
- duplicate message;
- stale Work;
- overloaded actor;
- duplicated work;
- review loop without improvement;
- poor team pattern;
- weak or bloated context;
- false completion claim.

## 15.3 Authority Plane and providers

- approval delayed;
- hard budget ceiling;
- credential revoked;
- provider unavailable;
- confirmed failure;
- unknown effect outcome;
- duplicate effect request;
- resource provisioning delay/failure;
- external freeze.

## 15.4 Business/world

- changing requirement;
- weak customer demand;
- contradictory feedback;
- strong interest but no payment;
- contractor/vendor underperforms;
- unexpected opportunity;
- reputational or compliance concern;
- key assumption disproved.

## 15.5 Failure-injection rule

Inject failures that test a real product claim. Do not create elaborate chaos merely to increase scenario difficulty.

---

# 16. Measuring exploration, repair, and evolution

## 16.1 Exploration quality

Evaluate whether the organisation:

- identifies material unknowns;
- generates credible alternatives;
- avoids branching on trivial choices;
- allocates bounded resources;
- defines discriminating evidence;
- kills weak branches;
- expands promising branches;
- converges when enough evidence exists.

Bad exploration produces many agents and artifacts without better decisions.

## 16.2 Repair quality

Evaluate whether the organisation:

- detects local deviation early;
- diagnoses the likely bottleneck;
- preserves useful work;
- applies the smallest effective intervention;
- avoids freezing unrelated work;
- checks whether the intervention helped;
- escalates only when local repair is insufficient.

## 16.3 Evolution quality

Evaluate whether repeated evidence changes:

- actor responsibilities and context;
- team patterns;
- playbooks and skills;
- tool/model choice;
- resource allocation;
- internal versus external delegation;
- company structure.

A claimed learning is weak until it changes a later decision or improves a later run.

## 16.4 Reuse test

For repeated or analogous scenarios, ask:

- Did the organisation retrieve the prior lesson?
- Did it apply the lesson appropriately rather than mechanically?
- Did performance improve?
- Was the old lesson revised when circumstances differed?

---

# 17. Actor and team evaluation

## 17.1 Evidence-based competence

Actor profiles may accumulate evidence about:

- task types completed;
- quality of accepted outputs;
- reliability and cost;
- useful collaborators;
- repeated failure modes;
- effective tools/models;
- domain taste and style consistency.

Do not reduce an employee to one permanent scalar score.

## 17.2 Avoid lock-in from early failures

Competence beliefs remain uncertain. Actors should sometimes receive fresh bounded trials, especially after:

- model changes;
- improved context;
- new tools;
- role redesign;
- evidence that prior conditions were unfair.

## 17.3 Team-pattern evaluation

For each pattern, compare:

- outcome quality;
- latency;
- cost;
- coordination overhead;
- recovery behaviour.

Patterns include:

- single owner;
- parallel exploration;
- producer–critic;
- pairing;
- specialist pipeline;
- Exec synthesis;
- recovery swarm.

A pattern remains a default only while it produces better outcomes in the relevant work shape.

---

# 18. Dogfood operating cadence

## 18.1 Run loop

```text
select company outcome
→ align success contract manually
→ choose baseline/OrgIntel mode
→ run company
→ collect evidence and owner attention
→ inspect artifacts/external outcomes
→ conduct concise after-action review
→ identify smallest useful product/process change
→ rerun or progress to reality
```

## 18.2 Review questions

1. What useful result exists now that did not exist before?
2. Does external or executable evidence support it?
3. Where did the owner intervene, and why?
4. What failure came from the business problem versus Restless itself?
5. Did OrgIntel explore, repair, or evolve effectively?
6. What should be deleted, simplified, or changed?
7. What is the next smallest informative run?

## 18.3 Outcome backlog and friction backlog

Maintain two distinct backlogs:

### Outcome backlog

Real company results to produce next.

### Observed-friction backlog

Problems seen in dogfood:

- dormant Exec;
- poor context;
- Git conflict;
- missing recovery;
- owner attention spam;
- effect ambiguity;
- weak evidence.

Engineering work should connect to a real observed friction or a required outcome, not merely architecture elegance.

## 18.4 No permanent fixture worship

Scenarios evolve as the product and companies evolve. A green historical fixture is not evidence that current companies succeed.

Retain a small regression set for critical mechanics, but keep real dogfood central.

---

# 19. Result report

Each evaluated run should produce a concise report:

```text
Scenario/version
Organisation mode
Models/tools/resources
Success contract
Outcome and acceptance decision
Evidence bundle
Owner attention
Cost and elapsed time
Major exploration branches
Failures and repairs
Organisational changes/lessons
Comparison with baseline
Limitations
Next decision
```

Reports should link to real artifacts and provider records rather than embed all output.

---

# 20. Evaluation data ownership

| Data | Authoritative owner |
|---|---|
| Success contract, goals, hypotheses, decisions, organisational lessons | OrgIntel |
| Artifacts, tests, builds, raw experiment data | Company Runtime/domain applications |
| Effects, receipts, resource usage, provider outcomes | Authority Plane/external provider |
| Owner interactions and attention items | Source-owning service; cockpit projection |
| Scenario configuration, hidden controlled-input state, baseline mode | Evaluation harness |
| Final run report | Evaluation harness with references to source-owned evidence |

The evaluation harness must not become a second company database.

---

# 21. Minimal evaluation harness

## 21.1 Required V0 capabilities

- load a versioned scenario package;
- choose `single_agent`, `minimal_team`, or `orgintel` mode;
- provision the same starting Runtime and Authority envelope;
- install the deterministic fake CLI and configure fault modes only in `_test`;
- record timestamps, cost, owner interactions, and key cross-layer events;
- collect artifact/evidence references;
- present a manual acceptance checklist;
- generate a comparable run report;
- reset/reseed the scenario safely.

## 21.2 Manual acceptance remains first-class

For creative and business outcomes, a human may provide final acceptance. The harness should make this inspection easy and record the decision/rationale.

Do not replace product judgment with a superficial automatic score.

## 21.3 Instrumentation scope

Capture enough to explain performance:

- actor/session lifecycle;
- Work nodes and blockers;
- meaningful messages/handoffs;
- artifact references;
- model/external spend;
- effects and resources;
- owner interventions;
- failure/recovery events.

Do not permanently capture private chain-of-thought or every shell/file action as evaluation state.

---

# 22. V0 acceptance scenarios for the harness

## 22.1 Baseline comparability

The same Cosmon smoke scenario runs in all three organisation modes with the same starting files, model budget, and time envelope.

The report makes differences explicit.

## 22.2 External unknown outcome

An Aris campaign send succeeds but returns no response.

- Authority Plane records unknown outcome.
- Company does not blindly repeat it.
- Reconciliation resolves the effect.
- Run report captures the recovery.

## 22.3 Runtime restore

A Thymelake deployment occurs, then the Runtime restores an earlier snapshot.

- external state and receipts remain current;
- OrgIntel goals/learning remain current;
- company reconciles instead of repeating deployment/onboarding;
- evaluation records whether owner intervention was required.

## 22.4 Local repair

A Cosmon worker stalls.

- OrgIntel detects deviation;
- preserves existing artifacts;
- changes scope, actor, tool, or teamwork pattern;
- verifies improvement;
- owner is not asked to micromanage unless local repair fails.

## 22.5 Evidence correction

An agent claims a success criterion is met, but executable inspection fails.

- evaluation rejects completion;
- OrgIntel updates the claim/plan;
- actor/process learning is recorded without treating the incident as a security violation.

## 22.6 Repeated-learning test

Run a related second scenario.

- prior lesson is available;
- company applies or explicitly rejects it based on context;
- report compares performance and explains whether evolution helped.

---

# 23. Implementation sequence

1. Define the success-contract format as readable Markdown plus minimal structured metadata.
2. Implement scenario package loading and versioning.
3. Add organisation modes for single agent, minimal team, and OrgIntel.
4. Add one deterministic fake CLI exercised through the generic Authority Plane effect runner in `_test`.
5. Capture owner attention and core cost/time metrics.
6. Implement evidence/artifact reference collection.
7. Build Cosmon smoke and standard scenarios.
8. Build an Aris `_test` mechanism scenario, then a controlled-real scenario.
9. Build Thymelake pilot scenario.
10. Add failure injection and restore/reconciliation tests.
11. Add repeated-learning comparisons.
12. Improve metrics only when a real decision needs them.

---

# 24. Explicit exclusions

V0 does not require:

- a universal benchmark leaderboard;
- one aggregate intelligence score;
- automatic grading of every business outcome;
- production-scale synthetic market;
- statistically powered trials for every decision;
- hidden chain-of-thought collection;
- exhaustive event sourcing;
- perfect cost attribution;
- multiplayer evaluation;
- cross-tenant benchmarking;
- a general synthetic economy;
- replacing real customer validation with LLM personas.

---

# 25. Anti-gaming and anti-drift rules

1. Completing internal tasks is not a substitute for meeting the success contract.
2. Agent self-assessment cannot be the sole evidence for material success.
3. Baselines must receive credible tools, models, budgets, and time.
4. Do not change the scenario after seeing a poor result without versioning it.
5. Do not add evaluation metrics merely because they are easy to capture.
6. Do not optimise one proxy while external outcomes deteriorate.
7. Controlled `_test` inputs validate behaviour, not real market demand.
8. An eval that never changes product or organisational decisions is ceremony.
9. A new OrgIntel mechanism must beat a simpler alternative on a real scenario.
10. Preserve failures and uncertainty honestly; do not reinterpret every run as success.

---

# 26. Final V0 contract

Restless evaluation is successful when:

> Each company run begins with a clear but flexible success contract, produces inspectable or externally grounded evidence, measures owner attention and economic cost, compares OrgIntel against credible simpler baselines, and feeds the result back into exploration, local repair, and organisational evolution.

The ultimate test is not whether Restless appears organised.

It is whether Cosmon builds, Aris sells, and Thymelake launches and operates—with less owner effort than the simpler alternatives.
