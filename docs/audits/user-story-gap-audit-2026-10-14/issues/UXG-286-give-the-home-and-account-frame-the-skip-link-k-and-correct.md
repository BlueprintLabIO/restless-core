# UXG-286 — Give the Home and Account frame the skip link, ⌘K and correct menu semantics the company shell has

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Shell & cross-cutting | low | a11y | US-168 |

## Problem

The company shell has a skip link and ⌘K, but the portfolio and Account pages use a separate frame with neither, so keyboard users must tab through the whole rail on every visit and cannot jump into a company document from Home. The account menu declares role="menu" while containing plain links and a form, without menuitem roles or arrow-key handling, which screen readers announce misleadingly.

## Evidence

- `web/src/lib/components/AppShell.svelte:287` (skip link exists in company shell)
- `web/src/lib/ui/views/AccountShell.svelte:164-262` (no skip link, no command menu)
- `web/src/lib/ui/views/AccountShell.svelte:229` (role=menu container)
- `web/src/lib/components/AccountFrame.svelte:164-173` (plain links and form inside it)

## Suggested direction

Add a skip link and a portfolio-level ⌘K to AccountShell and drop role="menu" in favour of a plain disclosure list.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
