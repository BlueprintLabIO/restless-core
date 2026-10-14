# UXG-156 — Let the owner name the company when creating it

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | medium | ux | US-085 |

## Problem

"Start a company" instantly creates a company with a random two-word name and opaque id, and starts its computer, with no confirm or name field. One stray click leaves a randomly named company running.

## Evidence

- `web/src/lib/components/CreateCompany.svelte:7-36,` :47-50

## Suggested direction

Ask for an optional name (or confirm the generated one) before creating.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
