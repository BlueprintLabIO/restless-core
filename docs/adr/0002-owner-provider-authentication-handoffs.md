# ADR 0002 — Owner-only provider enrolment and authentication handoffs

**Status:** Accepted

**Date:** 20 August 2026

## Context

A self-running company must acquire external accounts and capabilities. Signup, account connection,
identity or business verification, CAPTCHA, MFA, legal attestation, initial credential issuance and
provider-native approval can require the owner personally even when the Exec has prepared everything
around them.

The Company Runtime also contains a persistent browser that agents can use and the owner can attach
to. That is appropriate for ordinary company work and bounded provider identities. It is the wrong
place to establish a banking, treasury, identity-verification or provider-root administration
session: cookies and session authority would remain inside the agent-accessible company environment.

There are two adjacent modelling temptations to avoid. Restless could create a separate onboarding
workflow for these steps, or it could treat an owner click as proof that the provider connection now
works. The first duplicates Work, owner handoffs and Authority connection state. The second turns an
unobserved claim into evidence.

## Decision

Provider enrolment and authentication that requires the owner is presented as an ordinary prepared
last-mile **Attention item**. The item includes the exact provider and destination, requested scope,
why access is needed, what the company can do afterward, what happens if the owner declines, and the
condition Restless will observe before resuming.

For provider-root administration, financial accounts, identity or business verification, MFA and
initial credential issuance, the primary action opens the provider-hosted flow in the owner's normal
system browser. Restless-owned flows must not embed that authentication page or intentionally
materialise its privileged cookies in the persistent Company Runtime browser. A future prepared
owner browser is acceptable only if it is a separate ephemeral profile outside the Company Runtime,
unavailable to agents, and discarded after the handoff. Ordinary low-risk provider identities may
still live in the Runtime browser.

Provider passwords, MFA factors, recovery factors and identity evidence remain between the owner and
the provider. When a provider issues an API credential, the owner supplies it through a dedicated
authenticated secret ingress that delivers it to Infisical or another Authority-owned credential
backend. Raw secrets must not pass through Exec or Staff chat, OrgIntel messages, runtime files,
ordinary form telemetry or logs. OAuth callbacks and token exchange follow the same ownership rule:
the Authority credential integration stores or applies the resulting token, while company processes
receive only the bounded access path they were granted.

The owner opening the provider page, returning to Restless, or pressing a generic “done” action is not
evidence that a connection works. Resolution comes from a provider callback, an authenticated live
probe, provider reconciliation or another observable external condition. If the provider offers no
observable check, Restless reports the state as unverified; an explicit owner attestation may unblock
work but must not be presented as a verified connection.

Authentication, credential acquisition and effect approval remain separate boundaries. Signing in
or connecting an account does not approve a payment, contract, publication or other consequential
effect. Provider-native approval may itself become a later prepared owner handoff.

No onboarding entity or lifecycle is introduced:

- OrgIntel owns the capability-acquisition Work and any `owner_handoff`;
- the provider owns its account, verification and authentication state;
- the Authority Plane owns connection observations, grants, credential references and effects;
- Attention is a read-only projection of the currently required owner action and its immediate
  causal continuation.

This decision is provider-neutral. A provider that cannot offer a usable access path may remain a
manual owner-operated capability or lose a build/buy comparison to another provider; that does not
justify weakening the browser or credential boundary.

## Risk dispositions

| Risk | Disposition | Reason |
|---|---|---|
| A Restless-owned enrolment flow leaves a financial or provider-root session accessible to agents | **Invariant** | These flows open outside the Company Runtime; Restless does not import their cookies or profile. |
| A raw provider secret enters chat, OrgIntel, runtime files, telemetry or logs | **Invariant** | Credential material has one dedicated owner-to-Authority ingress and is stored or applied by the credential backend. |
| A return click is mistaken for a working connection | **Guarded** | Connection state requires an authenticated observation and otherwise remains explicitly unverified. |
| The owner's operating system or normal browser is compromised | **Accepted** | The local product already trusts the owner's OS account; Restless does not claim to replace endpoint security. |
| Leaving the cockpit adds friction or loses provider page state | **Accepted** | Rejoin through the callback or live probe; the reduced convenience is proportionate to the session authority. Promote if dogfood repeatedly prevents completion. |
| A provider has no suitable API, OAuth grant or scoped credential | **Accepted** | Keep it manual, use a bounded browser effect where appropriate, or choose another provider. Do not invent access the provider does not supply. |
| Attention becomes a second onboarding state machine | **Invariant** | The item remains a projection over source-owned Work, handoff and Authority/provider state. |

## Consequences

- “Connect provider”, signup, KYB, MFA and provider-native approval can use one prepared Attention
  interaction without becoming a new product area.
- Financial and provider-root sessions do not become ambient Company Runtime capabilities merely
  because agents have a browser.
- The Owner Cockpit needs an external-open action and, where API secrets are unavoidable, a narrow
  owner-authenticated secret ingress; it does not need to render provider authentication itself.
- Completion and continuation are driven by observed provider state, not an owner-maintained
  checklist.
- Provider selection remains an empirical capability-sourcing decision based on usable access,
  scope, price and observed outcomes.

