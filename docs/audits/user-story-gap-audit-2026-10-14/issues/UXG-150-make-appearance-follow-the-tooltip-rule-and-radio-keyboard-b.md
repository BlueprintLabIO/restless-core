# UXG-150 — Make Appearance follow the tooltip rule and radio keyboard behaviour

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Apps | low | a11y | — (found while tracing adjacent paths) |

## Problem

Each choice has a visible `<small>` subtitle (against the tooltip rule); role="radio" buttons lack arrow keys; the preference is browser-local only.

## Evidence

- `web/src/lib/components/AccountAppearance.svelte:2-6,` :10-27
- `web/src/lib/theme.svelte.ts:7-16`

## Suggested direction

Tooltip the detail, use native radios, and say "on this device" or sync.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
