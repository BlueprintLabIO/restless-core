# S08-T2 · Finance secrets terminate in a host-side adapter

**Layer:** Authority credential plane.
**Serves:** Sprint 08 criteria 4, 5 and 10.
**Depends on:** S08-T0.
**Observed friction:** the generic governed-effect child is selected and executed inside the mutable
Company Runtime. Its UID isolation and secret redaction are useful, but that path should not receive
a credential capable of submitting money movement.
**Makes deletable:** any `finance.*` binding delivered to a Runtime child and any admin provider key
created for Restless.

## Outcome

The Authority service owns one host-side finance adapter path. It resolves narrowly scoped read,
submit and webhook references from the existing Infisical backend; provider access tokens remain in
memory; no finance credential crosses into the Runtime.

## Scope

- Add dedicated Infisical finance paths and named bindings for the selected company/provider.
- Use separate provider keys for read and submit; refuse an admin key or Beneficiary-write scope.
- Exchange the long-lived scoped key for the provider's short-lived token inside trusted host code.
- Verify webhooks with a separate signing secret before Authority state changes.
- Prevent the generic effect endpoint/CLI from naming finance bindings or finance effect classes.
- Keep provider account/client identifiers as non-secret Authority metadata where appropriate.
- Reuse the proven Infisical backend, presence/invalid distinction and outage behaviour.
- Do not split a new service unless T0 proves a certificate/signing boundary the modular monolith
  cannot safely materialise.

## Verification

- Exact-value scans cover process environment, argv, database JSON, logs, receipts, Runtime and Git.
- A hostile Runtime command cannot resolve, proxy or infer the finance binding.
- Short-lived token expiry causes refresh in the adapter without persisting the token.
- Infisical outage pauses authenticated finance operations while safe reads from cached Authority
  history and unrelated company work continue honestly.
- Provider key revocation stops new submission without restarting or destroying the company.

## Risks

- **One trusted daemon can read several provider secrets — accepted:** the host Authority process is
  already the V0 trust boundary; provider scope, approval-only submission and limited wallet balance
  bound the first run. Split only on observed need.
- **Secret appears in provider error body — invariant:** errors and traces never retain request
  headers or unfiltered provider bodies; repeat Sprint 05's reflected-sentinel probe.
