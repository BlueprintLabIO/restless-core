# UXG-198 — Check attachment size, count and type before sending from the rail, and show upload progress

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Exec chat | medium | consistency | US-106 |

## Problem

The rail's file picker states "up to 6, 5 MB each" and "images, PDFs, Word, Excel or PowerPoint" but accepts any file of any size and count, and the rail sends without checking, so the owner learns of a refusal only after the upload. People rooms check size and count first and show upload progress; the rail does neither. Distinct from UXG-080 (converting uploads) and Library upload issues.

## Evidence

- `web/src/lib/primitives/AttachmentPicker.svelte:18-22` (adds all picked files)
- `web/src/lib/primitives/AttachmentPicker.svelte:53-62` (limit text in title, no accept attribute)
- `web/src/lib/components/ExecutiveRail.svelte:809-848` (no pre-check before onask)
- `web/src/lib/components/RoomConversation.svelte:1013-1017` (room pre-checks size/count)
- `web/src/lib/components/RoomConversation.svelte:1808` (room upload progress)

## Suggested direction

Reuse the room pre-check and progress line in the rail, and set an accept filter matching the stated types.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
