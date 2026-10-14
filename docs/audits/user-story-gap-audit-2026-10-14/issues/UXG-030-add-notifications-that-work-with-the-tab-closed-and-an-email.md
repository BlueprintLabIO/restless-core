# UXG-030 — Add notifications that work with the tab closed, and an email preference

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | medium | missing-function | US-021 |

## Problem

Desktop notifications fire only from page JS while a tab is open. The service worker has no push handler and the setting is per device. Email delivery exists via Cloud but the owner has no setting for it.

## Evidence

- `web/src/lib/model/desktop-notify.svelte.ts:66-67`
- `web/src/service-worker.ts` (no push/notificationclick)
- `crates/restless-owner/src/owner_notifications.rs:1-7,` :218-226

## Suggested direction

Add an owner notification preference (email digest on/off, time) and web push.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
