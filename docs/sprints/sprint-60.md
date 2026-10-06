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

- [x] C60-T1 — The plane takes `RESTLESS_PUBLIC_ORIGIN` (`https://app.restless.run`): origin, host
  and `sec-fetch-site` checks accept it as well as the plane's own hostname, and the plane still
  refuses forwarded requests from anything but its tunnel.
- [x] C60-T2 — The session cookie is named `restless_session_<plane-id>`. (`/<company>/entry` needs no
  plane change: the router rewrites it to the plane's `/entry`, T5. Entry already lands on
  `/<company>/…`.)
- [x] C60-T3 — A hosted plane's root goes Home on the issuer (`47ba77c`), and a signed-out page load
  goes there too (`2d4c776`); Home re-enters through the company card, so no `/enter` endpoint is
  needed. The plane's account sections are its own pages (T9).
- [x] C60-T4 — Untrusted content audit (6 October): nothing company-produced runs as the cockpit's
  origin. Document HTML (`DocumentEditor`) comes from OrgIntel's allowlist renderer: fixed node
  types, escaped text and attributes, links limited to http(s) and mailto. Chat Markdown is built
  from components, never raw HTML. The embedded resource viewer is sandboxed without
  `allow-same-origin`. The review and reference previews load the isolated review origin. **Found
  in passing:** that origin is loopback-only (`<ticket>.localhost`), so previews do not open on a
  hosted plane at all; a separate Cloud gap, not a one-address blocker.

### P2 — The edge router (*Cloud*)

- [x] C60-T5 — The Cloudflare Worker (restless-cloud #40, #44): the path table, the cookie filter,
  WebSocket pass-through, unknown keys refused; deployed by the Release workflow before the services.
- [x] C60-T6 — **Revised:** Fleet answers `/v1/route` and the router caches each answer for a minute,
  instead of Fleet writing a KV copy, so there is one directory and no edge credential (#41). Fleet
  sets `restless_home_plane` on the account pages.
- [x] C60-T7 — Fleet Web's assets are under `/_fleet` (#41). **Revised:** no asset bucket. The router
  fetches cockpit assets from the plane whose page asked (page Referer, else a routing hint), so the
  build always matches the page (#44).

### P3 — Same-origin entry and one Account (*Cloud + Cockpit*)

- [x] C60-T8 — With the switch on, company entry is a same-origin POST to `/<company>/entry` and
  account entry to `/account/entry` (#44).
- [x] C60-T9 — **Revised:** one Account in the rail, each section living once with its owner, rather
  than Fleet rendering the plane's sections. Profile, Security, Plan and Support are Fleet's; the
  plane serves `/account/connections`, `/account/ai-apps` and `/account/appearance` (`e4ff8f9`); the
  router sends each to its owner, and both rails list all seven (#44).

### P4 — Cut over and delete (*Cloud*)

- [ ] C60-T10 — Set the restless-cloud repository variable `RESTLESS_ONE_ADDRESS=true` and deploy
  with a Core release at or after `e4ff8f9`. Every owner moves to `app.restless.run`. A browser on `owner-<hex>.restless.run` is
  sent to the matching `app.restless.run` path. Verified on the founder's account: sign in once,
  Home, a company, Account, a lapsed session, and a member of another owner's company.
  Deployed 2026-10-06, restless-cloud run 37396115400 with Core `47b98a2`. Signed-out checks observed:
  - The route lookup works.
  - A company page and an old `owner-<hex>` link both 303 to `/account?return=<path>`.
  - API calls return 401 `no_session` / `no_account_plane`.
  - Assets are served via Referer.
  The founder's signed-in checks are still open.
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
