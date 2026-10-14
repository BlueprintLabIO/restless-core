# UXG-003 — Let the owner amend an Exec identity proposal before promoting, and make the reason optional

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Brand & voice | medium | missing-function | US-004 |

## Problem

A pending proposal can only be promoted or rejected whole, and both buttons stay disabled until the owner types a reason. To fix one word the owner must reject, wait for a redraft, or promote and then live with it (see UXG-001).

## Evidence

- `web/src/routes/[companyId]/company/identity/+page.svelte:225-258` (statements read-only, buttons disabled on empty decisionText)
- `web/src/lib/model/identity.ts:59-95` (only promote/reject endpoints)

## Suggested direction

Open the proposal in the editor as suggested changes the owner can edit, accept partially, then promote; default the change note.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
