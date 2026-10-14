# UXG-179 — Explain what archiving a company does before doing it

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | medium | ux | US-097 |

## Problem

"Archive company" runs on one click; the backend stops the runtime so work and schedules stop; the toast never says so.

## Evidence

- `web/src/routes/+page.svelte:83-97,` :191
- `crates/restless-engine/src/runtime.rs:1154-1160`

## Suggested direction

Confirm the consequence ("Exec and schedules stop; files and history are kept").

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
