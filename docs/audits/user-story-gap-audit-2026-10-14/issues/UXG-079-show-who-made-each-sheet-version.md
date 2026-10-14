# UXG-079 — Show who made each sheet version

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Library | low | ux | US-045, US-052 |

## Problem

Sheet history shows only "Autosave"/title and time; the Version type has no author.

## Evidence

- `web/src/routes/[companyId]/library/sheets/+page.svelte:51-56,` :877-881

## Suggested direction

Carry editors on each sheet version and show them.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
