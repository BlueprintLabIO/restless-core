# UXG-294 — Show the running release and allow update and rollback from the cockpit

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Endpoint & CLI parity | medium | missing-function | US-181 |

## Problem

Upgrading the appliance (drain, stage, activate, roll back automatically on failed readiness), rolling back to the previous release, and checking service status are CLI-only. The cockpit's appliance status has no version, no "update available" and no rollback. When the server reports "The last appliance upgrade was blocked… retry or roll back", the owner cannot do either from the page that tells them.

## Evidence

- `crates/restless/src/main.rs:550-573` (Upgrade / Rollback / Status)
- `crates/restless/src/main.rs:2313-2343` (dispatch)
- `crates/restless-owner/src/owner.rs:6145` (repair text "retry or roll back")
- `web/src/lib/model/appliance.ts:1-17` (ApplianceStatus has no release or version field)

## Suggested direction

Add the current release and the previous known-good release to appliance status, and offer Update and Roll back with the existing drain and confirmation semantics.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
