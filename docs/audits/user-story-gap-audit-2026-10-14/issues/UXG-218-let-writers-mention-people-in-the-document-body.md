# UXG-218 — Let writers @mention people in the document body

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Doc & sheet editors | high | missing-function | US-123 |

## Problem

The schema has a mention node, but it is only parsed back from Restless's own copied markup: typing @ in the body does nothing, and the "/" menu has no person entry. Comments and sheet comments already have an @ picker, so the owner learns the gesture and then finds it missing on the page itself. Plans and briefs cannot name an accountable person or Staff member in place. Distinct from UXG-114 (the chat composer picker for admins and members).

## Evidence

- `web/src/lib/model/document-editor-schema.ts:246-283` (Mention node parses only span[data-native-mention])
- `web/src/lib/components/DocumentBlockMenus.svelte:119-125` (INSERTS has no mention)
- `web/src/lib/model/documents.ts:1096-1135` (mentions built only for comment content)
- `web/src/lib/components/SheetCommentField.svelte:1-4` (comment box offers @ picker)

## Suggested direction

Add an @ suggestion in the editor that inserts the existing mention node from the company people list, and decide whether a body mention reaches For you.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
