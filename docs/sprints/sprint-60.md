# Sprint 60 — One address

## Outcome

A Cloud owner signs in once at `app.restless.run` and never leaves it. Home, every company and one
Account page live under that address, as in Linear, Vercel or Notion. Each owner's plane still
holds their credentials, executes their effects and authenticates every session; only the address
the browser sees changes. An enterprise can have its own address.

Decision and rationale: restless-cloud
[ADR 0007](https://github.com/BlueprintLabIO/restless-cloud/blob/main/docs/adr/0007-one-address.md),
which amends ADR 0001 §2 and keeps its four objections to a Fleet proxy.

## Observed friction

Sprint 59's audit (5 October 2026) traced most Cloud states the owner hit to there being two
origins: two company lists, two Account pages, a cross-origin hand-off, and a lapsed plane session
that left raw refusals on every section. Sprint 59 shipped stopgaps (re-entry, host-aware wording).
This sprint removes the cause.

## Shape

```text
browser ──► app.restless.run (Cloudflare Worker: route + cookie filter, no identity)
              ├── /, /account/**, /auth/**, /_fleet/**             ─► Fleet Web (Northflank)
              ├── /_app/immutable/**                               ─► edge bucket of released cockpit builds
              ├── /<company>/**, /api/companies/<company>/**,
              │   /desktop/<company>/**, /<company>/entry          ─► that company's plane (its tunnel)
              └── account /api/** (restless_home_plane hint)       ─► the owner's own plane
```

## Phases and tickets

Each phase leaves Cloud working. The first three can ship behind the router before any owner
traffic moves.

### P1 — Core accepts a public origin (*Plane*)

- [ ] C60-T1 — The plane takes `RESTLESS_PUBLIC_ORIGIN` (`https://app.restless.run`): origin, host
  and `sec-fetch-site` checks accept it as well as the plane's own hostname, and the plane still
  refuses forwarded requests from anything but its tunnel.
- [ ] C60-T2 — The session cookie is named `restless_session_<plane-id>`, and `/<company>/entry` is
  accepted as the entry door, alongside `/entry`. Entry redirects land on `/<company>`.
- [ ] C60-T3 — A plane's Home and Account pages redirect to the public origin's `/` and `/account`
  on a hosted plane. The Sprint 59 re-entry redirect points at Fleet's `/enter?company=<key>`.
- [ ] C60-T4 — Untrusted content audit: every surface that renders company-produced markup uses the
  isolated review origin; record the list and close any gap before P4.

### P2 — The edge router (*Cloud*)

- [ ] C60-T5 — A Cloudflare Worker on `app.restless.run`: the path table above, the cookie filter
  (only the target plane's session cookie is forwarded; Fleet's never is), WebSocket pass-through,
  and refusal of unknown company keys. Deployed by the Release workflow.
- [ ] C60-T6 — Fleet writes the company directory (key → plane tunnel host) to Workers KV on create,
  archive, delete and move, and sets `restless_home_plane` at sign-in.
- [ ] C60-T7 — Fleet Web moves its assets to `/_fleet` (SvelteKit `appDir`). The Core release
  publishes each cockpit build's `/_app/immutable` to the edge bucket, keeping every released build.

### P3 — Same-origin entry and one Account (*Cloud + Cockpit*)

- [ ] C60-T8 — Entering a company is a same-origin POST to `/<company>/entry`, and Fleet's
  `/enter?company=<key>` re-mints after a lapse. Cards link to `/<company>`.
- [ ] C60-T9 — One Account page on Fleet: Profile, Security, Plan and Support from Fleet, plus
  Connections, AI apps and Appearance from the owner's plane through the router. The Core account
  sections ship in `@restless/ui`, so both hosts render the same views.

### P4 — Cut over and delete (*Cloud*)

- [ ] C60-T10 — Every owner moves to `app.restless.run`. A browser on `owner-<hex>.restless.run` is
  sent to the matching `app.restless.run` path. Verified on the founder's account: sign in once,
  Home, a company, Account, a lapsed session, and a member of another owner's company.
- [ ] C60-T11 — Delete: the cross-origin entry page, the plane Home redirect, account-entry
  assertions used only for the second Account page, and the Sprint 59 stopgaps the router makes
  unreachable.

### P5 — Owner-chosen keys and dedicated addresses (*Cloud*)

- [ ] C60-T12 — Owner-chosen company URL keys (unique across Cloud), with old keys redirecting.
- [ ] C60-T13 — Enterprise dedicated address: a host the router maps straight to one owner's plane.

## Acceptance

1. A new owner signs up, creates a company and works in it without the address bar ever leaving
   `app.restless.run`.
2. One Account page holds Profile, Security, Plan, Support, Connections, AI apps and Appearance.
3. A lapsed plane session re-enters without a visible error or second address.
4. Captured at the router, a plane never receives Fleet's session cookie or another plane's.
5. A company's HTML artifact opened from the cockpit renders on the isolated review origin, never
   on `app.restless.run`.
6. Desktop streaming and live documents work through the router.

## Risks

| Risk | Disposition |
| --- | --- |
| One origin lets a script in one company's page act with the viewer's session elsewhere | **Guarded.** Strict CSP on the cockpit; company-produced markup only on the isolated review origin (T4); acceptance 5 |
| The router is in every request's path | **Accepted.** It runs on the Cloudflare edge every tunnel already crosses; it holds no secret and mints no identity |
| A cockpit asset needed by an older plane mid-rollout is missing | **Guarded.** The edge bucket keeps every released build (T7) |
| Company keys collide across owners | **Guarded.** Keys are the plane's globally unique handle until owner-chosen keys arrive with a uniqueness check (T12) |
