# S08-T5 · Prepared KYB/payment last mile in the existing owner surface

**Layer:** OrgIntel + Owner surface.
**Serves:** Sprint 08 criteria 1, 2, 7 and 11.
**Depends on:** S08-T1, S08-T4.
**Observed friction:** identity, legal attestation and payment confirmation are named owner-handoff
categories, but none has been exercised against a real provider state or resume condition.
**Makes deletable:** manual transcription instructions, conversation-as-approval and requests that
the owner report provider completion.

## Outcome

The existing Sprint 07 attention/handoff path presents the exact legal/KYB or payment participation
the provider requires. It opens the prepared provider-native destination, preserves provider-backed
facts and resumes from observed provider state.

## Scope

- For onboarding, show the legal entity being onboarded, current provider status, exact owner-only
  attestation/identity step and prepared provider link.
- Keep raw KYB documents in the provider channel; do not upload them into Runtime or OrgIntel.
- For payment, show source account label, immutable provider beneficiary, amount, currency, purpose,
  supporting invoice/contract, recommendation, approval state and no-action consequence.
- The primary action opens provider-native approval; it does not call an internal approve-payment
  endpoint.
- Poll or consume provider status/webhook so completion resolves the handoff observably.
- Conversation may clarify but cannot change the legal profile, envelope, beneficiary or provider
  approval state.
- Keep technical provider payloads and IDs behind evidence disclosure.

## Verification

- The owner can state the exact legal entity/payment and consequence from the first viewport.
- The displayed amount, currency, beneficiary and status match Authority/provider state exactly.
- A changed amount/beneficiary invalidates the old brief/action and requires attributed refresh.
- Closing the browser or saying “done” does not resolve the handoff; provider state does.
- No restricted legal field or credential appears in browser JSON or model conversation.

## Risks

- **Provider link is phished or stale — guarded:** the destination comes from configured provider
  origin/current intent and is shown with provider/account context; material source change retracts
  the old action.
- **Owner approval is mistaken for broad standing authority — invariant:** it resolves only the exact
  external payment. Any later autonomous envelope is a separate owner authority change.
