# UXG-161 — Cover all six surfaces in the tour and make it keyboard-friendly and replayable

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | low | a11y | — (found while tracing adjacent paths) |

## Problem

The tour skips People and Company, takes no focus, has no Escape/Back, writes a localStorage key nothing reads, and cannot be replayed.

## Evidence

- `web/src/lib/components/WelcomeTour.svelte:11-37,` :52, :66

## Suggested direction

Add the two steps, focus handling, Escape/Back, and "Show the tour again" in ⌘K.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
