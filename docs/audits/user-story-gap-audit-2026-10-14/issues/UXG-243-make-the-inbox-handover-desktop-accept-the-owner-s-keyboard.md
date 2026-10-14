# UXG-243 — Make the Inbox handover desktop accept the owner's keyboard and pointer

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company computer & intelligence | high | missing-function | US-139 |

## Problem

The Inbox renders the prepared computer with DesktopViewport but passes neither interactive, canResize nor onclaim, and both default to false. DesktopViewport sets noVNC viewOnly to true whenever both are false, so after "Take control" succeeds and the URL switches to the controlled route, the client still sends no input. The flagship prepared-last-mile flow (sign-in, CAPTCHA, payment confirmation brought to the CEO) cannot be completed in the place it is presented; only the separate Company → Computer page wires input correctly.

## Evidence

- `web/src/routes/[companyId]/+page.svelte:936-942` (DesktopViewport without interactive/canResize/onclaim)
- `web/src/lib/components/DesktopViewport.svelte:32-42` (interactive and canResize default false)
- `web/src/lib/components/DesktopViewport.svelte:218-219` (viewOnly = !(interactive or canResize))
- `web/src/lib/components/DesktopViewport.svelte:320-327`
- `web/src/routes/[companyId]/company/computer/+page.svelte:458-467` (the Computer page passes them)

## Suggested direction

Pass interactive={controller === 'owner'}, canResize and onclaim from the handover, or reuse one shared desktop controller for both views.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
