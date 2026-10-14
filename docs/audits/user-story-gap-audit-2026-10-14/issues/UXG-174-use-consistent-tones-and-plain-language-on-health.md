# UXG-174 — Use consistent tones and plain language on Health

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | low | ux | US-094 |

## Problem

Every failing check (including unknown) gets a red dot; "Show raw state" dumps runtime JSON on an owner page; page is "Health" but route/spec say "doctor".

## Evidence

- `web/src/routes/[companyId]/company/doctor/+page.svelte:185,` :236-241

## Suggested direction

Use the existing tone function; move raw JSON behind a "For support" label.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
