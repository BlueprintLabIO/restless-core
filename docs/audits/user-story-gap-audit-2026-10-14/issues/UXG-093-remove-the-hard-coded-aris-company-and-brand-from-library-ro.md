# UXG-093 — Remove the hard-coded "aris" company and brand from Library routes

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Library | low | consistency | — (found while tracing adjacent paths) |

## Problem

The knowledge compatibility route only works for `aris` and titles the page "Opening Aris sales leads — Restless"; library pages fall back to `'aris'`. Both break brand-neutral code.

## Evidence

- `web/src/routes/company/knowledge/[...filePath]/+page.svelte:9-10,` :27, :33, :53
- `web/src/routes/[companyId]/library/+page.svelte:99`
- `web/src/routes/[companyId]/library/documents/+page.svelte:55`

## Suggested direction

Resolve company from the link/session and brand from config.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
