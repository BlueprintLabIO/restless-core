# OrgIntel Core Specification

**Status:** Working source of truth  
**Version:** 0.3  
**Date:** 23 August 2026
**Parent:** ARCHITECTURE.md — Restless Architecture Source of Truth v0.9  
**Supersedes:** OrgIntel Core Specification for core product and implementation direction

---

## 0. Document contract

This document uses four labels:

- **Core contract** — implementation must preserve this.
- **Product hypothesis** — dogfood must test this; it may be wrong.
- **Default pattern** — recommended and overridable.
- **Example** — illustrative, not implementation scope.

The old v0.1 document mixed product thesis, organisational theory, schema design, APIs, dogfood, and UI. This version keeps the core contract, moves detailed implementation choices out of the conceptual centre, and separates durable organisational continuity from the writable work sandbox.

The coding principle is:

> **Build the smallest organisation that can explore, execute, repair, and evolve. Add durable structure only when observed work requires it.**

---

# 1. Product definition

## 1.1 Claim

**Core contract**

OrgIntel is the differentiated organisational-intelligence layer of Restless.

A generic harness gives models tools, memory, and a computer. OrgIntel should make those models behave as a persistent organisation that can pursue ambiguous economic goals over days, months, and years.

> **An Exec-led agent organisation should produce more accepted economic output with less owner attention than either one strong agent or several agents sharing the same Linux machine without organisational intelligence.**

OrgIntel is not primarily a task tracker, workflow engine, multi-agent chat system, or safety layer.

Its actual intelligence is:

1. **Exploration** — branch toward promising hypotheses and approaches.
2. **Execution** — advance the current best-supported direction.
3. **Repair** — detect and correct local bottlenecks without losing useful work.
4. **Evolution** — improve actors, tools, processes, delegation, and organisational structure from real outcomes.

Goals, Work nodes, messages, schedules, and context management are supporting mechanisms.

## 1.2 Success and falsification

**Core contract**

Primary metric:

> **Accepted economic output per unit of owner attention, cost, time, and bounded risk.**

Supporting measures:

- owner minutes and interventions per outcome;
- elapsed time to first useful artifact;
- model, compute, and external-service cost;
- time dormant, blocked, duplicating work, or repeating failed approaches;
- recovery success after actor, process, or tool failure;
- consistency across sessions and model changes;
- improvement across comparable runs;
- whether the organisation beats its strongest individual actor.

**Product hypothesis**

Serious dogfoods should compare:

- **A. Single agent:** one strong agent with Linux and tools.
- **B. Loose team:** several agents with minimal coordination.
- **C. OrgIntel:** one available Exec routing work through accountable team leads, focused workers,
  adaptive loops, explicit ownership, concrete artifacts, and proportionate review.

If C does not improve outcomes or owner attention enough to justify its overhead, OrgIntel is not yet differentiated.

## 1.3 Boundaries

**Core contract**

OrgIntel is one logical product layer deployed across two failure domains:

- the **durable OrgIntel service and database** live outside the writable work sandbox but inside the per-company deployment;
- the **Exec, planners, critics, recovery actors, and workers** run inside the Company Runtime, where they can inspect and change real work.

OrgIntel is not the Authority Kernel and is not the final security boundary.

```text
Owner
  │ mandate, capital, exceptional judgment
  ▼
┌──────────────── PER-COMPANY DEPLOYMENT ────────────────┐
│                                                       │
│ Authority Kernel                                     │
│ capabilities · budgets · approvals · secrets          │
│ external effects · receipts · stop/recovery           │
│                                                       │
│ OrgIntel service + OrgIntel database                  │
│ actors · goals · Work nodes · inboxes · schedules    │
│ decisions · learning · context assembly · continuity  │
│                         │                             │
│                         │ authenticated coordination  │
│                         ▼                             │
│  ┌────────────── COMPANY RUNTIME SANDBOX ──────────┐  │
│  │ runtime bridge                                  │  │
│  │ Exec / planner / critic ACP processes           │  │
│  │ worker ACP processes                            │  │
│  │ files · Git · browser · tools · project systems │  │
│  └─────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────┘
```

The separation exists to preserve organisational continuity when the work machine is corrupted, replaced, or restored. It must not turn OrgIntel into a remote workflow controller.

OrgIntel owns the current coordination picture:

- durable actors, roles, profiles, and sessions;
- goals and Work nodes;
- messages, schedules, decisions, hypotheses, and learning;
- references to artifacts and external effects;
- context packages, memory indexes, health signals, and process defaults.

The Company Runtime owns actual productive state:

- source code, documents, assets, builds, and working files;
- Git repositories and worktrees;
- browser profiles and sessions;
- project databases, services, and domain applications;
- temporary actor scratch state and active experiments.

The Authority Kernel owns root mandate and consequential authority:

- capabilities, budgets, approvals, secret references, external effects, receipts, lifecycle, and snapshot authority.

Agents must still be able to do ordinary internal work through Linux if OrgIntel is temporarily unavailable.

---

# 2. Operating model

## 2.1 Durable actors, replaceable sessions

**Core contract**

| Concept | Meaning | Lifetime |
|---|---|---|
| **Owner** | Root source of mission, capital, and authority | Persistent |
| **Exec** | Accountable organisational leader | Persistent identity |
| **Actor** | Durable organisational identity with history, style, and responsibilities | Months or years |
| **Role** | Current responsibilities and decision rights | Evolves |
| **Session** | Active or resumable model process | Minutes to days |
| **Model** | Replaceable cognitive provider/configuration | Task-dependent |
| **Principal** | Identity recognised by the Authority Kernel | Capability-dependent |

A Game Director can remain the same actor while its model changes, processes restart, and old transcripts are discarded.

Company initialisation creates the standing Owner and singleton Exec identities before the first
message, wake or Staff commission. Daemon startup repairs either missing row for restored or legacy
state. Their existence is a company lifecycle fact; it must not depend on an incidental `tell` path.

**Default pattern**

Start with:

```text
one owner
one persistent Exec identity
one or more standing or temporary accountable leads
zero to four concurrent task workers per active lead when justified
on-demand planners, critics, and recovery sessions
```

Create persistent roles only when recurring responsibility and continuity improve outcomes.

## 2.2 Exec responsibility

**Core contract**

The Exec is accountable for internal company coherence. It may:

- interpret the owner mandate;
- choose goals and milestones;
- allocate resources and form teams;
- assign, pause, reassign, or abandon work;
- select and change teamwork or process patterns;
- request reviews and resolve internal disagreements;
- modify internal prompts, tools, roles, and processes;
- request Authority Kernel effects;
- delegate narrower authority when allowed;
- escalate decisions that exceed its authority.

It cannot increase total company authority, approve its own expansion, rewrite kernel policy, or alter authoritative receipts.

The Exec is a continuously available dispatcher across parallel departments. Every owner request that
requires productive execution is delegated to exactly one accountable team lead. The lead may be a
standing department lead or be appointed for that outcome; the Exec never substitutes itself as the
producer or integrator when no lead already exists. The lead owns decomposition, direct production or
Staff delegation, canonical integration, native review preparation and completion judgement.

After dispatch the Exec quiesces rather than waiting, polling or joining production. It wakes for a
new owner request or a material callback requiring portfolio prioritisation, cross-department
arbitration, resource reallocation, authority escalation or company-level judgement. This preserves
executive availability while departments continue concurrently.

## 2.3 How change works

**Core contract**

> **Anyone may observe and propose. The nearest accountable owner decides. The capable actor or team implements. Escalate only when authority or blast radius exceeds that owner.**

For local, reversible work, proposer, decider, and implementer may be the same actor.

Review and separation increase with external consequence, irreversibility, cost, uncertainty, cross-team impact, and difficulty detecting failure.

---

# 3. The adaptive intelligence loop

## 3.1 One loop, four modes

**Core contract**

```text
Sense
  → Frame
  → Choose or branch
  → Execute
  → Evaluate
  → Repair locally
  → Evolve when warranted
  → Repeat
```

This is a thinking model, not a mandatory workflow state machine. Real work may skip, combine, or revisit stages.

The four modes are:

- **Exploration:** test alternative beliefs or approaches.
- **Execution:** advance the current best-supported direction.
- **Repair:** respond to a local deviation or bottleneck.
- **Evolution:** make a durable organisational change.

### Sense

Gather the smallest useful picture of reality: external outcomes, artifact and test results, customer behaviour, current Work nodes, blockers, resource use, and relevant environmental changes.

Agent narration is weak evidence unless supported by artifacts or observations.

### Frame

Make explicit:

- the desired outcome;
- what is observed or believed;
- assumptions and unknowns;
- the most decision-relevant uncertainty;
- likely failure modes;
- decision ownership.

Framing should reduce ambiguity enough to act, not eliminate uncertainty.

### Choose or branch

Choose whether to proceed, test a cheap alternative, gather one missing signal, compare approaches, defer, kill, or escalate.

### Execute

Work happens through normal Linux tools, files, Git, browsers, applications, ACP workers, and external specialists.

### Evaluate

Compare expected and actual results:

- Did the intended outcome occur?
- Was the artifact accepted or useful?
- What did it cost?
- Which assumptions survived?
- Is more work worth the opportunity cost?

### Repair locally

Preserve work and use the smallest likely intervention: clarify, narrow, reassign, switch tool/model, add a specialist, change team shape, combine duplicate work, or stop a weak branch.

### Evolve when warranted

Repeated evidence may justify changing actors, context, processes, tools, team structure, external delegation, or strategy.

One incident should usually create a hypothesis, not a permanent rule.

## 3.2 Self-exploration

**Product hypothesis**

A capable organisation should maintain a bounded portfolio of plausible hypotheses rather than spend everything on the first plan generated.

```text
question
→ hypothesis
→ prediction
→ cheapest informative experiment
→ observation
→ expand, revise, combine, or kill
```

**Default pattern**

A meaningful exploration record contains:

- question and hypothesis;
- prediction;
- evidence that would support or weaken it;
- owner;
- time, cost, or concurrency budget;
- output or measurement;
- stop and expansion criteria;
- result and decision.

It may begin as a readable file with lightweight metadata. Do not build an experiment state machine in V0.

Most resources should normally advance the current best approach; a bounded minority may test credible alternatives. The balance depends on uncertainty, reversibility, runway, test cost, and cost of being wrong.

Exploration must converge. An expanding swarm of unresolved branches is failure.

## 3.3 Self-repair

**Product hypothesis**

Detecting and repairing local failures should outperform trying to make every bad internal state impossible.

Useful signals include:

- no useful artifact appears within an expected window;
- an actor repeats the same failed approach;
- one dependency blocks several Work nodes;
- review loops without material improvement;
- overload, unresponsiveness, or duplicated work;
- rising cost with weak progress;
- unacknowledged handoffs;
- drift from the actual outcome;
- high activity but only coordination output.

**Default pattern**

```text
detect deviation
→ diagnose likely bottleneck
→ choose smallest intervention
→ preserve existing work
→ observe the result
→ escalate only if unresolved
```

A recovery session may receive the original outcome, attempted approaches, artifacts, failure evidence, dependencies, remaining budget, and authority limits. It should propose the smallest credible recovery rather than restart by default.

## 3.4 Self-evolution

**Product hypothesis**

OrgIntel should improve the organisation itself from real outcomes.

Evolution may target:

- actor responsibilities, context, and model;
- teamwork or process defaults;
- internal tools and services;
- team composition and decision rights;
- resource allocation;
- whether work is done internally, automated, purchased, or outsourced.

**Default pattern**

```text
Observed problem:
Proposed change:
Why it may help:
Predicted observable effect:
Scope and budget:
Baseline or comparison:
Result:
Adopt, revise, or revert:
```

Formal statistical proof is not required. The prediction must be explicit enough to learn from being wrong.

Promotion ladder:

```text
observation
→ hypothesis
→ bounded trial
→ repeated useful evidence
→ recommended playbook
→ optional helper tooling
→ stable structure only when justified
```

The reverse path must remain possible: weaken, revise, or remove defaults that stop helping.

### Actor development

Persistent actors may accumulate evidence about strengths, weaknesses, accepted work, effective collaborators, models, tools, cost, reliability, style, and recurring failure modes.

OrgIntel may then specialise or broaden roles, change context or model, pair complementary actors, reduce weak responsibilities, promote leads, or retire obsolete roles.

Competence estimates remain provisional; do not permanently trap actors based on a small early sample.

### Internal versus external capability

OrgIntel should repeatedly ask:

> Should the company do this internally, automate it, buy a tool, or delegate it to an external specialist?

Consider competence, time, price, opportunity cost, reliability, strategic importance, recurrence, confidentiality, and authority requirements.

**Default pattern**

The working postures are judgement vocabulary, not a provider algebra:

```text
reuse existing capability
do internally
build or automate
buy an input
rent a tool or bounded resource
commission a deliverable
delegate a function
partner
hire or otherwise internalise
```

A material missing capability is ordinary Work with one accountable internal actor. Its evidence
should make the required outcome, chosen posture, retained company responsibility, provider scope,
trial/acceptance evidence, authority/data needs and reconsider trigger explicit. Providers are not
OrgIntel Actors, and sourcing adds no special Work kind, lifecycle or edge: use the existing Work,
Attempt, artifact, decision, `requires` and `revises` semantics until repeated runs prove a smaller
reusable concept.

---

# 4. Thinking under uncertainty

## 4.1 Company-wide epistemic language

**Core contract**

OrgIntel should preserve the difference between evidence, belief, preference, and choice.

| Kind | Meaning |
|---|---|
| **Observation** | Directly measured, recorded, or witnessed |
| **Claim** | Assertion believed to describe reality |
| **Hypothesis** | Testable claim about what is true or what will work |
| **Assumption** | Untested premise temporarily used for action |
| **Judgment** | Interpretation or evaluation under incomplete evidence |
| **Principle** | Value, preference, or normative rule |
| **Decision** | Chosen course of action |
| **Unknown** | Explicitly unresolved question |

Do not create separate V0 primitives for `fact`, `opinion`, or `intuition`:

- A fact is a strongly supported, current, scoped claim.
- An opinion is usually a judgment or principle.
- An intuition is a low-evidence hypothesis or judgment worth considering.

**Example**

```text
Observation: 8 of 40 tutors replied.
Hypothesis: Tutors have stronger demand than parents.
Prediction: A tutor offer converts 10% of qualified demos.
Assumption: Tutors prefer subscriptions.
Judgment: Current messaging sounds too consumer-focused.
Principle: Do not claim exam alignment without review.
Decision: Run a two-week tutor experiment.
Unknown: Whether tutors retain after the exam cycle.
```

Consequential claims may include scope, evidence references, confidence, owner, date, review/expiry date, what would change the belief, and next test.

Use this selectively. In V0, keep it in readable files or structured document blocks indexed by OrgIntel—not a large ontology.

## 4.2 Practical action with limited evidence

**Core contract**

Evidence-backed management must not become paralysis or evidence theatre.

> **Match the evidence burden to consequence, cost, uncertainty, and reversibility.**

**Default pattern**

| Situation | Posture |
|---|---|
| Cheap, reversible, observable | Act on plausible judgment; monitor |
| Moderate cost or uncertainty | Run a bounded experiment or comparison |
| High cost but stageable | Commit in stages with evidence checkpoints |
| Irreversible or high external consequence | Seek stronger evidence, independent review, and kernel approval |
| Evidence unobtainable in time | Make explicit best judgment; record assumptions and downside controls |

Useful evidence ordering:

1. Real external outcomes: payment, retention, successful operation, accepted deployment.
2. Direct observation or measurement.
3. Reproducible artifact, test, or controlled experiment.
4. Credible external research or historical comparison.
5. Structured expert judgment.
6. Intuition or analogy.
7. Unsupported agent confidence.

Lower-ranked evidence can justify cheap exploration. It should not silently gain the authority of stronger evidence.

Practical rules:

- Prefer a cheap informative action over prolonged abstract debate.
- Record material assumptions and predictions before results where practical.
- Use ranges and scenarios rather than false precision.
- Preserve minority hypotheses when evidence is weak, but cap their cost.
- Stop collecting information when it is unlikely to change the decision.
- Revisit decisions when critical assumptions expire or are contradicted.
- Treat real-world feedback as more important than agent consensus.

---

# 5. Persistent identity, context, and memory

## 5.1 Actor identity

**Core contract**

Long-term stability must not depend on one uninterrupted transcript, model, or process.

A persistent actor should have a compact, versioned OrgIntel-owned identity package such as:

```text
actor/<actor-id>
  role
  responsibilities
  decision rights
  principles
  working style
  taste and standards
  accepted examples
  important decisions
  current organisational state
  evidence of performance
  durable memories
```

The package may be persisted as structured rows plus readable versioned documents, but OrgIntel remains its authoritative owner. Sessions receive context snapshots or read-only materialisations; arbitrary sandbox work must not be able to silently rewrite long-term actor identity.

Temporary actor working state—scratch notes, worktrees, caches, and active experiments—belongs in the Company Runtime.

Actor identity keeps four facts separate:

```text
actor_id = stable organisational machine identity
display  = stable human-readable colleague identity
kind     = owner | exec | staff | system
role     = current craft and responsibility
```

For newly created Staff, `actor_id` is a two-segment `{domain}-{craft}` kebab-case identity. It does
not encode `staff`, team position, environment, Work revision, retry, model or implementation stage.
Team membership and leadership are relations, not identity. Historical ids remain stable for
provenance; new creation rejects assignment-shaped ids rather than repairing them after Work exists.

Important company style must also live institutionally: mission, product doctrine, design system, architecture decisions, accepted examples, customer promises, and ethical limits.

Actor identity should evolve from evidence without being rewritten from every session summary.

## 5.2 Context: shared spine, local depth

**Core contract**

Every actor gets a compact common operating picture plus focused context for its responsibility.

Shared spine:

- owner mandate and hard constraints;
- current objective, milestone, and definition of done;
- strategy, team map, dependencies, and blockers;
- consequential decisions;
- location of authoritative artifacts;
- relevant resource and authority envelope.

Local depth:

- role and decision rights;
- active Work nodes and relevant messages;
- deep task files and history;
- linked hypotheses, evidence, and standards;
- expected output, receiver, blockers, and deadlines.

Context by function:

- **Exec:** outcomes, resources, weak assumptions, exceptions, and summaries.
- **Team lead:** owned outcome, canonical artifact, whole-project causal model, team contributions,
  integration seams, review evidence and completion contract.
- **Worker:** local task depth, files, dependencies, standards, and handoff.
- **Critic:** objective, rubric, and artifact; preserve independence where useful.
- **Recovery:** original brief, attempts, artifacts, failure evidence, and remaining options.
- **Improvement:** outcome history, process versions, deviations, and costs.

Assembly rules:

- retrieve rather than copy large knowledge bodies;
- preserve provenance and epistemic labels in summaries;
- prefer current authoritative files over stale summaries;
- include enough history to understand decisions, not every conversation;
- refresh on meaningful state changes, not every filesystem event;
- let actors inspect linked source material.

## 5.3 Memory

**Core contract**

Memory exists to improve future decisions, not archive everything.

1. **Working state:** current task notes and scratch files.
2. **Operational memory:** Work nodes, recent decisions, blockers, and handoffs.
3. **Actor memory:** durable lessons, style, relationships, and competence evidence.
4. **Institutional memory:** doctrine, major decisions, playbooks, and accepted knowledge.

Promote information upward only when it is likely to remain useful. Link important memory to evidence or artifacts where possible.

---

# 6. Coordination, teamwork, and processes

## 6.1 Minimal primitives

**Core contract**

The stable coordination substrate should stay small:

- **Actor** — durable organisational identity.
- **Goal** — desired outcome.
- **Work** — one actor’s durable responsibility for one outcome and workspace.
- **Attempt** — one execution of one Work revision with immutable inputs.
- **Work edge** — `requires` for hard acyclic dependency; `revises` for review feedback that may cycle.
  A reviewer with revision power requires that same producer, and the pair is created atomically.
- **Message** — targeted communication.
- **Decision** — named choice with owner and rationale.
- **Schedule** — durable future time fact that can release its exact blocker.
- **Owner handoff** — a prepared last mile for identity, CAPTCHA, MFA, legal attestation, payment confirmation, or irreducible owner judgement.
- **Artifact reference** — pointer to real work in files, Git, or external tools.
- **Event** — lightweight operational observation.

Hypotheses, experiments, process templates, actor profiles, and knowledge claims may begin as files with indexes and references.

A resolved owner handoff remains OrgIntel source truth. The cockpit may project it briefly as a
decision continuation alongside current Work, Attempt and provider observations; OrgIntel does not
gain a continuation entity or lifecycle.

A Work initially needs only:

```text
outcome
owner
status: proposed | active | blocked | completed | abandoned
priority
workspace: repo / base ref / integration branch / worktree
revision and optional attempt limit
requires and revises edges
expected artifact or decision
relevant links
```

The scheduler atomically claims only ready Work and creates the Attempt before Staff starts. Initial
dependencies are immutable once that Work has an Attempt; graph repair must not retrofit inputs after
execution has begun. The
Attempt records the exact upstream artifact versions, input fingerprint, and Work-linked feedback it
received. Review `changes_requested` invalidates the producer artifact and hard descendants into a
new revision. Do not add leases, custody protocols, a scripted conversation lifecycle, or universal
commands.

One durable actor has one live cognitive process. A free-form conversation wake, including the
singleton Exec wake, excludes that actor from Work claims until it ends; a running Work Attempt in
turn queues addressed conversation. Process supervision and the actor registry enforce this without
turning conversation into Work.

An owner handoff blocks successor release, but attaching it does not mark a still-supervised Attempt
terminal. The process remains attributable to that Attempt until it actually returns. Usually it
returns `blocked` and the observed owner result becomes input to the next Attempt; if the result
arrives while the same process is still live, that Attempt may observe it and finish. This prevents
unattributed work and a second process starting for the same durable actor.

## 6.2 Communication

**Core contract**

Communication should be asynchronous, targeted, and consequential.

Initial semantic vocabulary:

```text
request · commit · acknowledge · update
blocker · result · review · decision
```

These are conventions, not security-sensitive command types.

Defaults:

- one accountable owner per outcome;
- assignments are acknowledged, rejected, or clarified;
- results link to artifacts or observations;
- updates communicate changed state, not narration;
- blockers surface early;
- decisions name the decider and affected work;
- broadcasts are reserved for broad changes;
- OrgIntel synthesises state so actors need not reread all messages.

Deterministic work handoff:

```text
Exec creates Work and edges
→ scheduler claims a ready node and records Attempt inputs
→ producer links the exact artifact and passes deterministic gates
→ requires releases the dependent reviewer
→ reviewer accepts, or revises returns feedback and increments producer revision
```

Messages remain free-form and may be linked to Work as input context. They never become a second
kickoff, assignment, handover protocol, or implicit review decision. An `owner_judgement` handoff
resumes only when the owner explicitly accepts the outcome or requests changes; ordinary Work-linked
chat stays open for questions and feedback. Other handoff categories resume only when their external
condition is observed.

## 6.3 Teamwork patterns

**Core contract**

Templates are strong, explainable, overridable defaults. They describe when a pattern fits, roles, owner, brief, outputs, communication, decision rights, health signals, and exit conditions—not every action.

Initial library:

| Pattern | Best fit |
|---|---|
| **Single accountable lead** | Coherent or tightly coupled work; the lead may execute alone |
| **Parallel exploration** | Independent hypotheses or search spaces |
| **Producer–critic** | Hidden errors, subjective quality, external-facing output |
| **Specialist pipeline** | Genuine sequential specialties |
| **Recovery huddle** | Blocked, contradictory, or repeatedly failing work |

OrgIntel should recommend and explain a pattern, allow override, observe health, suggest repair, and learn from the result. It should normally warn rather than freeze work.

### 6.3.1 A team requires difference, or it is not a team

**Core contract.** Added 15 August 2026 from three sprints of evidence.

Across three companies and three sprints, **organic delegation happened zero
times.** The cause was not reluctance and not prompting — it was arithmetic. Every
staff member ran the Exec's own model under the generic role `staff`, so
delegating meant handing work to a copy of yourself with **less** context. That
buys parallelism and nothing else, and an Exec that declines is reasoning
correctly.

Therefore an actor assigned ready Work must be able to differ in:

- **Role** — its durable identity, recorded on the actor, so the owner can ask
  who did what. Never a generic label; the literal `"staff"` on every actor is
  what made three sprints of delegation invisible.
- **Model** — a different mind. A critic on the producer's model, with the
  producer's context, is an echo chamber with a second invoice.
- **Context** — deliberately *less*, and deliberately different. Narrow context
  is the feature, not a compromise: the value of a specialist is an opinion you
  do not already hold.

The first real specialist confirmed it in one run. A critic given a narrow brief
and no drafting context returned three specific, quoted objections to copy the
Exec had judged finished — including that it pattern-matched to spam and
contained two competing asks. The producing actor had not seen any of them.

**Two failure modes arrive with heterogeneity**, both observed on the first
specialist launch and neither reachable with one model:

1. A provider error that arrives as message *content* rather than transport is
   reported as the specialist's failure. Every path that reads an agent's final
   output must classify provider errors before blaming the model.
2. **One role's dead provider account can stop the whole company.** An
   unaccountable turn fail-closes the spend ledger by design, so a second
   provider running out of credit poisoned a company whose own provider was
   healthy. Correct behaviour, new blast radius; it needs an explicit
   disposition rather than a surprise.

## 6.4 Business processes

**Core contract**

Model a process as a stable contract around flexible execution.

| Layer | Purpose | Rigidity |
|---|---|---|
| **Outcome contract** | Goal, owner, outputs, success measures | Stable for the run |
| **Control points** | Budget, approval, legal, irreversible constraints | Hard where needed |
| **Playbook** | Recommended stages, roles, tools, reviews | Strong default |
| **Execution plan** | Actual steps, actors, experiments, files | Fully adaptable |

Evolution path:

```text
useful ad hoc behaviour
→ repeated pattern
→ documented playbook
→ optional helper tooling
→ automation only where valuable
```

A run references a process version. New versions do not silently rewrite active work.

Examples:

- **Cosmon:** outcome is a playable browser build; implementation method remains flexible.
- **Aris:** outcome is sales or strong qualified demand; channels, segments, and offers adapt.
- **Thymelake:** outcome is one retained restaurant; sales, onboarding, support, and repair adapt.

---

# 7. Proactivity, health, and safe self-building

## 7.1 Self-running operation

**Core contract**

OrgIntel should keep the company moving without continuous owner prompts through:

- durable scheduled wakeups;
- event-triggered wakeups;
- missed-wakeup recovery;
- tactical, strategic, and organisational cadences;
- follow-up and deadline awareness;
- bounded retries and cost awareness.

The Exec should not globally replan after every message or filesystem event.

## 7.2 Health observer

The observer combines deterministic signals with model judgment where interpretation is required.

It may surface dormancy, stale Work nodes, repeated retries, cost anomalies, conflicting ownership, missing artifacts, duplication, review loops, expired assumptions, and high activity without useful output.

It should trigger awareness or a wakeup, not become a universal blocker.

Internal wakeups and messages may be at-least-once. Consequential external effects rely on Authority Kernel idempotency and reconciliation.

## 7.3 Safe self-building

**Core contract**

The company may build and adopt new prompts, role packages, planners, critics, templates, tools, dashboards, context strategies, health checks, and project services.

The stable coordination core changes through normal platform engineering rather than uncontrolled tenant self-modification.

Company-wide extensions should be versioned, inspectable, tested on a bounded scope, reversible, and disableable through safe mode.

No actor should solely approve a change that removes owner visibility, recovery, or the mechanism supervising that actor.

This is ordinary engineering hygiene, not a universal governance workflow.

---

# 8. Technical architecture

## 8.1 Deployment and communication

**Core contract**

```text
Per-company deployment
├── Authority Kernel service
├── OrgIntel service
│   └── OrgIntel Postgres
└── Company Runtime sandbox
    ├── thin runtime bridge
    ├── Exec / planner / critic ACP processes
    ├── worker ACP processes
    ├── files and Git
    ├── browser and tools
    └── project-specific services
```

The OrgIntel service is outside the writable work sandbox so actor identity, schedules, messages, and organisational learning survive sandbox corruption, replacement, and rollback.

A thin runtime bridge inside the sandbox:

- establishes an authenticated connection to OrgIntel;
- launches, stops, and observes ACP processes;
- associates processes with durable `actor_id` and temporary `session_id` values;
- launches Staff only after OrgIntel has atomically created a ready Work Attempt;
- passes context packets and wakeups;
- streams meaningful session events and results;
- exposes the local OrgIntel tools used by agents.

The bridge contains no planning policy, company ontology, or external authority. It is process and transport plumbing.

The bridge launches Codex, Claude, or another compatible agent locally and communicates through ACP over stdio. Agents use a small local interface—MCP tools, CLI, or a Unix socket—to read inboxes, manage Work nodes, send messages, link artifacts, report blockers, request review, record decisions, and schedule follow-up.

Agents use Linux, files, Git, browsers, and project tools directly for productive work. OrgIntel does not proxy ordinary filesystem, shell, build, or browser activity.

A narrow authenticated interface connects the Company Runtime to the Authority Kernel for effect requests, capability and budget visibility, receipts, model access, secrets-backed services, and lifecycle operations. OrgIntel brokers organisational intent where useful; the Authority Kernel independently enforces authority. Infrastructure traffic such as model calls, telemetry, health, snapshot, and owner approvals need not pass through OrgIntel.

## 8.2 Sources of truth and state ownership

**Core contract**

OrgIntel and “the database” are not separate sources of truth. OrgIntel is the service; its database and managed documents persist the state it owns.

| State | Authoritative source |
|---|---|
| Owner mandate, company capabilities, budgets, approvals, external-effect intents and receipts, lifecycle | **Authority Kernel store** |
| Actor identities and profiles, roles, goals, Work nodes, dependency/revision edges, Attempts and their exact inputs, messages, schedules, owner handoffs, decisions, hypotheses, experiments, organisational learning, session status, artifact references | **OrgIntel service and its store** |
| Code, documents, assets, builds, browser state, project databases, working files, installed tools, active experiments | **Company Runtime filesystem, Git, and domain applications** |
| Email delivery, payments, deployments, CRM records, and other real-world outcomes | **External provider**, referenced by kernel receipts and OrgIntel records |

Practical persistence:

```text
Authority Kernel database
- mandate and company lifecycle
- capabilities and budgets
- approvals and secret references
- external effects and receipts

OrgIntel Postgres / managed documents
- durable actors and identity packages
- goals, Work nodes, edges, Attempts, gates and owner handoffs
- messages and schedules
- decisions, hypotheses, experiments, and learning
- session and health state
- artifact and effect references

Company Runtime persistent storage
- repositories and worktrees
- documents, code, assets, and builds
- browser profiles
- project databases and applications
- scratch work and active experiments
```

Rules:

- One concept has one authoritative owner. Derived views and caches are allowed; duplicate ownership is not.
- OrgIntel owns a Work and its status; the runtime owns the resulting artifact; the kernel knows nothing about the Work unless an external effect is requested.
- OrgIntel may reference a Git commit, file, build URL, provider record, or kernel receipt, but does not copy that object into a universal artifact store.
- Agents mutate OrgIntel-owned state only through authenticated APIs, not direct database access.
- There are no cross-layer foreign keys. References are stable identifiers plus reconciliation logic.
- Ordinary optimistic concurrency is sufficient. Do not introduce distributed leases or exactly-once semantics for routine internal work.
- After restart or restore, reconcile expected sessions with processes, Work nodes with files and Git, schedules with current time, and pending effects with Authority Kernel receipts.
- Stale coordination state must never invalidate useful productive artifacts.

## 8.3 Failure posture

**Core contract**

- **OrgIntel crash:** running agents may continue filesystem work; new scheduling, messaging, and launches pause; OrgIntel resumes from its own durable store.
- **Runtime crash or replacement:** OrgIntel remains visible, retains identities and responsibilities, and relaunches actors against the preserved or restored company storage.
- **Agent/model failure:** preserve artifacts; retry only when likely to help; reassign or change strategy when repeated.
- **Stale coordination state:** warn, reconcile, or repair; do not invalidate completed work.
- **Kernel unavailable:** internal work continues; consequential external effects pause.
- **Runtime snapshot restore:** OrgIntel and kernel history do not roll back with the work machine; reconcile files, Work nodes, and effects before repeating consequential actions.
- **Broken extension:** boot the stable OrgIntel service and runtime bridge with experimental extensions disabled while preserving company work.

---

# 9. Hypotheses and dogfood

## 9.1 Initial hypotheses

**Product hypotheses**

| ID | Hypothesis | Failure signal |
|---|---|---|
| **H1** | One available Exec routing through accountable leads beats both Exec-as-producer and a flat swarm on long-horizon company work | Exec-as-producer, single-agent or loose-team baseline wins |
| **H2** | Durable actor identity improves style, judgment, and continuity | Identity packages add cost without consistency |
| **H3** | Shared spine plus local depth beats full shared transcripts | Summaries distort critical state or miss dependencies |
| **H4** | Task-shaped teams beat fixed multi-agent structures | Fixed topology performs as well at lower cost |
| **H5** | Artifact-centred closed-loop handoffs reduce lost work | Communication overhead rises without better recovery |
| **H6** | Bounded exploration improves strategic and product choices | Branching adds cost without better convergence |
| **H7** | Local repair reduces owner rescue and preserves work | Mess accumulates faster than repair can handle |
| **H8** | Evidence-driven evolution improves repeated performance | Debriefs produce prose but no better outcomes |
| **H9** | Overridable templates beat rigid workflows and pure improvisation | Templates become ceremony or are ignored |
| **H10** | Proactive cadence reduces dormancy | Wakeups create activity without output |

H1–H4 and H6–H8 are the core thesis.

## 9.2 Evidence-informed defaults

These are useful starting hypotheses from human teamwork and agent engineering, not guarantees:

- shared mental models → compact common operating picture;
- transactive memory → know who knows what and where artifacts live;
- task interdependence → form teams around work shape;
- brief, huddle, debrief, and check-back → coordinate at meaningful moments;
- after-action review → learn from actual outcomes;
- independent critique → preserve judgment where hidden errors matter;
- concrete artifacts → make long-running work inspectable and recoverable.

## 9.3 Dogfood portfolio

### Cosmon — building

Outcome:

> Produce a working browser game proving space exploration, creature encounter, capture, and battle.

Tests creative decomposition, code/asset integration, technical hypotheses, persistent design taste, Git collaboration, local repair, and build-based evaluation.

### Aris — selling

Outcome:

> Sell selective-exam practice papers and identify the segment, offer, and channel that creates real demand.

Tests sales experiments, revenue evidence, follow-up, customer context, changing weak assumptions, and external effects.

### Thymelake — operating

Outcome:

> Acquire, configure, launch, and retain one restaurant using QR ordering.

Tests cross-functional handoffs, live exceptions, product repair, external delegation, customer retention, and business value.

### Throwaway-company effect scenarios

Use a deterministic fake CLI through the generic governed-process path in a `_test` company for
success, denial, budget exhaustion, delay, confirmed failure, duplicate keys, ambiguous outcomes and
reconciliation. Behavioural inputs may be seeded files or messages, but never a provider-shaped
capability whose output can be mistaken for a live company fact.

```text
deterministic fake CLI and controlled input
→ real tool dry-run/status probe
→ small controlled real operation
→ larger real dogfood
```

The `_test` run proves system behaviour, not real market demand.

## 9.4 Evaluation method

For each scenario:

1. Define an accepted economic or product outcome.
2. Define owner-attention and external limits.
3. Run appropriate baselines.
4. Capture artifacts, real outcomes, cost, elapsed time, and interventions.
5. Blind-review quality or use objective measures where practical.
6. Separate harness-caused failure from task difficulty.
7. Debrief using evidence.
8. Change the smallest supported mechanism.
9. Rerun a comparable scenario.

Do not optimise for internal task counts, benchmark completion alone, or agent self-ratings.

---

# 10. Initial implementation contract

## 10.1 V0 walking skeleton

**Core contract**

V0 includes:

1. One per-company OrgIntel service and OrgIntel Postgres outside the writable work sandbox.
2. One persistent Company Runtime sandbox with durable company storage.
3. One thin runtime bridge connecting OrgIntel to local ACP processes.
4. One durable Exec identity and durable actor records owned by OrgIntel.
5. Temporary ACP sessions launched by the bridge over local stdio.
6. A small local actor interface through MCP, CLI, or Unix socket.
7. OrgIntel records for actors, sessions, goals, Work nodes, dependency/revision edges, Attempts and inputs, messages, schedules, owner handoffs, gates, decisions, hypotheses, learning, artifact references, and lightweight events.
8. Runtime files and Git for actual work, project doctrine, active experiments, builds, and project-specific state.
9. Shared-spine/local-depth context assembly.
10. Durable scheduled and event-driven wakeups.
11. Basic health signals for dormancy, stale work, repeated failure, missing artifacts, and cost anomalies.
12. Initial teamwork patterns: single owner, parallel exploration, producer–critic, specialist pipeline, and recovery huddle.
13. Authority Kernel request and receipt integration.
14. Runtime and OrgIntel restart/restore reconciliation.
15. One real dogfood vertical slice.

## 10.2 Explicit exclusions

Do not build in V0:

- a universal workflow DSL or command algebra;
- a large epistemic ontology;
- immutable event sourcing for internal activity;
- exactly-once internal coordination;
- a custom filesystem, Git, browser, scheduler, or process supervisor;
- multiplayer, shared multi-tenancy, or a collaboration suite;
- automatic tenant modification of the stable OrgIntel core;
- a permanent agent for every role;
- a dashboard for every entity;
- perfect auditability of all reasoning and file activity.

## 10.3 Implementation sequence

1. Boot the OrgIntel service, OrgIntel Postgres, and a separate persistent Company Runtime sandbox.
2. Connect a thin runtime bridge and prove launch, health, disconnect, and reconnect.
3. Launch a durable Exec through ACP and accept an owner directive.
4. Add Work nodes and edges, atomic ready-Work claim, Attempt input capture, messaging, and artifact linking.
5. Assemble shared-spine and actor-specific context from OrgIntel state plus runtime artifacts.
6. Add durable wakeups and missed-wakeup recovery outside the sandbox.
7. Run one bounded hypothesis branch and let the Exec choose, repair, or stop.
8. Invoke a deterministic fake CLI through the generic effect runner in a `_test` company; interrupt it and reconcile from a separate status-check receipt.
9. Kill a worker, the runtime, and OrgIntel independently; recover without losing the wrong layer's truth.
10. Restore an older runtime snapshot and reconcile against current OrgIntel and kernel state.
11. Complete one dogfood outcome and compare it with a baseline.
12. Add only structures justified by repeated observed failure.

## 10.4 Acceptance scenario

V0 demonstrates that:

- the owner gives an ambiguous goal;
- the Exec frames the company-level intent, chooses an accountable lead and returns to availability;
- the lead chooses one main approach and at most a bounded alternative;
- the lead works alone or forms a small task-shaped team according to coupling and expected coordination cost;
- workers perform real Linux work and produce inspectable artifacts;
- a worker blocks or fails;
- OrgIntel preserves work and applies a local repair;
- the lead evaluates actual output rather than task status and Exec evaluates the portfolio consequence;
- the organisation records one useful learning that changes a later choice;
- one external effect crosses the Authority Kernel with a receipt;
- restart preserves identity, responsibilities, artifacts, and follow-up;
- the owner receives outcome, evidence, cost, risks, and only necessary requests for judgment.

---

# 11. Engineering discipline

## 11.1 Anti-drift rules

**Core contract**

- Build real company behaviour before broad ontologies.
- Observe repeated failure before adding a durable primitive.
- Prefer files, Git, Postgres, ACP, Linux, and real applications over custom machinery.
- Prefer warnings and repair over blocking internal work.
- Keep Authority Kernel correctness separate from OrgIntel flexibility.
- Treat coordination activity as cost, not output.
- Keep exploration bounded and force convergence.
- Do not confuse model confidence with evidence.
- Do not turn one anecdote into permanent process.
- Keep templates overridable and reversible.
- Preserve useful work across failure and reorganisation.
- Delete structures that do not improve dogfood outcomes.

## 11.2 Review questions

For every proposed feature:

1. What real company failure or opportunity does it address?
2. Is this coordination, productive work, or external authority?
3. Could a file, Git, Postgres, an existing application, or a prompt solve it first?
4. Does it improve exploration, execution, repair, or evolution?
5. What evidence would show it worked?
6. What is the cheapest reversible implementation?
7. Does it create a mandatory gate?
8. What happens when it is stale, wrong, or unavailable?
9. Can it be deleted or replaced later?
10. Does it increase accepted output or only make the model cleaner?

## 11.3 Current decisions

- OrgIntel is one logical layer, but its durable service and store live outside the writable work sandbox inside the per-company deployment.
- The Exec and other cognitive OrgIntel actors run inside the Company Runtime against real files and tools.
- The Exec is the primary persistent organisational actor.
- The Exec delegates every executable owner request to one accountable team lead and returns to
  availability; it does not own production or integration work.
- A team lead may execute coherent work alone. Staff are optional and are added when the lead judges
  that specialisation or parallelism will repay coordination cost.
- Actor identity persists independently of model sessions and runtime replacement.
- A thin runtime bridge connects OrgIntel to ACP processes; ACP over local stdio is the initial agent transport.
- Productive work uses normal Linux files, Git, browsers, and applications.
- OrgIntel Postgres stores a small recoverable coordination substrate; the Company Runtime stores productive artifacts; the Authority Kernel stores authority and consequential receipts.
- One concept has one authoritative owner; derived projections and references do not create duplicate truth.
- The Authority Kernel remains the sole authority boundary.
- Exploration, local repair, and evidence-driven evolution are the core intelligence.
- Epistemic distinctions are shared semantics, not a large database ontology.
- Evidence burden scales with consequence and reversibility.
- Teamwork and process templates are strong, explainable, overridable defaults.
- Multiplayer, shared hosting, and broad platform features remain out of scope until proven necessary.

---

# Appendix A — Minimal readable templates

## Hypothesis

```markdown
# Hypothesis: <title>

## Question
## Observations
## Hypothesis and prediction
## Important assumptions and unknowns
## Cheapest informative test
## Owner and budget/time box
## Stop and expansion criteria
## Evidence and artifacts
## Result and decision
## What changed in our beliefs?
```

## Organisational improvement

```markdown
# Improvement: <title>

## Observed failure or opportunity
## Proposed change
## Why it may help
## Predicted observable effect
## Scope, owner, and budget
## Baseline or comparison
## Result
## Adopt, revise, or revert
## Follow-up date
```

## Teamwork pattern

```markdown
# Pattern: <name>

## Use when / avoid when
## Accountable owner and roles
## Shared brief and expected artifacts
## Communication and handoffs
## Decision rights
## Health signals and exit conditions
## Common failure modes
## Allowed adaptations
```

---

# Appendix B — Evidence roots

These sources inform hypotheses; they are not unquestionable product requirements.

- Mathieu et al., shared mental models and team performance: https://pubmed.ncbi.nlm.nih.gov/10783543/
- Research on transactive memory, task interdependence, information sharing, and team cognition.
- Keiser and Arthur, after-action review meta-analysis: https://pubmed.ncbi.nlm.nih.gov/32852990/
- AHRQ TeamSTEPPS resources: https://www.ahrq.gov/teamstepps-program/resources/modules/index.html
- Agent Client Protocol: https://agentclientprotocol.com/get-started/introduction
- ACP transports: https://agentclientprotocol.com/protocol/v1/transports
- Anthropic, Building Effective Agents: https://www.anthropic.com/engineering/building-effective-agents
- Anthropic, Multi-Agent Research System: https://www.anthropic.com/engineering/multi-agent-research-system
- Anthropic, Effective Harnesses for Long-Running Agents: https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents

---

# Working summary

OrgIntel is a small dependable coordination substrate plus replaceable organisational cognition.

> **Its core job is to maintain coherent beliefs, explore promising alternatives, execute through real tools, repair local failures, and evolve the organisation from real outcomes—without waiting for perfect evidence or turning work into a rigid workflow.**

The product test remains:

> **Does the organisation build, sell, or operate more effectively—and require less owner attention—than capable agents sharing the same Linux machine without OrgIntel?**
