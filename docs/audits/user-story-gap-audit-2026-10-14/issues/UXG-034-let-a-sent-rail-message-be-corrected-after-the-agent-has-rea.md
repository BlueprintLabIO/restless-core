# UXG-034 — Let a sent rail message be corrected after the agent has read it

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | low | missing-function | — (found while tracing adjacent paths) |

## Problem

Edit and Retract appear only while the last message is unread and nothing is running. After that a wrong instruction can only be followed by another message.

## Evidence

- `web/src/lib/components/ExecutiveRail.svelte:549-559,` ~1293-1300

## Suggested direction

Allow "correct" as a follow-up flagged as superseding the earlier one.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
