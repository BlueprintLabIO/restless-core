# UXG-103 — Confirm model changes and say when they take effect

| Surface | Severity | Type | Stories |
|---|---|---|---|
| People | low | ux | US-060 |

## Problem

Choosing a model sends a PUT and silently refreshes; "applies to its next session" is only a code comment. Thinking effort is shown but not changeable; the comment says admins may choose but code gates on owner.

## Evidence

- `web/src/lib/components/IntelligenceChip.svelte:2-4,` :20-23, :49-76, :92-94

## Suggested direction

Toast "X will use Y from their next session"; align comment and gate.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
