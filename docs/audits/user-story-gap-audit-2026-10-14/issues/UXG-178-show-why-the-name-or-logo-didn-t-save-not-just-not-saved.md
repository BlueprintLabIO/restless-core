# UXG-178 — Show why the name or logo didn't save, not just "Not saved"

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | medium | a11y | US-096 |

## Problem

The reason is only in a `title` attribute, unreadable on touch/keyboard.

## Evidence

- `web/src/lib/components/CompanyNameField.svelte:137-150`
- `web/src/lib/components/CompanyLogoField.svelte:123-128`

## Suggested direction

Show the error inline below the field.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
