# UXG-292 — Let the owner invite, revoke and stop a live publication without the terminal

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Endpoint & CLI parity | medium | missing-function | US-179 |

## Problem

After a build is published, every control over who can reach it is CLI-only: minting a scoped expiring invitation, revoking one immediately, stopping the provider and checking cleanup, and listing publications. The web cockpit has no publication list and no owner route exists for these commands. An owner who needs to cut off access to something the company put online, which is a time-sensitive safety action, has to find the publication id and run a command.

## Evidence

- `crates/restless/src/main.rs:670-710` (publish invite / revoke / observe / reconcile / stop / show / list)
- `crates/restless/src/main.rs:2699-2721` (CLI commands sent to the engine)
- `crates/restless-engine/src/wire.rs:1306-1310` (publish-invite, publish-revoke, publish-stop are OWNER_ONLY)
- `crates/restless-owner/src/owner.rs:1271-1661` (no publication route in the owner API)

## Suggested direction

Give each published resource a detail view with its live invitations, Revoke per invitation and a confirmed Stop publishing, backed by owner routes over the existing engine commands.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
