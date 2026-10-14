# UXG-019 — Link the Inbox to "what I decided" and allow amending a recent answer

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | medium | missing-function | US-010, US-013 |

## Problem

Once answered, an item vanishes. Decision history exists but only under Company → Activity, with no link from the Inbox. A sent reply or decline cannot be revised.

## Evidence

- `web/src/lib/model/attention.ts:236-243` (used only in web/src/routes/[companyId]/company/activity/+page.svelte:47)
- `web/src/lib/components/InboxDetail.svelte:238-259`

## Suggested direction

Add a "Recently decided" link at the foot of the Inbox; allow a follow-up from there.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
