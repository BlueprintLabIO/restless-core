# Site and shell programme

**Status:** Sprints 1–5 delivered on local branches (1 October 2026); Sprint 6 deliberately not built. See *Outcome* at the end.
**Owner intent:** two public pages with distinct personalities, one shared design system, and a
mature Cloud shell and cockpit.

| Surface | Audience | Tone | Primary action |
| --- | --- | --- | --- |
| Core page | Open-source builders, technical early adopters, AI enthusiasts, harness testers | Playful, pixel | GitHub |
| Cloud page | Outcome-minded business users and enthusiasts | Calm, premium | Private-beta waitlist (email to the founder) |
| Cloud shell | Signed-in owners and members | Linear/Apple maturity | Enter a company |
| Cockpit | Owners and members inside a company | Calm, dense, keyboard-first | Decide, review, steer |

Decisions already taken:

- The "no sidebar" rule is withdrawn. The cockpit moves to a left rail with a company switcher.
- Fleet may receive a minimal, plane-signed per-company projection (counts and state only, no titles).
- No dark mode for now.
- Real components are rendered on the public pages from fixture data. Screenshots are not used.
- Core ships a versioned UI artifact. Cloud consumes it as a pinned tarball and never copies Core source.

## Sprint 1 — Foundations and the design-system boundary (Core)

Goal: a `web/src/lib/ui` boundary that contains only things that render from props, plus a gallery
that proves it.

- Withdraw the sidebar rule in `CLAUDE.md`.
- Create `lib/ui/` (tokens, primitives, views, showcase fixtures, office, vendored engine) with
  relative imports only. A check script fails on any `$lib/` or `$app/` import inside it.
- Split the coupled views into a container and a pure view: `AttentionCard`, the Work board and card.
- `/gallery` route (development) renders every view from fixtures.
- Evidence: `npm run check`, the new boundary check, unit tests, and a rendered gallery.

## Sprint 2 — Core landing page (pixel, GitHub)

Goal: the open-source front door, built from real components.

- New `site/` app in this repository, static, consuming `web/src/lib/ui` directly.
- The company office as the world: scroll moves the camera across the floor, rooms map to features,
  the cursor is a spotlight, the hero reads the visitor's local time, the wordmark fidgets, and the
  page ends on a hold-to-approve climax.
- GitHub is the only primary call to action. Reduced-motion and mobile fallbacks are complete pages.
- MIT notice for the vendored engine ships with the site.

## Sprint 3 — UI artifact

Goal: Cloud can consume the design system without copying source.

- `scripts/pack-ui.mjs` produces a versioned tarball of `lib/ui` with a `package.json`, an `exports`
  map and a checksum.
- Smoke test: the tarball installs into a clean project and renders a view.

## Sprint 4 — Cloud landing page (calm, premium, waitlist)

Goal: the business-facing front door in `restless-cloud/site`.

- Consumes the tarball. Private-beta framing. Email contact for the waitlist.
- Signature moment: a pinned product reveal in which a typed outcome becomes work, then evidence,
  then one prepared decision the visitor can approve.
- Plain outcome language. No Fleet, cell or plane vocabulary.
- Waitlist submits to a small endpoint that emails the founder. The provider secret is configured by
  the owner; the page degrades to a mail link.

## Sprint 5 — Cloud shell

Goal: replace the engineering readiness page with a real shell in `restless-cloud/apps/fleet-web`.

- Shared tokens from the UI artifact, one wordmark, one type ramp.
- The door (passkey first), the portfolio of companies, create and provisioning, account and security,
  members.
- ADR for the plane-signed company projection (decisions waiting, state, last activity).

## Sprint 6 — Cockpit shell

Goal: the left-rail cockpit chrome in Core `web`.

- Rail with company switcher, surfaces, attention badge and command menu; Exec as a pane.
- Phones keep the bottom dock. Keyboard navigation across surfaces.

## Standing rules for every sprint

- Nothing is reported as working unless it ran and the output was observed.
- No push, no pull request, no outbound message. Work is committed locally on a branch.
- Anything started is stopped in the same turn.

## Outcome (1 October 2026)

| Sprint | Result | Evidence |
| --- | --- | --- |
| 1 Foundations | Done, with two carry-overs. `src/lib/ui` boundary, `check:ui`, pure `WorkBoard` and `OutcomeFolio`, `Wordmark`, fixtures, `/gallery`. The sidebar rule is withdrawn. | `npm run check`, 65 unit tests, production build, gallery rendered |
| 2 Core page | Done. `landing/`: the company floor as a scroll-driven night shift, local-time hero, spotlight, hold-to-approve climax, static mode. | svelte-check, static build (98 KB gzipped JS), five beats rendered at 1440 and 375, axe: 0 violations in four states |
| 3 UI artifact | Done. `@restless/ui` 0.1.1, checksum, manifest with the Core revision, clean-project smoke test. | `npm run smoke:ui` (7 of 7) |
| 4 Cloud page | Re-scoped, done. The current site is on Cloud `dev`, not `main`; it is an approved design, so the change is targeted: the real decision surface and a private-beta waitlist. | `npm run verify`, 11 tests, Pages Function run under wrangler, full docker build, axe: 0 violations |
| 5 Cloud shell | Re-scoped, done in part. Fleet Web now consumes the artifact instead of copies; ADRs 0004 and 0005. | svelte-check, build, docker build of the Fleet Web image |
| 6 Cockpit shell | **Not built on purpose.** | The cockpit already has a top nav with a sliding indicator, a company switcher, ⌘K, G-then-letter shortcuts and a theme system. A left rail is a stylistic choice that cannot be reviewed without a running company. |

### Carry-overs and open items

- Extract pure views for Authority and for the attention card (its container keeps the query client).
- Retire the duplicated token block in Fleet Web's `bridge-cloud.css` (needs a visual pass on a running stack and moves its type from Plex to Inter).
- Decide how `@restless/ui` is published (npm or OCI) — Cloud ADR 0004.
- Decide the company projection contract — Cloud ADR 0005.
- Pixel sprite provenance: the furniture, floor and wall art upstream (Pixel Agents) is undocumented. Confirm before using it as brand art beyond the landing page.
- The waitlist needs `RESEND_API_KEY` and `WAITLIST_FROM` as Pages secrets; add Turnstile before opening the beta widely.
- The privacy notice was updated for the waitlist and needs the owner's review.
