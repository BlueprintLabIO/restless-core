# UXG-139 — Show why Exec wants an app in the chat card without hovering

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Apps | medium | a11y | US-077 |

## Problem

The rail card shows only "Exec wants X"; the reason and Work title are in a `title` attribute, invisible on touch.

## Evidence

- `web/src/lib/components/AppRequestCards.svelte:85-89`

## Suggested direction

Show the reason as visible secondary text or a disclosure.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
