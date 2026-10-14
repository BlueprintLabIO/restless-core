# UXG-092 — Let the requester choose a reviewer, and use plain language in Review

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Library | medium | ux | US-053 |

## Problem

Requesting a review asks only what to judge, not who. The panel uses a subtitle (against the tooltip rule) and jargon like "Accepted version name" and "Latest named version".

## Evidence

- `web/src/lib/components/DocumentInspector.svelte:1532,` :1560, :1614, :1683
- `web/src/lib/components/DocumentEditor.svelte:1255`

## Suggested direction

Add a reviewer picker, move the subtitle into a tooltip, default the version name.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
