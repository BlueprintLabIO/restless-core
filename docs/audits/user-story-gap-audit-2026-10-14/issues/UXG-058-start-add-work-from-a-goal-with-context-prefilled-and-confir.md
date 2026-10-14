# UXG-058 — Start "add work" from a goal with context prefilled, and confirm creation

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | low | ux | US-024 |

## Problem

`askExec()` is always called with no draft even in a goal view, and nothing tells the owner afterwards whether Work was created. Goal views have no add-work entry point.

## Evidence

- `web/src/routes/[companyId]/work/+page.svelte:426-430,` :602-606
- `web/src/lib/model/ask-exec.ts:5-12`

## Suggested direction

In a goal view, open Exec with "Add work toward <goal>:" prefilled; confirm when Work appears.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
