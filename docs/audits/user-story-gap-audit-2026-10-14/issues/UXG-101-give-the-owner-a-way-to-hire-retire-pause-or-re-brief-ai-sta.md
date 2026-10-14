# UXG-101 — Give the owner a way to hire, retire, pause or re-brief AI staff

| Surface | Severity | Type | Stories |
|---|---|---|---|
| People | high | missing-function | US-058, US-059 |

## Problem

The owner API has no staff create/retire/rename/brief endpoints and People offers no "ask Exec to…" shortcut, so firing an underperformer or adding a role has no entry point.

## Evidence

- `crates/restless-owner/src/owner.rs:1348-1354`
- `web/src/routes/[companyId]/people/+page.svelte:722`
- `web/src/lib/components/RoomConversation.svelte:1508-1510` (askExec already exists)

## Suggested direction

Add a person menu with "Ask Exec to retire / re-brief / add someone like this" prefilling an Exec draft.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
