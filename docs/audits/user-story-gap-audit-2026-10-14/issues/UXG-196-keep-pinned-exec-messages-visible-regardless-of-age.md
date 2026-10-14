# UXG-196 — Keep pinned Exec messages visible regardless of age

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Exec chat | medium | ux | US-105 |

## Problem

Pins are computed from reactions fetched for only the latest 60 message IDs, intersected with the loaded messages. A message pinned "above the conversation" silently disappears from the pinned bar once 60 newer messages arrive, defeating the point of pinning a standing instruction. The collapsed pin preview also shows the first 80 raw characters, including Markdown syntax.

## Evidence

- `web/src/lib/components/ExecutiveRail.svelte:185-193` (reaction ids .slice(-60))
- `web/src/lib/components/ExecutiveRail.svelte:202-205` (pins filtered from loaded messages)
- `web/src/lib/model/reactions.svelte.ts:47-52` (pinned only from fetched rows)
- `web/src/lib/components/ExecutiveRail.svelte:1053` (raw text.slice(0, 80))

## Suggested direction

Fetch pins for the conversation as their own list (not via recent reactions) and render a plain-text excerpt.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
