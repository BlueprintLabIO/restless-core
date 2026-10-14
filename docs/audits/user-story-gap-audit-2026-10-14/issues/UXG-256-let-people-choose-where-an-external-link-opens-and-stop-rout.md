# UXG-256 — Let people choose where an external link opens, and stop routing members to a page they cannot open

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company computer & intelligence | medium | ux | US-148 |

## Problem

Every plain click on an external link anywhere in a company (chat, documents, Library) is intercepted and sent to Company → Computer, which wakes the computer and opens a new tab in the browser agents use. Members cannot open Company → Computer, so a member's click lands on a refused route, shows a toast and bounces to the Inbox; the link never opens. Owners who only want to read an article spend runtime hours and add tabs to the agents' browser, with no visible choice.

## Evidence

- `web/src/routes/[companyId]/+layout.svelte:1320-1323` (every link goes to computer?focus=browser)
- `web/src/routes/[companyId]/+layout.svelte:1414-1416` (capture wraps the whole company)
- `web/src/lib/actions/company-browser-links.ts:11-31`
- `web/src/lib/model/company-access.ts:27` (members' Company sections exclude computer)
- `web/src/routes/[companyId]/+layout.svelte:532-537` (refused route toast and redirect)
- `crates/restless-owner/src/owner.rs:10796-10805` (opening a link wakes the computer)

## Suggested direction

Open in the owner's own browser by default, and offer "Open in company browser" where shared sign-in state matters; never route members there.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
