# UXG-189 — Wire up or delete the unused AccountHome view

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | low | consistency | — (found while tracing adjacent paths) |

## Problem

AccountHome.svelte (835 lines: usage, plan, protection checklist) is used only by /gallery/home; account usage never reaches owners.

## Evidence

- `web/src/lib/ui/views/AccountHome.svelte:16-21`
- `web/src/routes/gallery/home/+page.svelte`

## Suggested direction

Wire into `/` with real data, or delete it.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
