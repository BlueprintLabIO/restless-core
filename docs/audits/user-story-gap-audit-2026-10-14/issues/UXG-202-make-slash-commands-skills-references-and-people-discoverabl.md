# UXG-202 — Make slash commands, skills, # references and @ people discoverable in the composer

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Exec chat | medium | ux | US-108 |

## Problem

The composer's only hint covers Enter and Shift+Enter; nothing visible says "/", "$", "#" or "@" open menus, and "/" opens only at the very start of a message. The menu shows at most 8 entries with no "no matches" state and no way to browse further, and the command descriptions omit the hidden "clear" and "show" forms. Owners who never type "/" never learn /goal or /loop exist.

## Evidence

- `web/src/lib/primitives/Composer.svelte:432-436` (hint mentions only Enter keys)
- `web/src/lib/primitives/composer-menu.ts:16-19` ("/" only at message start)
- `web/src/lib/primitives/composer-menu.ts:60` (.slice(0, 8))
- `web/src/lib/primitives/Composer.svelte:397` (menu hidden when no items)
- `web/src/lib/model/skills.ts:121-136` (descriptions omit clear/show)

## Suggested direction

Add a small composer button listing commands, skills and reference triggers, with scrolling and an empty state.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
