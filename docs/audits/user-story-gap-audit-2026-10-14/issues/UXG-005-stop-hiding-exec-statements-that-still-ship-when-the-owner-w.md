# UXG-005 — Stop hiding Exec statements that still ship when the owner writes a pillar

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Brand & voice | high | ux | US-002 |

## Problem

Once the owner writes a pillar, the Direction view shows only the owner's text for that pillar and hides Exec's statements. The backend still includes those hidden statements in the release, so agents receive guidance the owner can no longer see on that row, possibly contradicting what they just wrote.

## Evidence

- `web/src/lib/components/IdentityEditor.svelte:213-222` (`written` replaces `others` in display)
- `crates/restless-orgintel/src/identity.rs:56-59`

## Suggested direction

Show all statements that ship for a pillar, mark conflicts, and let the owner retire the ones their text replaces.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
