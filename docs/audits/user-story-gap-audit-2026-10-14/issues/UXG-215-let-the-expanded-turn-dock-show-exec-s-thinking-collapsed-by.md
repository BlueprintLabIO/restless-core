# UXG-215 — Let the expanded turn dock show Exec's thinking, collapsed by default

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Exec chat | low | spec-drift | US-120 |

## Problem

The live dock drops every "thinking" activity item before rendering, even when the owner expands live notes. The spec's default pattern says thinking and tool activity are collapsed by default, not removed. During a long turn the owner cannot see why Exec is heading somewhere in time to redirect it.

## Evidence

- `web/src/lib/primitives/ConversationTurnDock.svelte:32-34` (filter kind !== 'thinking')
- `web/src/lib/primitives/ConversationTurnDock.svelte:227-248` (expanded timeline uses filtered list)
- `docs/specs/owner-cockpit.md:877-879` (thinking collapsed by default)

## Suggested direction

Include thinking items in the expanded timeline as folded rows, never in the collapsed summary.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
