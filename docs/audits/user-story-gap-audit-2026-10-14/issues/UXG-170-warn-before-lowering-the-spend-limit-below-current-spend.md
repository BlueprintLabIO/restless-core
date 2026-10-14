# UXG-170 — Warn before lowering the spend limit below current spend

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | medium | ux | US-092 |

## Problem

`saveLimit` accepts any value ≥ 0 and applies optimistically; a limit at or below spend so far stops all model work at once with no warning.

## Evidence

- `web/src/lib/components/CompanyLimits.svelte:113-145,` :200

## Suggested direction

Show the consequence and confirm when new limit ≤ spend.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
