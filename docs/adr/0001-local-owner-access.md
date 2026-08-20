# ADR 0001 — Local owner access and future network authentication

**Status:** Accepted

**Date:** 18 August 2026

**Supersedes:** The generated one-owner credential in Sprint 05 T01 for the current product posture

## Context

Restless runs as a single-owner appliance on the owner's own computer. Before this decision, the
Sprint 05 implementation required a generated bearer credential, stored its digest on the host and
placed the original value in an HttpOnly cookie after sign-in even though the owner gateway defaulted
to loopback.

That mechanism proved a transport boundary for the Sprint 05 remote-owner experiment. It is not the
account system a hosted Restless service would need, and in routine local dogfood it creates recurring
setup, restart and recovery friction. Keeping it as a placeholder would preserve code and UI that do
not serve the current product and would still need replacement when real human identity arrives.

## Decision

The current Owner Cockpit is local-only and has no separate owner credential.

When every supported owner entry point is confined to loopback, the daemon treats the local operator
as the authenticated `owner` principal. All reads, writes, authority decisions and audit attribution
retain that explicit principal; local access is not anonymous access.

Local mode is the only supported mode until real network authentication exists. Every entry point
shipped and configured by Restless must bind to loopback. The daemon refuses non-loopback owner and
review listeners, non-local browser Host/Origin values, cross-site writes and requests carrying
forwarding headers; those headers do not prove locality.

An independently configured OS-level tunnel or proxy can make a remote client appear as a local
socket peer while preserving a localhost Host and Origin. The daemon cannot distinguish that traffic
from a genuine local client. Such exposure is outside local mode and is explicitly unsupported; the
operator must not publish the port. A future Restless-supported network entry point requires real
authentication before it ships.

The local browser boundary still rejects cross-site requests through strict allowed-origin and host
validation, same-origin browser APIs and no permissive CORS policy. Those checks address the relevant
web threat without retaining a human sign-in ceremony.

The historical Sprint 05 ticket and run report remain unchanged as evidence of the earlier remote
experiment. This ADR, `ARCHITECTURE.md` and the current specs are the new canon.

## Future network and hosted authentication

The expiry condition for local-only access is the first Restless-supported owner entry point reachable
across a network, including a private VPN, reverse proxy or managed deployment. That entry point must
ship with a real account and session boundary; the deleted owner token must not return as a fallback.

The future authenticator will prove a human account and map it to the stable owner principal used by
OrgIntel and Authority. Candidate capabilities, added only when demanded by real deployments, are:

- OpenID Connect over OAuth 2.0 Authorization Code with PKCE for delegated identity, social login or
  enterprise SSO;
- first-party username or email plus password, using a modern password hash, only if customers need
  native credentials;
- email verification, password reset and account-recovery flows;
- revocable and expiring server-side sessions, secure cookies, CSRF protection and logout/revocation;
- passkeys/WebAuthn, multifactor authentication and recovery factors;
- additional human principals, organisations and roles only when repeated use proves that one owner
  is insufficient.

OAuth 2.0 by itself delegates authorization; OpenID Connect or an equivalent identity protocol is
required when an OAuth provider is used to authenticate a human. This decision intentionally does not
select an identity vendor, provider catalogue, tenant model or role hierarchy.

## Risk dispositions

| Risk | Disposition | Reason |
|---|---|---|
| Another process running as the same OS user exercises owner authority | **Accepted** | The current appliance trusts the owner's operating-system account. The bearer token offered little protection from a process with the same user access. Expires when Restless supports untrusted local users or processes. |
| A hostile website targets a loopback owner API | **Guarded** | Strict origin/host validation, same-origin requests and no permissive CORS; promote only if a real browser attack bypasses these controls. |
| A Restless-configured cockpit listener is accidentally exposed to a network | **Invariant** | Owner and review listeners refuse non-loopback binds; non-local browser origins and forwarding claims are refused. |
| An operator adds an external tunnel or proxy that impersonates a local origin | **Accepted** | This is technically indistinguishable at the daemon socket and outside supported local mode. The port must not be published; the risk expires when Restless ships authenticated network access. |
| Hosted authentication expands into speculative tenancy and permissions | **Accepted** | Accounts, multiple humans, organisations and roles wait for a real network deployment and observed demand. |
| Local and hosted authentication become parallel owner-authority implementations | **Invariant** | Both produce the same stable owner principal and use the same application and Authority operations; authentication only proves who may assume that principal. |

## Consequences

- The owner-token generation command, sign-in endpoint, bearer cookie and sign-in UI are removed. Any
  obsolete stored digest from an earlier installation is inert and may be deleted.
- Local restarts no longer depend on recovering or rotating a credential.
- Remote access is deliberately unavailable until proper authentication is implemented.
- Future account work replaces the transport authenticator without changing owner-authority semantics
  or introducing another source of truth.
