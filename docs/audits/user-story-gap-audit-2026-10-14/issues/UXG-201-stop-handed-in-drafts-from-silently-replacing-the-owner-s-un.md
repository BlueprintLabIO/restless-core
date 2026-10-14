# UXG-201 — Stop handed-in drafts from silently replacing the owner's unsent message

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Exec chat | medium | ux | US-107 |

## Problem

"Ask Exec" from another surface, "Turn into Work" on a message and "Edit" on a retracted message all assign straight to the composer, overwriting whatever the owner had typed with no warning or undo. The rail's own comment for tapped answers promises a composer draft is never replaced, but these paths do replace it.

## Evidence

- `web/src/lib/components/ExecutiveRail.svelte:365-371` (draftRequest assigns composer)
- `web/src/lib/components/ExecutiveRail.svelte:223-231` (Turn into Work sets composer = draft)
- `web/src/lib/components/ExecutiveRail.svelte:571-575` (Edit sets composer = last.text)
- `web/src/lib/components/ExecutiveRail.svelte:594-595` (comment: a draft there is never replaced)

## Suggested direction

Append handed-in text below an existing draft (or offer Replace/Keep), never discard it silently.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
