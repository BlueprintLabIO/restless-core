# UXG-124 — Distinguish a message from a directive in chat

| Surface | Severity | Type | Stories |
|---|---|---|---|
| People | medium | spec-drift | — (found while tracing adjacent paths) |

## Problem

§7.4/§12.4 require distinguishing Message, Feedback and Directive with an explicit directive action. The composer has one Send path.

## Evidence

- `docs/specs/owner-cockpit.md:792-820,` :1335
- `web/src/lib/components/RoomConversation.svelte:1736-1770`

## Suggested direction

Add send-as-directive at least in agent DMs.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
