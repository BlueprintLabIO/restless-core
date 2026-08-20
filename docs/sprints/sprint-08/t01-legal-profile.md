# S08-T1 · Authority-owned legal identity and safe company projection

**Layer:** Authority + Runtime projection.
**Serves:** Sprint 08 criteria 1, 2, 3 and 11.
**Depends on:** None.
**Observed friction:** the current company record has a runtime name and mission but cannot supply or
distinguish the legal facts required by an invoice, contract, registry check or provider onboarding.
**Makes deletable:** treating `CompanyConfig.name` as a legal name and ad hoc copies of company
details in prompts or project files.

## Outcome

The owner records one minimum legal profile through the Authority owner boundary. Restless live-
probes the public registry facts it can observe, distinguishes those observations from owner claims,
and exposes one explicitly safe read-only projection to the selected company.

## Scope

- Add current Authority-owned legal-profile state rather than an append-only legal ledger.
- Cover legal/trading name, entity type, jurisdiction, registration identifier, approved business
  address and safe invoice/display fields needed by this run.
- Treat an owner-approved public business registration number (for example an ABN) as a safe
  business field; personal/private tax identifiers remain restricted and have no input field.
- Attribute owner changes and retain source plus observation time for registry-probed facts.
- Render stale/unavailable registry state honestly.
- Keep restricted values as references or out of Restless entirely.
- Project only owner-approved safe fields into Runtime/agent context.
- Prepare a provider-native owner link for KYB or legal attestation; observe only the provider status
  needed to resume.

## Explicit exclusions

- No passports, driver licences, signatures, biometrics or raw beneficial-owner documents.
- No general officer/shareholder/cap-table model.
- No automatic legal attestation, company registration or registry mutation.
- No assumption that one brand/runtime name is itself the legal entity.

## Verification

- Owner-asserted and registry-observed values remain distinguishable after restart.
- A registry outage produces unavailable/stale state, not a negative registration claim.
- Exact sensitive sentinels are absent from Runtime, prompts, Git, OrgIntel, browser JSON and logs.
- Invoice/provider preparation can use the safe projection without reading the restricted source.

## Risks

- **Profile becomes legal truth by assertion — guarded:** every external observation carries source
  and time; provider/registry remains authoritative.
- **Safe projection exposes a residential address — invariant:** the owner explicitly selects the
  display address; no restricted address is projected by default.
