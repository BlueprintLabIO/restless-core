# UXG-263 — Move Telegram out of the Limits page

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company computer & intelligence | low | consistency | US-153 |

## Problem

The Limits page (route /resources) carries spend, authority, tools and computer limits, and also the Telegram channel. Telegram is a way the company reaches the owner, not a limit, so owners looking for it under notifications or Apps do not find it, and its jump link sits among budget sections.

## Evidence

- `web/src/routes/[companyId]/company/resources/+page.svelte:64-84` (Limits page with a Telegram section)
- `web/src/lib/components/TelegramChannel.svelte:89-91`

## Suggested direction

Place Telegram with other owner channels (notifications or Apps) and keep a redirect anchor.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
