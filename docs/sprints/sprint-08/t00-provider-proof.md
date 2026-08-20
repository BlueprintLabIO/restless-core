# S08-T0 · Live-probe the provider and freeze one runnable contract

**Layer:** Authority + Runtime integration.
**Serves:** Sprint 08 criteria 3, 4, 7, 8, 9 and 12.
**Depends on:** S08-T7, which creates the ordinary company Work this probe fulfils.
**Observed friction:** Airwallex documentation describes the needed scopes, approval workflow,
sandbox and webhooks, but Restless has not observed those capabilities on an actual eligible account.
**Makes deletable:** provider comparison notes, speculative provider interfaces and any candidate
that fails the live contract.

## Outcome

One short provider capability report records whether the real Airwallex sandbox/account can support
the exact Sprint 08 path. It names observed endpoints, scopes, states and gaps without including
credentials. Airwallex either becomes the one implementation canon or is rejected before adapter
code is written.

## Required probe

- Create/use the provider sandbox only after owner permission.
- Confirm this is the Business Account API, record its exact account API version and use its observed
  sandbox origin; do not substitute the separate Connected Accounts `api-demo` contract.
- Obtain separate scoped read and submit credentials; never create an admin key for Restless.
- Prove the submit key lacks Beneficiary write and account-administration scope.
- Submit a sandbox transfer and observe whether API-created transfers enter the configured approval
  workflow.
- Retrieve transfer state after deliberately discarding one local response.
- Verify webhook signatures, stable event IDs and duplicate delivery behaviour.
- Verify key regeneration/revocation and the short-lived API access-token lifetime.
- Record any account-level enablement, pricing, eligibility or support dependency that blocks the
  live run.
- Confirm from current provider/regulator material that the wallet is not being treated as an ADI
  treasury deposit.

If a required Airwallex capability fails, stop. Probe Wise Business only against the same contract,
update the sprint to the evidence and retain one provider. Do not implement both.

## Pass condition

The report contains provider IDs/states or sanitized API observations sufficient to distinguish a
live probe from a documentation summary. Every downstream ticket can name one concrete credential,
approval, status and webhook contract. The report is linked to the sourcing Work and informs an
explicit retain/reject/fallback decision; sprint prose alone does not own the choice.

## Risks

- **Opening an account becomes accidental authorisation — invariant:** this ticket may prepare the
  provider handoff but cannot accept terms, attest legal facts, import a live key, fund or pay without
  the owner boundary.
- **Sandbox becomes live-company evidence — invariant:** sandbox receipts stay labelled and outside
  live confirmed-money totals.
