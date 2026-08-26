# S16-T4 — Close the provider lane as explicitly unverified

**Layer:** Authority Plane + Company Runtime + Owner cockpit.

**Observed friction served:** Emerging-company research needs point-in-time market/reference data, but
Dogfood 1 had no authenticated data lane and public quote access later degraded under rate limiting.

## Outcome

The owner receives one prepared, provider-hosted choice for a bounded read-only data lane; Restless
does not claim that lane works until Authority observes an authenticated probe.

## Acceptance

- Prepare one handoff for the current Polygon U.S. Stocks candidate, naming the exact questions it
  should answer: point-in-time listing/reference data, adjusted daily aggregates and financial source
  metadata. Re-read current provider product, price, licensing and permitted-use terms in the handoff;
  do not hard-code stale pricing or assume commercial rights.
- Open provider signup, terms, identity and MFA only in the owner's normal browser. Do not materialise
  a provider-root session in the Company Runtime.
- Route any issued credential through authenticated Authority/Infisical ingress, never chat, files,
  logs or OrgIntel messages.
- Observe a successful scoped read-only probe against the promised endpoints before calling the lane
  live. A declined plan, unavailable endpoint or failed probe remains explicit and leaves no fake
  connection.
- Do not build a provider registry, signup wizard, generic OAuth layer or recurring-spend flow.

## Deletion target

Secret copy/paste, assumed provider connections and generic owner instructions in place of a prepared
last mile.

## Observed closure

**Disposition:** `unverified_provider`; no provider account, terms acceptance, credential ingress or
authenticated probe occurred.

Dogfood 2's current source bundle was decision-useful without a paid data connection. The frozen
public-only recovery Work therefore prohibited a second provider handoff and preserved this state in
its Runtime evidence manifest. Its owner review and after-action both say that the lane is
unverified; neither treats a configured secret, an owner click or a public fallback as a connection.

This is Sprint 16's permitted alternate exit, not a successful provider integration. The exact future
owner action remains recorded in [`dogfood-2.md`](../../scenarios/dogfood-2.md#prepared-provider-owner-moment): inspect
current Polygon terms in the owner's normal browser, use only Authority/Infisical ingress for an
issued key, then let Authority establish the lane through a scoped authenticated read-only probe. A
future Work must create that exact human-step handoff only if its outcome actually needs the lane.
