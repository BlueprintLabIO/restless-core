# UXG-148 — Name model "Connections" differently from app connections

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Apps | medium | consistency | — (found while tracing adjacent paths) |

## Problem

Account → "Connections" means AI model providers only, while Apps "connections" are services like Stripe; owners looking for Stripe at account level find only Anthropic/OpenAI.

## Evidence

- `web/src/lib/components/AccountSectionPage.svelte:18-24`
- `web/src/lib/components/CompanyProvider.svelte:613-615`
- `web/src/lib/model/connections.ts:1-3`

## Suggested direction

Rename the account section to "AI accounts" or similar.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
