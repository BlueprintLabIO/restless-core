# UXG-099 — Confirm or offer Undo for cancelling invitations and pausing access

| Surface | Severity | Type | Stories |
|---|---|---|---|
| People | medium | ux | US-056 |

## Problem

"Cancel invitation" and "Pause access" fire immediately with no confirmation and no Undo, unlike role changes (Undo) and Remove (inline confirm).

## Evidence

- `web/src/routes/[companyId]/company/members/+page.svelte:150-152,` :389-397, :457-465

## Suggested direction

Pass undo (reinstate) for pause; add a confirm step for cancel.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
