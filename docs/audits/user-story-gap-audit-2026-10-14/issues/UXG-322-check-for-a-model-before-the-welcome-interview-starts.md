# UXG-322 — Check for a model before the welcome interview starts

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Cross-surface journeys | medium | ux | US-195 |

## Problem

The welcome page shows a static first question and accepts an answer before checking whether Exec can think. "Connect one" appears only after the owner has typed. The link then leaves the full-screen welcome for Company → Intelligence, which has no way back, even though the copy promises "I'll pick up right here". Distinct from UXG-160 (general re-entry to an unfinished welcome), because the order of the check and this particular detour are the problem.

## Evidence

- `web/src/routes/[companyId]/welcome/+page.svelte:245-252` ("Connect one" shown only while waiting after an answer)
- `web/src/routes/[companyId]/welcome/+page.svelte:254-258` (static first question regardless of cannotStart)
- `web/src/routes/[companyId]/company/provider/+page.svelte:1-9` (no return to welcome)
- `web/src/lib/components/CreateCompany.svelte:57` (only route into welcome)

## Suggested direction

When no model is connected, put the connect step inline before the first question, and return to the welcome once the route is ready.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
