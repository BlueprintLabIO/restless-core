# ADR 0009 — Provider-neutral company access context

**Status:** Accepted; supersedes the vendor-shaped claim details in
[ADR 0007](0007-network-owner-entry-by-verified-assertion.md)

**Date:** 8 September 2026

**Parent:** [`ARCHITECTURE.md`](../../ARCHITECTURE.md) §7.4

## Context

Core must be multiplayer-capable when self-hosted and when deployed through Restless Cloud. The
historical network verifier uses a shared HMAC and owner-shaped claims; Cloud issues Ed25519/JWKS
handoffs containing Better Auth membership coordinates. Neither vendor-specific membership storage
nor a client-selected Actor belongs in Core.

## Decision

Core defines a versioned `CompanyAccessContext`: a signed, provider-neutral statement of issuer,
principal, account-plane owner, immutable company and cell identity, membership identity/version/role,
audience, issue/expiry, single-use identity and correlation. Cloud/Better Auth and a self-hosted
identity adapter may issue equivalent contexts. Core verifies the signature and scope, consumes the
handoff once, and maps the proven principal to one durable company Actor. The browser cannot nominate
that Actor.

The handoff establishes a short-lived, revocable account-plane session. Every company command,
query, SSE stream, desktop connection and document collaboration session derives its principal from
that verified session. Membership answers entry only; organisational role and Authority capability
are separate facts. Runtime Bridge may transport Core-issued agent session credentials but cannot
mint identity or permission.

Hosted bootstrap allocates the immutable `company_id`; local Core bootstrap allocates the same
semantic identifier. Authority records and protects it but does not create a competing ID.

## Consequences

- Replace shared HMAC verification with asymmetric verification through a published JWKS contract.
- Propagate the verified principal through request extensions and audit all owner-shaped handlers.
- Store replay/session and membership-version state durably enough to enforce the declared revocation
  bound across process restarts.
- Keep loopback local entry as the first self-hosted adapter; adding another self-hosted provider does
  not change company APIs.
- Membership-owner transfer never silently transfers root Authority ownership.

## Invariants

1. A client cannot select another Actor or company by changing a request body or path.
2. A context for one issuer, audience, plane, company, membership version or expiry cannot enter
   another.
3. Replaying a consumed handoff cannot create another session.
4. Revocation preserves historical attribution while ending future access within the declared bound.
