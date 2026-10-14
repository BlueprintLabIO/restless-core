# UXG-245 — Stop a 60-second pause from telling the agent the owner has finished

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company computer & intelligence | high | ux | US-140 |

## Problem

Both desktop views release control automatically after 60 seconds without pointer or key input, and the release error is swallowed. Returning control during a live Attempt posts a Work message telling the agent the owner is returning control and to continue the same Attempt. An owner who waits for an SMS code, reads a document beside the form or answers a call therefore hands the session back mid-step, and the agent wakes to act over a half-filled form. There is no warning, no countdown and no way to extend.

## Evidence

- `web/src/routes/[companyId]/company/computer/+page.svelte:108-116` (idle release at 60s, automatic=true)
- `web/src/routes/[companyId]/company/computer/+page.svelte:232-233` (automatic failures not shown)
- `web/src/routes/[companyId]/+page.svelte:411-421` (same in the Inbox)
- `crates/restless-owner/src/owner_desktop.rs:8` (CONTROL_TTL_SECONDS = 60)
- `crates/restless-owner/src/owner.rs:11146-11175` (return writes the "owner is returning control" Work message)

## Suggested direction

Warn before idle release with a "still here" option, and distinguish an idle lapse from a deliberate return so the agent waits rather than resumes.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
