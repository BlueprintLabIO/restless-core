# ADR 0017 — Provider webhooks reach a company through one door on its account plane

**Status:** Accepted

**Date:** 4 November 2026

## Context

A company has to hear from the outside world: a customer answering its email, a payout settling,
later a GitHub push, a Stripe payment or a form submission. Core already verified Resend's Svix
signatures (`ingress.rs`, `POST /inbound/<company>` on port 7792) and Airwallex's own HMAC
(`airwallex_ingress.rs`, port 7795). Neither port is published by a hosted plane, and the plane's
only public door is the owner gateway behind the owner-browser boundary. In production,
`restless.run/inbound/<company>` was read as a company key (404) and the plane's own host refused
the request as `network_owner_boundary` (403). No provider could reach a company, and the company
name in the old path told anyone who saw the URL which company it was.

## Decision

Providers call **`POST /hooks/<provider>/<token>`**, served by the owner gateway on the account plane
(`owner_hooks.rs`, `restless_engine::hooks`).

- **The token routes; the signature authenticates.** Each company has one token per provider in
  `restless_authority.hook_endpoints`, 256 random bits, minted on first use and never reused. It
  names no company. On a Cloud plane it starts with the plane id (`p<32 hex>-<64 hex>`), which is
  already public in the plane's own hostname, so the edge router can route without asking anyone
  about the token. The provider's own signature, checked with that company's own secret, is the
  trust boundary, exactly as before. A Resend token never routes an Airwallex delivery.
- **The door is not an owner API.** It answers only that path shape and only `POST`. The gateway
  exempts it from the owner-browser boundary before any session is read, strips `Cookie` and
  `Authorization` from the request, sets no cookie, and does nothing but verify and hand the event
  to the provider's receiver. Bodies are fixed strings, so a caller learns nothing about which
  check failed; the reason goes to the log and to the company's health row.
- **Bounded.** 256 KiB per body (the gateway's body limit and the handler), 120 deliveries a minute
  per token and 1,200 a minute for the whole door (the plane), and the same per-token limit at the
  edge (`HOOKS_LIMITER`). A provider retries a 429.
- **Replay and idempotency.** Svix timestamps outside five minutes are refused, as before. Authority
  dedupes by provider fact. A received email's key is its email id
  (`resend:email.received:<id>`), so the webhook and the poll below land on one record; every other
  Resend event keeps the Svix delivery id, because a bounce and a complaint about one sent email are
  distinct facts.
- **202 means custody.** The answer waits for Authority, as `ingress.rs` already did; OrgIntel
  projection may follow and is owed to the cursor reconciler.
- **Reachability.** The public base is `RESTLESS_HOOKS_PUBLIC_BASE` when set (a self-hosted plane
  behind its own tunnel), else the one address Cloud serves the plane under
  (`RESTLESS_PUBLIC_ORIGIN`, so `https://restless.run/hooks/...`), else the plane's network hostname.
  A local-only install has none; it receives email by polling.

Cloud's edge router (`services/edge-router`) sends `POST /hooks/<provider>/p<plane>-…` to that plane
via `/v1/route?plane=<id>`, with no cookie or bearer either way, and 404s anything else under
`/hooks/`. `hooks` is a reserved first segment, never a company key.

### Incoming email

When a company has a Resend key that can see a verified domain, Restless registers its own
`email.received`, `email.bounced` and `email.complained` webhook at the company's door, stores the
one-time signing secret in the company Vault as `resend.webhook`, and records the domains it
receives for (several companies may share one Resend account; mail for another company's domain is
answered 202 and not recorded). The owner sees **Incoming email: Connected** in Company ›
Integrations › Email with a **Test** button. The key the owner gave the company is the consent: a
webhook on the company's own Resend account that delivers the company's own mail to the company
needs no second approval. Every failure has a door (an Inbox card and the Integrations page).

A reconciler polls Resend's Receiving API every two minutes from a time cursor, so mail the webhook
missed is still delivered within minutes, deduped against the webhook by email id. A poll that finds
mail the webhook never delivered, a disabled webhook, or failed deliveries become one Inbox warning.

## Adding a provider

A provider is one arm of `hooks::Provider`: its path name, its signature check, and the receiver its
verified events go to. The receiver records an Authority fact before it returns and projects it to
the nearest accountable actor. The owner gets the address from `POST
/api/companies/<company>/hooks/<provider>` (owner only), or Restless registers it where the
provider has an API for that.

- **GitHub:** `X-Hub-Signature-256` (HMAC-SHA256 of the body, hex, `sha256=` prefix) with a
  per-company secret; dedupe by `X-GitHub-Delivery`. A GitHub App's installation webhook goes to
  `github_app` instead of a per-company secret.
- **Stripe:** `Stripe-Signature` (`t=<ts>,v1=<hmac>` over `<ts>.<body>`, five-minute tolerance);
  dedupe by event id; like Airwallex, a verified event only wakes an authenticated API read, it
  never attests that money moved by itself.
- **Forms:** a form provider that signs (Tally, Typeform: HMAC of the body) verifies its own way. A
  form that cannot sign is not a webhook; it is a public submission and belongs to a published site
  with its own abuse controls, not this door.

## Airwallex

Airwallex is served at `/hooks/airwallex/<token>` through the same receiver it already used
(`airwallex::receive_webhook`, then `continue_after_payment_observation`). The port 7795 listener
and Resend's port 7792 `/inbound/<company>` listener stay for self-hosted installs that already
point providers at them; neither is published by a hosted plane, and both are removed once no
install uses them.

## Risks and dispositions

- **A leaked token.** Accepted. It lets someone send requests that fail the signature check and
  count against the company's rate; it never writes anything. Rotating means a new endpoint row.
- **A provider's signing secret leaks.** Guarded as before: it lives only in the company Vault and
  `credential::resolve` refuses to hand `resend.webhook` to anything but the door.
- **The edge is skipped.** Accepted. The plane's host is public; the plane enforces every check
  itself, and the edge limit is only the first of two.
- **One Resend account shared by several companies.** Guarded by the recorded receiving domains.
  Two companies receiving for the same domain both see its mail.
