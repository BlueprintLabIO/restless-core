# UXG-129 — Confirm before removing an app and say what loses access

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Apps | high | ux | US-074 |

## Problem

Remove revokes grants, deletes the stored sign-in and navigates away in one click, with no confirmation, impact list or undo. Account Connections already uses confirmAction for the same kind of action.

## Evidence

- `web/src/routes/[companyId]/apps/[app]/+page.svelte:456-471`
- `web/src/lib/components/AccountConnections.svelte:252-258`

## Suggested direction

Use confirmAction listing agents/Work using the app.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
