# UXG-137 — Show permissions before auto-allowing an added app

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Apps | medium | ux | US-071, US-077 |

## Problem

`addAndAllow` grants Exec's proposed class for every tool as soon as the probe passes — from tiles, the ask box and the chat card — before the owner sees the tool list.

## Evidence

- `web/src/lib/model/connections.ts:213-226`
- `web/src/routes/[companyId]/apps/AppTile.svelte:231-235`
- `web/src/lib/components/AppRequestCards.svelte:62-72`

## Suggested direction

Land on the permission list after adding, or summarise what was allowed.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
