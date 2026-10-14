# UXG-252 — Say when the monthly hours limit is why the computer will not wake

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company computer & intelligence | medium | ux | US-146 |

## Problem

Once the monthly hours limit is reached the sleep loop stops the computer, and every later wake fails the start-allowed check. The Computer page still offers "Wake and enter", and the owner API answers "could not wake. Try again in a minute", while opening a link says the computer is "stopped". The failure also records a start backoff, so trying again fails the same way. The real cause and fix (raise or clear the limit in Limits → Computer) are never shown.

## Evidence

- `crates/restless-engine/src/runtime_sleep.rs:206-216` (cap reached triggers sleep)
- `crates/restless-engine/src/runtime_usage.rs:308-323` (start refused once the cap is exhausted)
- `crates/restless-engine/src/runtime_sleep.rs:338-358` (wake failure records backoff)
- `crates/restless-owner/src/owner_desktop.rs:108-131` (generic "could not wake" for a sleeping computer)
- `crates/restless-owner/src/owner.rs:10797-10805`
- `web/src/routes/[companyId]/company/computer/+page.svelte:69-74`

## Suggested direction

Project a "monthly hours reached" state on the company view and show it on Computer with a link to raise the limit.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
