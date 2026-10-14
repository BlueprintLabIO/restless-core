# UXG-054 — Make "Waiting on you" work for team owners (members)

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | medium | consistency | US-038 |

## Problem

`waitingOnOwner` is built only when `ownerAccess`; for members it is empty, yet the view and count still render and always read 0.

## Evidence

- `web/src/routes/[companyId]/work/+page.svelte:209,` :447-456

## Suggested direction

Feed member-assigned handoffs into the set, or hide the view for members.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
