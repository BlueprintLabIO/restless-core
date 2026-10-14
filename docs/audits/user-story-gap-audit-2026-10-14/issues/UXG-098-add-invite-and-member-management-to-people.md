# UXG-098 — Add invite and member management to People

| Surface | Severity | Type | Stories |
|---|---|---|---|
| People | medium | consistency | US-054, US-055 |

## Problem

People lists colleagues and pending invites, but Invite appears only while the company is nearly empty, clicking a colleague only opens a DM, and invite rows link away to Company → Members.

## Evidence

- `web/src/routes/[companyId]/people/+page.svelte:475-481,` :695-710, :720-727

## Suggested direction

Put Invite in the People header and a role/remove menu on colleague rows reusing members.ts.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
