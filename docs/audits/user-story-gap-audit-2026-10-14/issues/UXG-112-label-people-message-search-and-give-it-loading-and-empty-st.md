# UXG-112 — Label People message search and give it loading and empty states

| Surface | Severity | Type | Stories |
|---|---|---|---|
| People | medium | ux | US-065 |

## Problem

Message hits are appended under people with no heading, no "Searching…", no "no matches", and unknown senders show raw ids. In-room search has all of these.

## Evidence

- `web/src/routes/[companyId]/people/+page.svelte:729-745`
- `web/src/lib/components/RoomConversation.svelte:1405-1443`

## Suggested direction

Add a "Messages" heading, pending and empty states, and a "Someone" fallback.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
