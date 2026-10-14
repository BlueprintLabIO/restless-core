# UXG-127 — Add Google (Gmail/Calendar/Drive), Microsoft and accounting services to the Apps catalogue

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Apps | high | missing-function | US-070 |

## Problem

The catalogue has 26 entries (Slack, Linear, Stripe, Figma…) but no Google, Microsoft, QuickBooks or any email service. "Connect my email" — the most common first ask — ends in a free-text Exec request or an MCP URL paste.

## Evidence

- `web/src/lib/model/apps.ts:65-277` (no google/gmail/outlook/quickbooks keys)

## Suggested direction

Add probed Google/Microsoft/accounting entries, or route "email" in the ask box to the real email path.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
