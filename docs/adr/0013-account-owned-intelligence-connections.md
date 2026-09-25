# ADR 0013 — Sign in once, grant intelligence to companies

**Status:** Product decision; execution work remains open

**Date:** 25 September 2026

## The intended experience

An owner signs in to an intelligence provider once in Account → Connections. The account shows the verified provider identity, available models or agent runtime, and every company with access. Newly created companies receive no access by default. The owner grants or removes one connection for one company at a time. A company can then choose that connection as its default or assign it to an agent. The company never signs in again merely because another company already uses the same provider.

In Cloud, the connection belongs to one authenticated tenant's account plane. That tenant may have any number of companies. A connection or grant from another tenant must be unreachable, even if both tenants use the same provider and email address. Core uses the same model with one local owner.

The account page must be the source of truth for these grants. Company → Intelligence provider may show a granted connection and offer a link back to account settings, but it must not create a second copy of the login.

## Boundary

The account plane owns the provider login, refresh credential, and provider account identity. The company stores a connection ID and its own model/agent selection, not the OAuth profile, refresh token, provider cookie, or account-vault secret. Model requests are bound to the authenticated tenant, connection ID, company ID, agent/session, and current grant. The broker checks the grant when it serves each request, so removing access blocks the next request, including from an existing agent session. Stopping a company is not required to revoke access.

This is also the rule for account-owned API keys. An API key and an OAuth login differ in how the account plane obtains and refreshes the provider credential; they should have the same company-grant experience.

One tenant may connect more than one account at the same provider. Grants refer to the exact connection ID rather than only the provider name. Changing a company's model must not silently switch its provider account.

## Native agent runtimes

Bringing the Codex or Claude agent runtime into a company is distinct from putting the owner's subscription login inside that company. The runtime may run with a company-scoped model capability if its model traffic can go through the account broker. A vendor-specific subscription path is offered only when the vendor provides a supported way to keep the reusable credential and refresh in the account plane and to enforce the company grant on use. An account access-token copy, a shared writable CLI home, or a one-time check before agent launch does not meet this decision.

The current native Codex and Claude sign-ins are exceptions: each CLI stores OAuth in its company volume. The current host model-broker OAuth connection is another mechanism and does not authenticate those native CLI profiles. The account page currently lists company sign-ins but cannot grant them to another company. This is not the intended end state.

Existing company sign-ins stay intact until an account-owned route is proven. Migration should use an explicit account action, verify the account identity and a real model request, then switch a selected company to the new connection. Do not log out or delete the old profile merely because a connection row was created.

## Minimum usable slice

1. Account → Connections starts and completes a real provider login, confirms the provider identity, and shows current status without leaking tokens.
2. Account → Connections grants and revokes an exact connection for selected companies. A new company appears with access off.
3. Company → Intelligence provider lists only its granted account connections, their available runtimes/models, and its current default and agent assignments.
4. The model gateway or native-runtime adapter checks tenant, company, connection, grant, and model at use time. Revocation blocks the next request. A running session receives an explicit connection-revoked outcome rather than silently falling back.
5. A local live smoke uses one real sign-in for two disposable companies, proves both can use it, revokes one while leaving the other working, and verifies that no reusable credential entered either company volume or process environment. Cloud repeats the same path with two tenants to prove isolation.

Until all five are met for a provider/runtime, the UI should describe the available connection type precisely and must not label a company-local sign-in as shareable account OAuth.

## Implementation checkpoint

The local account page now starts a real ChatGPT/Codex device login through the host model broker profile. The authorization URL and one-time code are shown in the account page, and a successful login registers a reusable account connection. The broker can start for an account connection before any company has a grant. Company access still uses the existing explicit grant and revocation controls.

The Cloud account page now signs a distinct account-owner handoff after matching the signed-in Better Auth user to Fleet's owner record. Core verifies its own token type and audience, consumes it once, and gives that owner a short account session. Company handoffs continue to show only their granted connections and cannot create or change account grants.

This remains short of the decision above. The gateway still admits only one credential per provider on an account plane, and OAuth company settings still resolve by provider rather than exact connection ID. Existing native Codex and Claude CLI profiles remain company-local. The Cloud account handoff and the two-company/two-tenant paths need a live smoke before release. Cloud also still has its legacy separate company-creation form; chat-led creation must be integrated with Fleet provisioning before it can be removed.
