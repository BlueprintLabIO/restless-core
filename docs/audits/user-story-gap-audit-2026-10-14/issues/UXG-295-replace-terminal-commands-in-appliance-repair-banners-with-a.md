# UXG-295 — Replace terminal commands in appliance repair banners with actions

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Endpoint & CLI parity | medium | ux | US-182 |

## Problem

The home page shows appliance banners for recovering, draining and degraded states, and their text is server-written repair advice that tells the owner to run `restless appliance resume`, `start` or `install`. The banner has no button. Distinct from UXG-173 (health checks need a next step) because these are plane-level lifecycle states on the company list, and the server already knows the exact command it wants run. The owner is handed a shell instruction instead of the prepared last mile.

## Evidence

- `web/src/routes/+page.svelte:210-224` (banners render appliance.repair text only)
- `crates/restless-owner/src/owner.rs:6077` (Run `restless appliance start`)
- `crates/restless-owner/src/owner.rs:6115` (Run `restless appliance install`)
- `crates/restless-owner/src/owner.rs:6153` (Run `restless appliance resume`)
- `crates/restless-engine/src/wire.rs:1311-1313` (schedule-wake and appliance-resume are owner commands)

## Suggested direction

Expose resume and schedule-wake repair as owner actions and show a Repair button in the banner. Keep text guidance only where a host-level install is truly needed.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
