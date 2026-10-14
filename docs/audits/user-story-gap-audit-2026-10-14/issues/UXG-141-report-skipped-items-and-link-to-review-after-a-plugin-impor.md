# UXG-141 — Report skipped items and link to review after a plugin import

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Apps | low | ux | US-079 |

## Problem

The import notice ignores `import.skipped`, and "N skills for you to review" is plain text with no link.

## Evidence

- `web/src/routes/[companyId]/apps/+page.svelte:124-135`
- `web/src/lib/model/connections.ts:103-113`

## Suggested direction

List skipped entries with reasons; link the notice to Waiting on you.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
