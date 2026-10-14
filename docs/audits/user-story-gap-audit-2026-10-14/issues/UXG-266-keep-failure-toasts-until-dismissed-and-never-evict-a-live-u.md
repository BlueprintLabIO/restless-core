# UXG-266 — Keep failure toasts until dismissed and never evict a live Undo

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Shell & cross-cutting | medium | ux | US-166 |

## Problem

Every toast, including a failure, disappears after 6 s (10 s with Undo), and a fourth toast silently drops the oldest even when that one still carries Undo. A failure that says why a save did not happen can vanish while the owner is reading elsewhere, and rapid actions (archiving several items) quietly destroy earlier Undo chances. The toast is the app’s one confirmation channel, so losing it means losing the only record that something failed or could be reversed.

## Evidence

- `web/src/lib/ui/feedback/toast.svelte.ts:12-15` (6 s / 10 s lifetimes, MAX 3)
- `web/src/lib/ui/feedback/toast.svelte.ts:27-34` (failure tone gets the same timer)
- `web/src/lib/ui/feedback/toast.svelte.ts:37-45` (oldest dropped regardless of undo)
- `web/src/lib/ui/feedback/Toasts.svelte:17` (failure is role=alert yet still auto-dismissed)

## Suggested direction

Make failure toasts persistent until dismissed and collapse or queue overflow instead of dropping a toast that still offers Undo.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
