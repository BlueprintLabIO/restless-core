# UXG-017 — Order the Inbox by consequence and deadline, not age alone

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | medium | spec-drift | US-008 |

## Problem

The order is blockers first, then oldest. It ignores deadline, cost and irreversibility (spec §5.4), so an approval due today can sit below a week-old low-stakes question. Deadlines print as raw strings.

## Evidence

- `web/src/lib/model/attention-presentation.ts:53-58`
- `web/src/lib/components/InboxDetail.svelte:370-372`

## Suggested direction

Sort by deadline when present and format it relatively.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
