# UXG-270 — Give the "couldn’t open this company" dialog a way out when retrying cannot help

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Shell & cross-cutting | high | ux | US-160, US-162 |

## Problem

When the company’s first reads fail or stall, a modal alertdialog makes the whole shell inert and offers only "Try again". For an unknown or removed company id (404), a refused membership (403) or a signed-out session (401) the failure reading itself marks the error non-retryable and, for signed out, says "Sign in again to continue" without any link. The owner is trapped behind the scrim and must edit the address bar; the 404 page’s "Back to company" link leads straight back into the same trap.

## Evidence

- `web/src/routes/[companyId]/+layout.svelte:218-222` (principal/attention failure blocks startup)
- `web/src/routes/[companyId]/+layout.svelte:1478-1508` (only a Try again button)
- `web/src/lib/components/AppShell.svelte:286` (inert while blocked)
- `web/src/lib/model/failure.ts:146-152` (signed_out, retryable false, no link)
- `web/src/lib/model/failure.ts:162-178` (missing, retryable false)
- `web/src/routes/+error.svelte:10` (home is /<companyId>)
- `web/src/routes/+error.svelte:32` (Back to company)

## Suggested direction

Use FailureView.retryable to swap "Try again" for "All companies" / "Sign in" / "Ask for access" when a retry cannot succeed.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
