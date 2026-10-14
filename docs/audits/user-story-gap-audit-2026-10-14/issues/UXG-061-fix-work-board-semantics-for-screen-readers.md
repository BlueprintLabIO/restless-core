# UXG-061 — Fix Work board semantics for screen readers

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | medium | a11y | — (found while tracing adjacent paths) |

## Problem

The board is a `<div>` with `aria-label` and no role; column names are not headings; each card's `aria-label` overrides its content so owner, signal and revision are never announced.

## Evidence

- `web/src/lib/ui/views/WorkBoard.svelte:38,` :41, :49, :52-56

## Suggested direction

role="region" with headings per column; drop the aria-label override.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
