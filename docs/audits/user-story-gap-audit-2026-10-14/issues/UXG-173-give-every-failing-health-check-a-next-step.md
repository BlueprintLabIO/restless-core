# UXG-173 — Give every failing health check a next step

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | medium | ux | US-094 |

## Problem

Only the intelligence check and one "repair" check get an action; other failing checks show a summary and nothing to do.

## Evidence

- `web/src/routes/[companyId]/company/doctor/+page.svelte:81-84,` :186-199

## Suggested direction

Every failing check offers a bounded action or "Ask Exec to look into this".

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
