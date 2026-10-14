# UXG-126 — Show what an agent DM is about before the first message

| Surface | Severity | Type | Stories |
|---|---|---|---|
| People | low | ux | — (found while tracing adjacent paths) |

## Problem

An empty DM says "Nothing said yet. Start the conversation." with no role, team or current Work.

## Evidence

- `web/src/lib/components/RoomConversation.svelte:1555-1559`

## Suggested direction

Use the empty state to show role, current Work and suggested asks.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
