# UXG-238 — Follow the dark theme in the sheet grid

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Doc & sheet editors | low | consistency | US-136, US-169 |

## Problem

The sheet surface is hard-coded to a white background and dark grey text, so in the cockpit's dark theme a sheet is a bright white panel inside a dark page while documents follow the theme.

## Evidence

- `web/src/lib/components/SheetEditor.svelte:166-174` (background: white
- color: #252525)

## Suggested direction

Map the engine's grid colours to cockpit tokens, or at least frame the grid so the contrast change is deliberate.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
