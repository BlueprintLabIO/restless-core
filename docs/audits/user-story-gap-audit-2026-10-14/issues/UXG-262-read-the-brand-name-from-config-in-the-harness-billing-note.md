# UXG-262 — Read the brand name from config in the harness billing note

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company computer & intelligence | low | consistency | US-151 |

## Problem

The harness card's "Credential storage and billing" text hard-codes the product name ("not metered by Restless's relay"). CLAUDE.md requires display names to come from one brand config. Distinct from UXG-188, which covers the model-catalog tooltip.

## Evidence

- `web/src/lib/components/HarnessConnections.svelte:222-228`

## Suggested direction

Use the shared brand display name.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
