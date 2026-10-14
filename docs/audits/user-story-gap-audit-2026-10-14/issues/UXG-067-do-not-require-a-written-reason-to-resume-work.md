# UXG-067 — Do not require a written reason to resume work

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | low | ux | — (found while tracing adjacent paths) |

## Problem

Resume is disabled until a reason is typed, with no message explaining why the button is disabled.

## Evidence

- `web/src/routes/[companyId]/work/[workId]/+page.svelte:112,` :484-491

## Suggested direction

Make the reason optional for resume, or prefill it, and explain disabled state.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
