# UXG-237 — Add Print / save as PDF for sheets

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Doc & sheet editors | low | missing-function | US-136 |

## Problem

Documents have "Print or save as PDF" with a print stylesheet; the sheet's More menu has downloads, imports and history but no print. Browser print of the canvas grid prints the cockpit around it. An owner sending a quote or invoice built in a sheet must export to Excel and print from another app.

## Evidence

- `web/src/routes/[companyId]/library/sheets/+page.svelte:660-728` (Sheet actions menu: no print)
- `web/src/routes/[companyId]/library/documents/+page.svelte:232-235` (printDocument)
- `web/src/routes/[companyId]/library/documents/+page.svelte:568` (Print or save as PDF)

## Suggested direction

Add Print / save as PDF for the current tab or selection, rendering the used range as a printable table.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
