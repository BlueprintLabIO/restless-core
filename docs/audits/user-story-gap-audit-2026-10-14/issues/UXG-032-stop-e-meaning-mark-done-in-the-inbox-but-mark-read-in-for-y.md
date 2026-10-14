# UXG-032 — Stop E meaning "mark done" in the Inbox but "mark read" in For you

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | medium | ux | US-022 |

## Problem

J walks from the last Inbox item straight into For you. E is harmless "mark read" there but irreversible "mark done" (resumes work) in the Inbox, so muscle memory misfires.

## Evidence

- `web/src/lib/components/InboxList.svelte:253-256,` :267-270
- `web/src/lib/components/ForYou.svelte:96-98`

## Suggested direction

Use different keys for the irreversible action, or confirm it.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
