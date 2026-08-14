# Company Lifecycle and Cross-Layer Contract Specification

**Version:** 0.1  
**Status:** Working implementation contract  
**Scope:** Company bootstrap, lifecycle, shared semantics, source ownership, and interfaces between the Authority Plane, OrgIntel, Company Runtime, Runtime Bridge, and Owner Cockpit.

---

## 0. Document contract

This document defines how Restless's existing component specifications compose into one running company.

It is intentionally narrower than the component specs:

- **Authority Plane Specification** owns authority, effects, resources, credentials, and runtime lifecycle implementation.
- **OrgIntel Core Specification** owns organisational cognition, actors, goals, commitments, communication, adaptation, and organisational memory.
- **Company Runtime and Runtime Bridge Specification** owns the Linux work environment, agent processes, files, Git, tools, and bridge implementation.
- **Owner Cockpit Product Specification** owns the operator-facing product experience.
- This document owns **shared identifiers, cross-layer contracts, company bootstrap, lifecycle transitions, and reconciliation**.

The contract should remain small enough that all layers can implement it consistently.

### Classification

- **Core contract:** implementations must preserve it.
- **Recommended default:** preferred V0 behaviour, changeable with evidence.
- **Example:** illustrative, not mandatory API shape.

---

# 1. System model

## 1.1 One company, three planes

```text
Owner Cockpit
      │
      ├──────────────┬──────────────────┐
      ▼              ▼                  ▼
Authority Plane    OrgIntel         Company Runtime
- mandate          - actors         - Linux workspace
- authority        - goals          - Exec/workers
- budgets          - commitments    - files and Git
- effects          - messages       - browser/tools
- resources        - adaptation     - actual artifacts
- lifecycle             │                  ▲
                        └── Runtime Bridge ─┘
```

The layers cooperate, but they do not share ownership of the same facts.

## 1.2 The company is durable; processes are replaceable

A Restless company persists across:

- model-session restarts;
- agent crashes;
- Runtime Bridge reconnects;
- Company Runtime restarts or replacement;
- runtime snapshot restoration;
- OrgIntel process restarts;
- provider outages;
- software upgrades.

Durability comes from explicit ownership and reconciliation, not from one global transaction or event ledger.

## 1.3 No universal command model

The system exposes several small semantic interfaces:

- organisational coordination;
- agent-session management;
- effects;
- resources;
- runtime lifecycle;
- owner directives and approvals.

Do not force all cross-layer actions into one universal `Command`, mutation protocol, or state machine.

---

# 2. Shared identity model

## 2.1 Stable identifiers

The following identifiers are shared across layers:

| Identifier | Meaning | Stability |
|---|---|---|
| `company_id` | Durable company identity | Never reused |
| `actor_id` | Durable human, agent, or service identity inside a company | Persists across models and sessions |
| `session_id` | One temporary model/process execution | Ends with the process/session |
| `principal_id` | Authenticated security principal used at an authority boundary | May differ from `actor_id` |
| `runtime_id` | A provisioned Company Runtime instance | Replaced when the work machine is recreated |
| `runtime_generation` | Monotonic generation of the runtime's material state | Increments on replacement or restore |
| `operation_id` | Idempotency/correlation identity for a lifecycle or control operation | Stable across retries |
| `effect_intent_id` | Identity of one consequential effect intent | Stable across retries and reconciliation |
| `resource_grant_id` | Identity of one bounded productive-resource grant | Stable for the grant lifetime |
| `artifact_ref_id` | OrgIntel identity for a reference to work owned elsewhere | Stable while the reference remains useful |
| `attention_item_id` | Owner-facing projection of something requiring review | Stable until resolved/withdrawn |

Identifiers are opaque. Do not encode mutable state, role names, or deployment location into them.

## 2.2 Actor identity is not process identity

```text
actor_id: durable employee
session_id: temporary cognitive/process run
principal_id: authority identity
```

A persistent Game Director may keep the same `actor_id` for years while using different models, sessions, worktrees, and runtime generations.

## 2.3 V0 authority principal

For V0, the hard security boundary is the **company/Exec authority envelope**.

- `actor_id` provides attribution and organisational responsibility.
- Only the Exec principal calls consequential Authority Plane APIs by default.
- Workers ask the Exec through OrgIntel when external authority is required.
- Per-worker security isolation and independently enforced capability grants are deferred.

Do not claim strong per-agent security while agents share one permissive Linux work environment.

---

# 3. Shared vocabulary and ownership

## 3.1 Stable concepts

| Concept | Meaning | Authoritative owner |
|---|---|---|
| Company identity | Durable organisation and infrastructure lifecycle | Authority Plane |
| Mandate | Owner-defined purpose and outer constraints | Authority Plane |
| Operating phase | Exploration, validation/pre-profit, profit, or scale | OrgIntel |
| Actor and role | Persistent organisational identity and responsibility | OrgIntel |
| Session | Temporary model/process execution | OrgIntel records intent/status; Runtime owns process reality |
| Goal | Desired outcome at any abstraction level | OrgIntel |
| Commitment | One actor's responsibility to produce or decide something | OrgIntel |
| Message/directive | Organisational communication or durable owner instruction | OrgIntel; mandate changes also update Authority Plane |
| Observation/hypothesis/decision | Company belief and decision semantics | OrgIntel or referenced source |
| Artifact | Code, document, build, asset, dataset, or project output | Company Runtime or external application |
| Artifact reference | Pointer from organisational state to an artifact | OrgIntel |
| Capability/envelope | Bounded external authority | Authority Plane |
| Effect/receipt | Consequential external action and authoritative outcome | Authority Plane/provider |
| Resource grant | Bounded productive resource made available to the company | Authority Plane |
| External business record | Email delivery, payment, deployment, CRM state, etc. | External provider/application |

## 3.2 Source-of-truth split

```text
Authority Plane store
- company identity and infrastructure lifecycle
- owner mandate and authority envelope
- capabilities, budgets, approvals
- effect intents, outcomes, and receipts
- resource grants and authoritative usage
- runtime lifecycle operations and generations

OrgIntel store
- actors, roles, and persistent identity packages
- goals, commitments, messages, and schedules
- hypotheses, experiments, decisions, and learning
- operating phase and organisational health
- session intent/status and artifact/effect references
- owner attention items derived from organisational state

Company Runtime
- repositories, worktrees, documents, and assets
- browser profiles and local application state
- builds, outputs, installed tools, and project services
- active experiments, scratch work, and project databases

External systems
- delivery, payment, deployment, customer, and other world state
```

## 3.3 Ownership rules

1. A concept has one authoritative owner.
2. Other layers may retain identifiers, summaries, and derived projections.
3. A projection never becomes a second writer of the underlying truth.
4. Cross-layer foreign keys are logical references, not shared database constraints.
5. The Owner Cockpit writes through the owning service, never directly to databases.
6. Runtime snapshots never roll back Authority Plane or OrgIntel history.
7. External provider state is reconciled rather than guessed.

## 3.4 Examples

### Commitment and artifact

OrgIntel owns:

```text
Commitment: Build Cosmon capture loop
Owner: gameplay-engineer
Status: completed
Result reference: git:cosmon@abc123
```

The Runtime owns the repository and commit contents. The Authority Plane knows nothing about the commitment.

### External email

OrgIntel owns the campaign goal, prospect ownership, and follow-up schedule.  
The Runtime owns copy and research files.  
The Authority Plane owns the send intent and receipt.  
The email provider owns delivery, bounce, and reply reality.

---

# 4. Cross-layer communication principles

## 4.1 Explicit interfaces, private databases

Each layer exposes an authenticated API or protocol. No component outside a layer writes its database directly.

V0 may use one Postgres server operationally, but databases/credentials remain separated and there are no cross-layer foreign keys.

## 4.2 References over replication

Pass stable references to:

- files and paths;
- repositories and commits;
- builds and URLs;
- effect receipts;
- resource grants;
- external records.

Do not copy artifact contents, provider payloads, or whole organisational histories across layers unless needed for a specific projection.

## 4.3 No distributed transactions

A company action may update several systems at different times. The design uses:

- durable intent;
- idempotency where consequence requires it;
- observable intermediate states;
- retry where safe;
- reconciliation where state may have diverged.

It does not attempt one atomic transaction across Authority Plane, OrgIntel, Runtime, and external providers.

## 4.4 Asynchronous by default

Long-running operations return an accepted identity and status rather than holding an RPC open indefinitely.

Examples:

- runtime provisioning;
- snapshot/restore;
- dynamic GPU creation;
- approval-dependent effects;
- long ACP sessions.

## 4.5 Idempotency is selective

Require stable operation identities for:

- company creation;
- runtime create/restore/destroy;
- consequential external effects;
- resource provisioning with external cost;
- owner approval decisions.

Do not require idempotency keys for every message, file edit, or internal planning update.

## 4.6 Versioned contracts

Every cross-layer interface declares a contract version.

- Prefer additive changes.
- A newer service must tolerate an older peer during a staged upgrade where practical.
- The Runtime Bridge advertises supported capabilities at registration.
- Unknown optional fields are ignored.
- Breaking changes require explicit migration and version negotiation.

## 4.7 Honest status

A layer never invents success because another component is unavailable.

Useful shared result categories include:

```text
accepted
succeeded
failed
awaiting_approval
unknown_outcome
unavailable
```

Component-specific states remain component-specific; do not collapse all lifecycle concepts into one generic enum.

---

# 5. Authentication and trust

## 5.1 Service identities

Each deployed component has a distinct machine identity:

- Owner Cockpit/backend;
- Authority Service;
- OrgIntel service;
- Runtime Bridge;
- Infisical Agent Proxy;
- provider adapters where separately deployed.

Use short-lived or rotatable service credentials. Infisical may store and materialise these, but Restless remains responsible for semantic authority.

## 5.2 Runtime Bridge connection

The Runtime Bridge initiates an outbound authenticated connection to OrgIntel.

At registration it supplies:

```text
company_id
runtime_id
runtime_generation
bridge_version
supported_features
runtime_health
```

OrgIntel verifies that the runtime belongs to the company and that the generation is current.

## 5.3 Session credentials

When launching an actor session, the bridge provides short-lived credentials scoped to:

- `company_id`;
- `actor_id`;
- `session_id`;
- permitted OrgIntel operations;
- permitted Authority Plane operations, if the session is the Exec.

Credentials expire with the session and are not stored in project files.

## 5.4 No raw privileged credentials in general sessions

Provider-root credentials, Docker/containerd authority, host filesystem access, and cloud control credentials remain outside the Company Runtime.

Ordinary authenticated APIs may be used through bounded credentials or Infisical Agent Proxy. Consequential effects remain brokered.

---

# 6. Company bootstrap

## 6.1 Bootstrap objective

Bootstrap is complete when:

- the company has a durable identity and owner mandate;
- an authority envelope exists;
- OrgIntel has a persistent Exec actor;
- the Company Runtime is healthy and connected;
- base company files and guidance exist;
- the Exec can use models and ordinary tools;
- the owner can see and direct the company;
- the Exec has produced its first evidence-backed operating plan.

## 6.2 Canonical bootstrap flow

```text
1. Owner creates company
2. Authority Plane allocates company_id
3. Owner defines mandate, constraints, budgets, and initial providers
4. OrgIntel initialises organisational state and persistent Exec actor
5. Runtime lifecycle manager provisions Company Runtime and persistent volume
6. Runtime Bridge connects and registers runtime generation
7. Runtime receives company scaffold, instructions, skills, and tool configuration
8. OrgIntel launches the Exec with a bootstrap context
9. Exec inspects the environment and proposes first goals, hypotheses, and team
10. Owner reviews only genuine mandate/authority exceptions
11. Company enters running state and begins work
```

## 6.3 Bootstrap inputs

Minimum owner inputs:

- company name;
- mission/mandate;
- material constraints and prohibited actions;
- initial operating phase;
- model/compute budget;
- external spend ceiling;
- connected providers or mock world;
- first desired outcome.

Do not require the owner to design the org chart, workflows, task ontology, or agent prompts.

## 6.4 Initial Exec

OrgIntel creates one persistent Exec identity with:

- responsibility for internal company operation;
- access to company-wide organisational state;
- broad access to the Company Runtime;
- the owner-granted Authority Plane envelope;
- a durable identity package and working principles;
- responsibility to form additional actors only when useful.

## 6.5 Runtime scaffold

Recommended initial files:

```text
/company/
├── README.md
├── handbook/
│   ├── mandate.md          # read-only projection of owner truth
│   ├── operating-principles.md
│   └── evidence-and-decisions.md
├── projects/
├── shared/
├── skills/
├── services/
├── outputs/
└── scratch/
```

The scaffold is a starting point, not a required schema.

## 6.6 Idempotent bootstrap

Bootstrap uses one `operation_id`. Retrying after partial failure must not create duplicate companies, Exec actors, or runtime volumes.

Partial completion remains visible:

```text
company lifecycle: provisioning
orgintel: ready
runtime: failed
next action: retry runtime provision
```

Do not roll back successfully created durable identity merely because a later component failed.

---

# 7. Status and lifecycle model

## 7.1 Infrastructure company lifecycle

Owned by the Authority Plane:

```text
provisioning
running
externally_frozen
stopped
restoring
archived
```

Meaning:

| State | Meaning |
|---|---|
| `provisioning` | Durable company exists but required components are not yet ready |
| `running` | Runtime may operate and external authority is available within the envelope |
| `externally_frozen` | Internal work may continue; new consequential effects/resource expansion are paused |
| `stopped` | Company Runtime is not executing; durable Authority/OrgIntel state remains |
| `restoring` | Runtime material state is being restored/replaced and reconciled |
| `archived` | Company is inactive and preserved for inspection/recovery; no automatic work resumes |

## 7.2 Runtime instance lifecycle

Owned by the Authority Plane runtime lifecycle manager and observed by OrgIntel:

```text
absent
creating
starting
ready
stopping
stopped
failed
restoring
```

This describes infrastructure, not company progress.

## 7.3 Session lifecycle

OrgIntel owns organisational intent/status; the Runtime Bridge reports process reality.

```text
requested
launching
running
completed
failed
cancelled
lost
```

A completed session does not imply that its commitment is completed. A commitment is completed only when its accountable owner/OrgIntel accepts the outcome.

## 7.4 Operating phase

Owned by OrgIntel:

```text
exploration
validation_or_pre_profit
profit
scale
```

Operating phase influences defaults and priorities; it is not an infrastructure gate and does not automatically expand budget or authority.

## 7.5 Goal and commitment status

OrgIntel may use simple defaults:

```text
Goal stage: framing | exploring | building | validating | operating | reviewing
Commitment: proposed | active | blocked | completed | abandoned
```

Goal stages are descriptive and may vary by domain. Commitment states remain deliberately small.

---

# 8. Core interface contracts

## 8.1 Owner Cockpit → Authority Plane

Purpose:

- read mandate, envelope, spend, resources, effects, and lifecycle;
- approve/deny requests;
- update owner-controlled authority;
- freeze/resume external authority;
- start/stop/snapshot/restore/archive the company.

The cockpit never writes Authority Plane tables directly.

## 8.2 Owner Cockpit → OrgIntel

Purpose:

- send messages, feedback, and directives;
- inspect people, work, evidence, decisions, and attention items;
- accept or reject recommendations;
- change organisational priorities within the mandate;
- inspect operating phase and company health.

A directive becomes durable organisational state. A mandate change additionally invokes the Authority Plane owner interface.

## 8.3 OrgIntel → Runtime Bridge

Core operations:

```text
register_runtime / heartbeat
launch_session
stop_session
send_session_input
query_session
request_checkpoint
query_runtime_observation
```

OrgIntel sends responsibility and context, not shell commands for every work step.

### Session launch envelope

Conceptually:

```text
company_id
runtime_id/runtime_generation
actor_id
session_id
role and current commitment
working directory/worktree request
model/harness selection
focused context packet
applicable instructions and skills
resource class and model budget
session credentials
```

## 8.4 Runtime Bridge → OrgIntel

Meaningful events:

```text
runtime_registered
runtime_health_changed
session_started
session_progress_summary
blocker_reported
artifact_linked
decision_recorded
result_submitted
session_exited
resource_pressure
bridge_reconnected
```

Raw shell output and token streams may be retained operationally but are not organisational events by default.

## 8.5 Agent → OrgIntel

Agents use local MCP/CLI/socket tools for:

```text
read_inbox
send_message
accept_or_reject_commitment
report_blocker
link_artifact
submit_result
request_review
record_observation_or_decision
schedule_follow_up
request_exec_attention
```

Agents never write the OrgIntel database directly.

## 8.6 Exec/Runtime → Authority Plane

V0 operation families:

```text
effects.*
resources.*
runtime.*
authority.inspect
usage.*
```

The Exec can use the existing envelope, request an expansion, or voluntarily narrow/revoke authority. It cannot approve its own expansion or rewrite receipts.

## 8.7 Authority Plane → Runtime

The Authority Plane may materialise:

- a resource grant;
- an endpoint/device;
- a short-lived access token;
- an Infisical Agent Proxy configuration;
- a runtime lifecycle transition;
- a freeze/revocation signal.

The Runtime Bridge makes session-scoped resources available through environment, mounted config, local endpoints, or process resource assignment.

## 8.8 Service health

Every long-running component exposes:

```text
availability
version
company_id where applicable
runtime_generation where applicable
last successful heartbeat
current degraded reason
```

Health endpoints must not expose secrets or raw organisational data.

---

# 9. Artifact reference contract

## 9.1 Purpose

OrgIntel needs to point to real work without becoming an artifact database or custody system.

## 9.2 Supported locator forms

Examples:

```text
filesystem path + runtime_generation
repository + commit/tag/branch
build ID or URL
domain application record ID
object-storage URI
external provider record ID
Authority Plane receipt ID
```

## 9.3 Minimal reference semantics

An artifact reference should carry:

```text
artifact_ref_id
company_id
kind
locator
owning system
created_by actor/session where known
version/digest where useful
human-readable label
access hint
recorded_at
```

A checksum is useful for immutable delivered outputs, but not required for every live working file.

## 9.4 Runtime generation and staleness

A bare path is meaningful only within a runtime generation or durable volume lineage.

After restore or replacement, OrgIntel may mark a reference:

```text
available
stale
missing
superseded
unknown
```

It does not delete the historical commitment or decision merely because a referenced file is missing.

## 9.5 Accepted output

Meaningful outputs should migrate toward durable references:

- a Git commit or tag;
- an exported build;
- an object-store artifact;
- a provider record;
- a stable domain application record.

Scratch work may remain path-only.

---

# 10. Lifecycle operations

## 10.1 Start

`start` makes a stopped company productive again.

Expected behaviour:

1. Authority Plane starts/provisions the Runtime.
2. Runtime Bridge registers the current generation.
3. OrgIntel reconciles schedules and missed wakeups.
4. Running actors are not assumed to have survived.
5. OrgIntel wakes the Exec with a restart context.
6. External authority follows the company's current frozen/running state.

## 10.2 Stop

`stop` halts productive runtime execution while preserving durable state.

Recommended behaviour:

- request graceful session shutdown;
- checkpoint useful work where practical;
- stop the Runtime;
- retain OrgIntel schedules/messages/goals;
- retain Authority Plane history;
- keep the cockpit available.

Stopping is not archiving and does not erase the company.

## 10.3 Freeze external authority

`freeze` is the preferred incident-control action.

- New consequential effects are denied or paused.
- New costly resource provisioning is paused.
- Internal filesystem, Git, analysis, and ordinary local work may continue.
- Existing resources may continue or be selectively revoked based on risk.
- The owner and Exec see the reason.

Freeze consequences, not thought.

## 10.4 Resume external authority

`resume` restores use of the existing envelope after owner/system review. It does not recreate revoked capabilities or increase limits.

## 10.5 Snapshot

A runtime snapshot captures productive material state, not the whole company.

It may include:

- persistent volume/files;
- repositories and worktrees;
- browser profile;
- project databases;
- installed tools and services where supported;
- snapshot manifest with runtime generation and timestamps.

Authority Plane and OrgIntel stores have their own backups and do not roll back with this snapshot.

## 10.6 Restore

Restore creates a new `runtime_generation`.

Canonical flow:

```text
1. Freeze consequential effects as needed
2. Stop current Runtime
3. Restore selected snapshot into new generation
4. Register Runtime Bridge
5. OrgIntel reconciles sessions, artifacts, commitments, and schedules
6. Authority Plane reconciles outstanding effects/resources
7. Exec receives recovery context
8. Resume internal work
9. Resume external authority when safe
```

## 10.7 Replace runtime

Runtime replacement is similar to restore but may use a fresh image and existing persistent data or selected exports.

The company and actor identities remain unchanged. Only `runtime_id`/`runtime_generation` change.

## 10.8 Upgrade

Recommended sequence:

```text
snapshot
→ stop/reduce activity
→ upgrade one component
→ compatibility check
→ reconcile
→ canary company run
→ resume
```

Components may upgrade independently when interface compatibility permits.

## 10.9 Archive

Archive:

- freezes/revokes active authority as configured;
- stops the Runtime;
- cancels automatic wakeups;
- preserves Authority and OrgIntel history;
- preserves selected artifacts/backups;
- keeps the company inspectable.

## 10.10 Destroy

Permanent destruction is an explicit high-risk owner operation and is not required for the first dogfood. Prefer archive by default.

---

# 11. Restore and reconciliation

## 11.1 Why reconciliation is required

The Company Runtime can travel backward. Authority history, organisational learning, and the external world cannot.

Example:

- Aris sends 300 emails.
- The Runtime later restores a snapshot from before the campaign.
- OrgIntel still knows who was contacted and what replies arrived.
- Authority receipts still show the sends.
- The Exec must not repeat the campaign blindly.

## 11.2 Reconciliation domains

### Sessions and processes

- All pre-restore sessions are treated as ended/lost.
- Runtime Bridge reports actual running processes.
- OrgIntel launches fresh sessions where work should continue.

### Files and artifacts

- Validate important referenced commits, paths, and builds.
- Mark unavailable references honestly.
- Recreate or recover only what is still needed.

### Goals and commitments

- OrgIntel retains current organisational truth.
- Commitments are not rewound because the Runtime was restored.
- The Exec decides whether affected work needs reopening, reassignment, or acceptance from durable evidence.

### Effects

- Outstanding/unknown effects are reconciled against the Authority Plane/provider.
- Successful effects are never repeated merely because local files are older.

### Resources

- Revalidate resource grants.
- Reissue only still-valid access material.
- Expired/revoked grants remain unavailable.

### Schedules

- Detect missed wakeups.
- Coalesce obsolete timers rather than replaying every historical wakeup.

## 11.3 Recovery context

The first Exec session after restore receives:

- restore reason and snapshot time;
- new runtime generation;
- company goals and commitments that remained current;
- missing/stale artifact references;
- effects since the snapshot;
- outstanding owner attention;
- recommended reconciliation work.

## 11.4 No automatic global rollback

Do not attempt to make OrgIntel or Authority Plane data match an old runtime snapshot by deleting newer state. Preserve history and repair forward.

---

# 12. Failure and degraded operation

| Failure | Expected behaviour |
|---|---|
| Runtime Bridge disconnects | Running work may continue locally; launches/messages pause; reconnect and reconcile |
| Company Runtime fails | OrgIntel and Authority state remain; owner can inspect; restart/restore Runtime |
| OrgIntel unavailable | Running agents may continue local work; organisational writes/wakeups pause; Authority Plane remains independent |
| Authority Plane unavailable | Internal work continues; new effects/resources/lifecycle changes pause |
| Authority DB unavailable | Do not guess permission or effect outcome |
| OrgIntel DB unavailable | Do not invent commitments/messages; preserve local work and retry |
| Model gateway unavailable | Agents stop or use an approved alternative; files/services remain |
| External provider unavailable | Affected action/resource pauses; unrelated company work continues |
| Cockpit unavailable | Company may continue under existing mandate; owner interactions pause |
| Runtime restored to stale state | Reconcile forward; never rewind external history |
| Version incompatibility | Enter visible degraded state; do not silently discard messages or commands |

## 12.1 Buffering

The Runtime Bridge may buffer a small bounded set of meaningful events during transient OrgIntel outages.

- Buffering is best effort.
- Duplicate delivery is tolerated and deduplicated by event/session identity where useful.
- The buffer is not a new authoritative ledger.
- Ordinary work artifacts remain the strongest recovery source.

## 12.2 Local progress during outages

Already-running actors may continue working when OrgIntel or Authority is down, provided they do not require unavailable organisational or external operations.

This prevents a coordination-service outage from invalidating productive work.

---

# 13. Cockpit composition contract

The Owner Cockpit composes read models from each owning service.

## 13.1 Attention

An attention item may originate from:

- OrgIntel: decision, blocker, recommendation, phase change, weak evidence;
- Authority Plane: approval, budget, freeze, unknown effect, resource request;
- Runtime/Bridge: runtime failure, lost session, missing output.

The cockpit uses one attention envelope, but resolution writes back to the owning layer.

## 13.2 Work

The Work view is primarily OrgIntel state linked to Runtime artifacts and Authority receipts.

```text
Goal
→ Commitment
→ Artifact reference
→ Evidence/receipt
```

## 13.3 People

People and chat are OrgIntel-owned. Runtime session status is a live projection from the Bridge. Authority grants may be shown as read-only references.

## 13.4 Authority

The Authority view reads the Authority Plane directly. OrgIntel may explain organisational context, but cannot rewrite the authoritative envelope or receipts.

## 13.5 Degraded presentation

The UI must distinguish:

- stale projection;
- owning service unavailable;
- runtime offline;
- external authority frozen;
- unknown external outcome.

Do not display a generic green status when one underlying source is unavailable.

---

# 14. V0 deployment contract

## 14.1 Local Docker Compose

Recommended V0 deployment:

```text
docker-compose project
├── owner-ui / backend
├── authority-service
├── orgintel-service
├── postgres
├── infisical-agent-proxy
└── company-runtime
    └── runtime-bridge
```

Infisical itself may be cloud-hosted or separately deployed.

## 14.2 Network rules

- Runtime may reach public internet, OrgIntel, Authority Plane, model gateway, and Infisical Agent Proxy.
- Runtime cannot reach Docker/containerd control, host filesystem, Authority/OrgIntel databases, or Infisical secret-reading APIs.
- OrgIntel cannot directly execute shell commands except through the Runtime Bridge's bounded session interface.
- Cockpit reaches services through authenticated APIs.

## 14.3 One company first

The contract uses `company_id` and explicit service boundaries, but V0 optimises for one company, one owner, one Exec, and one shared Company Runtime.

It does not require multiplayer, fleet management, shared multi-tenancy, or generic hosted control planes.

---

# 15. V0 acceptance scenarios

## 15.1 Clean bootstrap

Given an owner mandate and broad permissive envelope:

- the company is created once;
- OrgIntel creates one durable Exec;
- the Runtime becomes ready and registers;
- the Exec receives context and starts work;
- the owner sees the company in the cockpit;
- restarting the bootstrap operation does not duplicate anything.

## 15.2 Runtime crash and restart

- Active session is marked lost/failed.
- OrgIntel state survives.
- Files on the persistent volume survive.
- Runtime restarts and registers the same or next generation.
- Exec receives recovery context and resumes useful work.

## 15.3 Runtime restore after external effect

- An email effect succeeds and has a receipt.
- The Runtime restores a snapshot from before the send.
- Authority receipt remains.
- OrgIntel follow-up state remains.
- The Exec does not resend blindly.

## 15.4 OrgIntel outage

- Existing actor continues local work.
- OrgIntel messages/launches pause.
- Meaningful result is buffered or recoverable from artifact.
- On reconnect, Bridge and OrgIntel reconcile without invalidating the work.

## 15.5 External freeze

- Owner freezes authority.
- Runtime and OrgIntel continue internal work.
- New consequential effect is denied/paused.
- Existing files and model sessions are not destroyed.
- Owner resumes without changing the mandate or re-provisioning the company.

## 15.6 Version mismatch

- Runtime Bridge advertises an older supported contract.
- OrgIntel either uses compatible operations or enters a visible degraded state.
- No silent loss or reinterpretation of organisational state occurs.

---

# 16. Implementation sequence

1. Define shared IDs and common envelope types in a small versioned package.
2. Implement idempotent company creation in Authority Plane.
3. Implement OrgIntel company/Exec bootstrap.
4. Implement Runtime create/start/health in Authority Plane.
5. Implement outbound Runtime Bridge registration and heartbeat.
6. Implement session launch/result contract.
7. Implement artifact references.
8. Compose the cockpit read models.
9. Implement stop/freeze/resume.
10. Implement runtime snapshot/restore and reconciliation report.
11. Add compatibility and degraded-state tests.

Do not begin with a generic service bus, event-sourcing framework, or universal schema registry.

---

# 17. Explicit exclusions

V0 does not specify:

- shared multi-tenancy;
- multiple human collaborators;
- strong per-worker security principals;
- cross-company actors;
- atomic global snapshots;
- exactly-once internal messages;
- universal artifact custody;
- full provider-specific payload schemas;
- generic workflow orchestration;
- semantic inspection of arbitrary network traffic;
- a distributed event ledger for every action.

---

# 18. Anti-drift rules

1. One concept, one authoritative owner.
2. Interfaces exchange references and intent, not duplicated domain state.
3. No layer writes another layer's database.
4. No global transaction is required for productive work.
5. Reconcile after failure; do not make every inconsistent state impossible.
6. Runtime failure must not erase organisational identity or external history.
7. OrgIntel failure must not invalidate files or running work.
8. Authority failure must pause consequence, not internal thought.
9. New shared semantics require evidence that two or more layers genuinely need them.
10. If a contract starts modelling internal business workflows, move that concern back to OrgIntel or the Runtime.

---

# 19. Final V0 contract

A Restless company is considered correctly composed when:

> The Authority Plane preserves mandate, authority, resources, and external truth; OrgIntel preserves organisational identity, intent, and learning; the Company Runtime preserves productive work; and the layers can restart, restore, and evolve independently without pretending that one database or workflow owns the whole company.
