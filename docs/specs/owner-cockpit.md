# Owner Cockpit Product Specification

**Version:** 0.1  
**Status:** Core product design and MVP implementation contract  
**Date:** 13 August 2026  
**Companions:** `OrgIntel Core Specification`, `Company Runtime and Runtime Bridge Specification`, `Authority Plane Specification`  
**Parent:** `ARCHITECTURE.md — Restless Architecture Source of Truth v0.9`

---

## 0. Document contract

This document defines the **Owner Cockpit**: the primary interface through which one owner or operator understands, steers, funds, and rescues a self-running company.

It uses four labels:

- **Core contract** — the product must preserve this behaviour.
- **Product hypothesis** — dogfood must test whether this actually reduces owner attention and improves outcomes.
- **Default interaction** — the recommended initial UX; change it when evidence disagrees.
- **Explicit exclusion** — do not accidentally expand the cockpit into this product.

The central principle is:

> **The cockpit surfaces outcomes, exceptions, evidence, people, and authority. It does not ask the owner to supervise continuous agent activity.**

The initial product posture is:

```text
one owner
one company
one persistent Exec
several agent employees
one isolated Company Runtime
```

Multiplayer, shared tenancy, and a general collaboration suite are not current requirements.

---

# 1. Product definition

## 1.1 Purpose

**Core contract**

The cockpit should let the owner answer five questions quickly:

1. **What is the company trying to achieve?**
2. **What useful progress or outcome has occurred?**
3. **What is blocked, uncertain, or at risk?**
4. **What specifically needs my judgment or authority?**
5. **What capital, capabilities, and real-world consequences are currently in play?**

The owner should not need to inspect raw agent transcripts, database rows, process logs, or filesystem activity to operate the company normally.

## 1.2 Product claim

**Product hypothesis**

A good cockpit should materially reduce the attention required to operate an agent company without making the owner blind.

The intended owner experience is:

```text
give direction
→ review only important exceptions and outcomes
→ grant or revoke authority when needed
→ inspect evidence
→ intervene selectively
→ let the company continue
```

The cockpit is successful when the owner can understand and steer the company through **minutes of focused review**, rather than continuous monitoring.

## 1.3 Primary user

The V0 user is the company’s sole owner/operator.

The owner:

- defines or changes the outer mandate;
- funds the company and sets authority limits;
- reviews major outcomes and risks;
- resolves genuinely owner-level ambiguity;
- may chat with any employee;
- may issue durable directives;
- approves authority expansion where required;
- can freeze consequences, stop, restore, or attach to the Company Runtime.

The owner is not expected to:

- manage every task;
- route every message;
- approve ordinary internal work;
- inspect every agent action;
- manually maintain organisational state;
- become the company’s full-time project manager.

## 1.4 Product boundaries

The cockpit is a presentation and action surface over three authoritative planes:

```text
Owner Cockpit
├── OrgIntel
│   └── people, goals, commitments, messages, decisions, attention
├── Authority Plane
│   └── mandate, budgets, capabilities, effects, resources, lifecycle
└── Company Runtime
    └── files, Git, builds, browser, services, actual artifacts
```

The cockpit does not become a fourth source of truth.

It may aggregate and project state, but writes must go to the layer that owns the concept.

## 1.5 Explicit non-goals

**Explicit exclusion**

The cockpit is not initially:

- a Slack replacement;
- a Google Docs replacement;
- a universal IDE or file editor;
- a full CRM;
- a generic workflow builder;
- a live swarm visualiser;
- an event-sourcing debugger;
- a multi-tenant admin console;
- an employee-surveillance dashboard;
- a place to model every agent action;
- the authoritative store for work, authority, or organisational state.

---

# 2. Product principles

## 2.1 Attention first

**Core contract**

The default home is the **Attention Inbox**, not a dashboard full of activity.

The UI should prioritise:

- owner decisions;
- approval or authority requests;
- major risks and failures;
- completed outcomes requiring review;
- strategic opportunities;
- contradictions that materially affect the plan.

Routine progress belongs in summaries and the Work view.

## 2.2 Outcomes over activity

The cockpit should emphasise:

- accepted artifacts;
- external outcomes;
- evidence;
- changes in company beliefs;
- cost and time;
- blockers and decisions;
- next meaningful steps.

It should de-emphasise:

- token streams;
- every tool call;
- process chatter;
- filesystem events;
- superficial “busy” indicators.

A company can be highly active while producing no economic value.

## 2.3 Owner as governor, not dispatcher

The UI should make it easy to set direction and limits, then let the Exec operate.

The owner can contact any employee, but ordinary organisational coordination remains the Exec’s responsibility.

The cockpit should distinguish casual communication from durable operating change so that a conversational message does not silently rewrite the company plan.

## 2.4 Progressive disclosure

The first view should show the smallest amount of information needed for a good decision.

Every important item should support deeper inspection into:

```text
summary
→ recommendation
→ evidence
→ relevant work and people
→ source records
→ raw diagnostics when necessary
```

Raw detail should remain available without becoming the normal interface.

## 2.5 One concept, one owner

The cockpit may combine data from all three planes, but it must preserve ownership:

- OrgIntel owns organisational meaning.
- The Authority Plane owns authority and consequential history.
- The Company Runtime owns actual work and artifacts.

A UI card is a projection, not a new durable business entity unless its owning plane explicitly defines it.

## 2.6 Honest uncertainty

The UI should preserve distinctions between:

- observation;
- claim;
- hypothesis;
- assumption;
- judgment;
- principle;
- decision;
- unknown.

The cockpit should not turn an agent’s confident summary into “truth.”

Consequential claims should expose evidence, confidence, freshness, and what would change the company’s mind where useful.

## 2.7 Evidence before self-report

The owner should see what supports a conclusion.

Prefer, roughly:

```text
real-world outcome or provider record
→ executable test or measured telemetry
→ customer or expert review
→ inspected artifact
→ agent narrative or self-assessment
```

Agent reports remain useful, but they are not the strongest source of trust.

## 2.8 Calm by default

The cockpit should feel like a well-run company, not an incident console.

Use urgency only where delay materially matters.

The absence of attention items should communicate:

> The company is operating within its mandate; no owner action is currently required.

---

# 3. Information architecture

## 3.1 Primary navigation

**Core contract**

The cockpit has four primary product areas:

1. **Attention** — the priority stack requiring owner awareness or action.
2. **Work** — goals, milestones, commitments, tasks, evidence, and outcomes.
3. **People** — the employee directory and direct chat with every actor.
4. **Authority** — mandate, budgets, capabilities, providers, resources, effects, and lifecycle controls.

```text
┌─────────────────────────────────────────────────────────────┐
│ Company · Phase · Health · Runtime · Spend · Authority      │
├────────────┬────────────┬────────────┬───────────────────────┤
│ Attention  │ Work       │ People     │ Authority             │
└────────────┴────────────┴────────────┴───────────────────────┘
```

The four areas should remain recognisable even as features grow.

## 3.2 Global company bar

A persistent company-level header should show:

- company name;
- infrastructure lifecycle state;
- operating phase;
- current top-level objective;
- derived company health;
- current spend versus envelope;
- Company Runtime status;
- external-authority state: active, partially restricted, or frozen.

It should expose fast actions for:

- send directive;
- open Exec chat;
- freeze or resume external authority;
- attach to the Company Runtime;
- inspect current mandate.

The global bar is a compact orientation aid, not another dashboard.

## 3.3 No mandatory overview page

**Default interaction**

V0 does not require a fifth “Dashboard” section.

The Attention page plus the global company bar should provide the default overview. Work, People, and Authority provide deeper operational views.

Add a separate overview only if dogfood shows that owners repeatedly need a stable synthesis that does not belong in Attention.

---

# 4. Shared product semantics

## 4.1 Stable vocabulary

The cockpit should consistently use the following concepts:

| Concept | Meaning | Authoritative owner |
|---|---|---|
| **Company** | The durable organisation | Authority Plane for identity/lifecycle; OrgIntel for operating state |
| **Actor** | Persistent human, agent, or service identity | OrgIntel |
| **Role** | Organisational responsibility and decision rights | OrgIntel |
| **Session** | Temporary model/process execution | OrgIntel + Runtime Bridge |
| **Goal** | Desired outcome at any abstraction level | OrgIntel |
| **Commitment** | One actor’s responsibility to produce or decide something | OrgIntel |
| **Task** | Owner-facing UI label for a commitment or small goal | Projection only |
| **Artifact reference** | Pointer to actual work | OrgIntel reference; Runtime owns artifact |
| **Observation** | Directly measured, recorded, or witnessed information | OrgIntel or referenced source |
| **Hypothesis** | Testable claim about what is true or will work | OrgIntel |
| **Decision** | Named choice with owner and rationale | OrgIntel unless it changes root authority |
| **Directive** | Durable owner instruction affecting company operation | OrgIntel; mandate changes also affect Authority Plane |
| **Attention envelope** | Common UI shape for something requiring owner review | Projection over source-owned request |
| **Operating phase** | Current economic/organisational mode | OrgIntel |
| **Effect** | Discrete consequential external action | Authority Plane |
| **Receipt** | Authoritative record of effect outcome | Authority Plane |
| **Resource grant** | Bounded access to productive resource | Authority Plane |
| **Artifact** | Code, document, build, asset, dataset, or external output | Company Runtime or external system |

## 4.2 Four different kinds of status

Do not collapse all company state into one generic status.

### Infrastructure lifecycle

Owned by the Authority Plane:

```text
provisioning
running
externally_frozen
stopped
restoring
archived
```

### Operating phase

Owned by OrgIntel:

```text
exploration
validation_or_pre_profit
profit
scale
```

### Goal or milestone stage

Owned by OrgIntel and adapted to the work. Examples:

```text
framing
exploring
building
validating
operating
reviewing
```

These are descriptive defaults, not universal gates.

### Commitment status

Owned by OrgIntel:

```text
proposed
active
blocked
completed
abandoned
```

The UI should always make clear which kind of status it is showing.

---

# 5. Attention Inbox

## 5.1 Purpose

**Core contract**

The Attention Inbox is the owner’s primary work queue.

It answers:

> What is the smallest set of things that genuinely deserves the owner’s time now?

It should combine owner-facing requests from OrgIntel and the Authority Plane without erasing their source ownership.

## 5.2 Attention envelope

Every item should present a common envelope:

```text
title
source plane and source object
what happened
why it matters
Exec or system recommendation
specific owner action requested
what happens if the owner does nothing
deadline or review date
evidence and artifact references
cost, consequence, and reversibility
available actions
```

The cockpit may standardise this envelope, but resolution writes back to the source plane.

Examples:

- OrgIntel decision request.
- Authority approval request.
- Completed outcome awaiting acceptance.
- Strategic opportunity.
- Major contradiction between evidence and current strategy.
- High-cost or repeated failure.
- Runtime recovery request.

## 5.3 Item categories

Initial categories:

| Category | Example |
|---|---|
| **Decision** | Choose between two product directions |
| **Approval** | Authorise spend or public deployment |
| **Outcome review** | Review Cosmon’s playable prototype |
| **Risk** | Aris campaign may breach an owner constraint |
| **Failure/recovery** | Company Runtime restore is recommended |
| **Opportunity** | A qualified partner offers distribution |
| **Contradiction** | Real customer evidence weakens the current thesis |
| **Information** | Important result requiring awareness but no immediate action |

“Information” items should be rare and easy to dismiss or digest.

## 5.4 Priority model

**Default interaction**

Rank items using practical factors rather than a complex visible formula:

- consequence of delay;
- urgency;
- whether owner authority is actually required;
- irreversibility;
- economic magnitude;
- confidence that the item matters;
- whether the Exec has a clear recommendation;
- whether the company can continue safely without a response.

The system should prefer a short, high-quality stack over completeness.

## 5.5 Review interaction

The default experience is a stack or focused queue:

```text
review item
→ inspect recommendation and evidence
→ act, ask, delegate, defer, or dismiss
→ move to next
```

Common actions:

- accept recommendation;
- reject and explain;
- issue a different directive;
- approve or deny authority;
- ask the Exec for more evidence;
- ask a named employee;
- defer until a date or condition;
- mark as reviewed;
- open the related work, person, artifact, or receipt.

Every item should state whether the company can continue while it waits.

## 5.6 Preventing attention spam

OrgIntel should learn from the owner’s handling of items.

Signals include:

- repeatedly dismissed categories;
- requests the Exec could have resolved itself;
- items with no clear action;
- duplicate attention items;
- alerts that arrive too early or too late;
- excessive low-value updates.

The desired progression is:

```text
surface exception
→ observe owner response
→ improve delegation or policy
→ reduce future owner interruption
```

The system should not optimise merely for an empty inbox. It should optimise for **necessary, high-signal owner attention**.

---

# 6. Work

## 6.1 Purpose

The Work area shows how the company’s current activity connects to its mission and outcomes.

It should support both:

- a high-level map of goals and breakdowns;
- a practical kanban of active commitments/tasks.

## 6.2 Work hierarchy

The underlying model should remain flexible:

```text
owner mandate
→ company goals
→ milestones or subgoals
→ commitments/tasks
→ artifacts, observations, and external outcomes
```

Goals may form a hierarchy or graph. Do not require every company to use the same number of levels.

The UI can provide familiar labels such as:

- mission;
- objective;
- milestone;
- task.

These are views over OrgIntel goals and commitments, not separate hard-coded entities.

## 6.3 Kanban view

**Default interaction**

The main operational view is an active-work board grouped by commitment status:

```text
Proposed | Active | Blocked | Completed
```

`Abandoned` work remains available in history but is not a permanent default column.

Each card should show:

- outcome or deliverable;
- accountable actor;
- parent goal;
- current status and next meaningful step;
- blocker, if any;
- expected artifact or evidence;
- latest meaningful update;
- cost/time signal where useful.

Cards should not display continuous activity logs.

## 6.4 Goal view

A goal detail page should show:

- intended outcome;
- parent and child goals;
- accountable owner;
- definition of done or success contract;
- current strategy;
- relevant assumptions, hypotheses, and unknowns;
- active commitments;
- evidence and artifacts;
- key decisions;
- cost and elapsed time;
- current recommendation: continue, branch, repair, pivot, or stop.

## 6.5 Exploration in Work

Exploratory branches should be visible without becoming a workflow engine.

Example:

```text
Goal: prove Cosmon capture loop
├── Main approach: timing-based capture
├── Bounded alternative: environmental puzzle capture
└── Rejected branch: random probability capture
```

Each meaningful branch may show:

- hypothesis;
- prediction;
- owner;
- budget/time box;
- evidence sought;
- stop and expansion criteria;
- result and decision.

The UI should make clear which approach is the current main bet and which are bounded explorations.

## 6.6 Evidence and artifacts

Work items should link to actual outputs:

- files;
- Git commits and branches;
- builds and deployment URLs;
- test results;
- customer records;
- provider receipts;
- external metrics;
- human or expert reviews.

The cockpit should preview common artifacts when cheap, but it should not build a universal editor or artifact database.

## 6.7 Owner actions in Work

The owner may:

- inspect outcomes and evidence;
- give feedback;
- issue a directive;
- ask the Exec or accountable actor a question;
- accept or reject a milestone result;
- reopen or stop work;
- change priorities;
- request a comparison or additional evidence.

Ordinary task assignment remains primarily an Exec or lead function. The owner may intervene directly, but the change should be visible to the Exec.

---

# 7. People and chat

## 7.1 Purpose

The People area presents the company as a durable organisation of employees rather than a list of temporary model sessions.

The UI term may be **People** or **Employees**. The shared system concept is `Actor`.

## 7.2 Directory

The directory should show every persistent employee with:

- name and role;
- responsibilities and decision rights;
- current focus;
- active commitments;
- current availability/session state;
- recent accepted outputs;
- important competence evidence;
- cost/usage summary where relevant;
- direct chat entry point.

The directory should distinguish:

- persistent actor identity;
- current model/session;
- organisational role;
- security principal or authority envelope where relevant.

A model restart or provider change must not appear as a new employee.

## 7.3 Actor profile

An actor profile may include:

- stable role and responsibilities;
- working style and principles;
- design or communication preferences;
- relevant strengths and weaknesses;
- trusted collaborators;
- current and past commitments;
- accepted artifacts and important decisions;
- current session and model;
- organisational learning associated with the actor.

Competence should be evidence-backed and revisable, not a permanent score based on early performance.

## 7.4 Chat semantics

**Core contract**

The owner can chat with any employee, but chat is not automatically the source of truth for company operation.

The interface should distinguish:

### Message

Conversational communication, questions, clarification, or advice.

A message does not silently alter goals, commitments, authority, or the mandate.

### Feedback

A comment tied to an artifact, goal, or result.

The responsible actor or Exec decides how to incorporate it unless the owner promotes it to a directive.

### Directive

A durable owner instruction intended to change company operation.

A directive:

- is visible to the Exec;
- records scope and priority;
- links to affected goals or work where possible;
- may trigger replanning;
- remains distinguishable from casual conversation.

### Authority decision

An approval, denial, grant, revocation, or mandate change written to the Authority Plane.

It is not merely a chat message.

## 7.5 Direct worker contact

The owner may contact a worker directly.

To avoid conflicting hidden instructions:

- ordinary chat remains local to the conversation but visible to the actor;
- durable directives are visible to the Exec and OrgIntel;
- the UI should warn when a directive conflicts with active company strategy or another directive;
- the actor may ask the owner to clarify scope or route the issue through the Exec.

This preserves owner access without destroying organisational coherence.

## 7.6 Chat context

Chat should show enough context to make the conversation useful:

- actor identity and role;
- current focus;
- linked work or artifact;
- recent relevant decisions;
- whether the actor is currently active;
- whether the owner’s message is a question, feedback, or directive.

Do not automatically inject the entire company history into every chat.

---

# 8. Authority

## 8.1 Purpose

The Authority area gives the owner a complete, understandable inventory of what the company is allowed to do and what consequentially happened.

It should expose owner-facing settings from the Authority Plane without exposing raw secrets or forcing the owner to understand internal policy implementation.

## 8.2 Authority inventory

Initial sections:

### Mandate and constraints

- company purpose;
- hard owner constraints;
- prohibited activities;
- current outer operating envelope;
- effective date and revision history.

### Budgets

- model spend;
- compute and infrastructure;
- external services;
- purchases;
- campaigns or communication;
- other company-specific categories;
- current usage, remaining amount, and period.

### Capabilities

- what the company may do;
- scope;
- approval threshold;
- delegability;
- expiry or review date;
- current state: active, restricted, or revoked.

### Connected providers and accounts

- provider/account name;
- connection health;
- granted scope;
- credential delivery mode;
- last successful use;
- expiry/rotation status;
- revoke or reconnect action.

Raw credentials must never be shown.

### Productive resources

- fixed resources available to the company;
- dynamic resource grants;
- limits and expiry;
- current usage;
- resource status.

### External effects and receipts

- recent consequential actions;
- actor/Exec attribution;
- result: success, failure, or unknown;
- receipt/provider reference;
- cost;
- reconciliation state.

### Runtime lifecycle

- current runtime generation;
- start/stop state;
- snapshots;
- restore status;
- attach controls;
- external-authority freeze state.

## 8.3 High-level controls

The owner should be able to:

- change budgets and ceilings;
- grant, narrow, or revoke capabilities;
- approve or deny pending requests;
- connect or disconnect providers;
- freeze all consequential external effects;
- resume authority;
- stop or restore the Company Runtime;
- inspect effect receipts and unknown outcomes;
- revise the owner mandate.

The UI should explain the practical consequence of each action.

Example:

> Freezing external authority prevents new emails, purchases, deployments, and brokered production actions. Internal work, files, builds, and planning continue.

## 8.4 Permissive MVP posture

**Default interaction**

V0 should make the broad operating envelope obvious:

```text
allow by default inside owner-set limits
hard budget/resource ceilings
small catastrophic denylist
owner freeze/revoke
consequential receipts
```

The cockpit should not present dozens of narrow controls that do not yet improve real dogfood.

Mock and real providers should appear consistently, with a clear `simulated` or `real` label.

## 8.5 Expert detail

Advanced diagnostic or raw configuration views may exist behind progressive disclosure.

They should not be the primary settings experience.

---

# 9. Company phase, lifecycle, and health

## 9.1 Operating phases

**Core contract**

A company should have a visible high-level operating phase:

| Phase | Primary optimisation |
|---|---|
| **Exploration** | Discover promising directions and reduce existential uncertainty cheaply |
| **Validation / pre-profit** | Prove real customer or product value and willingness to pay/use |
| **Profit** | Make delivery repeatable, reliable, and economically sustainable |
| **Scale** | Increase throughput and reach while preserving quality and unit economics |

These are operating profiles, not rigid workflow gates.

## 9.2 Phase effects

The current phase may influence OrgIntel defaults:

### Exploration

- more bounded hypotheses and prototypes;
- short planning horizons;
- emphasis on learning rate and falsification;
- lower commitment to infrastructure and process.

### Validation / pre-profit

- stronger external customer evidence;
- focus on acquisition, activation, use, retention, and payment;
- rapid product/offer iteration;
- disciplined spend.

### Profit

- focus on repeatability, margins, reliability, and customer value;
- promotion of proven work into playbooks and automation;
- clearer operational ownership.

### Scale

- focus on throughput, delegation, robustness, quality control, and capital allocation;
- more persistent teams and specialised systems where justified.

A phase change does not automatically grant more authority or capital.

## 9.3 Phase changes

The Exec may propose a phase change with:

- evidence;
- rationale;
- expected operating changes;
- unresolved risks;
- required capital or authority changes;
- success criteria for the new phase.

The owner should explicitly review a phase change when it implies material capital, risk, or strategic commitment.

Otherwise, the Exec may update the operating phase within the existing mandate and make the change visible to the owner.

## 9.4 Company health

Company health is a derived summary, not a source of truth.

Possible dimensions:

- progress toward current outcome;
- evidence quality;
- blocked or dormant work;
- cost against plan;
- runtime health;
- external authority state;
- unresolved owner attention;
- repeated failure or improvement trend.

Avoid one opaque “AI health score.” Show the few reasons behind the status.

Example:

```text
Health: At risk
Why:
- core milestone is 4 days behind;
- two approaches failed;
- current recovery plan is active;
- no owner action needed until Friday.
```

---

# 10. Evaluation and external grounding

## 10.1 Success contracts

Every meaningful goal or milestone should have a lightweight success contract:

```text
desired outcome
evidence required
source of trust
metric or acceptance test
time/cost envelope
important assumptions and unknowns
what would falsify the approach
continue, branch, pivot, or stop criteria
```

The cockpit should show this contract in the Work view and use it when presenting outcomes or attention items.

## 10.2 Evaluation levels

Evaluation occurs at several levels:

| Level | Question |
|---|---|
| **Artifact** | Does the build, document, analysis, or system meet its acceptance criteria? |
| **Work/process** | Is execution stalled, duplicative, wasteful, or repeatedly failing? |
| **Department** | Did the team produce its intended operational or economic result? |
| **Company** | Is accepted output improving relative to owner attention, cost, time, and risk? |
| **External reality** | Did customers buy, use, retain, reply, succeed, or otherwise validate the claim? |

## 10.3 Evidence presentation

Evidence should be:

- linked to the claim or decision it supports;
- labelled by source type;
- dated and freshness-aware;
- distinguishable from interpretation;
- inspectable by the owner;
- concise by default.

The UI should surface conflicting evidence rather than averaging it into a false consensus.

## 10.4 Evaluation feeds adaptation

The cockpit should expose how evidence changed the organisation:

```text
observed outcome
→ belief updated
→ decision made
→ plan/resource/team changed
→ next test or operating step
```

This makes self-exploration, repair, and evolution visible without exposing raw reasoning traces.

## 10.5 Dogfood grounding

The three initial companies provide different cockpit tests:

### Cosmon

- Is there a working browser game?
- Does the current build prove the intended gameplay loop?
- What technical or creative risk remains?
- What should be built next?

### Aris

- Were practice papers sold?
- Which segment, offer, and channel converted?
- What revenue and repeat demand exist?
- What external feedback changes the product or sales process?

### Thymelake

- Was a real restaurant acquired and launched?
- Were real orders processed correctly?
- Did staff and diners use the system successfully?
- Does the venue want to continue or pay?

The cockpit should not declare success because internal tasks are complete.

---

# 11. Cross-layer interaction flows

## 11.1 Owner directive

```text
Owner writes directive in Cockpit
→ Cockpit sends to OrgIntel
→ OrgIntel records directive and affected scope
→ Exec is notified/woken
→ Exec acknowledges, replans, or requests clarification
→ Work view reflects the resulting change
```

A mandate-level directive may additionally update the Authority Plane.

## 11.2 Employee chat

```text
Owner opens actor profile
→ sends message or feedback
→ OrgIntel delivers to actor inbox/session
→ actor responds
→ durable operating change occurs only if converted to directive, decision, or commitment
```

## 11.3 Authority approval

```text
Authority Plane creates approval request
→ Cockpit presents standard attention envelope
→ Owner inspects recommendation, scope, cost, and evidence
→ approve or deny writes directly to Authority Plane
→ OrgIntel receives resulting status/reference
→ company continues
```

OrgIntel cannot approve its own authority expansion.

## 11.4 Outcome review

```text
OrgIntel marks milestone ready for review
→ Attention item links success contract and evidence
→ Owner opens build/file/report/provider record
→ accepts, rejects, asks for revision, or issues new directive
→ OrgIntel updates goal and next plan
```

## 11.5 Phase change

```text
Exec proposes phase change
→ Cockpit shows evidence and operating implications
→ Owner reviews if capital/risk/mandate materially changes
→ OrgIntel updates operating phase
→ Authority Plane is changed separately only where required
```

## 11.6 Freeze and rescue

```text
Owner freezes external authority
→ Authority Plane blocks new consequential effects
→ OrgIntel and Runtime continue internal work
→ owner inspects people, work, effects, and runtime
→ owner may attach, stop a session, restore, or resume authority
```

The preferred emergency action is to freeze consequences while preserving useful work.

---

# 12. V0 screen contract

## 12.1 Application shell

Retain the existing Svelte SPA and shape it around:

```text
/attention
/work
/people
/authority
```

Useful detail routes may include:

```text
/work/:goal_id
/work/commitments/:commitment_id
/people/:actor_id
/authority/effects/:effect_id
/authority/resources/:resource_id
```

The exact route names are not product invariants.

## 12.2 Attention screen

Required V0 elements:

- ordered queue;
- focused item detail;
- source and category;
- why it matters;
- recommendation;
- requested action;
- evidence/artifact links;
- deadline and no-response behaviour;
- actions: accept, reject, ask, direct, defer, dismiss;
- resolved history.

## 12.3 Work screen

Required V0 elements:

- goal hierarchy/navigation;
- active commitments kanban;
- filter by goal, owner, status, and priority;
- goal detail with success contract;
- linked artifacts and evidence;
- hypotheses/alternatives where present;
- blockers and current recommendation;
- meaningful history and decisions.

## 12.4 People screen

Required V0 elements:

- employee directory;
- role/current-focus summary;
- active commitments;
- session/availability state;
- recent outputs;
- actor profile;
- direct chat;
- explicit message versus directive action.

## 12.5 Authority screen

Required V0 elements:

- mandate and constraints;
- budget categories and usage;
- capabilities and thresholds;
- connected providers/accounts;
- productive resources;
- pending approvals;
- effect/receipt history;
- runtime lifecycle and snapshots;
- freeze, revoke, stop, restore, and attach controls.

## 12.6 Global status bar

Required V0 elements:

- company name;
- operating phase;
- lifecycle/runtime state;
- top-level goal;
- health explanation;
- spend summary;
- authority freeze state;
- fast Exec chat and directive action.

---

# 13. Failure and degraded modes

## 13.1 OrgIntel unavailable

The cockpit should:

- clearly show OrgIntel as unavailable;
- preserve Authority controls and approvals;
- show the last known organisational projection with timestamp where safe;
- avoid presenting stale state as current;
- provide recovery status.

## 13.2 Company Runtime unavailable

The cockpit should:

- keep People, Work, and Authority state visible;
- show sessions/runtime as unavailable;
- preserve owner chat/directives for later delivery;
- provide restart, restore, and attach/recovery controls;
- explain what work state may be stale.

## 13.3 Authority Plane restricted or unavailable

The cockpit should:

- clearly indicate external effects/resources are paused;
- allow internal Work and People views to remain usable;
- show pending requests and last known receipts carefully;
- avoid implying an external action succeeded without an authoritative receipt.

## 13.4 Runtime rollback

After restore, the cockpit should distinguish:

- current OrgIntel commitments and decisions;
- restored runtime generation and artifacts;
- current Authority receipts and external history;
- reconciliation warnings.

The UI must not imply that the external world rolled back with the runtime.

---

# 14. Technical product contract

## 14.1 Existing Svelte SPA

**Core contract**

The existing Svelte/SvelteKit interface is a valid foundation and should be retained where useful.

The frontend should be simplified around the owner’s four primary areas rather than preserving UI concepts that only exist for the old governance model.

## 14.2 API ownership

The cockpit communicates with layer-owned APIs:

```text
OrgIntel API
- actors
- messages/directives
- goals and commitments
- decisions and hypotheses
- attention-ready organisational requests
- operating phase and health projections

Authority API
- mandate and envelope
- budgets and capabilities
- approvals
- effects and receipts
- resources
- runtime lifecycle

Runtime access
- artifact links/previews
- build/service URLs
- browser/desktop attach
- diagnostics when explicitly requested
```

The SPA must not write directly to Postgres or the runtime filesystem.

## 14.3 Unified read projections

A thin backend-for-frontend or server-side aggregation layer may combine data into owner-facing views such as:

- company header;
- unified attention queue;
- owner-ready company summary.

This projection layer does not become authoritative state.

Do not introduce a new sync platform until ordinary APIs and lightweight live updates prove insufficient.

## 14.4 Live updates

Use live updates only for meaningful state changes:

- new or resolved attention item;
- actor/session state;
- commitment status or blocker;
- outcome ready for review;
- budget/resource threshold;
- effect result;
- runtime health.

SSE or WebSocket delivery is sufficient for V0.

Do not stream every command or token into the primary UI.

## 14.5 Authentication

V0 supports one owner principal.

All writes are attributed to that principal.

Do not build invitations, presence, granular human roles, or multiplayer permissions now.

## 14.6 Stable cross-layer identifiers

At minimum, views and actions should preserve:

```text
company_id
actor_id
session_id
principal_id
goal_id
commitment_id
artifact_ref
attention_source_ref
effect_id
receipt_id
resource_grant_id
runtime_generation
```

The cockpit should pass references between planes rather than copying source-owned objects.

---

# 15. Acceptance scenarios

## 15.1 Cosmon playable-build review

1. The owner opens Attention.
2. The top item says the first Cosmon browser prototype is ready.
3. It explains the intended gameplay loop and current evidence.
4. The owner opens the playable build and relevant commit.
5. The owner chats with the Game Director.
6. Casual feedback remains feedback.
7. The owner promotes one point into a durable directive.
8. The Exec acknowledges and updates the Work plan.
9. A separate Authority request appears only when public deployment is needed.

Passing behaviour:

- no raw task archaeology is required;
- the build and evidence are easy to inspect;
- feedback and directive semantics remain distinct;
- work continues without owner micromanagement.

## 15.2 Aris sales decision

1. Work shows the goal “sell selective-exam papers” and its active segment hypotheses.
2. Attention surfaces that tutor outreach converts materially better than parent ads.
3. The item links real replies, sales, spend, and the Exec’s recommendation.
4. The owner approves a bounded campaign increase in Authority.
5. The Exec reallocates work without needing the owner to assign individual tasks.

Passing behaviour:

- external revenue and customer evidence dominate agent self-report;
- the owner reviews one meaningful decision, not every campaign action.

## 15.3 Thymelake restaurant launch

1. Work shows the restaurant pilot goal and current launch stage.
2. A genuine blocker requiring owner input appears in Attention.
3. The owner supplies the missing business judgment or contact detail.
4. Later, the owner receives an outcome review linked to real successful orders and venue feedback.
5. The Exec recommends continue, repair, or stop.

Passing behaviour:

- sales, onboarding, product, and operations remain connected to one outcome;
- only owner-level exceptions interrupt the owner.

## 15.4 Runtime failure and rescue

1. The global bar shows Company Runtime unavailable.
2. OrgIntel goals, actors, messages, and schedules remain visible.
3. Authority receipts and freeze controls remain available.
4. The owner freezes external authority, restores a snapshot, and watches reconciliation.
5. The Exec resumes with current organisational context.

Passing behaviour:

- the owner can rescue the company without losing authority history or organisational continuity.

## 15.5 No-action state

1. The owner opens Attention.
2. No decision is required.
3. The cockpit shows a calm summary of current objective, recent accepted progress, next expected review, and operating status.
4. It does not manufacture low-value alerts merely to appear active.

---

# 16. Product metrics

## 16.1 Primary metric

> **Accepted economic output per unit of owner attention, cost, time, and bounded risk.**

The cockpit contributes primarily by reducing owner attention while maintaining understanding and control.

## 16.2 Cockpit-specific measures

- owner minutes per day/week;
- owner interventions per accepted outcome;
- percentage of attention items requiring genuine owner judgment;
- percentage resolved using the recommended action;
- false urgency and dismissal rate;
- median time from attention item to decision;
- time required to understand company status;
- percentage of important claims linked to inspectable evidence;
- percentage of outcomes grounded in external or executable evidence;
- number of times the owner must inspect raw logs/files to understand normal operation;
- success of freeze, restore, and rescue flows;
- owner trust and confidence in what the cockpit reports.

Do not optimise for clicks, time in app, or notification volume.

---

# 17. V0 implementation scope

## 17.1 Must build

1. Existing Svelte application reshaped into the four primary areas.
2. Global company status bar.
3. Unified owner attention queue over OrgIntel and Authority requests.
4. Goal hierarchy and commitment/task kanban.
5. Goal detail with success contract, evidence, and artifacts.
6. Persistent employee directory and actor profile.
7. Direct chat with explicit message/feedback/directive semantics.
8. Authority inventory, approvals, budgets, capabilities, providers, and effect receipts.
9. Runtime state, freeze, restart/restore, and attach controls.
10. Meaningful live updates.
11. Clear stale/degraded-state handling.
12. Cosmon, Aris, and Thymelake acceptance scenarios.

## 17.2 Explicitly defer

- multiple human users;
- shared multiplayer presence;
- custom collaborative document editing;
- a full CRM or sales pipeline product;
- a universal artifact editor;
- custom workflow design;
- arbitrary user-created dashboards;
- detailed policy-language UI;
- per-worker security administration;
- tenant/fleet administration;
- mobile-native applications;
- continuous agent thought or command visualisation;
- exhaustive analytics;
- gamified productivity scores.

---

# 18. Implementation sequence

## Step 1: Freeze shared semantics

Align identifiers, ownership, statuses, attention envelopes, directives, and artifact references across OrgIntel, Authority, Runtime Bridge, and the SPA.

## Step 2: Build the application shell

Implement the global company bar, routing, source health, authentication, and degraded-state presentation.

## Step 3: Build Attention first

Connect real OrgIntel and Authority requests. Prove the complete review/action loop before building broad dashboard features.

## Step 4: Build Work

Implement goal hierarchy, active commitments kanban, success contracts, evidence, artifacts, and owner feedback/directives.

## Step 5: Build People and chat

Connect persistent actor identities, current sessions, outputs, and message/directive semantics.

## Step 6: Build Authority

Expose the permissive MVP envelope, approvals, budgets, providers, resources, effects, receipts, and rescue controls.

## Step 7: Run real dogfoods

Use Cosmon, Aris, and Thymelake. Observe where the owner still opens raw logs, asks repetitive questions, or receives low-value interruptions.

## Step 8: Simplify

Delete views, fields, and alerts that do not improve decisions, trust, outcomes, or owner attention.

---

# 19. Current decisions

1. The initial cockpit serves one owner and one company.
2. The existing Svelte SPA remains the frontend foundation.
3. The product has four primary areas: Attention, Work, People, and Authority.
4. Attention is the default home.
5. A persistent global bar shows company phase, health, runtime, spend, and authority status.
6. Work combines hierarchical goals with an active-commitment kanban.
7. `Task` is a UI term; `Commitment` remains the OrgIntel primitive.
8. The owner can chat with every employee.
9. Messages, feedback, directives, and authority decisions have distinct semantics.
10. Durable directives are visible to the Exec and affect OrgIntel state.
11. The Authority area exposes all owner-relevant Authority Plane settings without exposing raw secrets.
12. Operating phase is distinct from infrastructure lifecycle, project stage, and task status.
13. Company phases are adaptive profiles, not hard workflow gates.
14. Evaluation must ground work in success contracts and sources of trust.
15. External outcomes and executable evidence outrank agent self-report.
16. The cockpit is a projection and action surface, not a new source of truth.
17. Multiplayer, shared hosting, and general collaboration features remain deferred.
18. The cockpit should reduce owner attention, not maximise engagement.

---

# 20. Open product questions

These should be answered through dogfood rather than speculative design:

1. Should the owner’s default Work view be kanban-first or goal-map-first?
2. How many attention categories remain useful in practice?
3. Which actor-performance information improves delegation without becoming surveillance noise?
4. When should an owner message be automatically suggested for promotion into a directive?
5. How much artifact preview belongs in the cockpit versus opening the source tool?
6. Which phase transitions require explicit owner confirmation in real companies?
7. What company-health summary best predicts the need for intervention?
8. How much resolved attention history is useful before it becomes clutter?
9. When does a separate company overview become justified beyond Attention and the global bar?

---

# Working summary

> **The Owner Cockpit is the calm operating surface for one owner to understand and steer a self-running company. It centres four areas: a high-signal Attention Inbox, a hierarchical and kanban Work view, a persistent People directory with direct chat, and a complete Authority view. It preserves stable cross-layer semantics, grounds company claims in external evidence, distinguishes operating phase from lifecycle and work status, and lets the owner govern outcomes and exceptions without babysitting daily execution.**
