# UXG-146 — Show "couldn't load" instead of hiding skills or saying "not found"

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Apps | medium | ux | US-071 |

## Problem

A failed skills fetch is silently ignored and app requests fall back to []; on the app page a failed fetch renders "This app is not here. It may have been removed."

## Evidence

- `web/src/lib/model/apps-data.svelte.ts:50-53`
- `web/src/routes/[companyId]/apps/[app]/+page.svelte:57-60,` :438-441

## Suggested direction

Partial-failure notice with retry; never "not found" on a failed read.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
