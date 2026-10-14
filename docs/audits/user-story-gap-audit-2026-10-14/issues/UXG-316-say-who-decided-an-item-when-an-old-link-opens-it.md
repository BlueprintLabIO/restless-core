# UXG-316 — Say who decided an item when an old link opens it

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Cross-surface journeys | low | ux | US-210 |

## Problem

Opening a notification or Telegram link to an item that the owner, a cofounder or a Telegram button already answered shows "That item has been resolved or is no longer in the Inbox." and jumps to the next item. The owner doesn't learn who decided, what was decided or where to see it, and may ask Exec or decide again elsewhere. Distinct from UXG-019, which is about the owner's own recent answers.

## Evidence

- `web/src/routes/[companyId]/+page.svelte:205-224` (missing item gives generic notice and redirects)
- `web/src/routes/[companyId]/company/activity/+page.svelte:27-30` (decision history exists to link to)

## Suggested direction

Look the id up in decision history and show "Decided by X: …" with a link to that Activity entry.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
