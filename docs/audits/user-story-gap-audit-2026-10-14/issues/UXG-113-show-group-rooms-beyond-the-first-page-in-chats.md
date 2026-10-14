# UXG-113 — Show group rooms beyond the first page in Chats

| Surface | Severity | Type | Stories |
|---|---|---|---|
| People | medium | missing-function | US-065 |

## Problem

Chats builds from the first 30 rooms and never calls `loadMore`; older groups vanish and Chats search covers only loaded rooms. `searchRooms()` and `/rooms/search` exist unused.

## Evidence

- `web/src/lib/model/rooms.ts:308-319,` :345-358
- `web/src/lib/model/room-queries.svelte.ts:24,` :148-157
- `web/src/routes/[companyId]/people/+page.svelte:436-437`

## Suggested direction

Page Chats on scroll and use /rooms/search for queries.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
