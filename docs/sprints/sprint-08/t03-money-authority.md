# S08-T3 · Bounded money intent, reservation and confirmed receipt

**Layer:** Authority.
**Serves:** Sprint 08 criteria 6, 8, 9 and 10.
**Depends on:** S08-T0.
**Observed friction:** the current first-party approval gate and model-spend fuse do not bind an exact
financial account, provider beneficiary, amount, currency or total pending exposure.
**Makes deletable:** interpreting free-form argv/output to decide money authority and counting
self-attested process success as confirmed movement.

## Outcome

One typed financial consequence travels inside the existing governed-effect boundary. Authority
atomically reserves its amount, decides deny/awaiting-provider-approval/proceed by rule, and records
provider confirmation without becoming a provider-specific payments engine.

## Required semantics

```text
source_account_ref
provider_beneficiary_ref
amount_minor
currency
purpose
evidence_refs
idempotency_key
requesting principal/actor
```

- Store minor integer units and explicit ISO currency; never infer either from prose.
- Enforce one owner-set per-payment ceiling and aggregate period/exposure ceiling.
- Reserve pending and unknown intents atomically before provider submission.
- Release reservation only on provider-confirmed rejection/cancellation/failure.
- Convert provider states into the existing honest result categories while retaining the raw provider
  state/reference.
- Distinguish provider confirmation from self-attestation.
- Require reconciliation before retry of unknown outcome.
- Freeze only financial consequence, not internal work.

This is deterministic/enumerable authority. Invoice legitimacy, value and recommendation remain
model judgement in OrgIntel.

## Verification

- Concurrent individually valid intents cannot jointly exceed the envelope.
- Reusing one key with changed amount, beneficiary, account or currency fails before provider access.
- Unknown state continues to consume reservation across daemon and Runtime restart.
- A self-reported success cannot enter confirmed money totals.
- Revocation/freeze denies new intents while prior receipts remain inspectable.

## Risks

- **FX makes a single global ceiling ambiguous — accepted:** the first live envelope is per currency.
  Do not add an exchange-rate authority service for one AUD run.
- **Payment-specific structure grows into a universal bank model — guarded:** carry only the exact
  consequence needed for authority and reconciliation; provider-specific details remain in the
  adapter/provider.
