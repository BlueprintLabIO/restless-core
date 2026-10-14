# UXG-134 — Show removed skills with Restore, and show the skill's version

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Apps | medium | missing-function | US-081 |

## Problem

`buildApps` hides every retired skill, so Remove (no confirmation) makes it vanish. The API accepts re-accepting but nothing calls it; no version history or rollback.

## Evidence

- `web/src/lib/model/apps.ts:389`
- `web/src/routes/[companyId]/apps/[app]/+page.svelte:186-199,` :423-429
- `web/src/lib/model/skills.ts:303-311`

## Suggested direction

Add a "Removed" filter with Restore; show source commit/digest.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
