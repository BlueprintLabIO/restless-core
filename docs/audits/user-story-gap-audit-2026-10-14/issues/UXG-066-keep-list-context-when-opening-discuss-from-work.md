# UXG-066 — Keep list context when opening "Discuss" from Work

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | low | ux | — (found while tracing adjacent paths) |

## Problem

"Discuss" and "Ask {lead}" use `href="?talk=…"`, dropping `goal` and `view=board`, so Back lands somewhere else.

## Evidence

- `web/src/routes/[companyId]/work/[workId]/+page.svelte:297-304,` :456, :461

## Suggested direction

Build the talk link from the current URL with `talk` added.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
