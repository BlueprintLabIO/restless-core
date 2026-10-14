# UXG-255 — Point every "computer is stopped" message at the place that can start it

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company computer & intelligence | low | consistency | US-146 |

## Problem

Opening a link on a stopped computer says "Start it from Company → Computer", but the Computer page has no Start control; every other refusal says Health. The Computer page meanwhile tells an owner whose computer is simply stopped that it "hasn't passed its live check yet", which reads as a fault rather than a state. Owners follow the wrong breadcrumb.

## Evidence

- `crates/restless-owner/src/owner.rs:10803` (says Company → Computer)
- `crates/restless-owner/src/owner_desktop.rs:119` (says Company → Health)
- `crates/restless-owner/src/owner_library.rs:182`
- `web/src/routes/[companyId]/company/computer/+page.svelte:504-511` (live-check copy for any unavailable state)

## Suggested direction

Use one sentence and one destination, and give the Computer page a Start action when the computer is stopped.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
