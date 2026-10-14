# UXG-037 — Let an owner give a reason when declining an email mandate

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | low | consistency | — (found while tracing adjacent paths) |

## Problem

Declining an approval offers an optional note; declining an email mandate is a bare button, and its call is an inline fetch instead of going through lib/model.

## Evidence

- `web/src/lib/components/InboxDetail.svelte:308-328,` :640-644

## Suggested direction

Reuse the approval-note path and move the call into lib/model.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
