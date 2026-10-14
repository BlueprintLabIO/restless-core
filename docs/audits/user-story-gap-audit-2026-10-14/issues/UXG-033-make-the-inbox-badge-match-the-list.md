# UXG-033 — Make the Inbox badge match the list

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | low | ux | US-023 |

## Problem

The badge counts every attention item including snoozed ones, plus For you unread; the list header counts only visible decisions. Snoozing never quiets the badge.

## Evidence

- `web/src/routes/[companyId]/+layout.svelte:267,` :813-818
- `web/src/lib/components/InboxList.svelte:127-130,` :287

## Suggested direction

Exclude snoozed items from the badge.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
