# UXG-214 — Show the current model on the composer chip and allow effort to be chosen there

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Exec chat | low | ux | US-117 |

## Problem

The model chip is a bare brain icon; which model and effort Exec uses is only in a hover title, unreachable on touch. The menu shows effort but offers no way to change it, sending the owner to Company settings for the most common cost-versus-quality trade. Distinct from UXG-103 (confirming the change) and UXG-143 (technical wording).

## Evidence

- `web/src/lib/components/IntelligenceChip.svelte:84-89` (hover title, icon-only trigger)
- `web/src/lib/components/IntelligenceChip.svelte:91-95` (effort shown read-only in the menu)
- `web/src/lib/components/IntelligenceChip.svelte:127` (link out to all settings)

## Suggested direction

Show a short model name on the chip and add effort choices to its menu.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
