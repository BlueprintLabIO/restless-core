# UXG-300 — Let the owner create, rename, re-lead and disband teams

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Endpoint & CLI parity | medium | missing-function | US-187 |

## Problem

The CLI lets an owner form a team around an accountable lead, rename it or revise its charter, replace its lead, move members and disband it, with owed judgement falling to Exec. The People page only lets the owner set which person owns a team. Asking Exec in chat is possible, but there is no direct control when the owner disagrees with the organisation Exec built. Distinct from UXG-048 (reassigning Work and Goals) and UXG-101 (hiring and retiring individual AI staff).

## Evidence

- `crates/restless/src/main.rs:1231-1330` (TeamCommand List / Create / Update / Assign / Owner / Lead / Disband)
- `crates/restless/src/main.rs:3306-3375` (dispatch)
- `web/src/routes/[companyId]/people/+page.svelte:648-656` (team directory head offers only the owner select)
- `crates/restless-owner/src/owner.rs:1351-1354` (only teams/{team}/owner is routed)

## Suggested direction

Add a team menu in People with Rename, Change lead, Disband (confirmed) and New team, backed by owner routes over the existing team commands.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
