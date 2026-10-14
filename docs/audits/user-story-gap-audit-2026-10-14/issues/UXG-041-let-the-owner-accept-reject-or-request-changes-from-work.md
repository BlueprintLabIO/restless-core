# UXG-041 — Let the owner accept, reject or request changes from Work

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | high | missing-function | US-028 |

## Problem

owner-cockpit §6.7 promises feedback, accept/reject of a milestone and reopen. Work detail has only chat links, and there is no reopen/revise endpoint, so done work can be sent back only through free-form chat with no record on the Work.

## Evidence

- `web/src/routes/[companyId]/work/[workId]/+page.svelte:454-466`
- `crates/restless-owner/src/owner.rs:1356-1369` (only resume/abandon/standard)
- `docs/specs/owner-cockpit.md:722-735`

## Suggested direction

Add "Request changes" / "Reopen" that records owner feedback and creates the revise edge or attempt.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
