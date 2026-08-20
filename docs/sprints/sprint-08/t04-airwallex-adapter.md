# S08-T4 · One Airwallex approval and reconciliation adapter

**Layer:** Authority + external provider.
**Serves:** Sprint 08 criteria 3, 4, 7, 8, 9 and 10.
**Depends on:** S08-T2, S08-T3.
**Observed friction:** Restless has no provider-confirmed payment path. The generic runner can prove a
local command exited, but not that a transfer entered approval, scheduled or settled.
**Makes deletable:** Airwallex calls through arbitrary Runtime CLI/argv and any parallel Wise/provider
stub left after T0.

## Outcome

A small host-side Airwallex module implements only the live-probed operations needed by the first
slice: observe balances/transfers, submit to the required provider approval workflow, retrieve status,
verify webhook events and reconcile an unknown result.

## Scope

- Use the exact API version and account/scopes proven by T0.
- Submit only to an existing provider beneficiary reference.
- Require the account setting that routes API-created transfers through approval.
- Supply a stable provider request/idempotency identifier.
- Persist provider transfer ID and raw status without raw credential or unnecessary payload copies.
- Verify webhook signature before deduplicating by provider event ID.
- Treat `IN_APPROVAL`, scheduled/processing and settled as distinct facts.
- Re-query provider state after ambiguous local outcomes and webhook gaps.
- Expose a deterministic live probe for read, submit-scope and approval-workflow readiness.

Do not extract a general `BankProvider` framework. If a second provider later proves the same seam,
extract only the repeated contract then.

## Verification

- Fake provider matrix passes before sandbox.
- Airwallex sandbox returns real transfer identifiers and expected approval/status transitions.
- Invalid webhook signatures and duplicate events cannot advance Authority state twice.
- Lost submit response reconciles by provider request/transfer reference without a duplicate.
- The submit key cannot call a Beneficiary-write endpoint.

## Risks

- **Provider status mapping hides a novel state — guarded:** retain raw status and map unknown values
  to honest unknown, never failure or success.
- **Adapter becomes provider administration — invariant:** no beneficiary, user, key, approval-policy,
  account-capability or funding mutation endpoints are implemented.
