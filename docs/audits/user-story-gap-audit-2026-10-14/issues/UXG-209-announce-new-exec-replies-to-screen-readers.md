# UXG-209 — Announce new Exec replies to screen readers

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Exec chat | medium | a11y | US-113 |

## Problem

The transcript container has no log role or live region, and the turn dock's live status only announces phase words such as "Replied". A screen-reader user hears that Exec replied but not the reply, and must navigate back into the transcript each time.

## Evidence

- `web/src/lib/components/ExecutiveRail.svelte:1088-1098` (exr-msgs has no role="log"/aria-live)
- `web/src/lib/primitives/ConversationTurnDock.svelte:190-191` (status-only live region)
- `web/src/lib/primitives/ConversationTurnDock.svelte:213-220` (streamed reply not announced)

## Suggested direction

Mark the transcript as a polite log (or announce the durable reply's takeaway once it lands).

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
