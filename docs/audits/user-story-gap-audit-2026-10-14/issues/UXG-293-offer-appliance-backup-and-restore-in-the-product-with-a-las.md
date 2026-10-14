# UXG-293 — Offer appliance backup and restore in the product, with a last-backup indicator

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Endpoint & CLI parity | high | missing-function | US-180 |

## Problem

A whole-appliance backup (state root, plane and company databases, every company volume) and its restore onto a fresh install exist only as `restless appliance backup` / `restless appliance restore`. Nothing in the cockpit mentions backup. There is no schedule, no reminder and no "last backed up" fact. For a non-technical owner whose business runs on one laptop, the only protection against losing everything is a command they will never learn about. Distinct from UXG-182, which asks for an export of one company's data. This one is disaster recovery for the whole appliance.

## Evidence

- `crates/restless/src/main.rs:596-613` (Backup / Restore subcommands
- the archive holds secrets)
- `crates/restless/src/main.rs:2379-2388` (dispatch)
- `crates/restless/src/appliance_backup.rs:83` (backup)
- `crates/restless/src/appliance_backup.rs:198` (restore)
- `web/src/lib/model/appliance.ts:1-17` (appliance status carries no backup information)

## Suggested direction

Add a Backup section to the account page that runs and downloads a backup, shows when the last one was made, and explains restore on a new machine. Consider a scheduled local backup later.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
