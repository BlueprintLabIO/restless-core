# UXG-193 — Show Exec's live work even when the owner did not send the last message

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Exec chat | high | missing-function | US-104 |

## Problem

The rail only follows a live turn when the newest message in the conversation is the viewer's own. When Exec is woken by a schedule, a Staff result or a teammate, the rail shows no dock at all, so "what is Exec doing now" is invisible until a finished message appears. This contradicts the spec's chat-context requirement to show whether the actor is currently active.

## Evidence

- `web/src/lib/model/queries.svelte.ts:482-497` (follow() only when last.from_actor === viewer)
- `web/src/lib/model/queries.svelte.ts:531-540` (activeTurn derived from that follow)
- `web/src/lib/components/ExecutiveRail.svelte:1302-1306` (dock rendered only for that turn)
- `docs/specs/owner-cockpit.md:870` ("whether the actor is currently active")

## Suggested direction

Subscribe the rail to the actor's current activity regardless of who triggered it, and show a quiet "Exec is working on …" line.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
