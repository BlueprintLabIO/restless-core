# Sprint 45 — Trusted human entry and durable attribution

**Status:** Draft for founder alignment
**Programme:** [Core company collaboration](company-collaboration-programme.md)
**Paired Cloud sprint:** Cloud 16
**Depends on:** current Core account-plane entry and released identity contract

## Outcome

A hosted or self-hosted human enters one company through a provider-neutral signed access contract,
is mapped by Core to one durable human Actor, and performs an ordinary Core operation with truthful
attribution. Membership never grants root Authority, the client never selects its Actor, and removed
access is denied without erasing history.

This sprint also records the owner-approved architectural trigger that supersedes the old “multiplayer
deferred” posture. No invited-member surface may ship before request principal propagation is complete.

## Scope

- Update architecture/ADR/spec canon for multiplayer activation, bootstrap allocation of immutable
  `company_id`, and membership-owner versus Authority-owner separation.
- Define and publish a versioned `CompanyAccessContext` independent of Better Auth. Use asymmetric
  signatures/JWKS for network entry; keep the local adapter operational without Cloud.
- Distinguish Fleet's single-use browser handoff from the short-lived account-plane/company session.
- Thread a verified request principal through every company command, query and stream authorization
  path; remove literal `owner` attribution and implicit owner-shaped member access.
- Map `{issuer, subject, company_id}` to a durable Core Actor while validating current membership ID,
  role/version, audience, scope and expiry.
- Begin the compatibility migration from role-shaped Actor kinds (`owner`, `exec`, `staff`, `system`)
  to Actor class (`human`, `agent`, `service`) plus separate organisational roles.
- Make membership administration and root Authority ownership separately inspectable and transferable.
- Define the Core-issued, operation-scoped agent session credential. Runtime Bridge transports it but
  cannot mint identity or permission.

## Acceptance

1. A Cloud fixture and a local identity fixture enter the same Core API and produce equivalent Actor
   attribution and permissions.
2. Wrong issuer, audience, company, expiry, signature, membership version and replayed entry assertions
   fail without creating a session or Actor.
3. A member cannot supply another `actor_id`, call an owner-only membership action, or gain a root
   Authority capability.
4. A removed membership loses new HTTP/SSE/session renewal access within the declared revocation bound;
   its historical messages and decisions retain Actor attribution.
5. Hosted and local bootstrap retries reuse one `company_id`; Authority and Fleet/Core references agree.
6. Membership-owner transfer alone does not change Authority owner; each boundary requires an explicit,
   attributed operation and presents the combined case when both are intended.
7. Existing owner/Exec/Staff/system rows migrate or project deterministically with no identity collision.

## Ticket outline

- [ ] C45-T0 — canon and compatibility decision
- [ ] C45-T1 — access-context schema, key rotation and negative corpus
- [ ] C45-T2 — request-principal propagation and handler audit
- [ ] C45-T3 — durable human Actor mapping and actor-class migration
- [ ] C45-T4 — ownership/company bootstrap separation
- [ ] C45-T5 — agent-session issuer boundary
- [ ] C45-T6 — self-hosted and hosted end-to-end proof

## Deletion and exclusions

Delete the HMAC-only network assertion, hard-coded owner authorship, handlers that discard verified
role/Actor claims, and any Core dependency on Better Auth storage. Do not build general SSO, nested ACL
policy, SCIM, guest access, or a second membership database.
