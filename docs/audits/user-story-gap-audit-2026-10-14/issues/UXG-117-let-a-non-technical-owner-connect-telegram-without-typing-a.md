# UXG-117 — Let a non-technical owner connect Telegram without typing a Vault reference

| Surface | Severity | Type | Stories |
|---|---|---|---|
| People | high | ux | US-068 |

## Problem

Pairing requires creating a BotFather bot, storing its token in the Vault, then typing `infisical:/companies/{id}/telegram-bot` or `env:TELEGRAM_BOT_TOKEN`. That is developer configuration, and it sits in Company → Resources rather than near Inbox/People.

## Evidence

- `web/src/lib/components/TelegramChannel.svelte:128-149`
- `web/src/routes/[companyId]/company/resources/+page.svelte:10`

## Suggested direction

Accept a pasted bot token stored server-side, or let Exec prepare the bot and hand over only pairing.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
