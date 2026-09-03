# Sprint 37 — Turn published artifacts into prepared network experiences

**Status:** Draft for founder alignment; implementation not started

**Date:** 2 September 2026

**Companion:** [Cloud 15 — prepared service access](https://github.com/BlueprintLabIO/restless-cloud/blob/dev/docs/sprints/cloud-15-prepared-service-access.md)

**Depends on:** Sprint 36's released `published-service.v1` contract and Cloud 14's external
HTTPS/WebSocket and UDP evidence. This sprint must not paper over an unpassed Cloud 14 gate.

## Why this sprint exists

Sprint 36 established the right safety boundary: a company Runtime may produce an exact service
artifact, but the public workload runs beside the Runtime and receives neither its filesystem nor its
credentials. The next product gap is usability and transfer. A person should be able to open a demo,
join a game server or exercise another supported interactive artifact without learning provider ports,
copying bearer secrets, configuring a tunnel or asking an operator to keep a shell session alive.

The observed Cloud 14 work also showed why a generic “make this Runtime public” feature would be the
wrong abstraction. Provider-managed application networks can silently give workloads ambient network
access, build systems can inject runtime secrets into image construction, diagnostic domains are not
product identity, and HTTP success says nothing about UDP reachability. Those are provider mechanics
that need an explicit access fabric, not reasons to expose the company computer.

The missing outcome is a **prepared network experience**: the exact artifact is already running at a
Restless-owned endpoint; the intended person has one bounded way in; the native client is preconfigured;
and expiry or revocation removes access and infrastructure without returning setup work to the owner.

## Outcome

From an accepted Work artifact, the accountable lead can prepare one owner-review or invitee-review
target that another person can use from an independent network:

- a browser opens an owned HTTPS URL and upgrades to authenticated WebSocket where required; or
- a game client consumes an HTTPS join document, obtains a short-lived admission ticket and joins the
  authoritative UDP server without manually entering infrastructure details.

The endpoint survives Runtime and account-plane restarts within its grant, remains bound to the exact
artifact and audience, and disappears completely on expiry, revocation or owner stop. A third,
previously unseen HTTP/WebSocket artifact transfers through the same released path without Restless
gaining project-specific code.

This sprint productises Sprint 36. It does not create a second publication lifecycle, a public Runtime,
a general tunnel or an application-hosting platform.

## Product contract

### Publish services, not Runtimes

Every Company Runtime keeps private outbound-only control connectivity. It does **not** receive an
`sslip.io` hostname, wildcard route, public port range or general inbound listener.

Only an authorised, immutable publication receives a route. The public workload is an adjacent,
disposable service workload created from the exact image digest named by `published-service.v1`.
Stopping or replacing the Company Runtime does not silently replace that workload with a mutable
Runtime process.

### One endpoint namespace

Every ready publication receives a Restless-owned, certificate-valid service identity derived from
opaque publication identity rather than company or project secrets. The released shapes are:

```text
https://<opaque-publication>.preview.<restless-owned-domain>/
wss://<opaque-publication>.preview.<restless-owned-domain>/<declared-path>

join document:
https://<opaque-publication>.preview.<restless-owned-domain>/join
  → exact build digest
  → hostname + allocated UDP port
  → short-lived subject-bound admission ticket
  → native launch instructions or launcher payload
```

DNS wildcard, certificates, route allocation and public packet handling belong to Cloud/provider
infrastructure. `sslip.io`, raw node IPs and provider application IDs remain diagnostic evidence only.
The hostname is stable for the publication lifetime; it is not a permanent company or Runtime address.

### Access and invitation exchange

The account/Authority plane owns the decision that a named principal, invitee or public audience may
access an exact publication. It stores grant, expiry, revocation and consequential receipts. It does
not proxy the session payload or gameplay traffic.

Browser invitations exchange through HTTPS for a short-lived, HttpOnly access session; reusable bearer
capabilities do not remain in browser URLs, referrers or application logs. Native clients exchange a
bounded invitation through HTTPS for a short-lived ticket bound to publication, company, subject,
exact candidate digest, protocol and expiry. The gateway or authoritative server enforces that ticket
before application participation. Existing connections may continue only for the explicitly frozen
drain policy; new admission observes revocation within the acceptance window.

Owner membership is not automatically access to every publication, and an invite-only grant cannot be
reused for a public audience. Public access remains a separately authorised publication effect.

### Transport profiles

The fabric releases only evidence-backed profiles:

1. `https_websocket_demo` — HTTP(S), browser assets, APIs and WebSocket upgrade through one gateway.
2. `godot_enet_udp` — HTTPS discovery/admission plus one allocated UDP mapping to an authoritative
   server.

Any OCI application that obeys a released profile can use it; Restless does not inspect project
semantics. Static sites, dashboards, interactive demos and ordinary web APIs fit the first profile.
Realtime games fit the second when they implement the admission contract.

The reusable boundary is protocol and custody, not department or work type. A marketing team may
publish a campaign preview, a researcher an evidence explorer, a product team an interactive
prototype, and a game team an authoritative test server through the same publication lifecycle. The
company that produced the artifact changes; the access fabric, authority, isolation, observation and
cleanup contract does not.

| Produced artifact | Released path | Prepared human outcome |
| --- | --- | --- |
| Site, report, dashboard, API or interactive demo | `https_websocket_demo` | Open the useful HTTPS state; retain authenticated realtime interaction where declared |
| Authoritative multiplayer test server | `godot_enet_udp` | Open one HTTPS join action; launch the compatible client into the exact bounded server |
| Unsupported network shape | None | Remain private until a real workload proves a new bounded profile |

Arbitrary TCP, SSH, remote desktop, arbitrary port ranges, user-defined proxy configuration and raw
Runtime forwarding are not implied. A new transport profile requires a concrete workload, bounded
reachability, independent client proof, failure/cleanup semantics and an explicit contract revision.

### Runtime and provider responsibilities

The Runtime:

- builds an immutable OCI candidate and service-manifest digest;
- proves local health and protocol behaviour;
- requests publication outbound through the existing Bridge/Authority path; and
- receives a ready access descriptor for the linked `ReviewTarget`.

The provider access fabric:

- pulls only the exact approved image digest;
- creates one isolated internal network and one disposable workload per publication;
- attaches only the narrow protocol gateway to a public ingress network;
- denies workload egress unless the grant names and bounds it;
- applies connection, CPU, memory, PID, storage and lifetime ceilings;
- publishes readiness and usage observations; and
- reconciles and verifies complete teardown by publication identity.

It must not accept arbitrary commands, mounts, environment variables, labels, network names or
provider configuration from the Runtime. Runtime credentials may not become image build arguments,
service environment or logs. A deployed product requiring ordinary API access receives a distinct,
restricted service identity through the existing Authority resource path.

### Prepared last mile

OrgIntel links the ready access descriptor to the exact Work, Attempt, artifact and accountable lead.
The Cockpit presents the native target, audience, expiry and live/degraded state rather than a PaaS
dashboard. For a browser target it opens the useful page. For a game target it offers the compatible
build/launcher with server details already materialised. The owner supplies judgement, not DNS, ports,
tokens or launch arguments.

The access descriptor is a projection of the authoritative publication receipt, not a second mutable
service record.

### Availability, recovery and honest status

- A Runtime restart does not break an already-ready publication.
- An account-plane restart does not terminate established traffic; new invitation exchange may pause
  until authority is available.
- A gateway or workload restart restores the same publication endpoint and exact candidate, or reports
  degraded/failed. It never falls forward to `latest`.
- An ambiguous provider result reconciles by `publication_id` and idempotency key before another
  workload or route is created.
- Unknown connection, bandwidth, cost or cleanup observations remain unknown rather than zero.
- Expiry, revoke and stop re-observe absence of route, allocation, workload, network, invitation and
  temporary artifact before terminal success.

## Acceptance criteria

1. Cloud 14 first passes its frozen external HTTPS/WebSocket and two-client authoritative UDP cases;
   Sprint 37 does not redefine those failures away.
2. One exact web candidate receives a valid Restless-owned HTTPS/WSS endpoint. A browser on an
   independent network completes the native scenario without a bearer in the visible URL.
3. Two independent native game clients use the HTTPS join document and subject-bound tickets to join
   one authoritative UDP server and observe the same host-owned state.
4. A third, previously unseen HTTP/WebSocket application publishes through the same profile without
   changes to Core contract code, gateway code or provider supervisor code.
5. Expired, revoked, tampered, cross-company, wrong-subject, wrong-build and wrong-protocol admission
   fail. Public audience cannot reuse invite-only authority.
6. Runtime replacement, account-plane restart, gateway crash, workload crash and an ambiguous provider
   response preserve at most one live endpoint and exact candidate, with honest degraded state.
7. Only the declared profile port is reachable. The artifact workload has no public network, ambient
   egress, company mounts, Docker socket, provider root, build credentials or undeclared service
   identity.
8. Connection, CPU, memory, PID, storage, bandwidth where observable, lifetime and provider-spend
   ceilings are enforced or explicitly reported unsupported before publication.
9. Expiry and explicit stop re-observe zero routes, public port allocations, workloads, internal
   networks, invitations, service identities, leases and scoped temporary artifacts.
10. The prepared `ReviewTarget` gets a human from invitation to native use without provider knowledge,
    manual port entry, shell access or owner-authenticated Runtime attachment.
11. The final audit finds no Runtime-wide hostname, generic tunnel, arbitrary reverse proxy, mutable
    tag, provider-specific contract in Core, second invitation store or general hosting control panel.

## Slice per layer

**Authority / account plane.** Own exact audience authority, bounded publication/resource grant,
invitation exchange, short-lived access sessions, revocation, accounting and terminal receipts. It
does not carry application traffic or implement provider routing.

**OrgIntel.** Link Work and the exact artifact to the publication decision, access descriptor, native
review evidence and accountable judgement. It does not schedule packets, own invitations or become a
service catalogue.

**Company Runtime / Bridge.** Build and locally probe immutable candidates, submit outbound and
materialise the returned access descriptor. No public listener, infrastructure credential or
provider network authority enters the Runtime.

**Cloud/provider access fabric.** Own Restless preview DNS, certificates, public HTTP/WSS/UDP routing,
isolated service workloads, gateway enforcement, observations and mechanical teardown. It consumes
the released Core contract and does not reinterpret company authority.

**Cockpit.** Present one native prepared target, its audience, expiry and material health/cleanup debt.
It is not a deployment, DNS, token or port administration surface.

## Scope boundaries

In scope:

- productising the two Sprint 36 transport profiles;
- a Restless-owned preview namespace and valid automatic TLS;
- browser session exchange and native-client UDP admission;
- one portable access descriptor / join document;
- exact service workload isolation, limits, observation and teardown;
- Runtime/account/gateway/workload restart and ambiguous-result reconciliation; and
- two original dogfoods plus one unseen web transfer case.

Out of scope:

- public ingress to the Company Runtime, Bridge, OrgIntel, owner API, browser or filesystem;
- `sslip.io` as released identity;
- SSH, remote desktop, arbitrary TCP, arbitrary port forwarding or user-authored proxy rules;
- permanent production hosting, custom customer domains, durable databases or persistent service data;
- autoscaling, matchmaking, regional placement, migration, multi-region failover or DDoS product work;
- shared mutable application tenancy or a generic deployment/PaaS API; and
- treating reachability as evidence of quality, fun, demand or production readiness.

## Risk dispositions

| Risk | Disposition in this sprint |
| --- | --- |
| Company Runtime becomes internet reachable | **Invariant:** no supported path or provider route joins it to public ingress |
| Cross-company or wrong-build admission | **Invariant:** cryptographic scope and adversarial acceptance cases |
| Duplicate endpoint or unbounded spend after ambiguity | **Guarded:** idempotency, reconcile-before-create and hard ceilings |
| Gateway/account outage interrupts a test | **Guarded:** bounded tickets, reconnect and honest degraded state |
| Preview endpoint is abused within its grant | **Guarded:** invitation, connection/resource ceilings, expiry and stop |
| Provider or region outage loses an ephemeral demo | **Accepted:** no HA or multi-region in this slice |
| Native client UX differs by operating system | **Accepted until a real client fails the prepared launcher contract** |
| An unsupported protocol is needed | **Accepted:** add a profile only after the workload and proof exist |

## Failure and stop rules

Stop the affected branch for any public Runtime path, mutable artifact execution, credential/build-arg
leak, undeclared reachable port, workload ambient egress, cross-company admission, duplicate live
endpoint, public audience without exact authority, or inability to verify terminal absence.

One provider incompatibility may narrow the Cloud implementation. It must not broaden Core into a
provider catalogue or make a direct Coolify/application network part of the service contract. If the
owned-domain or public-UDP mechanics cannot pass, retain the diagnostic result and report the profile
blocked rather than substituting `sslip.io`, a local client or an HTTP-only simulation.

## Ticket index

| Status | Ticket | Outcome |
| --- | --- | --- |
| [ ] | [S37-T0](./sprint-37/t0-freeze-corpus-and-transfer.md) | Freeze the native access journeys, transfer case and adversarial corpus |
| [ ] | [S37-T1](./sprint-37/t1-owned-endpoints-and-descriptor.md) | Resolve every ready publication to one owned, portable access descriptor |
| [ ] | [S37-T2](./sprint-37/t2-access-exchange-and-admission.md) | Exchange human invitations for bounded browser sessions or native tickets |
| [ ] | [S37-T3](./sprint-37/t3-isolated-provider-access-fabric.md) | Provision exact isolated workloads and narrow public gateways |
| [ ] | [S37-T4](./sprint-37/t4-prepared-review-target.md) | Bring the working browser or native-client last mile to the reviewer |
| [ ] | [S37-T5](./sprint-37/t5-recovery-observation-and-cleanup.md) | Survive failure and prove limits, honest observations and terminal absence |
| [ ] | [S37-T6](./sprint-37/t6-external-dogfood-and-transfer.md) | Pass web, two-client game and unseen-application external dogfoods |
| [ ] | [S37-T7](./sprint-37/t7-purge-and-release-audit.md) | Purge diagnostic paths and release one documented canon |

Expected order: **T0 → T1/T2 → T3 → T4/T5 → T6 → T7**.

## Terminal decision

- **Pass:** all three external journeys work through owned endpoints, invitation/admission, exact
  workloads and prepared review targets; failure and teardown gates leave no hidden Runtime exposure or
  residue.
- **Revise once:** repair one bounded endpoint, ticket, gateway or provider defect and replay the
  affected profile plus teardown.
- **Profile blocked:** preserve the working profile and exact negative evidence for the other; do not
  generalise away its transport failure.
- **Stop negative:** if flexibility requires public Runtime ingress, arbitrary proxying, provider-root
  credentials in the cell or a second hosting/control plane, retain Sprint 36 and reject this shape.
