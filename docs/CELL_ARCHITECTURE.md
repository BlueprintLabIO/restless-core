# Restless cell-based deployment architecture

**Status:** Target elaboration. `ARCHITECTURE.md` remains authoritative.  
**Parent:** `ARCHITECTURE.md` §7.4.  
**Decision:** Restless deploys along its existing plane boundaries. The Authority Plane is
owner-scoped and holds credentials; OrgIntel and the Company Runtime are company-scoped and form the
cell; a thin fleet tier supervises cells. Restless Cloud is multi-owner at its fleet tier and
managed-single-tenant at each autonomous company boundary.

## 1. The unit of isolation is a company

A Restless company is a persistent, mutable computer operated by autonomous agents. Its tenant
boundary is therefore the company, not an owner account, browser session, agent or Work item.

“Cell” names the failure, recovery and customisation boundary; it does not imply one physical server
per company.

## 2. The deployment tiers are the plane boundaries

`ARCHITECTURE.md` §3.2 already assigns provider credentials, consequential effect execution, budgets
and receipts to the kernel, and §4 assigns actors, Work, messages and context to OrgIntel. Those are
not merely conceptual planes. **They are the deployment boundaries**, because they are the trust
boundaries:

```text
fleet tier — per host or region
  provisions, starts, stops, health-checks and upgrades cells
  holds no provider credential and no company state

account plane — per owner  (the Authority Plane / kernel)
  owner's provider accounts, OAuth and secret-backend identity
  effect execution, approvals, budget enforcement, receipts
  owner cockpit, CLI endpoint, owner↔company directory, spend rollup

        ├── company cell A — per company
        │     OrgIntel (own database) · Company Runtime (own container)
        │     company secret scope · volumes · browser profile
        └── company cell B — per company
              OrgIntel (own database) · Company Runtime (own container)
              company secret scope · volumes · browser profile
```

An owner may hold many companies. Each is a cell. The account plane spans **only that owner's**
companies; it never spans owners. The fleet tier spans owners in Cloud but holds no credential and
no company state, so it cannot act as any company.

### 2.1 The rule that places every boundary

> **Effects execute where the credential lives. The cell requests; it never holds.**

This is the generalisation of a rule Restless already enforces at the container edge — provider keys
and broker bearers never cross into the Company Runtime. The same reasoning applies one level up: a
process that serves a company's agents is inside that company's blast radius, so it must not hold the
owner's credentials either.

Consequences, all of which follow mechanically:

- A cell holds no provider key, no owner OAuth token and no secret-backend machine identity. It holds
  its own scoped secrets and asks the account plane for anything consequential.
- Authority **state** for a company (grants, budgets consumed, receipts) is that company's record.
  Authority **enforcement** runs beside the credential, in the account plane. A compromised cell
  cannot self-authorise, because authorisation is not a value it possesses.
- Two companies of one owner share the enforcement process but no mutable state, and neither can act
  as the other, because neither holds the credential.
- The owner's provider account is bought by a person and brokered down, not copied into every cell.
  Per-cell copies of one owner's token would be strictly worse security for no isolation gain.

### 2.2 What each tier owns

| Concern | Tier | Scope |
|---|---|---|
| Company Runtime: filesystem, Git, browser profile, project services | cell | company |
| OrgIntel: actors, goals, Work, Attempts, messages, context, schedules | cell | company |
| Company secret scope | cell | company |
| Authority record: grants, budget consumed, receipts for this company | cell reads, account plane writes | company |
| Provider credentials, OAuth, secret-backend machine identity | account plane | owner |
| Consequential effect execution and approval handling | account plane | owner |
| Budget enforcement and spend rollup | account plane | owner |
| Owner cockpit, CLI endpoint, owner↔company directory | account plane | owner |
| External ingress endpoint (email, payment webhooks) and inward routing | account plane | owner |
| Container/VM lifecycle, health, provisioning, upgrade | fleet | host or region |

Sharing a physical host does not weaken these boundaries. A service is shared only when it has a
smaller, auditable company-scoped contract and measured economics justify the operational coupling.

## 3. One architecture, two operating modes

Restless Core and Restless Cloud are not separate product architectures. They run the same three
tiers; only the fleet backend and the owner authenticator differ.

| Concern | Restless Core | Restless Cloud |
|---|---|---|
| Company cell | One local cell per company | One managed isolated cell per company |
| Account plane | One, for the local owner, supervised at login | One per owner, operated by Restless |
| Fleet tier | Thin local supervisor over Docker | Managed orchestrator across owners |
| Owner access | Local appliance entry point | Authenticated cloud account routed to that owner's plane |
| Public brand and research/results | Historical source/evidence only | One Cloud-owned public release surface, separate from every cell |
| Models | Owner's keys or local models, held by the account plane | Managed routing and metering, still per-owner |
| Secrets | Owner-operated Infisical/credential backend | Restless-operated secret service with per-cell scope |
| Runtime | Owner's machine or server | Managed container/VM/microVM as evidence requires |
| Backups/upgrades | Operator responsibility | Restless fleet operation |
| Company funds | Customer-owned accounts | Customer-owned accounts with scoped Authority access |

Cloud value is reliable operation, secure connection management, recovery, support and proven company
patterns. Core must remain a useful company appliance rather than a deliberately impaired edition.

Because the account plane is per-owner in both modes, Cloud's multi-owner fleet **never** holds a
universal credential capable of acting as every company. That property is structural, not a policy
promise.

### 3.1 Public Restless surface

Restless Cloud also owns the public landing page, product explanation and owner-authorised published
research/results. This static public surface is not an Owner Cockpit and is not a shortcut into a
company cell. It receives only explicitly public, versioned material through a separately governed
publication effect; it cannot read or mutate company OrgIntel, Runtime, browser or secret state.

The public-site source has transferred to the separate Cloud repository; Core retains historical
research evidence only. Public reachability proves a delivery effect, not customer demand or company
success. See [the Core/Cloud boundary pointer](specs/restless-cloud.md).

## 4. Lifecycle

Each tier has a different lifetime, and that is the point.

**Account plane.** Long-lived and supervised: a launchd agent on macOS, a systemd user service on
Linux, the container supervisor in Cloud. It starts at login and restarts on crash. It runs no
company work, so its restart must be a non-event for every running cell: capabilities are short-lived
and re-mintable, effects are idempotent, and reconnection reconciles unknown outcomes.

**Cell.** Tied to the company, never to a session or a turn. Created when the company is created,
running while the company is awake, stopped on request, destroyed only when the company is deleted.
A crash is repaired by the fleet restarting it; it recovers from its own database. It is explicitly
**not** a per-turn disposable sandbox (`ARCHITECTURE.md` §5, §12).

**Fleet.** The least stateful thing in the system, and preferably not resident at all — a supervisor
configuration plus a binary invoked on demand. Where it must be resident, it must be able to die and
re-derive its entire world from container labels plus the account plane's directory.

```text
owner creates company in the account plane
→ fleet provisions isolated cell identity, database and storage
→ cell initialises OrgIntel and boots the persistent Company Runtime
→ account plane routes owner sessions and effect requests to that exact cell
→ health and backup observed per cell
```

Suspend may stop compute while retaining durable state. Snapshot, restore, upgrade, quarantine and
destroy operate on one exact cell. Fleet automation coordinates these actions but does not become a
second writer of company state.

### 4.1 The invariant that tests whether the split is real

> **Each tier must be independently restartable without losing data or work in the others.**

Restarting the account plane must not stop a company. Restarting or losing a cell must not affect
another company or the owner's other surfaces. Restarting the fleet must not interrupt a running
cell. A design that cannot pass this test has fused tiers regardless of how many processes it runs.

Two corollaries worth stating, because their absence is how the tiers fuse in practice:

- **One company's configuration must never prevent another company from starting.** Credential
  resolution, model routing and config validation are per-cell concerns. The account plane validates
  its own credentials; a company that cannot resolve a model route is marked unstartable with a
  reason and the others proceed.
- **The account plane must never be started as a side effect of company work**, and starting it must
  never wake a cell.

## 5. Entry points: CLI, cockpit and waking

Both owner surfaces address cells **only through the account plane**, never directly. There is
exactly one endpoint an owner ever thinks about.

**Starting the plane is not waking a company.** The account plane is idempotent and inert: it holds
credentials but performs no work until asked. Waking a cell runs agents and spends money. These are
different acts and must not share a trigger.

- The account plane is registered with the platform supervisor at install time, so it is running
  before any owner surface is used. A CLI fallback may start it when no supervisor is registered —
  development machines, CI, a fresh clone — because starting it is free and side-effect-free.
- **No owner surface may auto-wake a cell.** `restless <verb> -c <company>` against a sleeping
  company reports that it is asleep and offers to wake it. Waking is always a deliberate act.
- The cockpit is served by the account plane, so a rendered cockpit is proof the plane is up. A
  development proxy that serves the shell while the plane is down is a development artifact and must
  fail loudly rather than render a disconnected shell.

### 5.1 The cockpit must be readable with every cell asleep

The cockpit carries projections, never source truth. The account plane retains each company's last
known state, so the owner can open the cockpit, see every company, and read recent outcomes,
attention items and spend **without waking anything**. Waking is one visible affordance.

Without this, an owner pays money to look at their own business, and the cost of a glance scales with
how many companies they run.

## 6. Data isolation

Each cell has **its own database and its own role**, not a schema inside a shared one. Schema
separation behind a single database role is a convention rather than a boundary: one connection can
read every schema, so a compromised or confused cell reads its neighbours. The same reasoning rejects
a shared ledger file with a company column.

Two details the implementation settled, both load-bearing:

- **The company name remains the schema name inside its own database.** Flattening every cell into
  `public` would be tidier and would break the wake path: the `LISTEN/NOTIFY` triggers derive the
  company from `TG_TABLE_SCHEMA`, so every wake would claim to come from a company called "public".
  Isolation comes from the database and role boundary; the schema name stays the company's identity.
- **`CONNECT` is revoked from `PUBLIC` on each cell database.** Postgres grants it by default, so
  without this any other cell's role could open its neighbour's database. This is what makes the
  boundary hold in the direction that matters — one compromised cell must not reach the others.

Because each cell's database is a host service rather than the cell's container, **the cockpit reads
a sleeping cell's state without waking it** (§5.1): OrgIntel is available while the Runtime is
absent.

Each company cell isolates:

- OrgIntel actors, goals, Work, messages and memory, in its own database with its own role;
- Runtime processes, filesystem, Git repositories, browser profile and project services;
- database identity and credentials;
- secret project/scope;
- its spend ledger, snapshots, restore points and destructive lifecycle operations;
- the authority record for that company.

The account plane isolates per owner: provider credentials, OAuth, secret-backend machine identity,
effect execution, approvals, and the owner-level spend rollup computed by reading up from cells.

The fleet tier holds only what it can re-derive: cell inventory, health and placement.

## 7. Provider and capability acquisition

External capability semantics are identical in Core and Cloud:

```text
missing outcome
→ internal versus external sourcing judgement in ordinary Work
→ owner/Authority grant where consequential
→ provider-native use in the cell, or a credentialed effect executed by the account plane
→ observed outcome and provider evaluation
```

Core owners install tools and provide scoped connections. Cloud may make those connections managed or
one-click, but a provider is never granted cross-company access and is not made part of the cell
merely because Restless operates the integration. Financial and legally consequential accounts remain
customer-owned initially; Restless brokers bounded access rather than holding operating funds.

Where two companies of one owner need **different** accounts with the same provider, the account
plane holds both and binds each to its cell. Provider custody is per-company-overridable; a single
shared credential set across an owner's companies is a default, not a constraint.

Provider packs and a catalogue may emerge after repeated integrations. They are not a prerequisite
for the cell architecture and do not alter its isolation boundary.

## 8. Failure and security posture

The cell is the default blast radius. The realistic threat is not hardware failure — it is an agent
that reads hostile content and is induced to act. A compromised or wedged cell must not expose
another company's filesystem, browser, credentials, Authority state or OrgIntel data, and must not be
able to spend beyond its budget, because it does not hold the credential that would let it.

A cell should be quarantinable, stoppable, restorable and upgradeable without stopping unrelated
companies.

The account plane is high-value infrastructure and should stay narrow: it executes effects, enforces
budgets and routes owner surfaces. It does not perform company work. The fleet tier is narrower
still: it may provision, route and operate cells; it holds no credential and executes no work.
Cross-cell analytics use explicit, minimised projections and consent rather than direct access to
company stores.

## 9. Deferred decisions

- exact cloud orchestrator and VM/container isolation mechanism;
- autosuspend and warm-start targets;
- cross-company benchmark data and consent model;
- multiple human roles inside one company;
- wallets, treasury custody, marketplace contracting and merchant-of-record responsibility;
- the scale point at which a shared stateless service is cheaper without weakening recovery or
  isolation.

Resolved by this document, and no longer open: the cell owns a dedicated database rather than a
shared installation with schema tenancy (§6); the Authority Plane is owner-scoped rather than
duplicated per cell (§2.1).

These choices follow managed deployment evidence. They must not fork Core and Cloud semantics or turn
speculative fleet machinery into a prerequisite for proving one useful company.
