# ADR 0007 — Network owner entry by verified assertion

**Status:** Accepted

**Date:** 30 August 2026

**Fulfils:** The expiry condition stated in [ADR 0001](0001-local-owner-access.md) — "the first
Restless-supported owner entry point reachable across a network". It does not overturn ADR 0001; it
meets the condition ADR 0001 set for itself.

**Parent:** [`ARCHITECTURE.md`](../../ARCHITECTURE.md) §7.4 ·
[`docs/CELL_ARCHITECTURE.md`](../CELL_ARCHITECTURE.md) §2, §5

## Context

Core's account plane is loopback-only by construction. The owner gateway refuses a non-loopback bind
(`crates/restlessd/src/owner.rs:717`, "must remain loopback-only until network authentication exists")
and refuses any request carrying a forwarding header (`owner.rs:850`). ADR 0001 made this deliberate
and named what would end it: a supported network entry point shipping with a real account and session
boundary.

Restless Cloud has now specified the other half. Cloud proves a human session with Better Auth and
mints a short-lived assertion; the owner's account plane verifies it and serves the cockpit directly.
Cloud's [ADR 0001](https://github.com/BlueprintLabIO/restless-cloud/blob/main/docs/adr/0001-owner-plane-entry-and-routing.md)
records why the alternative — a shared multi-owner service reverse-proxying every company surface — is
refused: it would place company data in the tier defined as holding none, and would constitute exactly
the universal credential `CELL_ARCHITECTURE.md` §3 claims cannot structurally exist.

That leaves the verifying half unbuilt, and it is Core's. Every hosted Cloud surface is blocked behind
it: without assertion verification, a Cloud sign-in page gates nothing, and the only way to reach a
plane is to tunnel to a gateway that trusts network position.

## Decision

The account plane gains one additional supported entry mode: **network mode, in which access is
decided by verifying a signed identity assertion.** Loopback local mode is unchanged and remains the
default.

### What the plane verifies

An assertion is refused unless all hold: known issuer and key version; audience matching this exact
plane; unexpired and not-before satisfied; a supported assertion contract version; a route naming this
owner and plane; and an unused single-use identity. The assertion carries stable user identity, owner
and plane identity, company and cell scope, active membership role, mapped company actor, issue and
expiry times, audience, key version and correlation identity.

### What the plane refuses to treat as proof

A bare client-provided company ID, a membership role claimed by the client, a hostname, a forwarding
header, or the network the connection arrived from. **Company scope is re-derived from the verified
assertion on every request, never from the URL the browser was reached at.** ADR 0001's rejection of
forwarding headers as proof of locality generalises: no transport fact is an identity fact.

### One principal, one code path

Per ADR 0001's standing invariant, local and network authentication must not become parallel
owner-authority implementations. Both resolve to the same stable owner principal and run the same
application and Authority operations. Authentication only proves who may assume that principal; it
never becomes a second authorisation system, and it never grants an Authority capability. Membership
answers whether a human may enter; Authority separately answers what external consequence they may
cause.

### Consumed once

An assertion is consumed at entry, after which the plane establishes its own revocable session. A
replayed assertion must not create a second session. Removal of membership ends new entry promptly;
existing sessions end by ordinary session revocation, not by hoping the assertion expires.

### The deleted token does not return

Network mode ships with assertion verification or it does not ship. The Sprint 05 owner bearer token
that ADR 0001 removed is not reintroduced as a fallback, a development shortcut or an escape hatch.

## Risk dispositions

| Risk | Disposition | Reason |
|---|---|---|
| A stolen assertion is replayed to gain entry | **Guarded** | Single-use identity, short expiry and audience binding. Promote to invariant only if a real replay bypasses these. |
| An assertion issued for one company is used to read another on the same plane | **Invariant** | Scope is re-derived from the verified assertion per request. This is the boundary multiplayer rests on; it may not be weakened for convenience. |
| Cloud's issuer key is compromised | **Accepted for V1** | Key version is carried in the assertion and rotation is supported. A compromised issuer can mint entry to that owner's plane; it still cannot mint an Authority capability or reach another owner's plane. |
| Network mode is enabled on a plane whose deployment did not intend it | **Guarded** | Network mode requires explicit configuration naming issuer, audience and key; absent that configuration the plane stays loopback-only and refuses to start in network mode. |
| Local and network entry drift into two authority implementations | **Invariant** | Carried forward unchanged from ADR 0001. |
| An operator tunnels a loopback plane instead of enabling network mode | **Accepted** | Technically indistinguishable at the socket, as ADR 0001 recorded. Unsupported; the port must not be published. Enabling network mode is the supported path and is not harder. |
| Assertion verification expands into a general permission system | **Accepted** | Roles stay the three Cloud membership roles. Organisational role and Authority capability remain separately owned by OrgIntel and Authority. |

## Consequences

- `ensure_loopback` becomes conditional on entry mode rather than absolute, and the forwarding-header
  refusal stays in force for loopback mode. Removing either guard without the verification path in
  place would be a regression, not a simplification.
- Core must publish artifacts Cloud can pin: an account-plane image digest, a release manifest, and a
  health/version endpoint reporting the same release identity.
- Core gains an assertion contract version that is part of the release manifest and moves under the
  release contract's change governance.
- This ADR selects no identity vendor. Cloud runs Better Auth; Core verifies a signed assertion and
  would accept an equivalent issuer under the same contract.
