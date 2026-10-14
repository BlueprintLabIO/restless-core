# UXG-168 — Add "Run now" and rename "Test trigger"

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | low | ux | US-091 |

## Problem

"Test trigger" only checks admission ("No agent or external action ran") and will be misread as running; there is no Run now; only the last three runs are shown.

## Evidence

- `web/src/routes/[companyId]/company/schedules/+page.svelte:181-185,` :289-301
- `crates/restless-owner/src/owner_skills.rs:716`

## Suggested direction

Offer Run now; move the admission test behind a diagnostic label.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
