# UXG-125 — Expose or delete unused people/room endpoints

| Surface | Severity | Type | Stories |
|---|---|---|---|
| People | low | consistency | — (found while tracing adjacent paths) |

## Problem

`GET /companies/{company}/mentions`, `GET /teams/{team}/owner` (owner history) and `/rooms/search` have no web caller.

## Evidence

- `crates/restless-owner/src/owner.rs:1067,` :1348-1354

## Suggested direction

Wire mentions and team-owner history into the UI, or remove the routes.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
