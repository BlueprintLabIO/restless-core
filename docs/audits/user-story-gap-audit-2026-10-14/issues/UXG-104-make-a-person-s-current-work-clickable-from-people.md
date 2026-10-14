# UXG-104 — Make a person's current Work clickable from People

| Surface | Severity | Type | Stories |
|---|---|---|---|
| People | medium | ux | US-061 |

## Problem

The directory shows "Blocked: <title>" but the whole row links to the DM; the office person-detail already offers "Open Work".

## Evidence

- `web/src/routes/[companyId]/people/+page.svelte:200-210,` :552-579
- `web/src/lib/office/OfficeCanvas.svelte:1816-1818`

## Suggested direction

Show the Work title as a secondary link or in the profile pane.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
