# UXG-096 — Handle a failed sheet-history load

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Library | low | ux | — (found while tracing adjacent paths) |

## Problem

If loading versions fails, `versions` stays null and History shows "Loading…" forever.

## Evidence

- `web/src/routes/[companyId]/library/sheets/+page.svelte:414-417,` :873-874

## Suggested direction

Show the failure with Retry inside the panel.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
