# UXG-115 — Show readable names in mentions while composing

| Surface | Severity | Type | Stories |
|---|---|---|---|
| People | low | ux | US-066 |

## Problem

Picking a person inserts `@exec-staff-xyz`-style ids into the draft; names render only after sending.

## Evidence

- `web/src/lib/model/composer-options.svelte.ts:97`
- `web/src/lib/components/RoomMessage.svelte:111-119,` :214

## Suggested direction

Render mention tokens as name chips in composer and edit field.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
