# UXG-043 — Give stopped work a place to be found and restored

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | high | missing-function | US-029, US-030 |

## Problem

Every list, count, board column and search excludes `abandoned`. Once stopped, work is reachable only by a remembered URL, and with no reopen endpoint a mistaken stop is unrecoverable.

## Evidence

- `web/src/routes/[companyId]/work/+page.svelte:138,` :158, :167, :191, :317, :349
- `web/src/routes/[companyId]/work/[workId]/+page.svelte:743-746`

## Suggested direction

Add a "Stopped" view with search coverage and a restore action.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
