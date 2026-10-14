# UXG-197 — Let the owner write and leave a message while Exec is unreachable

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Exec chat | medium | spec-drift | US-109 |

## Problem

When intelligence needs connecting the composer is replaced by a connect button, and whenever the connection status is anything but "available" (including the transient reconnecting state) the composer textarea is disabled so the owner cannot even type a draft. The spec requires the cockpit to preserve owner chat and directives for later delivery when the Runtime is unavailable. A teammate without fix rights sees no composer at all.

## Evidence

- `web/src/lib/components/ExecutiveRail.svelte:1321-1336` (needsProvider replaces the composer)
- `web/src/lib/components/ExecutiveRail.svelte:1366-1370` (disabled unless connectionStatus === 'available')
- `web/src/lib/components/ExecutiveRail.svelte:809-813` (deliver refuses unless available)
- `docs/specs/owner-cockpit.md:1385` ("preserve owner chat/directives for later delivery")

## Suggested direction

Keep the composer usable and queue the message server-side with a clear "will be read when Exec is back" receipt.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
