# UXG-013 — Make the "Today" line link to its lists

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | medium | spec-drift | US-015, US-193 |

## Problem

The spec says each clause of the Today summary links to its list and applied changes can be undone from there. The component is a plain `<p>`; the `directionApplied` client has no callers, so there is no list of applied changes.

## Evidence

- `web/src/lib/components/DirectionToday.svelte:21-26`
- `web/src/lib/model/direction.ts:58-60`
- `docs/specs/direction-and-permission.md` §4

## Suggested direction

Make each clause a link: requests filter the Inbox; applied changes open a list with Undo.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
