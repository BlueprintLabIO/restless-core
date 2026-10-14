# UXG-157 — Report a failed rename during onboarding instead of dropping it

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | high | ux | US-085 |

## Problem

`begin()` saves the charter then calls `rename()`; a failed `renameInAccount` returns silently and the setup PUT is `.catch(() => null)`. The read-back shows "Acme Bakery" but the company can stay "Hopeful Piano" with no message.

## Evidence

- `web/src/routes/[companyId]/welcome/+page.svelte:173-190`

## Suggested direction

Show a notice with retry, or carry the rename into Company as a to-do.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
