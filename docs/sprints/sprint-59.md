# Sprint 59 — One coherent Cloud, and a rail worth growing into

## Outcome

A Cloud owner never meets a state that only makes sense to an engineer. Signing in, an expired
session, a rollout, a renamed company and an archived company each read as what they are, in the
owner's words, on both origins (`app.restless.run` and the owner's plane). The account rail reaches
Linear and shadcn polish: icons, collapse, resize and a shortcut, and it carries the whole account on
either origin.

## Why now

The owner kept finding Cloud bugs one at a time (5 October 2026): a second company list, a local
repair banner, "Local appliance", a raw company id, and then an Account page where every section
read "this plane requires a verified entry assertion". This sprint audits by **class**, not by
screenshot, so the next ones are found before the owner does.

## Audit

Read on 5 October 2026 from Core `3734294`, Cloud `97451cd` and the owner's live plane.
Dispositions follow `LLM_CURE.md`: fix now, accepted, or decision needed.

### A. Sign-in seams between Fleet and the plane

| # | Finding | Evidence | Disposition |
| --- | --- | --- | --- |
| A1 | The plane serves its shell to a browser with no session and refuses every API with 401 `no_session` (or `stale_membership`). Nothing re-enters the owner, so each section prints the raw refusal | `owner.rs` network boundary: shell is public, `/api/*` is gated; the owner's Account page screenshot | **Fix now.** A page load without a session goes to Home on the issuer; an API 401 mid-session reloads once, which re-enters |
| A2 | The rail calls the owner "You" on a hosted plane, although the plane holds their verified name | `AccountFrame.svelte` hard-codes `name: 'You'` | **Fix now.** Hosted status carries the session's display name |
| A3 | A Cloud account's name defaults to the email prefix (`blueprintlabio`), and Exec built "Blueprint Lab" from it | Fleet Better Auth user record; Exec's naming question | **Fix now** in Fleet: sign-up asks for a name, and an account still named like its email prefix is asked once |

### B. Two origins, one account

| # | Finding | Evidence | Disposition |
| --- | --- | --- | --- |
| B1 | There are two Account pages: Fleet's (Profile, Security, Plan, Support) and the plane's (Connections, AI apps, Appearance), and each rail lists only its own | Fleet `/account/settings`; plane `/account` | **Superseded by Sprint 60.** The root cause is two addresses. One address makes one Account page possible, so no cross-origin rail is built |
| B2 | A rename inside the company never reaches Fleet, so Home keeps saying "New company" | Fleet org name; plane `display_name` | **Decision needed.** The name is owner-chosen and not company content, so sending it through Fleet's own org rename does not break the counts-only projection (ADR 0005). Not built until agreed |

### C. Local-install assumptions on a hosted plane

| # | Finding | Evidence | Disposition |
| --- | --- | --- | --- |
| C1 | Connections says Restless "holds every sign-in on this computer"; a local MCP server runs "on this computer" | `[companyId]/company/connections/+page.svelte` | **Fix now.** Say "your account plane" when hosted |
| C2 | Members shows "You · Owner · this computer" and offers Tailscale sharing steps | `[companyId]/company/members/+page.svelte`, `SharingSetup.svelte` | **No change.** Both render only when `core.mode === 'local'`; a hosted plane is in network mode |
| C3 | Resources says an item launched "on this computer" | `[companyId]/company/resources/+page.svelte` | **Fix now** |
| C4 | The sign-in fallback asks for "the callback on this computer … localhost URL" | `AccountConnections.svelte` | **Fix now.** On a hosted plane the browser never reaches the plane's localhost, so this paste is the normal path; say so plainly |
| C5 | Schedule-wake banner, "Local appliance", second company list, raw company id | Fixed in `47ba77c` | Done |

### D. Running companies described in setup words (Fleet Home)

| # | Finding | Evidence | Disposition |
| --- | --- | --- | --- |
| D1 | A running company whose plane is briefly degraded (a rollout) reads "Setup needs attention … setup did not finish" | Fleet `visualState`: `core_readiness_status === 'degraded'` maps to `failed`; the 25-minute degraded window on 5 Oct | **Fix now.** A company that was ever ready reads "Updating" or "Needs attention", never setup |
| D2 | A rollout that takes the plane out of `ready` reads "Setting up · Preparing a private workspace" | `companySetupState` treats anything not ready as setup | **Fix now** with D1 |
| D3 | Archived and deleted companies are excluded from polling but mapped through the setup states | `visualState` ignores `company_status` | **Fix now.** Archived reads dormant; deleted is not listed |
| D4 | Projection posts 404 for a minute while Fleet Web rolls out | Fleet logs, 5 Oct | **Accepted.** The plane retries; the card shows its age meanwhile |

### E. The account rail

| # | Finding | Disposition |
| --- | --- | --- |
| E1 | The rail has no icons, cannot collapse or resize, has no shortcut, and its items are flat text | **Fix now.** shadcn-svelte's sidebar is the behaviour reference (icon-collapse, an edge rail that toggles, ⌘B, persisted state, tooltips when collapsed); Linear is the polish bar (drag to resize, quiet hover, section labels only where they inform) |

## Tickets

- [x] C59-T1 — Re-entry (a stopgap until Sprint 60 deletes the second address): navigations without a session go Home on the issuer; API 401s reload once. *Plane / Cockpit*
- [x] C59-T2 — Hosted identity and wording: the owner's name in the rail (company entry carries it now; account entry accepts it once Fleet signs it); C1, C3 and C4 worded by host. *Cockpit*
- [x] C59-T3 — Account rail 0.7.0: icons, collapse with ⌘B and an edge toggle, drag to resize, persisted, tooltips when collapsed. *UI*
- [ ] C59-T4 — Fleet Web: lifecycle words (D1–D3), a real name at sign-up (A3), the display name in the account assertion, 0.7.0. *Cloud*
- [ ] C59-T5 — Decide B2 (rename reaches Fleet). *Owner*

**Decided 5 October 2026:** the owner chose one address (`app.restless.run`) for everything, the way
Linear, Vercel and Notion work, with a dedicated subdomain as an enterprise option. That is Sprint 60.
