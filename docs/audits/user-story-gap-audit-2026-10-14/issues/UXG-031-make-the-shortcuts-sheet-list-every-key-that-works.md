# UXG-031 — Make the shortcuts sheet list every key that works

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | low | a11y | US-022 |

## Problem

`?` omits `G then A`, Inbox `C`, `⌘.`, and request keys Y/A/N. `R` silently does nothing on approvals, direction requests and document items.

## Evidence

- `web/src/lib/components/ShortcutsDialog.svelte:10-15,` :40-46
- `web/src/lib/components/AppShell.svelte:131-138`
- `web/src/lib/components/InboxList.svelte:244-248,` :271-278

## Suggested direction

Build the sheet from the handler tables; give R a fallback.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
