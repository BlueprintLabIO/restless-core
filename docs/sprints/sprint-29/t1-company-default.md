# S29-T1 — Give the company one owner-set default

**Layer:** Company configuration + authenticated owner action.

**Serves:** Sprint 29 criteria 1, 2, 8, 11 and 14.

**Depends on:** S29-T0.

**Observed friction:** The owner's standing quality ambition exists only in prose and lead memory, so
new outcomes cannot inherit it reliably.

**Makes deletable:** Prompt-only company quality preference and any browser-local default.

## Outcome

Company configuration owns one typed `OutcomeStandard` default. New companies begin at
`Exceptional`; the owner can inspect and change it through the existing authenticated company-action
boundary, and all projections read the same value.

## Scope

- Add the frozen four-value type at the narrowest shared Rust boundary and generate its TypeScript
  representation through the existing binding seam.
- Extend company configuration serialization, parsing, bootstrap defaults and supported owner update
  action without adding a second settings store.
- Make old company files receive the documented default without destructive rewrite.
- Include the current value in the source-backed Company projection with the same provenance and
  error semantics as other company policy.
- Prove the setting does not change the company spend ceiling, model, provider, credentials or
  external authority.

## Acceptance

- Fresh company, legacy company, explicit setting and invalid setting fixtures are deterministic.
- Restart and config round-trip preserve the value exactly.
- Only the authenticated owner path can change the company default.
- A mode change emits the ordinary audit/evidence expected for company-setting changes.
- Invalid or unknown values fail closed and display a useful source error.
- There is no duplicate database preference, local-storage fallback or prompt-only source of truth.

## Branch and purge

Compare extending current company config with introducing an OrgIntel preference record. The expected
prior is company config: this is stable company operating policy, not organizational evidence. Keep
only the branch justified by the observed ownership boundary.
