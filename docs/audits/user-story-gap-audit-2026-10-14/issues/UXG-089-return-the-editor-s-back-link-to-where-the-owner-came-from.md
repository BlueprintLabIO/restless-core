# UXG-089 — Return the editor's back link to where the owner came from

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Library | low | ux | US-039 |

## Problem

LibraryCrumb always links to `?show=docs` / `?show=sheets`, dropping review/search/pinned/sort context.

## Evidence

- `web/src/lib/components/LibraryCrumb.svelte:7-12`

## Suggested direction

Carry the originating Library view in the link or history.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
