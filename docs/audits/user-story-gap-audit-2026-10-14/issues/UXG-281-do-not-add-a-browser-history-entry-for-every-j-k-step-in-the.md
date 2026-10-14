# UXG-281 — Do not add a browser history entry for every J/K step in the Inbox

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Shell & cross-cutting | low | ux | US-171 |

## Problem

Selecting an Inbox item with J/K or a click calls goto without replaceState, so walking ten items creates ten history entries. Browser Back then steps through previously selected items instead of leaving the Inbox, and on a phone the system back gesture appears to do nothing useful.

## Evidence

- `web/src/lib/components/InboxList.svelte:137` (?item= href)
- `web/src/lib/components/InboxList.svelte:223-232` (select uses goto without replaceState)
- `web/src/lib/components/InboxList.svelte:250-261` (J/K call select)

## Suggested direction

Replace state for keyboard selection steps and push only on deliberate opens.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
