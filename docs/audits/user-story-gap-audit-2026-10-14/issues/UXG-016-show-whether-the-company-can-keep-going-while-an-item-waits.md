# UXG-016 — Show whether the company can keep going while an item waits

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | medium | spec-drift | US-008 |

## Problem

The spec says every item states whether work continues while it waits. `canContinue` is on the model but never rendered, and "If nothing happens" is buried in the Evidence fold.

## Evidence

- `web/src/lib/model/view.ts:102` (no .svelte reference to canContinue)
- `web/src/lib/components/InboxDetail.svelte:480-482`
- `docs/specs/owner-cockpit.md` §5.5

## Suggested direction

Show a one-line "Work is paused / keeps going" next to the deadline.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
