# Company Runtime and Runtime Bridge Specification

**Version:** 0.1  
**Status:** Core design and initial implementation contract  
**Companion:** `OrgIntel Core Specification`

---

## 0. Document contract

This document defines Layer 3 of the system: the **Company Runtime**, plus the narrow **Runtime Bridge** connecting it to OrgIntel.

It is intentionally concrete enough to guide implementation, but it is not a complete operating-system design or a catalogue of every future runtime feature.

Statements are classified implicitly as:

- **Core contract:** preserve this boundary or behaviour.
- **Initial hypothesis:** build and test this first; change it when evidence disagrees.
- **Default convention:** make the good path easy, but allow deviation.
- **Explicit exclusion:** do not accidentally rebuild this.

The guiding principle is:

> Give the company a real, durable Linux computer; give each actor focused context, useful tools, and a bounded resource envelope; then let them perform ordinary work directly.

---

# 1. Product definition

## 1.1 Purpose

The Company Runtime is where the company creates and operates real economic output.

It hosts:

- Exec and worker ACP processes;
- files, repositories, worktrees, documents, and assets;
- browsers and desktop sessions;
- programming languages, package managers, and build tools;
- project-specific databases and services;
- experiments, scripts, models, and generated outputs;
- company-created skills and internal tools.

The runtime should feel like a capable company workstation or server, not a workflow appliance.

## 1.2 Core claim

A persistent Linux environment is a better primitive for open-ended company work than a bespoke domain model for every task, attempt, artifact, lease, or handoff.

Linux already supplies:

- files and directories;
- processes and process trees;
- networking and IPC;
- permissions and resource limits;
- package ecosystems;
- databases and services;
- mature development and business tools;
- ordinary recovery mechanisms.

Restless should add organisational intelligence and bounded external authority around these mechanisms rather than replace them.

## 1.3 Boundaries

The Runtime owns what the company **creates and operates**.

It does not own:

- owner mandate or root authority;
- capabilities, budgets, approvals, or authoritative effect receipts;
- durable actor identity, company commitments, schedules, or organisational learning;
- a universal model of all company work.

The Runtime may be messy, stale, partially broken, or internally inconsistent. Useful work should remain observable and recoverable rather than invalidated by coordination errors.

---

# 2. Deployment topology and trust model

## 2.1 Initial deployment

The first dogfoods should run as one per-company Docker Compose deployment on a local or dedicated machine.

```text
Per-company deployment
├── Owner cockpit
├── Authority Plane services
├── OrgIntel service
│   └── OrgIntel store
├── Postgres / imported infrastructure
├── Infisical Agent Proxy or adapter
└── Company Runtime
    ├── Runtime Bridge
    ├── Exec and worker ACP processes
    ├── persistent company storage
    ├── browser / desktop
    └── project services
```

Docker Compose is only a packaging and lifecycle mechanism. It must not model the company’s work or agent topology.

Do not create one Compose service or container per employee by default.

## 2.2 OrgIntel lives outside the work sandbox

The stable OrgIntel service and store live outside the writable Company Runtime so that the organisation can survive:

- runtime corruption;
- runtime replacement;
- restoration of an old runtime snapshot;
- broken packages or services;
- runaway or confused agents;
- loss of the entire working container.

The intelligent OrgIntel actors—Exec, planner, critic, recovery actor—still run as ACP processes inside the Runtime because they need direct access to the company’s real work.

```text
OrgIntel service outside
- actors
- goals and commitments
- inboxes and schedules
- hypotheses, decisions, learning
- session state and context inputs

Company Runtime inside
- Exec cognition
- worker cognition
- files, Git, browser, builds
- actual productive execution
```

## 2.3 Trust model

The Company Runtime is one company-level trust domain in V0.

Persistent `actor_id` values provide organisational identity and attribution, not strong hostile-process isolation. Actors may share a Linux user and filesystem initially.

The hard boundary is between the Company Runtime and the Authority Plane, not between every pair of internal workers.

Consequences:

- Internal actors may accidentally interfere with one another.
- OrgIntel records are useful coordination truth, not constitutional security truth.
- External authority is never granted merely because a process claims to be an actor.
- Strong per-actor Unix, container, or credential isolation is added only when real workloads require it.

## 2.4 Sources of truth

| State | Authoritative source |
|---|---|
| Mandate, capabilities, budgets, approvals, external effects, receipts, lifecycle | **Authority Plane store** |
| Actors, goals, commitments, messages, schedules, decisions, hypotheses, experiments, organisational learning, session state, artifact references | **OrgIntel service and store** |
| Code, documents, assets, builds, browser state, project databases, installed tools, working files, active experiments | **Company Runtime filesystem, Git, and project applications** |
| Actual email, payment, deployment, CRM, cloud, or other provider state | **External provider**, referenced by Authority Plane receipts and OrgIntel records |

OrgIntel owns the commitment. The Runtime owns the result. The Authority Plane owns the right to cause consequential external effects and the receipt that they occurred.

There are no cross-layer database foreign keys. Stable references and reconciliation are sufficient.

---

# 3. Runtime shape

## 3.1 One shared company computer

**Initial hypothesis:** use one shared runtime per company rather than one disposable sandbox per actor or attempt.

Benefits:

- shared files and applications are immediately visible;
- browser sessions and local services persist;
- workers can build on one another’s outputs;
- no artifact export/import or custody choreography;
- environment knowledge and installed tooling accumulate;
- recovery is whole-machine recovery rather than per-attempt reconstruction.

Parallel work is isolated through worktrees, working directories, process scopes, and project conventions—not separate company universes.

## 3.2 Mutable workstation semantics

The Runtime should behave like a long-lived, evolvable workstation:

- agents may install user-space tools;
- create scripts and internal applications;
- run local services;
- maintain browser profiles;
- create project databases;
- change their development environment;
- build reusable company skills.

For Docker V0, persist at least:

```text
/company
/home/company
/var/lib/company
/browser-profile
/project-service-data
```

The container root filesystem is not the most durable source of truth. Important work, tools, configuration, and service data should live on persistent volumes or be recoverable from files and repositories.

A full mutable system container or VM may replace Docker later if persistent system packages, desktop behaviour, or whole-machine snapshots become recurring friction.

## 3.3 Base image

The base image should be broad enough for productive work, but not attempt to predict every company domain.

Likely defaults:

- Debian or Ubuntu base;
- Git and common CLI utilities;
- shell, Python, Node, Rust tooling, and package managers;
- browser automation dependencies;
- ACP-compatible harnesses;
- a small imported process supervisor;
- observability and bridge dependencies;
- user-space environment managers where useful.

The image is versioned. Company data and work remain separate from the image.

## 3.4 Runtime lifecycle

The surrounding infrastructure needs only a small lifecycle contract:

```text
create
start
stop
health
attach
snapshot
restore
destroy
```

This is infrastructure lifecycle, not company workflow.

---

# 4. Runtime Bridge

## 4.1 Purpose

The Runtime Bridge is transport and process plumbing between OrgIntel and the Company Runtime.

It exists because OrgIntel must launch and observe actors without proxying their actual work.

```text
OrgIntel service
      ↕ authenticated long-lived channel
Runtime Bridge
      ↕ ACP over stdio
Exec / worker process
```

## 4.2 Responsibilities

The Runtime Bridge may:

- register the runtime instance with OrgIntel;
- maintain an outbound authenticated connection;
- receive actor wake and session-launch requests;
- create or select a working directory or worktree;
- materialise the session context packet;
- expose applicable instructions, skills, and tools;
- launch the ACP process and communicate over stdio;
- associate process trees with `actor_id`, `session_id`, and commitment references;
- stream meaningful session events;
- report health, exit status, and resource use;
- terminate or replace a session;
- proxy local OrgIntel tools for the session;
- buffer a small number of important events during brief OrgIntel outages.

## 4.3 Non-responsibilities

The Runtime Bridge must not:

- choose company strategy;
- decompose goals;
- allocate work;
- decide whether an external effect is authorised;
- mediate every shell command, file write, browser action, or Git operation;
- become the service supervisor for all company applications;
- own actor identity or long-term memory;
- become a second workflow engine.

## 4.4 Communication direction

The Bridge should establish an outbound connection to OrgIntel rather than expose a broad inbound management API to the runtime network.

The exact transport may begin as authenticated WebSocket, streaming HTTP, or gRPC. The important contract is:

- reconnectable;
- session-addressable;
- supports commands and events;
- resilient to temporary disconnection;
- does not require distributed exactly-once semantics.

## 4.5 Agent access to OrgIntel

Agents receive a small local MCP, CLI, or Unix-socket interface backed by the Bridge.

Initial operations may include:

```text
orgintel.read_inbox
orgintel.send_message
orgintel.accept_commitment
orgintel.update_commitment
orgintel.report_blocker
orgintel.submit_result
orgintel.request_review
orgintel.record_decision
orgintel.record_hypothesis
orgintel.schedule_followup
```

The Bridge attaches the current session identity and forwards the request to OrgIntel.

This interface records meaningful coordination changes. It is not required for ordinary productive work.

## 4.6 Bridge failure posture

If the Bridge fails:

- already-running agents may continue local filesystem and process work;
- OrgIntel launches and messaging pause;
- the Authority Plane remains independent;
- useful files and processes remain valid;
- the Bridge restarts and reconciles current processes and sessions.

The Bridge is replaceable. It contains no unique durable company state.

---

# 5. Actors, sessions, and processes

## 5.1 Identity distinctions

Keep these separate:

```text
Actor identity
- durable employee or organisational role
- owned by OrgIntel
- persists for months or years

Session
- temporary ACP/model process
- attached to an actor and current work
- may be restarted or replaced

Authority principal
- security identity used by the Authority Plane
- may initially be company or Exec scoped
- does not define organisational personality
```

A durable Game Director may use many Claude, Codex, or future-model sessions without losing role continuity, style, or organisational history.

## 5.2 Session process tree

The Bridge tracks the process tree created by an actor session:

```text
ACP actor process
├── compilers and tests
├── scripts and data jobs
├── temporary development server
└── helper processes
```

Resource use is attributed to the session where practical.

Raw command history may be retained temporarily for diagnostics, but it is not the company’s organisational memory.

## 5.3 Three process classes

### Session processes

Short-lived work tied to an actor wakeup:

- model session;
- builds and tests;
- research scripts;
- temporary servers;
- one-off data jobs.

### Durable company services

Processes intentionally expected to outlive the actor that created them:

- Cosmon development server;
- Aris analytics service;
- Thymelake staging backend;
- company-created dashboards or automations.

Use an imported supervisor. Each durable service should have a lightweight runbook covering purpose, owner, start, stop, health, data location, and recovery.

### Local subagents

A helper spawned within a session is not automatically a durable organisational actor.

OrgIntel promotes it to an actor only when persistent identity, responsibility, inbox, or long-term learning is useful.

---

# 6. Work modes

Work modes span OrgIntel and the Runtime.

> OrgIntel chooses and adapts the organisational shape. The Runtime provides the concrete mechanisms to perform the work.

## 6.1 Flexible execution

This is the default for ambiguous work.

OrgIntel supplies:

- desired outcome;
- accountable actor;
- relevant constraints;
- definition of done;
- current priority and escalation path.

The Runtime lets agents decide:

- technical approach;
- tools and packages;
- file and repository structure;
- experiments;
- local division of implementation work;
- scripts, services, and applications to create.

## 6.2 Exploratory work

OrgIntel may branch a question into bounded hypotheses or approaches.

The Runtime supports this through:

- separate Git branches or worktrees;
- experiment directories;
- distinct process trees;
- isolated project databases or datasets;
- benchmarks and test harnesses;
- concrete artifacts for comparison.

A typical shape is:

```text
Question
├── Hypothesis / approach A
├── Hypothesis / approach B
└── Hypothesis / approach C
```

OrgIntel owns the question, evidence expectations, resource allocation, and expand/kill decision. The Runtime owns the actual experiments and outputs.

## 6.3 Repetitive workflows

Repeatable work should normally emerge from successful flexible execution:

```text
ad hoc work
→ repeated useful pattern
→ documented playbook
→ reusable SKILL.md
→ deterministic script or tool
→ scheduled service where justified
```

OrgIntel owns the business process, accountability, scheduling, exceptions, and learning.

The Runtime owns the scripts, skills, files, services, and mechanical execution.

Do not prematurely encode a poorly understood process as a rigid workflow engine.

---

# 7. Filesystem and workspace conventions

## 7.1 Company structure

A useful starting convention is:

```text
/company/
├── README.md
├── handbook/
├── projects/
├── shared/
├── skills/
├── services/
├── outputs/
├── worktrees/
└── scratch/
```

Meaning:

- `handbook/`: company principles, style, technical and operational guidance;
- `projects/`: real initiatives and products;
- `shared/`: common references, datasets, and assets;
- `skills/`: company-created reusable know-how;
- `services/`: service definitions and runbooks;
- `outputs/`: accepted or delivered outputs;
- `worktrees/`: isolated Git work areas;
- `scratch/`: disposable exploration.

This is a scaffold, not a schema. Companies may evolve it.

## 7.2 Project structure

A project may begin with:

```text
/projects/<project>/
├── README.md
├── STATUS.md
├── decisions/
├── experiments/
├── outputs/
└── repo/
```

`README.md` explains purpose and how to orient. `STATUS.md` is a human- and agent-readable current summary, not an authoritative replacement for OrgIntel commitments.

## 7.3 Session materialisation

The Bridge may create:

```text
/run/helm/session/
├── context.md
├── manifest.json
├── skills.json
├── resources.json
└── endpoints.json
```

This directory is a generated projection for the current session, not durable company truth.

## 7.4 Guidance model

Use four levels:

1. **Convention:** recommended file and project practices.
2. **Scaffold:** create useful starting files and directories.
3. **Helper:** automate worktree creation, checkpoints, service registration, or output publishing.
4. **Warning:** identify missing runbooks, unreferenced outputs, or risky destructive actions.

Missing a template must not remove permission to work.

---

# 8. Git and artifact handling

## 8.1 Git’s role

Git is for meaningful checkpoints, review, integration, attribution, and rollback.

It is not the realtime state bus of the company.

Live work may exist as uncommitted files, project databases, running services, browser state, messages, and OrgIntel records.

## 8.2 Defaults

- Coding actors use separate branches or worktrees when concurrent edits may conflict.
- Commit at meaningful milestones, not after every tool call.
- Review and handoff should reference a commit when the artifact is suitable for versioning.
- Shared integration branches are not force-pushed.
- A named lead or integrator owns material merges.
- Snapshot or checkpoint before destructive repository operations.
- Secrets, browser profiles, caches, and large transient state stay out of Git.
- Non-code work uses Git only where deliberate version history helps.

## 8.3 Artifact references

OrgIntel generally references work through:

- path;
- repository and commit;
- branch or worktree;
- build URL;
- project-owned database record;
- external provider identifier;
- Authority Plane receipt.

Do not export and re-import every artifact through a custody protocol.

## 8.4 Large assets

Large game, media, or dataset assets may use:

- Git LFS;
- object storage;
- a domain-specific asset system;
- project-local databases.

This is a runtime/project decision. OrgIntel needs stable references, not ownership of the binary data.

---

# 9. Context, instructions, skills, and tools

## 9.1 Separate the concepts

| Concept | Purpose | Primary owner |
|---|---|---|
| Actor profile | Who the employee is over time | OrgIntel |
| Session context | What matters now | Generated by OrgIntel/Bridge |
| Project instructions | How work in this repository or project is normally done | Runtime files and Git |
| Skill | Reusable know-how and procedure | Runtime files and Git |
| Tool | Executable mechanism | Runtime or external service |
| Capability | Authority to cause an external consequence | Authority Plane |

## 9.2 Actor profile

OrgIntel maintains durable identity material such as:

- role and responsibility;
- decision rights;
- working style and design taste;
- competence evidence;
- accepted examples and important decisions;
- relationships and current commitments.

A session receives only the relevant subset.

## 9.3 Session context

A focused context packet may include:

- owner mandate and hard constraints;
- current milestone and definition of done;
- actor responsibility and decision rights;
- team map and dependencies;
- recent messages and decisions;
- observations, hypotheses, assumptions, and unknowns;
- relevant paths, commits, services, and external records;
- expected handoff.

Do not inject the whole company transcript by default.

## 9.4 `AGENTS.md` and native equivalents

Use `AGENTS.md` for stable repository- or directory-level working guidance:

- how to build and test;
- project architecture;
- coding and design conventions;
- important locations;
- review expectations;
- project-specific cautions.

Keep it concise and versioned with the project.

For harnesses with different native instruction formats, the Bridge may inject or generate the relevant equivalent while preserving the project’s canonical guidance.

Do not use instruction files for secrets, current task state, permission enforcement, or large historical dumps.

## 9.5 Skills

A skill is a directory containing a `SKILL.md` plus optional scripts, references, templates, and assets.

Suggested locations:

```text
/opt/restless/skills/                 # Restless defaults, read-only
/company/skills/                  # company-created skills
/projects/<project>/.agents/skills/  # project-specific skills
```

The Bridge exposes a catalogue of applicable skills and loads full skill content only when relevant.

Skills may be:

- assigned by role;
- recommended for a commitment;
- discovered by the actor;
- created or improved by the company;
- retired when evidence shows they are ineffective.

Skills are know-how, not authority.

## 9.6 Tool classes

### Native Linux tools

Shell, files, Git, language toolchains, package managers, browsers, and local databases.

### Company tools

Company-created commands and applications available through files, `PATH`, or local services.

### Structured service tools

MCP or APIs for OrgIntel, Authority Plane requests, CRM, analytics, issue trackers, and other services.

### Skill scripts

Deterministic scripts packaged with reusable procedures.

The Bridge configures which structured tools a session sees. The underlying service still enforces access.

---

# 10. Resource model

## 10.1 Two levels of allocation

### Hard company envelope

Enforced by runtime and infrastructure:

- maximum CPU;
- maximum RAM;
- process count;
- disk allowance;
- optional fixed GPU devices;
- network and runtime limits.

### Soft OrgIntel allocation

OrgIntel decides:

- actor concurrency;
- work priority;
- model budget;
- approximate resource class;
- whether costly work should continue;
- whether a scarce resource should move to higher-value work.

Start with simple resource classes such as `small`, `medium`, `large`, and `gpu`.

## 10.2 Local fixed GPU

For V0, the simplest GPU mode is fixed allocation at deployment time.

Docker Compose or the runtime provider exposes an approved GPU to the Company Runtime. OrgIntel manages concurrency and priority within the company’s standing resource envelope.

No Authority Plane decision is needed for every kernel invocation or build.

## 10.3 Dynamic resource provisioning

For cloud GPUs, temporary databases, storage, or other scarce resources:

```text
Exec / actor requests resource
        ↓
Authority Plane checks grant, budget, and scope
        ↓
Resource controller provisions the resource
        ↓
Runtime receives a bounded resource grant
        ↓
Agent or service uses the resource directly
```

The Runtime consumes a resource-grant contract conceptually containing:

```text
resource_id
kind
endpoint or device information
short-lived or proxied access
limits
expiry
status
usage reference
```

The Bridge materialises the grant into the relevant session or company environment through environment variables, mounted files, endpoints, or project configuration.

The Runtime never receives provider-root cloud credentials.

## 10.4 GPU worker pattern

If a GPU cannot be attached dynamically to the long-lived Runtime, the Authority Plane may create a separate bounded worker:

```text
Company Runtime
      ↕ shared project storage or object storage
GPU worker
      ├── approved image
      ├── bounded duration and resources
      └── temporary job identity
```

The Runtime connects directly to the worker or submits a job. Outputs return to company-owned storage.

## 10.5 Resource pressure

When resources are exhausted:

1. surface the pressure to the actor or Exec;
2. reduce concurrency;
3. stop the local runaway process where appropriate;
4. preserve useful outputs;
5. request more resources only when economically justified.

Do not require permission for every ordinary command.

---

# 11. Networking, credentials, and external authority

## 11.1 Default networking

Initial posture:

- outbound internet generally allowed;
- no public inbound ports by default;
- private authenticated access to OrgIntel, model gateway, Authority Plane, and credential proxy;
- project services are published explicitly through a controlled mechanism;
- no Docker socket, host filesystem, instance-metadata credential, or host administrative API;
- no semantic inspection of every HTTP request.

## 11.2 Direct resources versus brokered effects

Use this distinction:

| Access | Default path |
|---|---|
| Public browsing and package download | Direct |
| Productive bounded resource: GPU, database, storage, development service | Direct after bounded grant |
| Low-risk authenticated API | Direct through scoped credential or Infisical Agent Proxy |
| Consequential discrete effect: refund, payout, mass email, production deletion | Authority Plane effect request and provider adapter |
| Deployed product’s routine service access | Dedicated restricted service identity |

The principle is:

> Grant direct access to resources that enable work; broker actions that create significant external consequences.

## 11.3 Infisical

Infisical may provide:

- secret storage for trusted Authority Plane services;
- machine identities;
- credential rotation;
- an Agent Proxy that applies credentials without exposing them to the agent process.

The Runtime may connect through the Agent Proxy for configured services such as GitHub, analytics, or low-risk APIs.

Infisical determines how credentials are safely applied. The Authority Plane determines whether the company is permitted to use them for a purpose.

The Runtime must not receive broad access to the Infisical secret-reading API.

## 11.4 Stripe example

- Stripe test mode may be used directly through CLI or SDK with test credentials.
- Production refunds, payouts, account administration, and similar actions should normally be brokered by the Authority Plane.
- A deployed product may receive its own restricted production service identity for routine checkout or webhook operations.
- Interactive ACP agents do not receive a broad production Stripe secret by default.

## 11.5 Model access

Model-provider credentials remain outside the Runtime.

Agents call a model gateway or are launched through an ACP harness configured against that gateway. Model traffic does not need to pass through OrgIntel, but usage should be attributable to company, actor, and session where practical.

---

# 12. Browser, desktop, and company services

## 12.1 Browser

Begin with one persistent company browser profile and one attachable desktop session.

- Cookies, history, downloads, and ordinary sessions survive restart.
- The owner can attach to the same environment.
- Only one actor controls the shared profile at a time initially.
- Parallel profiles are added only when real contention appears.

A logged-in browser is itself authority. High-impact accounts should use restricted identities, brokered effects, or human takeover.

## 12.2 Durable project services

Agents may create services that outlive their current model process.

Use an imported service supervisor and simple runbooks rather than a custom durable workflow system.

OrgIntel may track service ownership and health references, but the Runtime owns the service process and data.

## 12.3 Publication

A local service may be developed freely. Publishing it externally crosses the Authority Plane when it creates meaningful external exposure, spend, or production consequence.

---

# 13. Recovery, snapshots, and upgrades

## 13.1 Snapshot contents

Runtime snapshots may contain:

- files and repositories;
- browser state;
- installed user-space tools;
- project databases and service data;
- current working state.

They do not roll back:

- OrgIntel actor identity, commitments, schedules, or organisational learning;
- Authority Plane capabilities, budgets, external effects, or receipts;
- the external world.

## 13.2 Restore reconciliation

After restoring an older Runtime:

1. the Bridge registers the restored runtime generation;
2. OrgIntel retains current goals, messages, and responsibilities;
3. the Authority Plane retains current external-effect history;
4. current files, commits, services, and processes are inspected;
5. stale artifact references and lost uncommitted work are surfaced;
6. pending external effects are reconciled before repetition;
7. the Exec receives a recovery context and decides how to continue.

Temporary inconsistency is acceptable. Blind repetition of consequential effects is not.

## 13.3 Failure posture

- **Agent crash:** preserve files; restart, reassign, or change strategy.
- **Bridge crash:** local work may continue; restart and reconcile.
- **Runtime crash:** restart against persistent storage.
- **Runtime corruption:** restore or replace; OrgIntel continuity remains.
- **OrgIntel outage:** running local work continues; new launches and coordination pause.
- **Authority Plane outage:** internal work continues; consequential effects and new privileged resources pause.
- **Broken service:** use its runbook and preserve data before replacement.

## 13.4 Upgrades

- Version the base runtime image.
- Snapshot before material upgrades.
- Separate image upgrades from persistent company data.
- Canary important runtime changes on one company or clone.
- Keep a known-good image and rollback path.
- Do not require the broken new runtime to perform its own recovery.

---

# 14. Observability and economic use

The Runtime should expose enough telemetry to understand productivity and failure without making every action a governance event.

Useful signals include:

- active sessions and process trees;
- CPU, memory, disk, GPU, and model usage;
- process exits and repeated failures;
- important build or test results;
- changed Git state;
- registered services and health;
- submitted artifacts;
- runtime generation and snapshot state.

Raw logs remain operational diagnostics. OrgIntel records meaningful organisational events and conclusions.

The goal is not perfect surveillance. It is to answer:

> Is this work progressing, blocked, wasteful, or producing useful outputs?

---

# 15. V0 implementation contract

## 15.1 Components

```text
Docker Compose
├── owner-ui
├── authority-service
├── orgintel-service
├── postgres
├── infisical-agent-proxy or adapter
└── company-runtime
    ├── runtime-bridge
    ├── ACP harnesses
    ├── browser / desktop
    ├── imported process supervisor
    └── persistent volumes
```

## 15.2 V0 runtime features

- One persistent company Runtime.
- One persistent Exec actor identity in OrgIntel.
- Replaceable Exec and worker ACP sessions.
- Bridge-to-OrgIntel authenticated connection.
- ACP over local stdio.
- Persistent `/company`, home, browser, and service data.
- Files and Git as primary work primitives.
- Separate worktrees for concurrent coding where useful.
- Focused session context packets.
- Project `AGENTS.md` guidance.
- Restless, company, and project skills.
- Native Linux and selected MCP tools.
- Hard company CPU/RAM/disk limits.
- Optional fixed local GPU.
- Model access through gateway.
- One or two Authority Plane effect/resource requests.
- Snapshot, restart, and restore reconciliation.

## 15.3 Implementation sequence

1. Build the Runtime image and persistent volume layout.
2. Implement the Bridge connection and runtime registration.
3. Launch one ACP Exec session over stdio.
4. Materialise context, project instructions, and tools.
5. Let the Exec create files and use Git directly.
6. Launch one worker and complete an artifact-centred handoff.
7. Add process/resource attribution and failure recovery.
8. Add browser persistence and owner attachment.
9. Add one brokered external effect and one resource grant.
10. Run the Cosmon browser-game vertical slice.

---

# 16. Acceptance scenarios

## 16.1 Productive restart

- Exec and worker produce useful files and commits.
- Runtime is restarted.
- OrgIntel preserves actors, commitments, and messages.
- Runtime work persists.
- Exec resumes with focused recovery context.

## 16.2 Runtime rollback after external action

- Aris sends an email through the Authority Plane.
- Runtime is restored to a snapshot from before the send.
- Authority Plane receipt and OrgIntel follow-up remain current.
- The email is not blindly repeated.

## 16.3 Flexible and exploratory work

- Cosmon assigns one outcome.
- Workers create two bounded implementation approaches in separate worktrees.
- Builds and evidence are compared.
- Exec selects one and preserves useful work from the other.

## 16.4 Repetitive workflow promotion

- Thymelake performs restaurant onboarding manually.
- A repeated step becomes a `SKILL.md` and script.
- Future onboarding uses the helper but can deviate when needed.

## 16.5 Dynamic resource

- An actor requests temporary GPU compute.
- Authority Plane grants a bounded resource.
- The Runtime receives an endpoint or worker handle without cloud-root credentials.
- Outputs return to company storage.
- The resource expires cleanly.

## 16.6 OrgIntel outage

- OrgIntel becomes temporarily unavailable.
- A running coding session continues local work.
- The Bridge reconnects and submits the meaningful result.
- No productive artifact becomes invalid.

---

# 17. Explicit exclusions

Do not build initially:

- one sandbox per actor or attempt;
- a custom filesystem or source-control system;
- universal artifact custody and materialisation;
- a custom service supervisor;
- a custom container runtime;
- a syscall-level activity ledger;
- a remote RPC abstraction for every shell, Git, browser, or file action;
- mandatory workflow state for all work;
- a semantic firewall that classifies every HTTP request;
- per-agent hostile-process isolation;
- full system-package reproducibility before work can proceed;
- multiplayer collaboration infrastructure;
- shared multi-tenant hosting machinery.

---

# 18. Engineering rules

1. Productive work must remain possible without an OrgIntel API call for every action.
2. Runtime mess is repaired with files, Git, process tools, and snapshots before new domain machinery is introduced.
3. A new runtime abstraction must solve repeated observed friction.
4. Prefer imported Linux mechanisms over Restless-owned equivalents.
5. Preserve useful work even when coordination records are stale.
6. Keep identity continuity outside the Runtime; keep actual work inside it.
7. Direct access is preferred for bounded productive resources; consequential effects are brokered.
8. Runtime conventions guide and scaffold; they do not grant permission to work.
9. Build the smallest vertical slice that produces a real artifact.
10. Delete runtime machinery that does not improve accepted output or recovery.

---

# 19. Current open questions

These should be answered through implementation and dogfood rather than speculative architecture:

1. Is Docker’s mutable-workstation behaviour sufficient, or does the Runtime need an Incus/system-container or VM model?
2. How much actor-level Unix separation is useful before it harms collaboration and tooling?
3. Which process supervisor works best for company-created durable services?
4. How much filesystem and Git state should the Bridge report automatically versus only through explicit handoffs?
5. Is one shared browser profile sufficient for the first companies?
6. Which user-space package strategy best balances freedom and recoverability?
7. When should GPU work run inside the main Runtime versus a temporary worker?
8. Which external APIs are safe and productive through Infisical Agent Proxy, and which must remain effect-brokered?
9. Which runtime conventions are consistently followed because they help, rather than because they are enforced?

---

# Working summary

The Company Runtime is one permissive, persistent Linux company computer.

OrgIntel lives outside it to preserve organisational continuity, while Exec and worker cognition run inside it with direct access to real work.

The Runtime Bridge is a small, replaceable process launcher and transport adapter. It does not plan, govern, or proxy ordinary work.

Files, Git, processes, browsers, project services, and domain applications remain the productive primitives. Context, instructions, skills, and tools guide actors without becoming permissions. CPU, RAM, disk, and fixed resources are bounded at the runtime level; scarce dynamic resources are approved by the Authority Plane and materialised as bounded direct access.

The success criterion is simple:

> Can durable actors repeatedly create, continue, inspect, and recover real company work inside this environment without the harness becoming the work?
