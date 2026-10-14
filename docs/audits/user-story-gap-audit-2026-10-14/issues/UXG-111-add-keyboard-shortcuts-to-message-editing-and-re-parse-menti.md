# UXG-111 — Add keyboard shortcuts to message editing and re-parse mentions on edit

| Surface | Severity | Type | Stories |
|---|---|---|---|
| People | medium | a11y | US-064 |

## Problem

The edit textarea has no Esc/⌘↵; edits send only body and revision so new @mentions never notify; revisions render raw `@actor_id`.

## Evidence

- `web/src/lib/components/RoomMessage.svelte:285-299,` :430-440
- `web/src/lib/model/rooms.ts:429-445`

## Suggested direction

Add Esc/⌘↵, send recomputed mentions on PATCH, run withNames on revisions.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
