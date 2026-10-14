# UXG-280 — Use the shared date, time and money formatters everywhere

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Shell & cross-cutting | low | consistency | US-170 |

## Problem

time.ts declares that raw toLocaleString never reaches a screen, yet app activity, sheet version names and AI app tokens use it ("14/10/2026, 09:12:44"); one message component shows times in the locale’s 12-hour form in its header and forced 24-hour in its gutter; and relative times render as a bare "5m", which reads as a duration rather than "5m ago". Spend is written with two different currency displays (company overview vs Limits), so the same budget can read "US$12.50" and "$12.50" for non-US locales.

## Evidence

- `web/src/lib/ui/time.ts:1-4` (one way to write a date)
- `web/src/lib/ui/time.ts:42-55` (past times without "ago")
- `web/src/routes/[companyId]/apps/[app]/+page.svelte:357` (raw toLocaleString)
- `web/src/routes/[companyId]/library/sheets/+page.svelte:518` (raw toLocaleString in version names)
- `web/src/lib/components/AccountAiApps.svelte:75` (raw date)
- `web/src/lib/primitives/ConversationMessage.svelte:71` (locale hour form)
- `web/src/lib/primitives/ConversationMessage.svelte:86` (hour12 false)
- `web/src/routes/[companyId]/company/+page.svelte:15-16` (default currency display)
- `web/src/lib/components/CompanyLimits.svelte:173-178` (narrowSymbol)

## Suggested direction

Route all timestamps through formatMoment/formatRelative and add one shared formatMoney used by every spend figure.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
