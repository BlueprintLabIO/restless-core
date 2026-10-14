# UXG-325 — Point to the accepted result after an Inbox decision

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Cross-surface journeys | low | ux | US-196 |

## Problem

After an answer or acceptance, the Inbox shows a six-second notice whose only link is "Open conversation". Nothing leads to the accepted document, site or file, or to where it now lives in the Library, so the owner who just approved a deliverable has to search for it.

## Evidence

- `web/src/routes/[companyId]/+page.svelte:193-197` (notice auto-clears after 6s)
- `web/src/routes/[companyId]/+page.svelte:966-969` (only "Open conversation" link)
- `web/src/lib/components/InboxDetail.svelte:206-218` (resolved payload is text plus conversation href)

## Suggested direction

Include the review target or artifact link in the resolution notice when the decision accepted an outcome.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
