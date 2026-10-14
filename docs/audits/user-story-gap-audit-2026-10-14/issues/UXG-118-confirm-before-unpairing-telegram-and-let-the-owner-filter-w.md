# UXG-118 — Confirm before unpairing Telegram and let the owner filter what it sends

| Surface | Severity | Type | Stories |
|---|---|---|---|
| People | low | ux | US-068 |

## Problem

"Unpair" deletes immediately; every Attention item is sent with no choice of kinds.

## Evidence

- `web/src/lib/components/TelegramChannel.svelte:89-92,` :105-110

## Suggested direction

Add a confirm dialog and an optional "urgent only" filter.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
