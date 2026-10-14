# UXG-287 — Add print styles beyond documents

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Shell & cross-cutting | low | ux | US-175 |

## Problem

Only documents have a print stylesheet; printing a Work detail, Activity, Limits or a conversation prints the top bar, dock, Exec rail and scroll-clipped panes. Owners who need a paper or PDF copy for an accountant or a partner get an unusable page.

## Evidence

- `web/src/lib/components/document-print.css` (the only @media print in the app)
- `web/src/lib/components/AppShell.svelte:288-500` (topbar, dock and rail always rendered)

## Suggested direction

Add one shell-level print sheet that hides chrome and the rail and lets the main pane flow.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
