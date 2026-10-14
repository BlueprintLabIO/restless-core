# UXG-163 — Make charter history findable, comparable and restorable

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | medium | missing-function | US-088 |

## Problem

History opens only by clicking faint "Saved 3d ago" text; revisions are full Markdown blocks with no compare or restore; every revision is labelled "Owner" even from onboarding or copies.

## Evidence

- `web/src/routes/[companyId]/company/charter/+page.svelte:219-226,` :271-297
- `crates/restless-owner/src/owner.rs:6459-6475`

## Suggested direction

Visible History control with compare and restore; record the real source.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
