# Sprint 36 — Make exact artifacts publishable as bounded services

**Status:** Core implemented and verified; Cloud 14 public-hosting gate open

**Date:** 1 September 2026

**Companion:** [Cloud 14 — bounded published services](https://github.com/BlueprintLabIO/restless-cloud/blob/main/docs/sprints/cloud-14-bounded-published-services.md)

**Depends on:** Sprint 27's verified network identity/release contract and Sprint 30's exact artifact,
accounting and terminal-cleanup substrate. Dogfood 4 may continue local improvement while this sprint
runs; only its remote human acceptance depends on the completed Core and Cloud slices.

## Why this sprint exists

Restless can build useful software inside a powerful company Runtime, but a person outside that
Runtime cannot safely experience a generated demo or join a test game without bespoke port forwarding,
manual provider operation or exposure of the company computer itself. Giving every Runtime a public
URL would turn source, browser sessions, model processes and company credentials into an internet
attack surface. Treating every target as a bespoke deployment would make continuous product work stop
at the most economically important boundary: another person using the result.

The missing Core primitive is not hosting. It is a narrow, attributable request to publish one exact
artifact as one bounded service. Cloud owns public DNS, ingress and isolated workloads; Core owns the
artifact identity, company authority, request, invitation semantics, evidence and reconciliation.

## Outcome

From ordinary Work, a Runtime produces an immutable service candidate and requests one of two released
profiles:

- an invite-only HTTPS/WebSocket interactive demo; or
- an invite-only Godot ENet/UDP test server.

The accountable owner can inspect and authorise the exact candidate, audience, lifetime and resource
envelope. Core hands the request to a provider adapter without receiving provider-root credentials,
records the ready or failed receipt, issues or revokes scoped invitations, observes health and usage,
and reconciles terminal cleanup. The company Runtime itself remains unreachable from the public
network.

The Cloud companion must then prove that two external clients can use the exact released services and
that expiry removes every route, workload and invitation. Passing the Core fixture alone is not a claim
that public hosting works.

## Product contract

### One published service

A publication request binds:

- stable `publication_id`, `company_id`, source Work and accountable actor;
- immutable OCI image digest plus service-manifest digest;
- released profile: `https_websocket_demo` or `godot_enet_udp`;
- one declared internal port and the protocol-specific health/readiness contract;
- audience: owner-only, named invitees or explicitly public;
- start deadline, expiry, connection ceiling, CPU, memory and storage envelope;
- egress posture and absence of company/provider credentials;
- idempotency key, provider operation and lifecycle receipts; and
- exact endpoint and invitation-verification key identity once ready.

Paths, mutable tags, ambient Runtime ports and “latest” are invalid artifact identities. A candidate is
not reachable merely because it built successfully.

### Publication authority

Invite-only test publication is a consequential effect with a bounded resource grant. Explicitly
public publication additionally requires the existing owner publication authority; this sprint does
not infer public consent from a test grant. Renewal, audience widening, resource expansion and expiry
extension are new decisions rather than mutations hidden inside an old receipt.

The account plane authorises and performs the effect where the owner-scoped credential lives. Fleet
does not become a company data path or a second writer of company truth. The Runtime receives neither
DNS, ingress, registry-root, host-supervisor nor cloud-provider credentials.

### Runtime boundary

The Runtime may build, test and submit a service candidate through its outbound Bridge. It cannot:

- open an inbound public listener on the company container;
- publish an arbitrary mutable Runtime directory or process;
- request undeclared ports or protocols;
- mount source, browser state, model homes or company secrets into the service workload; or
- mark a service ready or cleaned using its own assertion.

The service runs from the exact image in an adjacent disposable workload. For the accepted profiles it
has an ephemeral writable layer, no persistent company volume and no outbound egress unless the grant
names a bounded need. Durable hosted applications and production databases remain a later decision.

### Invitation contract

An invitation is a signed, expiring capability for one publication and audience. It contains no owner
session or provider secret. The client-facing join document may resolve to HTTPS/WSS or to a hostname,
UDP port, build digest and game-specific launch ticket. Revocation, publication termination or build
supersession makes the invitation unusable.

The game server remains authoritative for player, vehicle, cargo, combat and mission state. An
invitation proves admission, not gameplay authority.

### Observation and termination

Core records provider-observed readiness, endpoint identity, connection counts, bounded health/crash
observations, spend/resource usage and terminal cleanup. Unknown measurements remain unknown. Runtime
restore, daemon restart or a lost provider response reconciles by `publication_id` and idempotency key;
it never creates a duplicate endpoint.

Expiry, revocation and owner stop drain or terminate the service according to the frozen profile. A
terminal success requires re-observed absence of endpoint, workload, invitations, leases and temporary
artifacts—not merely a delete request.

## Acceptance criteria

1. An exact image and manifest produce one idempotent publication request; mutable references and
   digest mismatches are refused before provider spend.
2. The owner sees artifact, audience, protocol, expiry and resource consequences before granting the
   effect. Public audience cannot reuse an invite-only grant.
3. No supported path makes the company Runtime, Bridge, OrgIntel, owner API or filesystem publicly
   reachable.
4. The released local provider proves HTTPS/WebSocket and ENet/UDP manifests without special-case
   company or Swift Arrival code.
5. Invitations are publication-scoped, signed, expiring and revocable; cross-company, expired,
   superseded, tampered and out-of-scope replay cases fail. An unchanged bearer remains usable by its
   named subject until expiry or revocation so a live service can reconnect; Cloud must bind that
   subject to authenticated client identity before public admission.
6. Provider failure, process death, daemon restart, Runtime replacement and ambiguous terminal receipt
   reconcile without duplicate endpoints, double accounting or lost cleanup obligations.
7. The service workload has no company mounts, Runtime credentials, provider roots or undeclared
   egress; only the declared port is reachable in the fixture.
8. Terminal cleanup re-observes zero service processes, routes, invitations, resource leases and scoped
   temporary artifacts.
9. Cloud 14 consumes the released contract without vendoring Core semantics and proves one interactive
   web demo plus two external Swift Arrival clients on an authoritative UDP server.
10. The final audit finds no Runtime-wide public URL, generic tunnel, arbitrary port-forward API,
    deployment workflow engine, Kubernetes dependency or second publication authority system.

## Slice per layer

**Authority Plane.** Own effect and resource admission, idempotency, audience/expiry changes, provider
operation and terminal reconciliation. It does not implement DNS, TLS or packet routing.

**OrgIntel.** Link Work, candidate, decision, publication, observations and cleanup evidence. It does
not become a deployment database, ingress controller or generic service catalogue.

**Runtime.** Build and locally verify immutable candidates, then submit through the outbound Bridge.
It receives endpoint and observation results but no public inbound route or infrastructure credential.

**Account plane / authentication.** Prove the human decision and issue narrowly scoped invitations.
Membership and owner sessions do not automatically become service access.

**Cloud provider adapter.** Outside Core, consume the released request/receipt contract. Cloud 14 owns
isolated workload, registry, DNS/TLS, HTTP/WebSocket ingress and UDP allocation.

**Cockpit.** Show consequential pending publication, exact active services and material failures or
cleanup debt. It is not a PaaS dashboard, terminal, DNS editor or log warehouse.

## Scope boundaries

In scope:

- one immutable container artifact format;
- HTTPS/WebSocket and Godot ENet/UDP profiles;
- owner-only, invite-only and separately authorised public audiences;
- signed temporary access, bounded observation and verified teardown;
- local deterministic provider fixture and exact Cloud contract tests; and
- a web-demo transfer case plus Swift Arrival remote multiplayer dogfood.

Out of scope:

- exposing an entire Runtime or assigning every Runtime a public hostname;
- arbitrary TCP tunnels, SSH, remote desktop or arbitrary user-defined port ranges;
- durable databases, production persistence, custom domains or permanent public services;
- autoscaling, matchmaking, regional placement, migration or multi-region failover;
- general CI/CD, registry, PaaS, billing or application-management products;
- `sslip.io` as a service identity or production dependency; and
- claims that reachability establishes demand, quality, security or production readiness.

## Failure and stop rules

Stop the affected branch for any Runtime-public path, mutable artifact execution, credential or mount
leak, undeclared reachable port, cross-company invitation acceptance, duplicate live endpoint after
recovery, inability to prove terminal absence, or public audience without the exact owner effect.

One provider-specific incompatibility may narrow the Cloud adapter. It must not broaden Core into a
provider abstraction framework before the two released profiles pass. A failed game build is product
evidence; a failure to route valid UDP is publication-infrastructure evidence.

## Ticket index

| Status | Ticket | Outcome |
| --- | --- | --- |
| [x] | [S36-T0](./sprint-36/t0-corpus-and-contract.md) | Freeze valid and adversarial publication cases plus the Core/Cloud contract |
| [x] | [S36-T1](./sprint-36/t1-service-candidate.md) | Produce one immutable, locally verified service candidate |
| [x] | [S36-T2](./sprint-36/t2-authority-and-accounting.md) | Authorise exact audience/resources and reconcile one provider operation |
| [x] | [S36-T3](./sprint-36/t3-runtime-handoff.md) | Submit candidates outbound without public Runtime ingress or provider custody |
| [x] | [S36-T4](./sprint-36/t4-invitations-and-observation.md) | Issue scoped access and retain bounded provider observations |
| [x] | [S36-T5](./sprint-36/t5-recovery-and-cleanup.md) | Survive ambiguity/restart and verify terminal absence |
| [x] | [S36-T6](./sprint-36/t6-provider-contract-dogfood.md) | Prove both profiles locally and release the contract to Cloud 14 |

Expected order: **T0 → T1 → T2/T3 → T4/T5 → T6**.

Core evidence and the deliberately unclaimed Cloud gates are recorded in
[`sprint-36/RESULTS.md`](./sprint-36/RESULTS.md).

## Terminal decision

- **Pass:** all Core gates pass and Cloud 14 proves both external service profiles from the released
  contract with zero Runtime exposure or terminal residue.
- **Core pass / Cloud blocked:** preserve the released contract and local provider evidence without
  claiming hosted service capability.
- **Revise once:** repair one bounded identity, invitation, protocol or provider-contract defect and
  replay the affected profile plus teardown.
- **Stop negative:** if useful publication requires ambient Runtime ingress, mutable artifacts or
  provider-root credentials in the cell, retain the evidence and reject this shape.
