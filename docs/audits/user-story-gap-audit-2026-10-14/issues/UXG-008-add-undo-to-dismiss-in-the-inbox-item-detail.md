# UXG-008 — Add Undo to "Dismiss" in the Inbox item detail

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | high | consistency | US-012 |

## Problem

The list's "Not needed" shows a toast with Undo; the detail's "Dismiss" does the same thing with no confirmation and no Undo, only a sentence pointing to the Set aside fold. It sits next to the title, so one misclick tells the asker to stop waiting.

## Evidence

- `web/src/lib/components/InboxDetail.svelte:227-237,` :375-382
- `web/src/lib/components/InboxList.svelte:108-125`

## Suggested direction

Route the detail's dismiss through the same toast-with-undo the list uses.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
