# Sprint 58 — One portfolio, one company page, one account page, on every host

**Status:** in progress (Core half C58-T1–T6; Cloud half C58-T7–T9 in `restless-cloud`)
**Programme:** owner surfaces
**Decision:** extends Cloud [ADR 0005](https://github.com/BlueprintLabIO/restless-cloud/blob/main/docs/adr/0005-plane-signed-company-projection.md)
(counts-only company projection) to contract version 2 under its proposed "registry of content-free
record kinds". Follows ADR 0012's pattern: Core reads, the owning service writes, one page merges them.
**Depends on:** `@restless/ui` as a signed release artifact (Cloud ADR 0004), the plane-signed company
projection (`crates/restless-owner/src/company_projection.rs`, Fleet `company_projection.rs`)

## Outcome

An owner opens Restless and sees **their companies**: one card each, ordered by what needs them. Each
card leads with the one thing worth doing, such as "2 decisions waiting" or "Give access". Beneath it
are how many people are working now and how many outcomes landed in the last day. A company that can't
start looks different and shows its fix. The page looks and means the same on a local appliance and on
Cloud, because both render the same view from the same company-signed numbers.

Inside a company, **Company** is an overview, not a settings index. A status line comes first, then one
row per area showing its current value (the model in use, active schedules, connected tools, members).
Each row opens in place. Health and activity live in the status line. The eleven-item settings sidebar
is gone.

**Account** is one page. Locally it has Connections, AI apps and Appearance. On Cloud it has Account,
Security, Billing and Support. The sections differ by host; the design does not.

## Observed friction

Read on 5 October 2026 from Core `main` at `b0790fc`, Cloud `main` at `de5109f`, the parked Cloud
branch `feat/account-sidebar` (`244eaf8`) and the live appliance.

| Friction | Evidence |
|---|---|
| The root page is an admin table | Four columns of truncated prose ("Current focus", "Next"); healthy and broken companies look alike; tiny `…` status tiles |
| Blank first paint locally | Logo on an empty canvas for ~2 s before the table appears (live, 1440 px) |
| One component, two different pages | Core fills `CompanyPortfolio` with local prose; Cloud passes `focus: null` and "As of 9:12" because Fleet may not read companies (ADR 0005). The design is built for the lowest common denominator |
| The company settings sidebar is a dump | `/[companyId]/company/*`: twelve routes under four uppercase group labels (Company, Capabilities, Operations, Records), mixing configuration with records. It squeezes content beside the Exec rail |
| Account settings are split across apps | Core: Connections, AI apps, Appearance. Cloud `fleet-web`: Account, Billing, Security, Support. The parked Cloud branch and Core `def503e` move both into a Linear-style left sidebar, the pattern this sprint replaces |
| Cloud-only company facts live in a second app | `fleet-web` `/account/company/{id}/compute` and `/service` |

## Design stance

**Core contract**

- **One view per surface, one contract per view, data from its owner.** `@restless/ui` holds the
  views. Each host maps its own data into the view's typed entry. A host that lacks a capability omits
  that section; it never shows a disabled imitation. Probe, never guess.
- **The portfolio card is the company's own signed, content-free projection** (ADR 0005), extended to
  version 2. Locally the account plane hands the same card to the root page directly; on Cloud the
  plane pushes it to Fleet. Neither host shows a field the other can't.
- **Fleet still holds no company content.** Version 2 adds counts and a yes/no only. There are still no
  titles, text, names or amounts. Spend therefore stays inside the company.

**Default pattern**

- Account-level pages share the company app's top bar: wordmark, a Companies control, the profile
  menu. There is no left sidebar anywhere.
- Rows and cards lead with state and the next action. Configuration hides behind the row.
- Tooltips, not subtitles; no eyebrow labels; a skeleton instead of a blank canvas.

**Product hypothesis**

- People working now and outcomes in the last day are the two numbers that make a company feel alive
  at a glance. Dogfood decides whether they earn their place.

## The card contract (company projection v2)

| Field | Type | v1 | Meaning |
|---|---|---|---|
| `decisions_waiting` | integer | yes | Items waiting for the owner |
| `last_activity_at` | time | yes | When work last moved |
| `people_working` | integer | **new** | Actors with a turn in flight now |
| `outcomes_last_day` | integer | **new** | Work items completed in the 24 hours before `projected_at` |
| `exec_ready` | boolean | **new** | Whether Exec can start (has working intelligence) |

`contract_version` becomes 2. Fleet accepts v1 and v2, so an older plane keeps working. A plane emits
v2 once its Cloud release pins this Core, and Cloud deploys Fleet first. Field bounds and the
"no free text" refusal stay exactly as in v1.

## Scope by layer

### Kernel / plane (`restless-owner`)

- Projection v2 claims, summariser and fixture (`contracts/company-projection.v2.fixture.json`); the v1
  fixture stays for Fleet's backward-compatibility test.
- The same summary, unsigned, on the local companies endpoint, so the local root page reads the card the
  plane would sign.

### `@restless/ui` (0.5.0, breaking)

- `CompanyPortfolio`: cards ordered *needs you → working → quiet → dormant*. Each card has the company
  mark, name, run state, one primary action, and the counts with their "as of" age when stale.
- `CompanyMark`: a deterministic, content-free mark seeded from the company id.
- `AccountShell`: the top-bar frame for account-level pages (replaces the sidebar from `def503e`), with
  the profile menu and the mobile layout.
- `AccountPage`: one page of titled sections with an in-page index once there are four or more.

### Core cockpit

- Root: the new portfolio from the card, a skeleton while loading, archived companies behind one quiet
  link.
- `/account`: one page with Connections, AI apps and Appearance sections. The old
  `/account/settings/*` URLs redirect to their section and keep their query (connection grants deep-link
  into it).
- `/[companyId]/company`: the overview, with the status line (health, computer, spend, people) and rows
  for Charter, Intelligence, Members, Connections and skills, Schedules, Limits, Identity and vault, and
  Computer. Each existing detail page stays as the row's destination, under a "Company" back link. The
  settings sidebar and its group labels are deleted.

### Cloud (paired, `restless-cloud`)

- Fleet API accepts projection v2, stores the three new fields (one migration), and exposes them in
  `contracts/fleet-api.openapi.json` and the generated client.
- Fleet Web adopts `@restless/ui` 0.5.0. Its portfolio maps fleet run state plus the v2 card into the
  same view. Its account settings become one `AccountPage` with Account, Security, Billing and Support.
  Compute and Service move into each card's menu. The parked branch's deletion of the dead shadcn
  sidebar stack is kept; its sidebar layout is not.

**Staged, not in this sprint:** folding Cloud's account sections into the cockpit's Account page through
the issuer's and Fleet's own APIs, and moving Compute/Service into the cockpit's Company overview. Both
need cross-origin calls to Fleet from each owner plane, which ADR 0012 has so far allowed only for
membership.

## Acceptance

1. **Same card, both hosts.** The gallery renders the portfolio from one fixture set through the local
   mapping and through the Fleet mapping, and the two render identically for the same numbers.
2. **Local root.** On a `_test` stack with one healthy, one blocked and one archived company:
   - order: blocked first, then working, then quiet;
   - the blocked company's action opens its fix;
   - the counts match the plane's own summary;
   - no blank first paint;
   - desktop 1440 px, phone 390 px, light and dark.
3. **Projection v2.**
   - Core signs v2 claims that verify against the fixture.
   - Fleet's tests accept v1 and v2 and reject a v2 claim with an extra field.
   - Fleet's portfolio shows the new counts.
4. **Company overview.**
   - Every row shows a current value from the live company.
   - Every row opens its detail page.
   - No settings sidebar remains.
   - Health shows in the status line.
5. **Account.**
   - `/account` shows the three local sections.
   - `/account/settings/connections?grant=…` lands on Connections with the grant open.
   - Cloud's Account page renders its four sections from Fleet data.
6. **Checks.** Core `npm run check`, web unit tests and Rust tests pass. Cloud `fleet-web` check and
   tests and `fleet-api` tests pass. `artifact:check` passes against the signed `@restless/ui` 0.5.0.

## Deletable machinery

`AccountNavigation`/the sidebar `AccountShell` and `AccountFrame` sidebar nav; the company settings
sidebar in `/[companyId]/company/+layout.svelte`; `CompanyPortfolio`'s prose columns (`focus`, `next`);
Fleet Web's sidebar layout and the dead shadcn sidebar stack (from `244eaf8`).

## Risks and dispositions

| Risk | Disposition |
|---|---|
| A v2 plane talking to a v1-only Fleet loses its projection | **Guarded.** Fleet deploys first; v1 stays accepted |
| `people_working` reads as surveillance | **Accepted.** It is a count, covered by ADR 0005's per-company off switch, like `last_activity_at` |
| Removing the sidebar hides rarely used settings | **Accepted.** Every area keeps a row with its current value; nothing loses its page |
| Breaking `@restless/ui` 0.5.0 strands Cloud on 0.4.0 | **Accepted.** Cloud pins the artifact and moves in the paired change |

## Tickets

- [x] C58-T1 — Projection v2: claims, summariser, fixture, tests. *Plane*
- [x] C58-T2 — Local companies endpoint carries the card. *Plane*
- [x] C58-T3 — `@restless/ui` 0.5.0: portfolio cards, company mark, top-bar account shell, account page. *UI*
- [x] C58-T4 — Core root page and `/account` single page with redirects. *Cockpit*
- [x] C58-T5 — Company overview; delete the settings sidebar. *Cockpit*
- [ ] C58-T6 — Visual pass: desktop, phone, dark; Beautiful UI / Cult UI bar plus one source-first reference. *Cockpit*
- [ ] C58-T7 — Fleet API: accept and store projection v2; OpenAPI and client. *Cloud* (in review: restless-cloud PR #30)
- [ ] C58-T8 — Fleet Web: adopt 0.5.0; portfolio mapping; single Account page; Compute/Service in card menus; delete the sidebar stack. *Cloud*
- [ ] C58-T9 — Cloud checks and `artifact:check` against the signed artifact. *Cloud*
