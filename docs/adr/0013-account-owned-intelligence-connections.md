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

Account OAuth grants now store the exact account connection ID in the company's reference, and the relay checks that the referenced connection is still registered on every model request. The Codex Responses relay forwards a granted `openai-codex` model to the account-held OMP gateway, so a Codex runtime can use that route without copying OAuth into its company volume. The account page lets the owner explicitly make a new grant the company's default, while preserving any existing native profile. A second Codex sign-in is refused while the account already has one registered connection, preventing an accidental account switch through this UI.

The company-only provider editor retains an existing OAuth reference but no longer creates a new one. New reusable OAuth access must be granted from the account page, where its connection ID and per-company revocation are visible.
When the broker supplies an email or account ID for its sole active OAuth credential, the account page shows that identity to the owner. The credential snapshot itself is never sent to the browser.

The account page can now start a Claude sign-in through the same host-owned broker. A local browser completes the loopback callback directly; a remote browser can paste its final callback URL into the account page, where Core verifies the callback origin and OAuth state before handing it to the broker. The Claude Agent model route uses the host gateway with a company-scoped capability, and the token-count route reads the current access token on the host. A real Claude sign-in and agent request have not yet been exercised, so this path must not replace an existing company sign-in automatically.

Each OAuth connection now records the provider's stable account ID (or email when no account ID exists). A same-account reconnect keeps the connection ID and its company grants. If the broker holds another account or no verifiable identity, the account page marks the connection unavailable and the model relay refuses the next request. Existing company-local CLI profiles are not changed by reconnecting the account.

Core now starts the account credential broker even when no company has an admitted model provider. This lets the first account sign-in complete and be verified before a company grant exists. If all configured credentials are unavailable, the account cockpit and broker stay available for repair while affected companies remain unable to start model work.

The gateway still admits only one credential per provider on an account plane; its broker has not yet been split by provider account identity. Existing native Codex and Claude CLI profiles remain company-local. The Cloud account handoff and two-tenant path need a live smoke before release.

On 25 September, the local owner explicitly imported an existing company Codex sign-in into the account broker. The original company profile was left in place and no company received access automatically. Two disposable companies were granted the exact connection; each returned a real Codex-backed Exec response. Removing the first grant caused its next Exec turn to fail with “This agent's account connection is no longer granted to the company,” while the second company returned another response. Neither disposable company's volume had a native Codex OAuth profile or provider credential environment variable. Both companies were archived after the smoke. The original company's Codex status still reported connected. Broker reloads after company changes briefly interrupted account status checks; the account page now presents that interval as “Checking sign-in” and refreshes automatically.

An agent can now explicitly select a granted account connection with the local Codex or Claude Agent runtime. The selected connection ID stays in the agent assignment; the company keeps its older native CLI profile for rollback, while the new runtime session sends model traffic through the account relay. New sessions reject a removed or replaced grant, and the relay checks it again on each model request. The chooser shows the account connection separately from the company-local sign-in. A disposable company received a real reply from the local Codex agent with this assignment and had no Codex OAuth profile in its volume. Revoking the grant stopped the next native-agent turn with an explicit “no longer granted” error. The disposable company was archived; the original company profile was left untouched. The first native turn encountered one upstream HTTP 400, then the durable wake retried and produced the reply. That transient refusal warrants follow-up before calling this path seamless. Claude still needs a live sign-in and request before its older profile can be considered migrated.

Hosted account owners can now read and change account-backed agent assignments. The Runtime Bridge has a Claude Agent ACP launch path: it prepares a separate, credential-free session profile in the company Runtime, injects only the short-lived model capability, and checks the adapter build, model, effort and permission mode before a prompt. The bridge also carries the first-party Codex runner's JSONL stream. Codex receives the same short-lived model capability, a scoped home for session continuity, and a bounded MCP environment; the bridge reaps the process group and checks the persistent home for capability residue. Core binds its saved Codex thread to the company, actor, responsibility, model, effort, MCP contract and exact account connection. Both hosted paths compile but still need real Cloud provider requests and revocation smokes. Copying an individual setting between hosted companies requires the account-owner session, which spans only that tenant's account plane. Model choices follow an exact account connection only when the destination company has the same grant. A company-scoped session cannot read another company's settings through the copy endpoint.

Cloud's portfolio now asks for an opening message instead of a company-name form. Fleet provisions an unnamed company and keeps that message with a stable command ID. On the founding owner's first secure entry, Core delivers it to Exec through the same idempotent conversation store as a normal owner message; a later entry only rechecks the receipt. This path needs a live provision-and-entry smoke before release.
