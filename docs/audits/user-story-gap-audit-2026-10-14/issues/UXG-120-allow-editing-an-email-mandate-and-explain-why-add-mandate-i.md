# UXG-120 — Allow editing an email mandate and explain why Add mandate is disabled

| Surface | Severity | Type | Stories |
|---|---|---|---|
| People | medium | missing-function | — (found while tracing adjacent paths) |

## Problem

A mandate can only be revoked; changing a limit means revoke and re-propose. "Add mandate" is disabled whenever one exists, with no tooltip.

## Evidence

- `web/src/lib/components/EmailMandates.svelte:236-245,` :280-288

## Suggested direction

Add "Propose changes" prefilled from the current mandate; add a disabled-reason tooltip.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
