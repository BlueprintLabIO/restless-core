# Restless cell-based deployment architecture

**Status:** Target elaboration. `ARCHITECTURE.md` remains authoritative.  
**Parent:** `ARCHITECTURE.md` §7.4.  
**Decision:** Restless Cloud is multi-company at its control plane and managed-single-tenant at each
autonomous company boundary.

## 1. The unit of isolation is a company

A Restless company is a persistent, mutable computer operated by autonomous agents. Its tenant
boundary is therefore the company, not an owner account, browser session, agent or Work item.

```text
Restless Cloud control plane
  accounts · subscriptions · company directory
  provisioning · routing · fleet health · upgrades

        ├── company cell A
        │     Authority Plane · OrgIntel · Company Runtime
        │     database identity · volumes · browser · secret scope
        └── company cell B
              Authority Plane · OrgIntel · Company Runtime
              database identity · volumes · browser · secret scope
```

This is a cell-based SaaS architecture, also described as cell-per-tenant, managed single tenancy,
a silo tenancy model or per-tenant deployment stamps. “Cell” names the failure, recovery and
customisation boundary; it does not imply one physical server per company.

## 2. One architecture, two operating modes

Restless Core and Restless Cloud are not separate product architectures. They preserve the same
Authority, OrgIntel, Runtime, owner-surface and provider semantics.

| Concern | Restless Core | Restless Cloud |
|---|---|---|
| Company cell | One local/self-hosted deployment | One managed isolated deployment per company |
| Owner access | Local appliance entry point | Authenticated cloud account routed to the company cell |
| Public brand and research/results | Historical source/evidence only | One Cloud-owned public release surface, separate from every company cell |
| Models | Bring your own keys or local models | Managed routing, metering and optional BYO connection |
| Secrets | User-operated Infisical/credential backend | Restless-operated secret service with per-cell scope |
| Runtime | Owner's machine or server | Managed container/VM/microVM as evidence requires |
| Compute | Local or connected cloud account | Managed capacity with company-level limits and billing |
| Providers | Installed/configured by the operator | Managed connection and provider packs where proven |
| Backups/upgrades | Operator responsibility | Restless fleet operation |
| Company funds | Customer-owned accounts | Customer-owned accounts with scoped Authority access |

Cloud value is reliable operation, secure connection management, recovery, support and proven
company patterns. Core must remain a useful company appliance rather than a deliberately impaired
edition.

### 2.1 Public Restless surface

Restless Cloud also owns the public landing page, product explanation and owner-authorised published
research/results. This static public surface is not an Owner Cockpit and is not a shortcut into a
company cell. It receives only explicitly public, versioned material through a separately governed
publication effect; it cannot read or mutate company Authority, OrgIntel, Runtime, browser or secret
state.

The public-site source has transferred to the separate Cloud repository; Core retains historical
research evidence only. Public reachability proves a delivery effect, not customer demand or company
success. See [the Core/Cloud boundary pointer](specs/restless-cloud.md).

## 3. Shared control plane, isolated data plane

The Cloud control plane may share:

- human account authentication and sessions;
- subscriptions and Restless's own billing;
- company directory and request routing;
- provisioning and upgrade orchestration;
- fleet-level health metadata;
- container/VM orchestration;
- model gateway infrastructure with per-company accounting;
- object-storage and Postgres infrastructure when each cell has isolated credentials, namespace and
  independently restorable state.

Each company cell isolates:

- Authority grants, budgets, effects and receipts;
- OrgIntel actors, goals, Work, messages and memory;
- Runtime processes, filesystem, Git repositories, browser profile and project services;
- database identity and credentials;
- secret project/scope and provider tokens;
- snapshots, restore points and destructive lifecycle operations.

Sharing a physical cluster does not weaken these semantic boundaries. A service is shared only when
it has a smaller, auditable company-scoped contract and measured economics justify the operational
coupling.

## 4. Provisioning and lifecycle

```text
owner creates company in control plane
→ provision isolated cell identity and storage
→ initialise Authority and OrgIntel
→ boot persistent Company Runtime
→ route owner session to that exact cell
→ observe health and backup independently
```

Suspend may stop compute while retaining durable state. Snapshot, restore, upgrade, quarantine and
destroy operate on one exact cell. Fleet automation coordinates these actions but does not become a
second writer of company state.

Early Cloud may use one Compose stack, VM or cluster namespace per company. Later revisions may pack
many cells onto one host and share selected stateless services. Full table-level shared tenancy for
Authority, OrgIntel or mutable Runtime state is not the default destination; it must be justified by
measured cost or scale and must preserve independent restore and blast-radius properties.

## 5. Provider and capability acquisition

External capability semantics are identical in Core and Cloud:

```text
missing outcome
→ internal versus external sourcing judgement in ordinary Work
→ owner/Authority grant where consequential
→ provider-native use in Runtime or a host-side adapter
→ observed outcome and provider evaluation
```

Core users install tools and provide scoped connections. Cloud may make those connections managed or
one-click, but a provider is never granted cross-company access and is not made part of the company
cell merely because Restless operates the integration. Financial and legally consequential accounts
remain customer-owned initially; Restless brokers bounded access rather than holding operating funds.

Provider packs and a catalogue may emerge after repeated integrations. They are not a prerequisite
for the cell architecture and do not alter its isolation boundary.

## 6. Failure and security posture

The cell is the default blast radius. A compromised or wedged Runtime must not expose another
company's filesystem, browser, credentials, Authority state or OrgIntel data. A cell should be
quarantinable, stoppable, restorable and upgradeable without stopping unrelated companies.

The shared control plane remains high-value infrastructure and should stay narrow. It may provision,
route and operate cells; it does not execute company Work or hold a universal credential capable of
acting as every company. Cross-cell analytics use explicit, minimised projections and consent rather
than direct access to company stores.

## 7. Deferred decisions

- exact cloud orchestrator and VM/container isolation mechanism;
- whether Postgres and Infisical are shared installations with isolated tenants or dedicated per cell;
- autosuspend and warm-start targets;
- cross-company benchmark data and consent model;
- multiple human roles inside one company;
- wallets, treasury custody, marketplace contracting and merchant-of-record responsibility;
- the scale point at which a shared service is cheaper without weakening recovery or isolation.

These choices follow managed deployment evidence. They must not fork Core and Cloud semantics or
turn speculative fleet machinery into a prerequisite for proving one useful company.
