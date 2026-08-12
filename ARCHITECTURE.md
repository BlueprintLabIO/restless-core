# Helm Architecture Source of Truth

**Status:** Working draft  
**Version:** 0.9  
**Date:** 12 August 2026  
**Purpose:** Define the target architecture for a system that can run useful companies through autonomous agents while keeping consequential authority bounded and recoverable.

---

## 1. Core thesis

Helm should use **three logical layers**:

1. **Constitutional Kernel** — governance, authority, isolation, secrets, budgets, external effects and recovery.
2. **Organisational Intelligence (`OrgIntel`)** — self-running, self-healing and self-building coordination across agents.
3. **Company Linux Runtime** — the flexible computer where agents perform actual work and produce economic output.

There should still be only **two meaningful trust domains**:

- The trusted host kernel.
- The untrusted-but-useful company environment, containing both OrgIntel and the Linux work runtime.

OrgIntel is a separate architectural layer because it is a distinct product capability, not because it must be a separate security boundary.

```text
Owner
  │ mandate, capital and root authority
  ▼
┌──────────────── HOST TRUST BOUNDARY ────────────────────┐
│ 1. CONSTITUTIONAL KERNEL                                │
│ authority · secrets · effects · budgets · lifecycle     │
│                                                        │
│  ┌──────────── COMPANY ENVIRONMENT ──────────────────┐  │
│  │                                                  │  │
│  │ 2. ORGINTEL PLANE       3. EXECUTION PLANE       │  │
│  │ goals · teams ·         files · Git · browser     │  │
│  │ commitments · memory ⇄ ACP workers · tools       │  │
│  │ Exec · planners         project applications      │  │
│  └──────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────┘
```

This is a responsibility and trust map, not a mandatory call chain. The Exec runs inside the company environment, coordinates through OrgIntel, works directly in the Linux runtime, and calls the kernel only when exercising real-world authority.

The optimisation target is:

> **Useful economic output per unit of owner attention, time, cost and bounded risk.**

The system should not optimise for complete internal auditability, globally perfect state, or making every organisational mistake impossible.

---

## 2. Architectural principles

### 2.1 Strict at authority boundaries; permissive inside

Material spend, external representation, destructive publication, privileged credentials and authority expansion must cross the host kernel. Planning, coding, research, drafting, analysis, internal communication and reversible work should proceed freely inside the company environment.

### 2.2 Governance constrains consequences, not thought

The kernel should decide what the company is permitted to do to the outside world. It should not decide how agents organise a project, edit a file, structure a team or reason about a problem.

### 2.3 OrgIntel coordinates work; it does not grant permission to work

OrgIntel should provide strong guidance, cadence, context and organisational continuity. A temporary OrgIntel failure must not invalidate existing files or prevent already-running agents from continuing ordinary internal work.

### 2.4 Files are the primary work primitive

Documents, code, research, plans, outputs and local knowledge should normally be ordinary files. Git supplies deliberate history and integration. Databases should index and coordinate work, not replace the work itself.

### 2.5 Observable and repairable beats impossible-to-corrupt

Internal state may become stale, duplicated or inconsistent. The normal response is detection, repair, replanning or restoration—not a company-wide governance failure.

### 2.6 Mature infrastructure over bespoke machinery

Use Linux, OCI, Git, Postgres, established process supervision, browser infrastructure and snapshots. Do not build a custom database, container runtime, content-addressed custody protocol or durable workflow engine unless a demonstrated workload requires it.

### 2.7 Engineering practice is the primary anti-drift mechanism

Helm should stay aligned through shared judgement, real dogfood, outcome-oriented planning, small reversible changes and regular simplification—not by adding another architectural gatekeeper. Hard enforcement belongs at true authority boundaries. Everywhere else, the team should prefer evidence, conventions, review and recovery.

### 2.8 Future scope must earn its way in

Multiplayer collaboration, managed hosting and shared multi-tenancy are product hypotheses, not initial architecture requirements. Helm should first prove that one owner, one Exec and a small group of agents can produce useful economic output. Add broader collaboration or cloud infrastructure only after repeated real use exposes the need.

---

# 3. Layer 1: Constitutional Kernel

## 3.1 Mission

The kernel is the small trusted computing base. It controls the company’s **authority and blast radius**, not its daily work.

It must remain small enough to reason about, test aggressively and operate reliably.

## 3.2 What the kernel owns

### Company identity and lifecycle

- Company identity and owner relationship.
- Active, suspended, stopped and destroyed status.
- Creation, boot, stop, restart, snapshot and restore of the company environment.
- Isolation profile and resource limits.

### Outer constitution and authority

- Owner-controlled mission boundaries.
- Standing capability grants.
- Revocation and expiry.
- Budgets, approval thresholds and rate limits.
- Human attachment and emergency stop.

### Secrets and credentials

- Provider API keys.
- Payment, email, deployment and infrastructure credentials.
- Credential scoping and rotation.
- Delivery of short-lived or narrowly scoped credentials where unavoidable.

Helm should use **Infisical as the default imported secrets and machine-identity backend**, behind a kernel-owned adapter. Infisical stores, rotates and supplies credentials; Helm remains responsible for capability semantics, budgets, approvals, effect idempotency and receipts. Low-risk tools may use a credential-brokering proxy so agents never receive raw secrets, while consequential actions still pass through Helm's effect broker.

The deployment choice should remain replaceable: OSS users may [self-host Infisical](https://infisical.com/docs/self-hosting/overview), while the managed product may use [Infisical Cloud or a managed self-hosted deployment](https://infisical.com/docs/documentation/getting-started/concepts/deployment-models). Infisical is an implementation dependency, not part of Helm's constitutional ontology.

### Model and compute access

- Model-provider credential isolation.
- Company-level cost accounting and limits.
- Provider routing and rate limiting.
- Optional model restrictions for specific capability classes.

### Consequential external effects

Examples include:

- Sending externally attributable email.
- Publishing or deleting public content.
- Spending money.
- Deploying to production.
- Creating, closing or modifying external accounts.
- Changing access control.
- Executing a contract or legally meaningful representation.

The kernel should execute these through a small effect broker with:

- authenticated company identity;
- explicit capability checks;
- scoped arguments;
- idempotency keys;
- approval handling;
- authoritative receipts;
- unknown-outcome reconciliation.

### Recovery and authoritative audit

The kernel records only governance-relevant truth:

- authority changes;
- approvals;
- capability grants and revocations;
- material external effects;
- effect receipts and ambiguous outcomes;
- budget consumption;
- lifecycle and recovery actions.

It does **not** permanently govern every internal message, thought, file edit or process event.

## 3.3 Hard invariants worth encoding

These are appropriate for code-level enforcement, database constraints and adversarial testing:

1. A company cannot exercise a capability it has not been granted.
2. A guest process cannot modify host authority policy or retrieve unrestricted host credentials.
3. Grants are scoped, attributable, revocable and optionally expiring.
4. Budget and approval thresholds are checked before effect execution where technically possible.
5. External effects use idempotency and do not blindly repeat an ambiguous outcome.
6. The owner can stop the company and revoke future external authority.
7. A compromised company environment can be replaced without inheriting host control.
8. Kernel failure pauses new privileged effects but does not need to freeze unrelated internal work.

## 3.4 What must not enter the kernel

- Internal tasks, attempts or project plans.
- Team structures and staffing decisions.
- Ordinary file ownership or artifact versioning.
- Agent scratchpads and reasoning.
- Internal reviews and review-read receipts.
- Workspace leases for ordinary work.
- Universal commands for all mutations.
- The canonical state of Git branches or working directories.
- Planner, critic or resource-allocation logic.

## 3.5 Exec operating envelope

The Exec may operate across the whole company, but it does not receive host root.

- In the runtime, it has broad access to files, Git, browser, tools and worker processes.
- In OrgIntel, it is the accountable owner of priorities, teams, delegation and internal operating processes.
- At the kernel boundary, it may use granted capabilities, allocate sub-budgets, delegate narrower permissions, revoke delegated authority and request additional authority.

The kernel remains the sole writer of authoritative grants, approvals, receipts and lifecycle state. The Exec cannot expand its own total authority, change the owner mandate, rewrite kernel policy, alter receipts or disable owner stop and recovery controls.

No LLM should be authoritative in the kernel decision path. Models may summarise or advise, but authentication, capability checks, budgets, approvals, effect execution and receipts remain deterministic.

## 3.6 Likely implementation

- A small **Rust modular monolith** rather than many services.
- **Postgres** for host truth.
- **Infisical** for secret storage, machine identities, credential rotation and optional agent-side credential proxying.
- OCI containers for development and early deployment.
- Stronger isolation such as gVisor or microVMs when hosted multi-tenancy justifies it.
- A narrow authenticated API, preferably local sockets for same-host deployments.
- Snapshot storage backed by volumes and object storage where needed.

A plausible host schema should remain small: companies, environments, grants, secrets, approvals, effects, receipts, budgets and snapshots.

---

# 4. Layer 2: Organisational Intelligence (`OrgIntel`)

## 4.1 Mission

OrgIntel turns a durable agent computer into a continuously operating organisation.

Its role is to maintain:

- direction;
- coordination;
- continuity;
- organisational memory;
- proactive work generation;
- adaptation and recovery;
- visibility for the owner.

Without this layer, Helm risks becoming only another general-purpose agent harness with a persistent shell and tools.

## 4.2 Product promise

OrgIntel should make the company:

### Self-running

- Maintain active goals and next actions.
- Wake the right actors without waiting for a human prompt.
- Plan at daily, project and strategic horizons.
- Schedule follow-ups and recurring operations.
- Notice new results and trigger dependent work.

### Self-healing

- Detect crashed or silent agents.
- Detect stalled, duplicated or contradictory work.
- Restart, reassign, simplify or replan.
- Preserve useful artifacts when a process fails.
- Escalate only when the organisation cannot recover autonomously.

### Self-building

- Create new specialist agents and roles.
- Develop playbooks and reusable procedures.
- Install or build internal tools and services.
- Change team structures and planning patterns.
- Evaluate whether new organisational mechanisms improve outcomes.
- Remove mechanisms that add overhead without value.

Self-building applies inside the company environment. OrgIntel may **request** new host capabilities, but it may not grant them to itself.

## 4.3 Exec and agent placement

The Exec logically belongs to OrgIntel but physically runs as an ordinary ACP process inside the Linux company environment. Its identity, mandate, inbox and organisational memory persist even when the model session or process restarts.

A sensible initial topology is:

```text
1 persistent Exec identity
├── on-demand planner, critic and recovery sessions
└── 1–4 task-focused ACP workers
```

OrgIntel combines two kinds of machinery:

- **Deterministic substrate:** actors, messages, goals, commitments, schedules, wakeups, process health, artifact references and context retrieval.
- **Model-driven judgement:** Exec, planning, staffing, review, recovery, memory curation and process improvement.

The kernel has no autonomous LLM authority. Runtime workers contain task intelligence for coding, design, research and production. One model may power several distinct agents, and one agent may use several models.

## 4.4 Stable coordination core

OrgIntel should have a small stable substrate, potentially exposed by a resident service. This is the justified role for a `companyd`-like component, although the final name should reflect the product rather than an arbitrary daemon convention.

The stable core may own:

### Actors and sessions

- Human, agent and service identities used for coordination.
- Roles and lightweight competency profiles.
- Current availability and active sessions.
- Requests to spawn, wake, pause or retire ACP agents.

### Goals and commitments

A deliberately small ontology:

- **Goal** — an outcome the company is pursuing.
- **Commitment** — an actor has agreed to produce or decide something.
- **Actor** — the person, agent or service responsible.
- **Artifact reference** — where the output lives.
- **Decision** — a consequential internal choice and its rationale.

A commitment should have only a few states, such as proposed, active, blocked, completed and abandoned.

### Messaging and inboxes

- Assignments.
- Updates.
- Questions.
- Review requests.
- Decisions.
- Attachments or links to files and commits.

Messages are organisational communication, not security-sensitive state transitions.

### Scheduling and wakeups

- Periodic executive planning.
- Deadlines and follow-ups.
- Event-driven wakeups.
- Retry and escalation timing.
- Daily, weekly and strategic review cadence.

### Context assembly

When an actor wakes, OrgIntel should assemble relevant context from:

- mission and current priorities;
- active goals and commitments;
- recent messages;
- linked files and repositories;
- relevant decisions;
- current blockers;
- role-specific guidance;
- selected organisational memory.

### Operational event stream

Examples:

- actor started or stopped;
- commitment became blocked;
- artifact was produced;
- review was requested;
- plan changed;
- external effect returned a receipt.

This stream supports UI, debugging and organisational awareness. It is not a constitutional ledger and may be compacted, repaired or regenerated.

## 4.5 Replaceable intelligence modules

The stable coordination core should not contain every theory of management. Higher-order behaviour should be modular and replaceable:

- Executive planner.
- Resource allocator.
- Team designer.
- Reviewer and critic router.
- Stagnation detector.
- Organisational historian.
- Progress summariser.
- Cost and owner-attention optimiser.
- Knowledge curator.
- Process and playbook evaluator.

These modules may be ACP agents, prompt-driven policies, scripts or ordinary services. They read shared coordination state and propose or perform changes. They should not require a new constitutional entity every time the theory changes.

## 4.6 What OrgIntel can encode

### Strong conventions

- Every active goal should have an owner or an explicit reason it is paused.
- Every active commitment should have a next step, expected result or blocker.
- Important completed work should point to a concrete artifact or decision.
- Review effort should scale with consequence, uncertainty and reversibility.
- Repeated failure should trigger a changed approach rather than infinite retry.
- Stale work should be re-evaluated.
- Duplicate work should be surfaced.
- Owner attention should be reserved for genuine judgement, authority or ambiguity.

### Soft policies and heuristics

- Planning cadence.
- Team-shape recommendations.
- Review routing.
- Work-in-progress limits.
- Escalation thresholds.
- Context-selection policies.
- Agent competency estimates.
- Resource allocation heuristics.

These should behave like management guidance, linting and automated coaching—not kernel permissions.

## 4.7 What OrgIntel must not do

- Gate every filesystem write or shell command.
- Require all work to pass through its API.
- Hold unrestricted secrets or privileged external credentials.
- Directly grant new host authority.
- Treat stale internal state as a security incident.
- Make a completed artifact invalid because its task record is inconsistent.
- Replace Git, Postgres, process supervision or the browser.
- Become the only place where mission, knowledge or outputs exist.
- Recreate a universal command and event algebra.

## 4.8 Failure posture

If OrgIntel fails:

- Already-running agents may continue internal work.
- Files, Git repositories and browser state remain usable.
- New scheduling, messaging and agent spawning may pause.
- Host-governed external effects remain independently safe.
- OrgIntel restarts against recoverable coordination state.
- It may rebuild indexes and reconcile itself against files, processes and Git.

If OrgIntel must be perfectly available for any work to remain valid, it has become too sovereign.

## 4.9 Data and implementation

For the expected multi-agent concurrency and real-time UI, **Postgres is a reasonable default** for OrgIntel state. The important point is not SQLite versus Postgres; it is that this database is ordinary, recoverable company state and is not part of the constitutional trust boundary.

A small schema might cover:

- actors;
- goals;
- commitments;
- messages;
- schedules;
- reviews;
- decisions;
- artifact references;
- operational events;
- derived health signals.

Postgres may be host-operated for convenience with isolated company credentials, while remaining logically owned by the company environment. Avoid a second workflow database or DBOS-style durable orchestration layer.

The coordination core can remain in Rust if that preserves existing expertise and components. Intelligence modules should be language-agnostic and easy for agents to add or replace.

---


## 4.10 Business processes: stable contract, flexible execution

OrgIntel should model business processes as a stable contract around a flexible plan—not as a mandatory sequence of state transitions.

A process has four layers:

| Part | Encodes | Rigidity |
|---|---|---|
| **Outcome contract** | Desired result, accountable owner, required outputs and success measures | Stable for the run |
| **Control points** | Budget, approval, legal or irreversible-effect boundaries | Hard only where necessary |
| **Versioned playbook** | Recommended stages, roles, tools and review patterns | Default and overridable |
| **Execution plan** | The actual steps, agents, experiments and files used this time | Freely adaptable |

The process owner may change the plan during a run when evidence changes. Anyone may propose a better method. After the run, the accountable owner decides whether successful deviations should update the default playbook.

```text
ad hoc solution
→ repeated useful pattern
→ documented playbook
→ optional tooling
→ hard invariant only when failure is genuinely unacceptable
```

The reverse path matters too: rules that no longer help should be weakened or removed.

OrgIntel should version process templates and record which version a run began from, but it should not require every action to match that template. Deterministic software handles reminders, schedules, outputs and control points. Model-driven agents choose strategy, handle exceptions and improve the process.

# 5. Layer 3: Company Linux Runtime

## 5.1 Mission

The runtime is where productive work happens. It should feel like a capable company computer, not a restricted workflow appliance.

The runtime is intentionally flexible, imperfect and recoverable.

## 5.2 What it provides

- Persistent filesystem and home directories.
- Shell and ordinary Linux processes.
- ACP Codex, Claude and other agents as real processes.
- Browser and desktop session.
- Git repositories and worktrees.
- Programming languages, package managers and build tools.
- Documents, spreadsheets, datasets and media tools.
- Project-specific databases and services.
- Network access appropriate to the company profile.
- Local IPC, logs, caches and temporary files.
- Snapshot and restore support.

Agents may create new directories, scripts, applications, services and internal workflows without first extending a global domain model.

## 5.3 Filesystem model

Files are the default representation of work.

A company may adopt conventions such as:

```text
/company/
  mission.md
  constitution.md
  org/
  goals/
  projects/
  decisions/
  knowledge/
  outputs/
  repos/
  workspaces/
```

These conventions should make the organisation legible to both humans and agents, but they should remain evolvable.

The filesystem may contain:

- live drafts;
- raw research;
- code;
- plans;
- decision records;
- reports;
- datasets;
- generated outputs;
- project-local state.

Not every file needs a corresponding relational entity.

## 5.4 Git model

Git is for **meaningful checkpoints, attribution, integration and rollback**. It is not the real-time state bus of the company.

### Live state

Normal files, working directories, agent messages and OrgIntel state represent work in progress.

### Durable state

Meaningful outputs become commits when they are ready for handoff, review, integration or long-term preservation.

### Recommended rules

1. Software agents use separate branches or worktrees rather than one shared mutable working tree.
2. Commit at meaningful milestones, not after every tool call.
3. A handoff or review request should point to a commit when the work is suitable for versioning.
4. Shared integration branches are not force-pushed.
5. A designated integrator, project lead or Exec owns final integration where conflict risk is material.
6. Snapshot or checkpoint before destructive repository operations.
7. Secrets, browser profiles, caches and large transient state do not belong in Git.
8. Non-code work uses Git only where deliberate version history is useful.

Realtime coordination belongs in OrgIntel. Git records durable project evolution.

## 5.5 Runtime freedom and bounded risk

The runtime may be messy:

- agents can create poor directory structures;
- dependencies can conflict;
- plans can become stale;
- duplicate work can occur;
- repositories can require repair;
- internal tools can fail.

These are acceptable operational failures. Use Git, process restart, snapshots, backups and OrgIntel recovery loops rather than mediating every action.

The runtime must not receive ambient high-impact credentials or host container authority. Broad work capability should come from tools and network access; consequential authority should come from the kernel.

## 5.6 Likely implementation

- A standard Ubuntu or Debian-derived company image.
- OCI containers for local development and early single-tenant deployment.
- Stronger VM or microVM isolation only when real managed-hosting or multi-tenant demand justifies it.
- Persistent volumes for company home, workspace and browser state.
- Mature process supervision rather than a custom durable workflow engine.
- A persistent browser/desktop that a human can attach to directly.
- Optional project-level containers or environments for dependency isolation.

Do not build a custom Linux kernel. Build a well-defined company image, profile and host contract.

---

# 6. Cross-layer contracts

## 6.1 Kernel ↔ company environment

The host should expose only a small set of capabilities:

- model access;
- capability inspection;
- effect request and status;
- approval request and status;
- scoped secret use;
- lifecycle and snapshot requests;
- human attachment;
- authoritative receipt retrieval.

The request interface should identify the company and operation, carry an idempotency key where needed, and return a clear result: succeeded, denied, awaiting approval, failed or outcome unknown.

## 6.2 OrgIntel ↔ agents and runtime

OrgIntel may provide:

- actor/session launch and wake requests;
- inbox and messaging;
- goal and commitment coordination;
- scheduling;
- context packages;
- artifact and Git references;
- health and progress signals;
- review and escalation requests.

Agents remain free to use the filesystem, Git, shell and ordinary applications directly.

## 6.3 Artifact references, not custody protocols

OrgIntel should generally refer to work using paths, repository locations, commits and URLs. It should not export, import, materialise and reattach every artifact through a custody state machine.

Examples:

- filesystem path;
- repository plus commit;
- worktree and branch;
- database record owned by a project;
- external URL plus receipt where appropriate.

---

# 7. Operating and change model

Real companies do not route all change through one universal workflow. The default model is:

```text
observe or propose → nearest accountable owner decides → capable actor or team implements → inspect the result
```

Anyone may identify a problem, propose a change, gather evidence or prototype a solution. Decision and implementation ceremony scale with blast radius, cost and reversibility.

## 7.1 Exec authority

Use one persistent Exec identity across three interfaces:

| Surface | Exec authority |
|---|---|
| **Linux runtime** | Broad direct access to files, Git, browser, tools, processes and workers |
| **OrgIntel** | Full authority over internal goals, teams, delegation, planning, review and operating processes |
| **Kernel** | Capability-based requests inside the owner-granted envelope; no direct database or policy edits |

The Exec controls the company’s internal operation. The owner controls the outer mandate and total authority. The kernel enforces that boundary.

## 7.2 Change ownership by layer

| Change | Default decision-maker | Typical implementer |
|---|---|---|
| Local, reversible work | Individual worker | Same worker |
| Shared runtime system or repository | Named lead or maintainer | Relevant team |
| Team structure, planning or internal process | Exec or delegated manager | OrgIntel/runtime team |
| Company-wide OrgIntel extension | Exec or delegated OrgIntel owner | Builder plus independent test or critique |
| Use of an existing capability | Authorised Exec or worker | Kernel executes deterministically |
| Expansion of budget, capability or mandate | Owner/root authority | Kernel records and enforces |
| Helm kernel or stable platform code | Helm platform maintainers | Platform engineering team or coding agents under review |

Most changes remain within one layer. Runtime changes do not normally reach OrgIntel. Organisational changes do not normally reach the kernel. Escalate only when a change exceeds the current owner’s authority or blast radius.

## 7.3 Single agent versus team

- A single agent may make local, obvious and reversible changes.
- A lead or maintainer decides changes to shared project state.
- A builder plus independent reviewer is appropriate for company-wide coordination machinery or changes with subtle failure modes.
- A small cross-functional team handles changes spanning several disciplines or materially changing company strategy.
- Owner approval is reserved for authority expansion, major capital exposure, public commitments and weakened recovery or oversight.

This is ordinary delegated management, not a universal governance protocol.

## 7.4 Initial product and deployment posture

The near-term product is a **single-company operating system**: one owner, one persistent Exec and a small set of agents working inside one isolated company environment. This is sufficient to test Helm's core claim.

Multiplayer and hosted deployment remain deliberately deferred:

- Support only the human interactions required by real dogfood: directives, approvals, inspection and browser or desktop takeover.
- Do not build presence, a general collaboration suite, fine-grained multiplayer permissions, a shared realtime filesystem or a tenant fleet control plane yet.
- Avoid obvious dead ends by using actor/principal identifiers, isolating company state, keeping layer interfaces explicit and supporting backup and upgrades. Do not design the full future platform.
- Add a second human when repeated dogfood shows that human collaboration improves outcomes and Helm itself is the obstacle.
- Build managed hosting when users want Helm but will not operate it. Begin with a dedicated deployment per company.
- Build shared multi-tenancy only when proven demand exists and the cost of dedicated deployments materially blocks scale.

The expected progression, if evidence supports it, is:

```text
single-company appliance
→ managed dedicated company instances
→ shared multi-tenant infrastructure only if economically necessary
```

---

# 8. Placement test for new features

Ask these questions in order:

1. **Can this action create material external harm, spend, representation or authority expansion?**  
   Put enforcement in the Constitutional Kernel.

2. **Does this help multiple actors coordinate across time, goals or dependencies?**  
   Put it in OrgIntel.

3. **Is this actual productive work, a tool, an artifact or project-specific state?**  
   Put it in the Linux runtime.

4. **Can the problem be repaired by restart, replan, Git recovery or snapshot restore?**  
   It probably does not belong in the kernel.

5. **Would enforcing this invariant block useful internal work when its state is stale?**  
   Prefer OrgIntel guidance over hard enforcement.

---

# 9. Hardening priorities

Harden the seams that preserve productive work and prevent irreversible harm. Do not harden ordinary organisational mess into a security protocol.

## 9.1 Unambiguous state ownership

- **Kernel:** identity, grants, budgets, approvals, effects, receipts and lifecycle.
- **OrgIntel:** actors, goals, commitments, messages, schedules and coordination health.
- **Runtime:** files, code, assets, builds, project databases and actual outputs.

Derived views may cross layers, but each fact has one authoritative home. Avoid cross-layer foreign keys and duplicated truth.

## 9.2 External history must survive runtime time travel

A runtime snapshot can be restored; the outside world cannot be rolled back. Kernel receipts and effect history must therefore survive every company restore. After restoration, OrgIntel reconciles outstanding effects before retrying anything ambiguous. Unrelated internal work may continue.

## 9.3 Reliable coordination, not perfect workflow semantics

Harden durable inbox delivery, scheduled wakeups, process visibility, restart reconciliation and artifact references. Internal messages and wakeups may use at-least-once delivery and tolerate duplicates. Consequential external effects require idempotency and authoritative reconciliation.

## 9.4 Safe self-building and upgrades

Keep a known-good bootstrap capable of starting the basic inbox, scheduler and Exec with optional extensions disabled. Company-built OrgIntel modules and runtime upgrades should be versioned, canaried, reversible and disableable. Snapshot before material upgrades and keep rollback independent of the new version functioning correctly.

## 9.5 Owner truth and context boundaries

Mount the owner mandate and current grants as read-only authoritative inputs. Keep editable strategy and plans separate. Context assembly should distinguish owner directives, internal decisions, working hypotheses, historical memory and untrusted external content.

## 9.6 Economic controls and human rescue

Bound model spend, external service spend, concurrency, retries and runaway background activity. Failure should remain local: pause the actor, expensive capability or affected effect rather than freezing the company. The owner must be able to freeze new consequences, inspect the same workspace/browser, stop one actor, restore and resume.

## 9.7 Collaboration around files and Git

Use separate worktrees for code agents, meaningful commits for handoff, automatic checkpoints before destructive operations and lightweight warnings for likely write collisions. Prefer these conventions and recovery tools over asset custody or lease protocols.

---

# 10. Cosmon reference company and smoke tests

Cosmon is the reference company: a small game studio building a 3D space MMORPG with creature discovery, capture, training and battle mechanics. It stresses creative judgement, software delivery, art pipelines, multiplayer risk, long-running work, spending and public effects.

## 10.1 Real-company shape

Early Cosmon should be a small cross-functional studio, with actors wearing multiple hats:

```text
Owner
└── Exec / Studio Head
    ├── Game Director
    ├── Technical Director
    ├── Producer / OrgOps
    └── milestone team
        ├── gameplay/design worker
        ├── gameplay engineer
        ├── technical artist
        └── online feasibility worker
```

The organisation should change with the phase. Discovery uses one small team. A vertical slice adds clearer gameplay, art and online workstreams. Production and live service may justify persistent leads and specialised teams. OrgIntel should help the structure evolve rather than encode a permanent org chart.

## 10.2 First economic objective

Cosmon does not yet have a game, so its first bottleneck is product creation—not playtest recruitment or growth. The Exec should not attempt the full MMORPG, but it should build the smallest integrated browser game that proves the core product can exist:

> Produce a working browser game with an end-to-end exploration–encounter–capture–battle loop and a credible foundation for later iteration.

A useful first milestone produces:

- one explorable 3D zone;
- three original creatures;
- one encounter and capture loop;
- one basic battle;
- a browser-deployable playable build;
- a bounded multiplayer feasibility spike;
- a concise technical and product assessment;
- a recommendation for the next build milestone.

Playtesting becomes valuable once this first playable artifact exists. The primary pass condition is the integrated working game, not research or planning about a future game.

## 10.3 Layer responsibilities in Cosmon

| Layer | Cosmon responsibility |
|---|---|
| **Kernel** | Studio budget, licences and purchases, public deployment, external communications, player-data authority, monetisation, receipts and owner approvals |
| **OrgIntel** | Milestones, teams, commitments, risk ordering, review, playtest cadence, blocker detection, reallocation and owner summaries |
| **Runtime** | Game-engine project, source code, 3D assets, builds, playtest recordings, scripts, repositories, browsers and project tools |

The Exec can operate across all three: it manages the studio through OrgIntel, works directly in the runtime, and invokes kernel capabilities within its granted envelope.

## 10.4 Organisational-intelligence principles

1. **Work backwards from existential risk.** Build the smallest playable core before growth work or MMO-scale infrastructure, then test fun and production feasibility.
2. **Playable output is the strongest evidence.** Completed tasks without a compelling build are not success.
3. **Organise around outcomes.** A capture-loop team owns the player-visible result across design, code, art and testing.
4. **Make decision rights explicit.** Workers own local reversible choices; leads own shared domains; Exec owns milestones and resource allocation; owner controls mandate and capital exposure.
5. **Run several planning horizons.** Unblock immediate work, inspect milestones, reconsider strategy and change the organisation when repeated friction appears.
6. **Review proportionally.** Scratch work needs little ceremony; core creative direction needs playtesting; major architecture needs technical review; public effects cross the kernel.
7. **Treat failure as information.** Failed prototypes should change the plan, not trigger blind retries.
8. **Let the company build its own tools.** Editors, telemetry, asset validators, build pipelines and specialist agents belong in the company environment.

## 10.5 Canonical user story

The owner directs Cosmon to prove whether the core space-creature experience deserves further investment, while keeping the prototype private, using original or licensed assets and staying within a fixed budget.

A passing run looks like this:

1. Exec interprets the mandate and chooses a bounded risk-reduction milestone.
2. OrgIntel forms a temporary cross-functional team and assigns clear outcomes.
3. Workers create the game project, assets, files and Git worktrees directly in Linux.
4. OrgIntel tracks commitments and blockers without mediating ordinary edits.
5. A disagreement or requirement change is resolved using design pillars, prototypes and expected economic value.
6. A stalled or crashed worker is replaced without discarding useful files and commits.
7. A playable build is tested by an independent critic or playtest function.
8. Any purchase or public action crosses the kernel and produces a receipt or approval request.
9. Exec integrates evidence and recommends continue, pivot or stop.

The owner receives the build, important artifact links, playtest evidence, technical findings, cost, unresolved risks and only the decisions requiring owner judgement.

## 10.6 Smoke-test scenarios

| Scenario | Expected behaviour |
|---|---|
| Vague mission | Exec converts the ambition into a bounded, risk-driven milestone |
| Midstream requirement change | Only affected work is revised; useful files and commits remain usable |
| Stalled or crashed worker | OrgIntel detects the issue and reassigns from preserved work |
| Duplicate work | The Exec merges, compares or stops the duplication deliberately |
| Creative disagreement | The accountable lead decides using pillars, prototypes and playtest evidence |
| Premature infrastructure | Resources move back toward proving the core loop |
| Bad playtest | The company revises or kills the idea rather than declaring task-based success |
| External purchase | Only the purchase waits for approval; internal work continues |
| OrgIntel outage | Running agents and filesystem work continue; coordination resumes after restart |
| Snapshot after external effect | Kernel history survives and the effect is reconciled before any retry |
| Prompt injection or compromised worker | Internal work may be disrupted, but authority remains bounded by credentials and capabilities |
| Human takeover | Owner enters the same workspace/browser, corrects the situation and returns control |

These scenarios begin as manually observed company runs. Before automating one, the team should agree on the useful artifact or decision, expected autonomy, OrgIntel memory required, exact kernel boundary and acceptable failure posture.

---


## 10.7 Department-level dogfood portfolio

Department dogfoods are the bridge between toy agent tasks and a whole autonomous company. The first department should match the company’s current bottleneck rather than forcing every business through the same growth-first sequence.

| Company | First team | Primary proof |
|---|---|---|
| **Cosmon** | Game Product Team | Agents can build and integrate a working browser game |
| **Aris** | Sales & Marketing | Agents can create demand, close sales and learn which segment, offer and channel work |
| **Thymelake** | Restaurant Launch Team | Agents can sell, onboard and operate a real B2B product through continued use |

Together these test three materially different forms of economic work:

- **Cosmon:** building a product;
- **Aris:** selling an existing product;
- **Thymelake:** selling, deploying and operating a product in the customer’s environment.

### 10.7.1 Cosmon: Game Product Team

Cosmon’s first department is a small cross-functional product team, not Growth and Player Research.

Example mission:

> Produce a working browser game that demonstrates exploration, creature encounters, capture and battle in one integrated experience.

The initial team may include a game/design lead, gameplay engineer, 3D technical artist and browser/platform engineer. Review or playtesting follows once a playable build exists.

The passing artifact is a playable URL or build. This dogfood should exercise:

- ambiguous creative decomposition;
- multi-agent code and asset production;
- Git branches and worktrees;
- cross-disciplinary integration;
- build and deployment failures;
- technical disagreement and milestone replanning;
- preservation and reassignment of partially completed work.

The team should avoid premature MMO infrastructure. The first success is a coherent game loop, not a complete backend, content catalogue or growth campaign.

### 10.7.2 Aris: Sales & Marketing

Aris already has a sellable product: selective-exam practice papers generated from a large question supply. Its immediate bottleneck is distribution and commercial validation.

Example mission:

> Sell practice papers to real parents, students, tutors or coaching centres, identify the strongest segment, offer and channel, and produce evidence for the next commercial decision.

The operating loop is:

```text
choose segment
→ create offer
→ find prospects
→ outreach or campaign
→ close purchase
→ deliver papers
→ collect usage and objections
→ refine the offer
```

The passing evidence is real revenue, repeat purchase, or a strong qualified sales pipeline—not more question generation. This department should exercise:

- prospect research and segmentation;
- positioning, pricing and offer design;
- campaign content and landing pages;
- CRM-like ownership and follow-up;
- email, payments, deployment and customer-data capabilities;
- funnel analysis and feedback into the product.

For direct-to-parent or direct-to-student sales, the team may behave mainly like growth marketing. For tutors, coaching centres or schools, it may use a more explicit sales pipeline. The department can adapt its method while remaining accountable for commercial outcomes.

### 10.7.3 Thymelake: Restaurant Launch Team

Thymelake’s first proof should cross sales, onboarding, operations and product rather than isolate a generic marketing department.

Example mission:

> Acquire one restaurant, configure its real menu, launch QR ordering at real tables, process real orders reliably, resolve operating issues, and prove that the venue wants to continue using or paying for the product.

The operating loop is:

```text
restaurant prospect
→ discovery and demo
→ pilot agreement
→ menu and venue setup
→ QR deployment
→ staff onboarding
→ live orders
→ support and issue resolution
→ value review
→ paid continuation
```

The first proof is **pilot viability**:

- a restaurant agrees to use the system;
- staff can operate it without constant intervention;
- diners understand the ordering flow;
- orders reach the correct destination accurately and promptly;
- unavailable items, edits, refunds and failures can be handled;
- the restaurant perceives enough value to continue.

The second proof is **repeatability**:

- a second and third restaurant can be launched without bespoke engineering;
- menu setup and onboarding become faster;
- repeated support problems become playbooks, automation or product improvements;
- sales and launch cost can plausibly be recovered from revenue.

The third proof is **economic value**, such as lower ordering workload, fewer errors, higher basket size, faster service or another measurable venue benefit.

Thymelake is the strongest whole-company dogfood of the three because product, sales, onboarding, support and external side effects must operate as one continuous loop.

## 10.8 Simulated external world

Helm should support external-world simulation behind the same kernel provider interfaces used in production.

```text
company request
→ kernel capability and budget checks
→ provider adapter
   ├── real provider
   └── simulated provider
```

Use two simulator types:

- **Deterministic simulators** for kernel correctness: success, denial, exhausted budget, duplicate requests, delayed approval, lost responses and ambiguous outcomes requiring reconciliation.
- **Behavioural simulators** for OrgIntel: customer personas, replies, conversion behaviour, objections, changing requirements and noisy market feedback.

The company should not need different logic for a simulated provider. Development should progress from scripted simulation, to noisy behavioural simulation, to a small controlled real run. Simulation tests operating behaviour; only the real world validates demand, product quality and customer value.

The simulated world should support the first dogfoods without becoming a second product:

- **Cosmon:** build deployment, asset purchase, collaborator feedback and failure conditions;
- **Aris:** email, landing-page traffic, payments, replies, objections and follow-ups;
- **Thymelake:** restaurant prospects, pilot approval, menu data, test orders, outages, refunds and support incidents.

# 11. Success metrics

Architecture decisions should be judged by real company performance:

- Accepted artifacts or outcomes shipped.
- Revenue generated or cost saved.
- Time from goal to useful result.
- Owner interventions per accepted result.
- Cost per accepted result.
- Recovery time after agent or runtime failure.
- Percentage of active work with a concrete artifact or decision.
- Duplicate work and abandoned-work rates.
- Time spent on productive work versus governance machinery.
- External incidents and maximum realised blast radius.

A green invariant suite is useful, but it cannot compensate for a company that fails to produce outcomes.

---

# 12. Explicit anti-goals

Helm should not attempt to:

- make all invalid internal organisational states impossible;
- capture every action in an immutable ledger;
- route every mutation through a universal command type;
- model every file as a governed asset lifecycle;
- turn Git into a real-time transaction system;
- require OrgIntel availability for agents to edit files or run tools;
- let OrgIntel grant itself host authority;
- create a bespoke workflow engine before real workloads require one;
- build a custom secret manager, container runtime or Linux kernel;
- treat internal messiness as equivalent to a security breach;
- build a general multiplayer collaboration product before a second human is proven necessary;
- build shared multi-tenant cloud infrastructure before dedicated deployments have proven demand and poor economics.

---

# 13. Current architectural decisions

1. Adopt three logical layers: Kernel, OrgIntel and Linux Runtime.
2. Preserve only two hard trust domains: host and company environment.
3. Place OrgIntel inside the company trust domain by default so it can evolve and self-build.
4. Keep the kernel authoritative only for identity, authority, secrets, external effects, budgets, lifecycle and recovery.
5. Keep OrgIntel authoritative only for recoverable coordination state.
6. Use normal files as the primary work representation.
7. Use Git for meaningful checkpoints and integration, not continuous organisational state.
8. Use Postgres for OrgIntel when concurrency and real-time coordination justify it, while keeping it outside constitutional truth.
9. Allow ACP Codex, Claude and other agents to run as ordinary processes in the company environment.
10. Optimise for economic output and low owner attention, with bounded rather than perfect safety.
11. Use one persistent Exec identity with broad runtime and OrgIntel authority, but only capability-based kernel access.
12. Allow anyone to propose change; let the nearest accountable owner decide and the capable actor or team implement.
13. Keep the kernel deterministic in its authority path and prevent any actor from expanding its own total authority.
14. Use Cosmon as a reference company and smoke-test portfolio for creative, technical, organisational and external-effect behaviour.
15. Model business processes as stable outcome contracts plus control points, versioned playbooks and flexible execution plans.
16. Use department-level dogfoods before attempting a whole autonomous company.
17. Match each first department to the company’s real bottleneck: Cosmon Game Product, Aris Sales & Marketing, and Thymelake Restaurant Launch.
18. Use the three companies as a complementary dogfood portfolio for building, selling, and live B2B deployment and operations.
19. Test kernel effects through interchangeable real and simulated provider adapters.
20. Use Infisical as the default imported secret and machine-identity backend, while keeping Helm authoritative for capabilities, approvals and consequential effects.
21. Treat multiplayer and managed hosting as unproven product hypotheses rather than initial requirements.
22. Focus the first product on one owner, one Exec and agents inside a single isolated company environment.
23. If managed demand emerges, begin with dedicated per-company deployments; add shared multi-tenancy only when its economics are demonstrated.
24. Future-proof minimally through actor/principal identifiers, company-state isolation, explicit layer interfaces, backups and upgrades—not speculative collaboration or fleet infrastructure.

---

# 14. Open questions

These require further design rather than immediate implementation:

1. Final name and exact scope of the stable OrgIntel coordination service.
2. Whether OrgIntel Postgres is per-company, schema-isolated in a shared service, or embedded for local development.
3. How actor identity is attributed across ACP sessions without turning it into a security-heavy identity system.
4. The minimum filesystem conventions worth standardising.
5. The exact Git/worktree integration and automatic checkpoint policy.
6. How OrgIntel reconstructs state from files, Git and processes after corruption.
7. Which external actions must always use brokered effects versus a normal restricted browser account.
8. How self-built organisational modules are evaluated, promoted and removed.
9. The smallest viable host effect and capability API.
10. The exact initial capability and budget envelope for each live Exec, including which effects remain owner-approved.
11. The exact first acceptance criteria for the Cosmon playable browser build, Aris sales outcome and Thymelake restaurant pilot.
12. Which company should provide the first walking-skeleton dogfood and which portions begin in simulation.
13. Which provider interfaces require deterministic simulation before the first live run.

---

# 15. Immediate validation slice

The next architecture should be tested with one end-to-end department rather than another broad framework expansion:

1. Boot one durable company Linux environment.
2. Run one human owner, one persistent Exec and multiple ACP workers.
3. Maintain goals, commitments, messages and scheduled wakeups through a thin OrgIntel core.
4. Produce real files and Git commits through ordinary tools.
5. Recover from an agent crash without losing useful work.
6. Recover from an OrgIntel restart without invalidating the workspace.
7. Complete receipt-backed external effects first against deterministic simulated providers, then in a small controlled live run.
8. Require one human judgement or handoff.
9. Measure the company-specific outcome—playable build, sales revenue or live restaurant usage—alongside elapsed time, cost and owner interventions.
10. Delete or simplify any mechanism that did not materially help the outcome.

---

# 16. Engineering operating model

Architecture drift is primarily a product and team-feedback problem, not a missing-enforcement problem. Helm should preserve its priorities through the way the team discovers, scopes, builds, reviews and dogfoods work.

This operating model must not become another control plane. There is no central architecture veto over routine work. Alignment comes from engineers repeatedly seeing the whole system run, discussing concrete trade-offs and correcting course together.

## 16.1 Build from observed work, not speculative ontology

For each capability, use this sequence:

1. Define a real company outcome and how acceptance will be judged.
2. Attempt it with ACP agents, normal Linux tools and the smallest existing substrate.
3. Observe where the organisation actually stalls, duplicates work, loses context or requires owner intervention.
4. Add the smallest OrgIntel helper that addresses that recurring failure.
5. Add kernel machinery only where the scenario crosses a real authority, secret, budget or irreversible-effect boundary.
6. Run the same outcome again and measure whether the change improved useful output.
7. Simplify or delete the mechanism if it did not materially help.

The team should not model a general lifecycle before seeing the concrete work that requires it. OrgIntel should be grown from repeated organisational failure modes, not imagined completeness.

## 16.2 Work in end-to-end vertical slices

Early work should be owned end to end rather than divided into separate kernel, OrgIntel and runtime teams. One small team should be able to change all three layers for a real scenario and see the total effect.

A slice is complete only when it produces a useful artifact, decision or external outcome. A database schema, API, invariant suite or orchestration path is not independently a successful slice.

Every slice should contain only the kernel control, OrgIntel support and runtime tooling needed by that outcome.

## 16.3 Clean-slate core, not a blind rewrite

The recommended migration is a new V2 execution path with no dependency on the existing universal command/domain kernel.

- Freeze the existing architecture except for critical fixes and evidence-gathering.
- Keep it available as a reference implementation and source of proven components.
- Build the new path in a separate workspace or repository so old types and assumptions do not become accidental dependencies.
- Reuse only isolated components with demonstrated value, such as model credential isolation, browser experiments, useful ACP integration and external-effect receipt logic.
- Do not pursue feature parity. Pursue one successful real-company outcome.
- Move product traffic or dogfood scenarios across incrementally once the new path is better.
- Delete old subsystems as their replacement proves itself.

This is a strangler migration guided by outcomes, not a multi-month rewrite intended to reproduce the current system.

## 16.4 Product and platform backlogs

Maintain two visible backlogs:

### Outcome backlog

Real company jobs that should produce economic value or materially reduce owner work.

Examples:

- Research a market and produce an accepted recommendation.
- Implement and review a product change.
- Prepare a customer campaign and send one approved external message.
- Detect a failed agent, reassign the work and preserve the useful output.

### Friction backlog

Concrete failures observed while attempting outcome work.

Examples:

- The Exec did not wake after a dependent result arrived.
- Two workers unknowingly duplicated the same research.
- A reviewer could not locate the relevant artifact.
- An agent crash lost uncommitted useful work.
- An external action had an ambiguous outcome.

Platform work should normally originate from the friction backlog. This keeps abstractions tied to actual use without requiring a top-down approval system.

## 16.5 Team cadence

### Continuous dogfood

The team should keep at least one real company running on the newest viable path. Productive runs are part of development, not a final validation phase.

### Regular outcome review

Review the latest company run together:

- What useful result was produced?
- Where did owner attention enter?
- Where did the agents stall or become confused?
- Which mechanism helped?
- Which mechanism added ceremony without changing the outcome?
- What can now be deleted or reduced?

The output of this review is the next outcome slice and a small set of observed friction items.

### Lightweight architecture conversation

When work introduces a durable entity, state machine, service, protocol or cross-layer dependency, the team pauses for a design conversation rather than invoking a formal gate.

The discussion should focus on:

- Which observed failure requires this?
- Could a file, process, Git operation or ordinary database record solve it?
- Is this authority, coordination or productive work?
- What is the smallest reversible experiment?
- How will a real run tell us whether it helped?

The aim is shared judgement, not permission seeking.

### Regular deletion and simplification

Reserve explicit engineering time to remove obsolete abstractions, adapters, experiments and duplicated concepts. Complexity rarely disappears as a side effect of feature work.

## 16.6 Default engineering heuristics

These are habits for design, implementation and review—not mechanically enforced gates.

### Observe before modelling

Let the behaviour occur through files, messages and processes first. Introduce a first-class entity, lifecycle or durable protocol only after repeated real scenarios reveal the same need.

### Recover before preventing

For OrgIntel and runtime failures, first ask whether the system can detect, repair, reassign, restart or restore cheaply. Reserve prevention-by-construction for failures with genuinely irreversible or high-impact consequences.

### Prefer conventions before mechanisms

Start with filesystem conventions, prompts, playbooks, Git practices, lightweight helpers and diagnostics. Promote a convention into mandatory machinery only when repeated failure demonstrates that the stronger mechanism improves outcomes.

### Preserve escape hatches

OrgIntel should provide the easiest path, not the only path. Agents must remain able to use ordinary files, shell commands, Git, scripts and project-local tools when the organisational abstraction does not fit the work.

### Prefer visible mess over hidden rigidity

A stale plan, duplicated task or untidy workspace is visible and repairable. A deeply coupled state machine that silently blocks valid work is often more damaging. Internal neatness is not worth sacrificing economic output.

### Treat deletion as product progress

After each substantial dogfood cycle, identify abstractions, adapters, tables, protocols and tests that no longer improve a live outcome. Removing them is part of completing the feature, not optional cleanup.

## 16.7 Pull-request and review norms

Code review should reinforce product judgement rather than enforce architectural doctrine. A meaningful change should make clear:

- the outcome or observed friction motivating it;
- why existing Linux/runtime primitives were insufficient;
- which layer the change belongs to and why;
- what remains deliberately flexible;
- how the change was exercised in a realistic run;
- what prior machinery it replaces or makes deletable.

Reviewers should prefer small, reversible changes and ask for simpler alternatives conversationally. A checklist must not become another compliance artifact.

## 16.8 Layer-appropriate testing

Different layers require different confidence styles:

- **Kernel:** focused invariants, adversarial tests, idempotency, failure injection and security review.
- **OrgIntel:** behavioural scenarios, recovery tests, simulations and comparison of organisational outcomes.
- **Linux runtime:** real tool use, integration tests and dogfood work producing concrete artifacts.

Do not demand kernel-grade proof from OrgIntel heuristics. Do not rely on dogfood alone for credential, authority or external-effect correctness.

## 16.9 Team shape during the rebuild

Initially, use one small cross-functional team responsible for the walking skeleton. Avoid separate layer teams until the contracts are proven; premature ownership boundaries create interface bureaucracy and local optimisation.

The team should share responsibility for:

- the real company outcome;
- observed agent behaviour;
- architectural simplicity;
- deletion of superseded code;
- safety at actual authority boundaries.

A rotating team member may explicitly argue the simplest workable alternative during design discussions, but this is a perspective, not a veto role.

## 16.10 Source-of-truth discipline

This document records current shared judgement. It should remain short enough to reread and challenge.

- Update settled architecture after evidence or a clear team decision.
- Record unresolved issues as open questions rather than prematurely resolving them in code.
- Prefer a few concise ADRs for consequential choices.
- Do not turn the document into an exhaustive executable specification.
- Archive superseded reasoning instead of preserving every historical rule in the active constitution.

The document should help the team think consistently, not substitute for thinking.

## 16.11 Signals that the team is tunnelling

These are reasons to stop the current line of abstraction work and return to a real company run:

- Internal types, states and invariants dominate discussion while the target company behaviour remains vague.
- Test count, schema coverage or architectural completeness is being treated as product progress.
- A new entity or transition exists mainly to repair complexity introduced by another internal mechanism.
- Agents must translate ordinary file, Git or process activity through multiple Helm-specific protocols.
- A safety or observability feature materially reduces useful output while its actual risk reduction is unclear.
- Platform tickets accumulate without a direct link to an observed outcome or friction item.
- The team has not recently watched the newest end-to-end path attempt real work.
- Engineers can explain the internal model precisely but cannot state which owner intervention or company failure the current work removes.

The recovery move is practical rather than bureaucratic:

1. Run the simplest representative outcome on the current path.
2. Identify the exact useful behaviour that is blocked, slow or unreliable.
3. Bypass, simplify or delete machinery until the company can attempt the work again.
4. Reintroduce only the minimum structure justified by repeated evidence.

## 16.12 Match decision speed to reversibility

Most OrgIntel and runtime choices should be made quickly, shipped in a vertical slice and revised after observation. Architectural certainty is not a prerequisite for letting the company attempt useful work.

Kernel decisions affecting authority, secrets, isolation or irreversible external effects deserve slower and more deliberate treatment. The team should not apply that same process to prompts, planning strategies, task conventions, filesystem layouts or internal coordination heuristics.

---

# 17. Recommended implementation sequence

## Step 1: Freeze and establish a baseline

- Stop adding new concepts to the current command/domain kernel.
- Select one representative company scenario.
- Run it on the existing system and record useful output, elapsed time, owner interventions, failure points and operational burden.
- Identify proven components worth salvaging.

## Step 2: Build the walking skeleton

Create the smallest new path containing:

- one durable Linux company environment;
- one persistent Exec ACP process;
- one or two worker ACP processes;
- files as the primary work substrate;
- Git for meaningful checkpoints;
- minimal OrgIntel messaging, commitments and wakeups;
- model access through the host;
- one narrow receipt-backed external effect;
- restart and snapshot recovery.

Avoid broad APIs and generalized schemas. Hard-code or configure the first scenario where doing so keeps the learning loop fast.

## Step 3: Dogfood the real outcome repeatedly

Run the same scenario until the company can produce an accepted result with materially less owner attention and fewer architecture-induced failures than the current system.

Improve prompts, filesystem conventions, process patterns and OrgIntel behaviour before introducing new durable machinery.

## Step 4: Generalise only repeated friction

After several runs, extract the recurring primitives that have demonstrated value. Likely candidates are actor sessions, messages, goals, commitments, schedules, artifact references and effect receipts.

Do not generalise one-off recovery choreography or represent every observed event as a permanent domain concept.

## Step 5: Add a second materially different outcome

Choose a scenario that stresses different behaviour—for example, software delivery after a research workflow. This reveals whether the first abstractions are genuinely reusable or merely overfit.

## Step 6: Strangle the old architecture

Move successful flows to the new path, preserve adapters only where necessary, and delete old command, custody, lease and workflow machinery as soon as it no longer serves live traffic or dogfood.

The migration is complete when the old control plane no longer determines how daily internal work happens—not when every historical feature has been recreated.

Multiplayer collaboration, managed hosting and shared multi-tenancy are not part of this sequence. Each should begin only when the trigger conditions in Section 7.4 are observed in real use.

---

## Engineering maxim

> **Let agents work. Observe where the organisation repeatedly fails. Encode only the minimum help that improves the next real outcome. Protect the few boundaries where failure creates irreversible harm.**

---

## Working summary

> **The Constitutional Kernel bounds authority. OrgIntel keeps the organisation coherent and proactive. The Linux Runtime gives agents the freedom and tools to produce real economic work.**

The kernel should be small and strict. OrgIntel should be opinionated but recoverable. The runtime should be powerful, messy and productive.

---
