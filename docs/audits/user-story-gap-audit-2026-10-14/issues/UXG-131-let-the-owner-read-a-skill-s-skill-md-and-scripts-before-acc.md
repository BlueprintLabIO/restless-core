# UXG-131 — Let the owner read a skill's SKILL.md and scripts before accepting it

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Apps | high | missing-function | US-079 |

## Problem

Candidate review shows a description, a "Scripts" badge and Accept/Decline. Path, digest, origin url/ref are loaded but never shown, and no owner route returns skill content. Accepting unread third-party instructions and scripts is not review.

## Evidence

- `web/src/routes/[companyId]/apps/[app]/+page.svelte:374-401`
- `web/src/lib/model/generated/orgintel.ts:217-221`
- `crates/restless-owner/src/owner.rs:1330-1337`

## Suggested direction

Render SKILL.md and list its scripts inline.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
