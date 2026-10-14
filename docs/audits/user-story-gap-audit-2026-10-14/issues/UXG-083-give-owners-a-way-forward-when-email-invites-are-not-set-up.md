# UXG-083 — Give owners a way forward when email invites are not set up

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Library | medium | ux | US-046 |

## Problem

In local mode, inviting from Share throws "Turn on sharing under Company, Members…" as plain red text with no link; the setup it points to asks for server origins and a JSON download.

## Evidence

- `web/src/lib/model/members.ts:136-137`
- `web/src/lib/components/SharingSetup.svelte:28-40`

## Suggested direction

Hide email invite until it works, or link to setup with plain-language guidance.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
