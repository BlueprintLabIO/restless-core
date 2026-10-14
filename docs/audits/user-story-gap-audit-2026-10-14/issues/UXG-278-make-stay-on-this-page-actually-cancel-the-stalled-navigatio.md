# UXG-278 — Make "Stay on this page" actually cancel the stalled navigation

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Shell & cross-cutting | medium | ux | US-173 |

## Problem

After 12 s the stalled-navigation dialog offers "Stay on this page", but that only closes the dialog; the pending navigation keeps running and can complete later, moving the owner away mid-task despite their choice. The dialog will not reappear for the same destination, so there is no further warning.

## Evidence

- `web/src/routes/+layout.svelte:53-66` (stall timer keyed on pending destination)
- `web/src/routes/+layout.svelte:81-84` (stay only closes the dialog)
- `web/src/routes/+layout.svelte:223-243` (dialog copy promises staying)

## Suggested direction

On "Stay", abort the in-flight navigation (e.g. by navigating to the current URL with replaceState) so the choice holds.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
