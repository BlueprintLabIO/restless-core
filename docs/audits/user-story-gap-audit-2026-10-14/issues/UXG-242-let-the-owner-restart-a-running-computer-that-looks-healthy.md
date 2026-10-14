# UXG-242 — Let the owner restart a running computer that looks healthy but is stuck

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company computer & intelligence | high | missing-function | US-138, US-191 |

## Problem

The only restart path is a button on Health that appears beside a failing runtime check, and the engine refuses any recovery action the doctor did not recommend. A computer whose container, supervisor and browser probes pass but whose desktop is frozen, whose app is hung or whose browser is wedged on a modal therefore cannot be restarted from the product at all. The Computer page itself has no Start, Restart or "sleep now" control, and nothing can close a hung window. The owner of a self-running business is left with a stuck company and no lever.

## Evidence

- `crates/restless-engine/src/company.rs:1683-1711` (Running with passing probes recommends nothing)
- `crates/restless-engine/src/company.rs:1932-1936` (recover bails when the action is not a current recommendation)
- `web/src/routes/[companyId]/company/doctor/+page.svelte:81-84` (repair button attached only to a failing image/runtime check)
- `web/src/routes/[companyId]/company/doctor/+page.svelte:193-199`
- `web/src/routes/[companyId]/company/computer/+page.svelte:427-453` (focus toolbar offers only Release/Leave)
- `web/src/routes/[companyId]/company/computer/+page.svelte:504-511`

## Suggested direction

Offer an always-available, confirmed Restart (and Start / Sleep now when they apply) on the Computer page, allowed by the engine as an explicit owner request.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
