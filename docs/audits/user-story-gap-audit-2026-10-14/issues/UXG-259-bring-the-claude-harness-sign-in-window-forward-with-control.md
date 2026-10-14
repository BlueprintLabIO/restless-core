# UXG-259 — Bring the Claude harness sign-in window forward, with control, instead of opening the bare desktop

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company computer & intelligence | medium | ux | US-151 |

## Problem

"Continue Claude sign-in in company desktop" links to the generic computer focus. The page attaches as an observer with the generic runtime-rescue item, does not bring the sign-in window forward (the app switcher appears only after taking control) and gives no hint of which window to look for. This is the prepared-last-mile case, yet the owner is handed a desktop and left to find the step.

## Evidence

- `web/src/lib/components/HarnessConnections.svelte:186-190` (link to computer?focus=desktop)
- `web/src/routes/[companyId]/company/computer/+page.svelte:102` (attach on focus=desktop)
- `web/src/routes/[companyId]/company/computer/+page.svelte:161` (ticket for 'runtime-rescue')
- `web/src/routes/[companyId]/company/computer/+page.svelte:405-426` (switcher only for the controller)

## Suggested direction

Have the sign-in link focus the sign-in window, take control for this tab and say what to do there.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
