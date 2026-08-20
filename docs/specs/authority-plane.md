# Authority Plane Specification

**Version:** 0.1  
**Status:** Core design and MVP implementation contract  
**Date:** 13 August 2026  
**Companions:** `OrgIntel Core Specification`, `Company Runtime and Runtime Bridge Specification`  
**Parent:** `ARCHITECTURE.md — Restless Architecture Source of Truth v0.9`

---

## 0. Document contract

This document defines Layer 1 of the system: the **Authority Plane**.

The Authority Plane is broader than the Authority Kernel. It contains the small deterministic kernel plus the privileged mechanisms that execute external effects, provision productive resources, manage the Company Runtime, meter model access, and safely apply credentials.

This document distinguishes:

- **Core contract** — preserve this boundary or behaviour.
- **Initial hypothesis** — build and test this first; change it when evidence disagrees.
- **Default policy** — the initial operating posture, intentionally permissive.
- **Explicit exclusion** — do not accidentally rebuild this.

The central MVP principle is:

> **Keep the authority boundary real, but make its initial policy permissive. Test mechanics with a fake CLI in `_test`, never a parallel provider architecture.**

The Authority Plane should make useful work possible. It should not turn ordinary company operation into an approval workflow.

---

# 1. Product definition

## 1.1 Purpose

The Authority Plane controls the point where internal company work gains privileged resources or real-world consequence.

It answers three questions:

1. **May the company do this?**
2. **How is the approved access or action safely materialised?**
3. **What consequentially happened?**

It owns:

- owner mandate and company authority;
- standing grants and outer operating envelopes;
- hard budgets and resource ceilings;
- approvals and revocations;
- consequential effect intents and receipts;
- bounded productive resource grants;
- runtime lifecycle and recovery authority;
- external-tool credential references and trusted access paths;
- authoritative usage records where required for enforcement.

It does not own:

- goals, Work nodes, teams, messages, schedules, or company learning;
- project files, code, documents, builds, or browser state;
- company strategy or creative judgment;
- agent reasoning or ordinary internal work;
- a universal model of all business activity.

## 1.2 Core boundary

```text
Owner
  │ mandate, capital, root authority
  ▼
┌──────────────────── AUTHORITY PLANE ────────────────────┐
│                                                        │
│ Authority Kernel                                       │
│ ├── identity and grants                                │
│ ├── budgets and approvals                              │
│ ├── deterministic policy evaluation                    │
│ └── effect/resource/runtime authority state             │
│                                                        │
│ Privileged brokers                                     │
│ ├── generic governed-process runner                    │
│ ├── resource controller                                │
│ ├── runtime lifecycle manager                          │
│ ├── model gateway / metering                           │
│ └── credential integration                             │
│                                                        │
│ Imported infrastructure                                │
│ ├── Infisical and Infisical Agent Proxy                │
│ ├── Docker/containerd                                  │
│ ├── Postgres and object storage                        │
│ └── external provider APIs                             │
└───────────────────────┬────────────────────────────────┘
                        │ bounded authenticated interfaces
          ┌─────────────┴─────────────┐
          ▼                           ▼
   OrgIntel service            Company Runtime
   coordination state          Linux, ACP agents,
   and Exec requests           files, Git, tools
```

The Authority Plane wraps the company’s consequential authority. It does not sit between agents and ordinary Linux work.

## 1.3 Core product posture

**Core contract**

- Internal work should continue when the Authority Plane is temporarily unavailable.
- New consequential effects and privileged resource provisioning should pause safely.
- The owner should be able to freeze consequences without destroying productive state.
- Authority must not increase merely because an agent requests it.
- Runtime snapshots must never roll back authoritative external history.

**Default V0 policy**

Within a broad owner-configured company envelope, most supported requests are automatically allowed.

The V0 plane should primarily provide:

- a real boundary;
- hard outer ceilings;
- a small catastrophic denylist;
- one generic effect runner plus deterministic fake-CLI fixtures for `_test` companies;
- clear outcomes and receipts;
- owner freeze and revoke;
- enough observability to learn what later needs hardening.

---

# 2. Components

## 2.1 Authority Kernel

The Authority Kernel is the small deterministic decision core.

It answers:

> Is this authenticated principal allowed to perform this operation under the current mandate, grants, budget, approval state, and limits?

It owns:

- company and owner identity;
- authoritative mandate versions;
- grants and revocations;
- hard budget and resource envelopes;
- approval requirements and decisions;
- effect intents, idempotency state, and receipts;
- resource requests and grants;
- runtime lifecycle authority;
- freeze and stop state.

It should contain no open-ended LLM judgment in the authoritative path.

An LLM may later summarise a request or flag an anomaly for a human. It cannot grant authority, approve an operation, reveal a secret, modify a receipt, or expand a budget.

## 2.2 Generic governed-process effect runner

Effects are discrete operations that cause meaningful external consequences.

Examples:

- send an external campaign;
- issue a production refund;
- publish a production deployment;
- purchase an asset;
- register a domain;
- delete production data;
- create a legally meaningful external promise.

The effect runner:

- accepts effect intents;
- asks the Kernel for an authority decision;
- waits for approval where required;
- launches the actor-selected installed CLI with exact argv in an isolated child;
- injects only named credential bindings into that child;
- records success, failure, or unknown outcome;
- reconciles ambiguous outcomes;
- returns an authoritative receipt or denial.

Tool-specific implementation remains in the mature CLI or SDK installed in the Company Runtime. The
Authority Plane records a generic receipt containing effect class, purpose, tool, exact original
argv, declared artifacts, outcome, idempotency key and execution number. It must not grow one
Restless command, payload schema, or adapter per external service.

### What is and is not an effect

**Core contract**

The effect service is an **accountability boundary, not an API gateway**. The test is *meaningful external consequence*, never *does this touch the network*.

Most of what a company does outside itself is ordinary work and must not be routed through this service. An agent researching competitor pricing in a browser, reading a marketplace listing, cloning a public repository, or navigating a supplier's site is working — not requesting an effect. Forcing that through effect intents would make the surface the company's proxy for the internet, which is unbounded, defeats the runtime's whole purpose, and buys no accountability.

Three shapes, and only the first belongs to this service:

| Shape | Examples | Where it runs |
|---|---|---|
| **Effect** | send an email, charge a card, publish a deployment, post a public listing, register a domain | effect service, via an adapter; receipt required |
| **Ordinary external work** | research, reading, browsing, cloning, testing against a public endpoint | the Company Runtime, directly; no receipt |
| **Prepared last mile** | identity verification, payout details, 2FA, an account that is legally a person | preserved state handed to the owner |

**A receipt does not require an API.** An effect may be performed by an HTTP adapter or by driving the company's own authenticated browser session in its runtime; the receipt, idempotency key, party and reconciliation are identical either way. Accountability attaches to the consequence, not to the transport. This matters because the set of consequential actions with clean APIs is much smaller than the set of consequential actions, and an adapter per provider does not scale.

The practical consequence for adapters: build a small number of API adapters where an API exists and the volume justifies it, plus **one browser adapter** that performs consequential actions through the runtime's own session — rather than pursuing coverage of every provider.

### 2.2.1 Attestation is not confirmation

**Core contract.** Added 15 August 2026, from building this backwards.

When the company performs an effect itself and reports the outcome, the receipt
is an **attestation**: the company's own account of what happened, with an
idempotency key attached. When a provider performs it and answers, the receipt
carries **confirmation**. Both are receipts and both reconcile — but they are not
equal evidence, and the record must say which it holds.

- The receipt's `provider` field names the attester: a provider's own name when
  the provider answered, `self-reported` when the company did.
- The outcome of a self-reported effect is **arbitrary JSON by design.** We do
  not know the shape of every consequential action in the world, and a schema
  here would be a provider catalogue wearing a different hat.
- Any ledger, summary or owner-facing surface that tallies effects must keep the
  two apart. A company once reported £45 of revenue against £18 confirmable; a
  ledger that counted self-attested effects as confirmed would launder exactly
  that error into evidence.

**Implementation note, recorded because the order was wrong.** The first live
provider was built as an HTTP adapter, and the next capability was drafted as a
second adapter, before the self-reported path existed at all. That is this
section read backwards: the adapter is the exception justified by a credential
that must stay host-side, and self-reported is the general case.

## 2.3 Resource controller

Resources are continuing, bounded capabilities used to perform work.

Examples:

- local or cloud GPU capacity;
- a temporary database;
- object storage;
- a development environment;
- a bounded cloud worker;
- a scoped API account;
- a temporary public preview environment.

The resource controller:

- receives approved resource requests;
- provisions or allocates the resource;
- creates bounded access;
- reports readiness and usage;
- expires or revokes the resource;
- cleans it up when appropriate.

Once provisioned, the Company Runtime should usually connect directly to the resource. The Authority Plane should not proxy every byte.

## 2.4 Runtime lifecycle manager

The runtime manager controls the privileged mechanics of the Company Runtime lifecycle:

```text
create
start
stop
health
attach
snapshot
restore
destroy
freeze_external_authority
resume_external_authority
```

This is infrastructure lifecycle, not company workflow.

The manager may call Docker/containerd or a future VM provider. The Company Runtime never receives the Docker socket, host filesystem authority, or cloud root credentials.

## 2.5 Model gateway and metering

Model inference is a productive, metered resource rather than an individually approved effect.

The model path should support:

- provider credential isolation;
- company and session attribution;
- coarse spend ceilings;
- rate and concurrency limits where useful;
- provider/model substitution;
- usage reporting to OrgIntel and the owner.

The Runtime should be able to call models continuously within its standing envelope. It should not request owner approval for each inference.

The gateway may be an Authority Service module or a separate deployable component if traffic, provider compatibility, or failure isolation later justifies it.

## 2.6 Credential plane

Infisical is the default imported credential backend.

It has two main roles.

### Trusted service secret access

Authority Plane components authenticate to Infisical using machine identities and retrieve only the provider credentials they need.

Examples:

```text
Stripe adapter       → Stripe production credential
Resource controller  → cloud/GPU provider credential
Email adapter        → email provider credential
Runtime manager       → infrastructure credential if required
```

The Authority Plane stores references and policy. Infisical stores or applies the secret material.

### Direct agent API access through Agent Proxy

For ordinary authenticated APIs, the Company Runtime may connect through the Infisical Agent Proxy.

```text
Agent or company service
       │ ordinary CLI / SDK / HTTP request
       ▼
Infisical Agent Proxy
       │ applies scoped credential
       ▼
GitHub / analytics / low-risk SaaS API
```

The runtime process does not receive the real provider secret.

Infisical decides how credentials are safely stored or applied. It does not decide whether a refund, campaign, purchase, or deployment is economically or constitutionally authorised.

## 2.7 Throwaway-company fake effects

A deterministic fake CLI installed only in a `_test` company should support:

- deterministic success;
- deterministic failure;
- timeout before execution;
- execution followed by lost response;
- duplicate request;
- delayed completion;
- partial success where the provider permits it;
- fake external state that must be queried through a separate status command during reconciliation;
- deterministic test approval or rejection.

Behavioural inputs such as customer replies may be controlled files or messages in a `_test`
company. They are never live-company evidence and do not justify a provider-shaped simulation layer.

---

# 3. Sources of truth

| State | Authoritative source |
|---|---|
| Owner mandate, root company authority, grants, budgets, approvals, effect intents, receipts, resource grants, runtime lifecycle | **Authority Plane store** |
| Actors, goals, Work nodes, messages, schedules, decisions, hypotheses, experiments, organisational learning, artifact references | **OrgIntel service and store** |
| Code, documents, assets, builds, browser state, project databases, installed tools, working files, active experiments | **Company Runtime filesystem, Git, and project applications** |
| Actual external provider state | **External provider**, referenced by Authority Plane receipts and reconciled state |
| Raw secret material | **Infisical or provider-native credential system** |

The Authority Plane may reference an OrgIntel actor or goal for attribution, but it does not own that organisational concept.

OrgIntel may reference an effect receipt or resource grant, but it cannot rewrite it.

The Authority Plane also owns the company's safe legal profile projection and operating-money
controls. Restricted personal identity evidence remains in the provider or an owner-controlled
vault; Restless stores only the minimum business fields needed by the company and an owner
assertion or public-registry observation describing where those fields came from.

There should be no cross-layer database foreign keys. Stable identifiers and reconciliation are sufficient.

---

# 4. Principals and authority

## 4.1 V0 security principal

**Initial hypothesis**

The hard runtime authority boundary in V0 is the **company/Exec envelope**, not every individual worker.

```text
owner principal
    ↓ grants root company envelope
company/exec principal
    ↓ may use it and later delegate narrower subsets
internal actor identities
    ↓ organisational attribution in V0
```

Persistent `actor_id` values remain important for responsibility and learning, but shared access inside one Company Runtime means they are not yet strong hostile-process security identities.

Do not claim per-worker isolation until processes, credentials, and tool access are actually separated.

## 4.2 Who calls the Authority Plane

**Default V0**

- The owner calls authority-management and approval operations through the owner cockpit.
- The Exec calls effect and resource APIs.
- Workers request consequential actions from the Exec through OrgIntel.
- The OrgIntel service may call runtime lifecycle operations needed for recovery or wakeup under a narrow system grant.
- The Runtime Bridge may authenticate sessions and consume already-issued grants, but should not mint authority.

Later, a worker may receive a narrower direct capability when a real workload justifies it.

## 4.3 Authority flow

Authority flows downward and may only remain equal or become narrower.

```text
Owner
  ↓ root mandate and company envelope
Authority Kernel
  ↓ bounded usable grants
Exec
  ↓ optional attenuated delegation
Workers or services
```

Requests and evidence flow upward.

The Exec may:

- use standing grants;
- allocate internal sub-budgets;
- revoke or narrow delegated authority;
- request a larger envelope;
- request owner approval;
- request productive resources;
- request consequential effects.

The Exec may not:

- increase its own total authority;
- approve its own escalation;
- alter the owner mandate;
- rewrite Kernel policy;
- modify receipts or provider history;
- disable owner freeze, revoke, or recovery;
- access provider-root secrets merely because it can use a capability.

## 4.4 Mandate and envelope

The owner mandate should be small and durable.

It may define:

- the company’s authorised purpose;
- absolute prohibitions;
- maximum capital or periodic spend;
- allowed external categories;
- exceptional approval requirements;
- owner stop and recovery rights.

The operating envelope turns this into practical standing authority, such as:

```text
model spend per month
compute spend per month
local GPU access
allowed deployment environments
email domains and approximate send limits
purchase threshold
allowed providers
whether delegation is permitted
```

OrgIntel interprets the mandate into company strategy. The Authority Plane enforces only the outer boundary.

---

# 5. Operation families

Do not create one universal command algebra. Keep three distinct operation families because they have different semantics.

## 5.1 Effects

An effect is a discrete consequential operation.

Conceptual request:

```text
effect_type
principal
company_id
arguments
purpose
idempotency_key
estimated_cost
related_actor / goal / artifact references
```

Conceptual lifecycle:

```text
requested
  → authorised | denied | awaiting_approval
  → executing
  → succeeded | failed | unknown
  → reconciled when required
```

Possible results:

- **succeeded** — provider-confirmed result with receipt;
- **failed** — confirmed not executed or safely failed;
- **unknown** — provider may have executed, but response is ambiguous;
- **denied** — policy or approval rejected the request;
- **awaiting_approval** — only this effect waits; unrelated internal work continues.

### 5.1.1 Operating-money transfers

**Core contract.** A money transfer is a narrow typed effect because two facts must be enforced
atomically before a provider call: the exact payment must fit both its per-payment and aggregate
envelopes, and the same owner handoff must never reserve or submit twice. The durable intent binds:

```text
company + Work + pending owner PaymentConfirmation handoff
source account + existing provider beneficiary
exact amount and currency + purpose
idempotency key + provider request/transfer ids
reserved aggregate amount + observed provider state
```

Submission does not create beneficiaries and does not amount to Restless owner approval. Where the
provider supports native approval, Restless submits into that workflow and brings the exact
provider-native approval last mile to the owner. Only an authenticated provider read may advance or
reconcile provider state. An ambiguous create or read becomes `unknown`, retains its reservation,
and must not be retried as a new payment.

The authenticated observation is current external truth, not a locally absorbing terminal state. A
provider may report a later correction or reversal (including a paid transfer later becoming failed);
Authority records that transition atomically with the current payment projection, and OrgIntel is
notified without inventing a second payment or a second owner approval.

This is a deliberately bounded exception to the generic governed-process runner. It is justified by
host-only financial credentials, atomic money envelopes, and ambiguous-outcome reconciliation. It
does not establish a provider catalogue, universal procurement state machine, or one adapter per
external capability.

**Initial hypothesis.** The first implementation is AUD-only, uses pre-existing provider
beneficiaries, and supports one provider. A real sandbox run must prove the provider's request-id,
approval, status, and recovery behaviour before any low-value live transfer.

## 5.2 Resources

A resource is bounded ongoing access used for productive work.

Conceptual request:

```text
resource_kind
principal
company_id
purpose
resource_class
limits
maximum_cost
requested_duration
related_actor / Work
```

Conceptual lifecycle:

```text
requested
  → authorised | denied
  → provisioning
  → ready | failed
  → expired | released | revoked
```

A resource grant should contain only what the runtime needs:

```text
resource_id
kind
endpoint or device reference
access mechanism
limits
expiry
usage reference
cleanup behaviour
```

The runtime should receive job-scoped or resource-scoped access, not provider-root authority.

## 5.3 Runtime lifecycle operations

Runtime operations are privileged infrastructure actions:

```text
create
start
stop
snapshot
restore
attach
destroy
freeze_external_authority
resume_external_authority
```

They use their own lifecycle and records. They are not forced into effect or resource semantics.

A restore creates a new runtime generation or records a clear generation transition. It does not roll back OrgIntel or Authority Plane history.

## 5.4 Model usage

Model usage is a high-frequency metered resource.

It should normally follow:

```text
session receives model access within company envelope
→ calls provider-compatible gateway or proxied endpoint
→ usage is attributed and metered
→ hard ceiling or rate policy may stop further use
```

Do not create an effect record for every completion.

---

# 6. Permissive MVP policy

## 6.1 Policy stance

**Default V0**

```text
allow by default within the configured company envelope
+ hard company spend/resource ceilings
+ tiny catastrophic denylist
+ optional explicit approval for selected operations
+ owner freeze/revoke
+ record consequential outcomes
```

This is intentionally not zero trust inside the company.

The purpose of V0 is to prove that the company can build, sell, and operate. Controls should prevent obvious catastrophic exposure without forcing the owner to babysit normal work.

## 6.2 What is still hard from day one

Even under permissive policy:

- no Docker or host-control socket enters the runtime;
- no provider-root infrastructure, payment, or secret-manager credentials enter the runtime;
- the runtime cannot directly write the Authority Plane database;
- every brokered effect has an authenticated principal, intent ID, idempotency key, and result;
- unknown outcomes are reconciled before blind retry;
- runtime restore never rolls back effect history;
- hard spend/resource ceilings cannot be exceeded by ordinary Exec requests;
- the owner can freeze external effects and revoke access.

## 6.3 Tiny catastrophic denylist

The exact list should stay small and company-specific. V0 examples may include:

- bank transfers or payouts;
- changing root owner authority;
- exporting raw provider secrets;
- disabling receipts, freeze, or recovery;
- destructive production deletion without explicit enablement;
- uncapped production spend;
- access to host or container-control authority.

Most unsupported high-impact operations can simply have no adapter yet rather than require a complex deny policy.

## 6.4 Approvals

Approvals should be rare exceptions, not the default operating path.

Use them when:

- an operation exceeds a standing threshold;
- authority would expand;
- the action is materially irreversible;
- a public or legal promise falls outside the mandate;
- the owner explicitly reserved the decision.

An approval pauses only the affected effect. It must not freeze unrelated company work.

## 6.5 No policy language in V0

Do not build a generic policy DSL or enterprise rule engine.

Use small typed checks and configuration for:

- allowed operation kind;
- amount or count ceiling;
- provider/environment scope;
- expiry;
- whether approval is required;
- whether delegation is allowed;
- hard company freeze state.

Add more expressive policy only when repeated real operations cannot be represented clearly.

---

# 7. Direct, proxied, and brokered access

The Authority Plane should not become the data path for all work.

| Access type | Initial mechanism |
|---|---|
| Public browsing, package installation, public APIs | Direct from Company Runtime |
| Local fixed CPU/RAM/disk/GPU already assigned to runtime | Direct use inside runtime limits |
| Dynamically provisioned GPU/database/storage | Authority resource request, then direct scoped access |
| GitHub, analytics, ordinary authenticated SaaS | Direct through Infisical Agent Proxy or scoped service identity |
| Model provider | Model gateway or credential proxy within metered envelope |
| Test-mode Stripe or mocked payments | Direct test credentials or mock adapter |
| Production refund, payout, mass campaign, destructive deployment | Brokered Authority effect |
| Deployed product’s routine provider access | Separate restricted service identity |
| Provider administration/root account access | Kept outside runtime; human or tightly brokered |

The practical rule is:

> **Grant direct access to bounded resources that enable work. Broker discrete actions that create material consequences.**

## 7.1 Logged-in browser sessions

A logged-in browser is itself a capability.

For V0:

- ordinary low-risk accounts may live in the persistent runtime browser;
- sensitive accounts should use restricted roles, brokered APIs, or human takeover;
- the Authority Plane should not attempt to semantically inspect every HTTP request;
- high-impact credentials should not be present merely because the browser is generally available.

Provider-root enrolment, financial-account administration, identity or business verification, MFA
and initial credential issuance use a provider-hosted flow in the owner's browser outside the Company
Runtime. Any resulting API secret enters through a dedicated owner-authenticated Authority ingress
and is stored or applied by Infisical; provider passwords, MFA factors, session cookies and identity
evidence do not enter Runtime or OrgIntel. Connection state becomes verified only after an
authenticated provider observation. Authentication does not itself approve a consequential effect.

The cross-plane ownership and risk dispositions are in
[`ADR 0002`](../adr/0002-owner-provider-authentication-handoffs.md).

---

# 8. Credential and Infisical model

## 8.1 Machine identities

Use a small number of meaningful identities initially:

- Authority Service identity;
- Infisical Agent Proxy identity;
- per-company proxy/client identity;
- optional separate identity for especially privileged adapters.

Do not create one secret-manager identity per ephemeral agent before the workload requires it.

## 8.2 Secret references

The Authority Plane database should store references such as:

```text
provider
credential_reference
purpose
scope metadata
rotation status
```

It should not duplicate raw secret material from Infisical.

## 8.3 Agent Proxy grants

When the owner enables an ordinary authenticated service:

1. Authority state records the company’s allowed service and scope.
2. The Infisical integration provisions or updates the relevant proxy permission.
3. The Runtime Bridge configures the session or company service to use the proxy.
4. The agent uses the normal CLI, SDK, or HTTP API.
5. The proxy applies the real credential only to matching provider traffic.
6. Revocation removes proxy access and may terminate the affected session if urgent.

Proxy permission is credential access, not business-effect approval.

## 8.4 Deployed services

A product service created by the company should not automatically inherit the interactive Exec’s access.

Where practical, give it:

- its own service identity;
- its own scoped Infisical path or proxy permission;
- the minimum provider role needed for routine operation;
- separate handling for refunds, payouts, account administration, and other exceptional effects.

---

# 9. Effect correctness

## 9.1 Idempotency

Every brokered effect request must have an idempotency key stable across retries of the same intent.

The Authority Plane should detect:

- a completed equivalent request;
- an already-executing request;
- a prior unknown outcome requiring reconciliation;
- a genuinely new intent.

Do not promise exactly-once semantics across the internet. Provide idempotent intent handling and explicit ambiguity.

## 9.2 Authoritative receipt

A successful receipt should record enough to answer:

- what was requested;
- who requested it;
- under which grant and approval it was authorised;
- which runtime tool and exact argv executed it;
- declared artifact references and the tool's result or external reference;
- cost where known;
- when it occurred;
- whether later reconciliation changed the interpretation.

The receipt should not contain raw provider secrets or unnecessary payload copies.

## 9.3 Unknown outcome

Unknown is a first-class result, not an exception hidden by retries.

Example:

```text
Authority records an Aris send intent
→ the installed email CLI may accept the send
→ the daemon loses the child result
→ Authority records unknown
→ unrelated work continues
→ a separate governed status command queries the provider's own state
→ receipt becomes succeeded or confirmed failed
```

The effect must not be blindly repeated while unknown.

## 9.4 Reconciliation

Reconciliation uses a separate successful generic effect receipt containing the external system's
own status observation. Restless does not implement a service-specific reconciliation API.

Reconciliation may use:

- provider idempotency records;
- provider operation IDs;
- query by external reference;
- list/recent-operation lookup;
- explicit human confirmation where no reliable API exists.

---

# 10. Resource materialisation

## 10.1 Fixed local resources

CPU, RAM, disk, process count, and optional fixed local GPU can be assigned when the Company Runtime starts.

The owner grants the outer envelope once. OrgIntel allocates it among sessions without an Authority decision for every build or process.

## 10.2 Dynamic resources

For cloud GPUs, temporary databases, storage, or workers:

```text
Exec requests resource
→ Kernel checks scope, hard budget, and freeze state
→ Resource controller provisions it
→ Authority records a bounded resource grant
→ Runtime Bridge materialises endpoint/device/token/config
→ agent or service connects directly
→ usage is metered
→ resource expires, releases, or is revoked
```

The Runtime Bridge may expose a grant through:

- environment variables;
- a mounted read-only configuration file;
- a local Unix socket;
- a job-specific endpoint and token;
- a device assignment when supported.

It must not expose cloud-provider root credentials.

## 10.3 Resource cleanup

A resource should define:

- owner company and requesting actor;
- expected duration;
- maximum cost or quota;
- automatic expiry behaviour;
- persistence expectations;
- cleanup responsibility;
- where outputs must be stored before termination.

OrgIntel decides whether the work remains worth continuing. The Authority Plane enforces the outer ceiling and performs the mechanical release or revocation.

---

# 11. Runtime lifecycle and time travel

## 11.1 Runtime generations

Each created or restored Company Runtime should have a clear runtime-generation identity.

This allows the system to distinguish:

- current OrgIntel state;
- current Authority history;
- files and processes belonging to an older restored runtime state.

## 11.2 Snapshot ownership

Runtime snapshots contain:

- files and repositories;
- browser state;
- installed runtime tooling that is captured by the provider;
- project databases and services;
- local caches and active work.

They do not contain authoritative OrgIntel or Authority Plane history.

## 11.3 Restore reconciliation

After restore:

1. Runtime manager records the new generation.
2. Runtime Bridge reconnects and reports visible state.
3. OrgIntel retains current actors, Work nodes, messages, and learning.
4. Authority Plane retains current grants, receipts, and resource/effect history.
5. The Exec receives a recovery context.
6. Stale artifact references and missing work are surfaced.
7. Unknown or completed external effects are reconciled before repetition.

Temporary inconsistency is acceptable. Silent duplication of consequential effects is not.

---

# 12. Budgets and economic controls

## 12.1 Budget categories

Start with coarse company-wide categories:

- model usage;
- compute and GPU;
- external services and APIs;
- direct purchases or effect spend.

Do not require perfect per-task accounting in V0.

OrgIntel may attribute usage approximately to actors, goals, or experiments for organisational learning. The Authority Plane owns only the hard enforceable ceiling and authoritative provider spend where available.

## 12.2 Enforcement

When a hard ceiling is reached:

- deny or pause new operations in that category;
- preserve internal files and work;
- notify the Exec;
- allow the Exec to stop lower-value work or request a larger envelope;
- escalate to the owner only when the envelope must expand.

A model-spend ceiling should not destroy the runtime. A GPU ceiling should not block ordinary editing. A purchase denial should not freeze unrelated research.

## 12.3 Runaway control

The Authority Plane may enforce coarse rate or concurrency limits for:

- model usage;
- external campaign sends;
- resource provisioning;
- provider API use where misuse creates cost or exposure.

OrgIntel should handle the organisational diagnosis and replanning. The Authority Plane should enforce only the outer boundary.

---

# 13. Freeze, revoke, and human rescue

The preferred emergency action is:

> **Freeze new consequences while preserving the company’s internal work.**

A company freeze may:

- reject new brokered effects;
- prevent new privileged resources;
- revoke or disable Agent Proxy access;
- stop selected running resources;
- continue owner access, OrgIntel visibility, files, Git, and local inspection;
- optionally allow model access for diagnosis within a small safe budget.

The owner should also be able to:

- revoke one service or provider;
- lower a budget;
- stop one resource;
- stop or snapshot the Runtime;
- attach to the same browser/desktop;
- restore and resume after inspection.

Do not make “kill the whole company” the only rescue mechanism.

---

# 14. Observability and audit scope

Record what is necessary to understand authority and consequence:

- authenticated request principal;
- requested operation and purpose;
- policy decision and grant used;
- approval state where relevant;
- adapter/controller execution result;
- effect receipts and unknown outcomes;
- resource grants, expiry, release, and usage;
- runtime lifecycle changes;
- hard-budget consumption;
- freeze and revocation events.

Do not turn these into governance records:

- ordinary shell commands;
- every file edit;
- internal agent reasoning;
- every network packet;
- every OrgIntel message;
- every model token as a permanent audit event.

Operational logs may exist for debugging and telemetry, with normal retention and pruning.

---

# 15. Failure posture

| Failure | Expected behaviour |
|---|---|
| Authority Service unavailable | Internal runtime work continues; new effects/resources pause |
| Authority DB unavailable | Do not guess authority; preserve work and retry later |
| Tool fails before execution | Return confirmed failure when known |
| Tool result is ambiguous | Record unknown and reconcile before retry |
| Resource controller fails | Existing ready resources may continue; new provisioning pauses |
| Infisical Agent Proxy unavailable | Affected authenticated direct APIs fail; local work continues |
| Credential revoked | New proxied/provider access fails closed; terminate urgent sessions if needed |
| Runtime fails | Authority and OrgIntel history remain; restart or restore runtime |
| OrgIntel fails | Authority state remains; running local work may continue; organisational requests pause |
| Budget exhausted | Only the affected category is denied or paused |
| Bad runtime tool release | Disable or replace that tool; other tools and internal work continue |

No component should invent a successful effect because another service is unavailable.

---

# 16. V0 deployment

## 16.1 Deployment shape

```text
Docker Compose per company
├── owner-ui
├── authority-service
│   ├── authority kernel
│   ├── effect service + adapters
│   ├── resource controller
│   ├── runtime manager
│   ├── model gateway integration
│   └── Infisical adapter
├── orgintel-service
├── postgres
│   ├── authority database
│   └── orgintel database
├── infisical-agent-proxy
├── mock-provider services where useful
└── company-runtime
    └── runtime bridge + ACP agents + work environment
```

Infisical itself may be Infisical Cloud or a separately hosted installation. It need not run in the per-company Compose stack.

## 16.2 Modular monolith first

The Restless-owned Authority Plane should initially be one Rust `authority-service`, modular internally:

```text
authority_service
├── policy_core
├── identity_and_grants
├── effect_service
├── resource_controller
├── runtime_manager
├── model_metering
├── infisical_adapter
└── provider_adapters
```

Internal interfaces may include:

```text
EffectProvider
ResourceProvider
RuntimeProvider
CredentialProvider
ModelProvider or ModelMeter
```

Do not deploy one microservice per adapter merely because the concepts are distinct.

Split a module only for a concrete reason such as a stronger privilege boundary, independent failure behaviour, long-running provisioning, traffic scale, language requirement, or operational ownership.

## 16.3 Network posture

The Company Runtime may reach:

- public internet;
- OrgIntel service;
- Authority Service;
- model gateway;
- Infisical Agent Proxy;
- explicitly provisioned resources.

It may not reach:

- Docker/containerd control sockets;
- host filesystem mounts outside its allocated storage;
- Authority or OrgIntel databases directly;
- Infisical raw secret-reading APIs;
- cloud-instance metadata credentials;
- provider-root administration credentials.

---

# 17. Logical data model

This is a small ownership map, not a final schema.

Likely durable records:

```text
companies
owner_mandates
grants
revocations
budgets
approvals
effect_intents
effect_receipts
resource_requests
resource_grants
runtime_instances
runtime_snapshots
usage_records
credential_bindings        # references and metadata, not raw secrets
legal_profiles             # safe business fields and observation metadata only
provider_connections       # environment, account and scope evidence; never raw secrets
money_envelopes            # source, beneficiary, currency and aggregate hard limits
payment_intents            # typed transfer reservations and reconciled provider state
```

Avoid adding a durable entity for every external tool concept. Generic effect intents and receipts
may carry bounded JSON outcome data; tool-specific state remains in the external system.

The four bounded records above are not a general external-capability registry. They exist because
legal identity and operating money are Authority-owned facts; external sourcing decisions remain
ordinary Work in OrgIntel.

No Work, Attempt, Team, Asset, Review, or internal process entities belong here.

---

# 18. Conceptual API surface

Keep APIs grouped by meaning.

## 18.1 Owner and authority

```text
authority.get_mandate
authority.get_envelope
authority.update_envelope          # owner only
authority.approve
authority.deny
authority.freeze
authority.resume
authority.revoke
```

## 18.2 Effects

```text
effects.request
effects.get
effects.reconcile
```

## 18.3 Resources

```text
resources.request
resources.get
resources.release
```

## 18.4 Runtime

```text
runtime.create
runtime.start
runtime.stop
runtime.health
runtime.snapshot
runtime.restore
runtime.attach
runtime.destroy
```

## 18.5 Usage

```text
usage.current
usage.history
```

Tool-specific operations remain ordinary argv under one generic effect operation rather than an
unbounded top-level API namespace.

Do not force all APIs into one universal `Command` enum.

---

# 19. Generic effect-runner contract

One deterministic fake CLI should exercise the same runner used by installed real tools. The matrix
should include:

- success;
- safe failure;
- duplicate idempotency key;
- timeout before execution;
- execution with lost response;
- reconciliation to success;
- reconciliation to failure;
- budget denial;
- freeze denial;
- delayed approval;
- credential revocation;
- daemon restart during operation.

The runtime uses ordinary tool syntax in both cases. Only the executable and `_test` company differ;
Restless's generic envelope and receipt do not.

---

# 20. Dogfood examples

## 20.1 Cosmon

Initial Authority Plane use:

- model budget;
- optional local GPU allocation;
- deploy a playable browser build through the installed deployment tool's staging target;
- optionally purchase one low-cost licensed asset under a broad threshold.

Passing evidence:

- Exec requests the needed resource/effect without owner micromanagement;
- runtime receives the resource or deployment result;
- receipt remains after runtime restart or restore;
- the playable build remains the actual product outcome.

## 20.2 Aris

Initial Authority Plane use:

- scoped email access or brokered campaign send;
- model and API budgets;
- payment/conversion observations from a controlled real sale;
- optional landing-page deployment.

Passing evidence:

- sales work proceeds within a broad envelope;
- a send with ambiguous provider response is reconciled rather than duplicated;
- spend ceiling is respected;
- OrgIntel schedules follow-up using the authoritative receipt.

## 20.3 Thymelake

Initial Authority Plane use:

- staging deployment;
- restaurant onboarding email/calendar access;
- scoped product service credentials;
- mocked then real payment/provider integrations where appropriate.

Passing evidence:

- the company can launch a restaurant pilot without exposing provider-root credentials;
- routine product operation uses a restricted service identity;
- exceptional financial/admin actions remain brokered;
- owner can freeze new consequences while preserving support and diagnostic work.

---

# 21. V0 acceptance scenarios

## 21.1 Permissive pass-through

- Owner configures a broad staging-deployment envelope.
- Exec requests deployment.
- Policy automatically allows it.
- Mock adapter executes.
- Receipt is returned and referenced by OrgIntel.

## 21.2 Hard ceiling

- Exec requests a resource that would exceed the configured compute budget.
- Only the resource request is denied or paused.
- Existing internal work continues.
- Exec may reallocate work or ask the owner to expand the envelope.

## 21.3 Unknown effect

- Provider executes an Aris campaign send but drops the response.
- Authority records `unknown`.
- Retry with the same intent does not duplicate the campaign.
- Reconciliation finds provider state and produces a receipt.

## 21.4 Dynamic resource

- Exec requests a bounded GPU worker.
- Resource controller provisions a mock or real worker.
- Runtime receives job-scoped access.
- Outputs return to company storage.
- Resource expires and usage is recorded.

## 21.5 Runtime time travel

- External effect succeeds.
- Runtime is restored to a snapshot from before the effect.
- Authority receipt remains current.
- The company does not repeat the external action blindly.

## 21.6 Freeze and rescue

- Owner freezes external authority.
- New effects and privileged resources stop.
- Files, Git, OrgIntel visibility, browser inspection, and local diagnosis remain available.
- Owner resumes authority after inspection.

## 21.7 Agent Proxy revocation

- Company uses GitHub through Infisical Agent Proxy.
- Owner revokes the service grant.
- New proxied access fails.
- Local Git work and unrelated company activity continue.

---

# 22. V0 implementation sequence

1. Create the Authority Service skeleton and separate Authority database ownership.
2. Add company, owner mandate, broad operating envelope, freeze state, and authenticated principals.
3. Implement the permissive policy core with hard ceilings and a tiny denylist.
4. Implement one generic governed-process runner with idempotency, receipts, unknown outcome, and reconciliation.
5. Connect OrgIntel/Exec to the effect request path.
6. Implement runtime lifecycle operations against the existing Docker provider.
7. Implement one resource provider: fixed local GPU or mock temporary worker.
8. Add coarse usage and budget metering.
9. Integrate Infisical for Authority Service provider credentials.
10. Add Infisical Agent Proxy for one ordinary authenticated API such as GitHub.
11. Preserve or integrate the model gateway under a standing company model budget.
12. Run runtime-restore and external-effect reconciliation tests.
13. Run a deterministic fake CLI in a `_test` company, then a real tool dry-run/status probe, then one controlled live effect.
14. Add controls only in response to observed dogfood risk or friction.

---

# 23. Explicit exclusions

Do not build in V0:

- a universal command or event ontology;
- a general policy language;
- per-worker hostile-process isolation;
- one service or adapter per external tool;
- semantic inspection of arbitrary network traffic;
- a custom secrets manager;
- a custom container runtime;
- exactly-once semantics across providers;
- perfect cost allocation to every task and token;
- full banking, payouts, or unrestricted production payment administration;
- broad shared multi-tenancy;
- an LLM that decides authoritative permissions;
- mandatory approvals for ordinary company work;
- permanent audit history for every internal action.

---

# 24. Engineering anti-drift rules

1. **Boundary before policy:** preserve the real interface even when the policy is permissive.
2. **Work before governance:** a new control must address concrete risk, cost, or external harm.
3. **Broad envelopes by default:** ordinary company operation should not require owner approval.
4. **Local failure stays local:** deny the effect or resource, not the company’s unrelated work.
5. **Different semantics stay separate:** effects, resources, runtime lifecycle, and model usage are not one command system.
6. **Adapters are replaceable:** provider details must not leak into the Kernel’s authority model unnecessarily.
7. **No ambient root credentials:** productive access should be scoped, proxied, brokered, or provisioned.
8. **Mocks must be production-shaped:** test ambiguity, duplication, revocation, and failure—not only success.
9. **Import commodity mechanisms:** Infisical, Docker/containerd, Postgres, object storage, and provider APIs remain external tools.
10. **Do not harden speculation:** add finer capability rules only when real operation demonstrates the need.
11. **Freeze consequences, preserve work:** recovery should minimise destruction of useful state.
12. **Delete unused controls:** policy that does not improve safety or economic operation should not accumulate permanently.

---

# 25. Current decisions

- Layer 1 is the **Authority Plane**; the Authority Kernel is only its deterministic decision core.
- The initial Authority Plane is one modular Rust Authority Service plus imported infrastructure.
- The Company Runtime and OrgIntel stores cannot directly write Authority state.
- The V0 hard principal is the company/Exec envelope; actor identities provide attribution rather than strong isolation.
- Only the Exec calls effect and resource APIs initially; workers ask the Exec through OrgIntel.
- Policy is permissive within a broad owner-granted envelope.
- Effects, resources, runtime lifecycle, and model usage remain distinct operation families.
- Provider-root credentials never enter the Company Runtime.
- Infisical stores/applies credentials; Restless owns business authority and effect semantics.
- Ordinary authenticated APIs may use Infisical Agent Proxy.
- Productive resources are provisioned and then used directly through bounded access.
- Consequential effects remain brokered and receipted.
- Mock and real providers share the same interfaces.
- Runtime snapshots never roll back Authority or OrgIntel history.
- The owner can freeze new consequences while preserving internal work.
- Multiplayer, shared multi-tenancy, complex policy languages, and per-worker security remain deferred.

---

# 26. Open questions

These should be answered through implementation and dogfood:

1. Which first real effect adapter gives the best evidence: email, staging deployment, or low-cost purchase?
2. Should Docker authority remain in-process in V0 or reuse a narrow existing runtime daemon?
3. Which model calls are best served by a provider-compatible gateway versus Infisical Agent Proxy?
4. Is one company-level Agent Proxy identity sufficient initially, or do Exec and deployed services need separate identities immediately?
5. Which low-risk APIs are productive enough for direct proxied access, and which effects should remain brokered?
6. What minimum resource usage data is reliable enough to enforce hard ceilings without creating a metering platform?
7. Which provider APIs support strong reconciliation, and where must unknown outcomes require human confirmation?
8. What is the smallest owner mandate and envelope format that remains understandable and useful?
9. How should a deployed company product request or receive a long-lived service identity without inheriting interactive agent authority?
10. When does provider or privilege blast radius justify splitting a broker from the modular Authority Service?

---

# Working summary

The Authority Plane is the trusted outer operating boundary of the company.

Its Kernel decides whether a consequential action or privileged resource is allowed. Its brokers execute effects, provision resources, manage runtime lifecycle, meter model usage, and integrate with credentials. Infisical and other mature infrastructure provide commodity mechanics.

The MVP should be intentionally permissive:

> **Allow most supported operations within a broad owner-set envelope, enforce only hard outer ceilings and a small denylist, record real consequences, and mock the external world behind production-shaped interfaces.**

The Authority Plane succeeds when it gives the company enough real authority to build, sell, and operate without owner babysitting—while preserving a narrow place to harden external risk once dogfood proves which controls are actually needed.
