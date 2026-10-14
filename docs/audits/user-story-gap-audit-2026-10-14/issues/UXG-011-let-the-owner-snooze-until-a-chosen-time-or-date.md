# UXG-011 — Let the owner snooze until a chosen time or date

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | medium | missing-function | US-011 |

## Problem

Snooze is fixed at one hour. The spec lists "defer until a date or condition", so "decide on Monday" is impossible and the item is back within the hour.

## Evidence

- `web/src/lib/components/InboxList.svelte:189-197,` :351
- `docs/specs/owner-cockpit.md` §5.5

## Suggested direction

Offer later today, tomorrow morning, next week, or a picked date.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
