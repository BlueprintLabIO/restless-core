# UXG-018 — Stop single-keystroke approvals of direction requests

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | high | ux | US-016, US-022 |

## Problem

A window-wide key listener approves on `Y`, or approves and grants standing permission on `A`, with no confirmation and no undo. It skips only text fields, so a stray key while a rail button or menu has focus approves silently. Other approvals need hold-to-confirm.

## Evidence

- `web/src/lib/components/DirectionRequestDetail.svelte:66-83,` :155-164

## Suggested direction

Scope keys to focus inside the request, put A behind a hold/confirm, and offer undo.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
