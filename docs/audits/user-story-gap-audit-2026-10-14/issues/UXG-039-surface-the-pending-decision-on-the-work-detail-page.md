# UXG-039 — Surface the pending decision on the Work detail page

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | high | ux | US-026 |

## Problem

"Needs you" and "Waiting on you" rows link to Work detail, which reads no handoff or attention item and shows no decision or link to the Inbox. The owner lands on a page with nothing to do.

## Evidence

- `web/src/routes/[companyId]/work/+page.svelte:209,` :223, :411
- `web/src/routes/[companyId]/work/[workId]/+page.svelte` (no attention/handoff usage beyond line 27)

## Suggested direction

Show the pending judgement inline with the same actions, or deep-link to the Inbox focus.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
