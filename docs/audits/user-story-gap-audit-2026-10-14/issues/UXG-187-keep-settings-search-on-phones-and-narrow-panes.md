# UXG-187 — Keep settings search on phones and narrow panes

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | low | mobile | — (found while tracing adjacent paths) |

## Problem

Under 760px the sidebar and its search are hidden; the overview has no search.

## Evidence

- `web/src/lib/ui/views/SidebarShell.svelte:140-147`
- `web/src/routes/[companyId]/company/+layout.svelte:79-81`

## Suggested direction

Show the search field atop the narrow overview.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
