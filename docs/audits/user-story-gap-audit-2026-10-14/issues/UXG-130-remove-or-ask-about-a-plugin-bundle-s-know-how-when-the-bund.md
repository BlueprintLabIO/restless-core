# UXG-130 — Remove (or ask about) a plugin bundle's know-how when the bundle is removed

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Apps | medium | missing-function | US-074 |

## Problem

Remove loops only over `app.connections`; the bundle's skills stay accepted and reappear as separate apps, still followed.

## Evidence

- `web/src/routes/[companyId]/apps/[app]/+page.svelte:465-468`
- `web/src/lib/model/apps.ts:420-425,` :453-458

## Suggested direction

Retire the bundle's skills with its connections, or ask which to keep.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
