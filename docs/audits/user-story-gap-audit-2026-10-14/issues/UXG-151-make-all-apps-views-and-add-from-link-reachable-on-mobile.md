# UXG-151 — Make all Apps views and "add from link" reachable on mobile

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Apps | medium | mobile | US-084 |

## Problem

At narrow widths the sidebar (add-from-link, Skills/Services/Local tools, eight categories) becomes four chips; categories are unreachable.

## Evidence

- `web/src/routes/[companyId]/apps/+layout.svelte:83-99,` :156-166

## Suggested direction

Add a "More" chip/menu for kinds, categories and add-from-link.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
