# UXG-277 — Offer "Ask Exec about …" with the typed text when ⌘K has no answer

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Shell & cross-cutting | medium | missing-function | US-158 |

## Problem

⌘K has an "Ask the Exec…" action, but it ignores what was typed, and an unmatched query ends in "No match". The product’s stance is to let intelligence do the work; the obvious move from a failed search is to hand the words to Exec. The rail already accepts a prefilled draft through askPerson, so the plumbing exists and is just not connected to the menu.

## Evidence

- `web/src/lib/components/CommandMenu.svelte:191-196` (choose runs a command with no query)
- `web/src/lib/components/CommandMenu.svelte:296-299` (dead-end empty state)
- `web/src/routes/[companyId]/+layout.svelte:994-999` (Ask the Exec without draft)
- `web/src/routes/[companyId]/+layout.svelte:874-892` (askPerson supports a draft)

## Suggested direction

Always append an "Ask Exec: “<query>”" row that opens the rail with the query as a draft.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
