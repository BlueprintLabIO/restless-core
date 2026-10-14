# UXG-275 — Say when ⌘K search could not reach a source instead of "No match"

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Shell & cross-cutting | medium | ux | US-157 |

## Problem

Every server search behind ⌘K (documents, sheets, comments, room messages, kept files) swallows its error into an empty result, and the menu then prints "No match for “…”". During an outage or for a source that is failing, the owner is told the thing does not exist, which is wrong and may lead them to recreate it. The connection banner does not cover this because these calls are not query-cache reads.

## Evidence

- `web/src/routes/[companyId]/+layout.svelte:1222-1228` (each search .catch to null or [])
- `web/src/routes/[companyId]/+layout.svelte:1196-1209` (list failures become empty lists)
- `web/src/lib/components/CommandMenu.svelte:74-76` (rejection becomes no results)
- `web/src/lib/components/CommandMenu.svelte:296-299` (No match copy)

## Suggested direction

Return partial-failure information from the search callback and show "Couldn’t search documents right now" beneath the results.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
