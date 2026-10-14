# UXG-279 — Ignore Enter while an IME is composing in ⌘K and other pickers

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Shell & cross-cutting | medium | a11y | US-168 |

## Problem

The command menu, the Apps ask box, the share picker, conversation search and sheet rename act on plain Enter without checking event.isComposing. People typing Japanese, Chinese or Korean press Enter to confirm a conversion, which here opens the highlighted result or submits instead. The main composer already handles this correctly, so behaviour differs by box.

## Evidence

- `web/src/lib/components/CommandMenu.svelte:206-217` (Enter chooses with no composition check)
- `web/src/routes/[companyId]/apps/+page.svelte:185-193` (ask box Enter)
- `web/src/lib/components/SharePanel.svelte:148-154` (picker Enter)
- `web/src/lib/primitives/ConversationHistoryTools.svelte:58-61` (search Enter)
- `web/src/lib/primitives/composer-keys.ts:14-29` (composer does handle isComposing)

## Suggested direction

Share the composer’s composition guard and apply it to every Enter-to-act handler.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
