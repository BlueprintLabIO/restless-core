# UXG-147 — Stop Tab from sending a message to Exec in the Apps ask box

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Apps | medium | a11y | — (found while tracing adjacent paths) |

## Problem

While results show, Tab is captured and sends the typed text to Exec as a real message, so keyboard users cannot tab out without acting.

## Evidence

- `web/src/routes/[companyId]/apps/+page.svelte:194-197`

## Suggested direction

Let Tab move focus; keep Enter on the highlighted row.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
