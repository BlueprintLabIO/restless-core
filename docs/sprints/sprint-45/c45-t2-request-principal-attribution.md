# C45-T2 — Request-principal propagation and handler audit

**Layer:** restlessd (Kernel/owner API) + restless-orgintel (authorization invariant)
**Status:** In progress — concrete violations found and fixed; not a claim of exhaustive coverage

## Observed friction this closes

Sprint 45's audit (see conversation record, not reproduced here) found that most of the owner API
already threads a server-derived `RequestPrincipal` through handlers correctly (Rooms, Documents,
company principal projection). But several handlers still hard-coded the literal string `"owner"` as
the Authority-record `actor_id` regardless of who actually acted:

- `archive_company`, `restore_company`, `recover_company_computer` (lifecycle)
- `set_company_outcome_standard`, `set_company_harnesses` (policy)
- `promote_company_identity`, `reject_company_identity`, `decide_company_identity_migration`
  (identity decisions)
- `approval::grant`/`revoke`/`decline` (Authority approvals) — these already took a `principal: &str`
  parameter but still discarded it in favour of the literal at the `authority.emit` call site

In local single-owner mode this was harmless: `RequestPrincipal::local_owner().actor_id() == "owner"`,
so the literal happened to be correct. In hosted/multiplayer entry mode (Cloud 16 / Sprint 45's whole
point) the acting human's actor id is a durable `human-{uuid}` — the literal would have silently
mis-attributed every hosted decision to nobody real, which directly contradicts Sprint 45's stated
outcome ("performs an ordinary Core operation with truthful attribution").

A sharper defect sat one layer down: `restless_orgintel::identity::{promote_identity_proposal,
reject_identity_proposal}` and `constitution::decide_identity_migration` used `decided_by != "owner"`
as **both** the authorization gate and the attribution value. Once a real hosted owner's actor_id is
`human-{uuid}` instead of the literal, that check would reject every legitimate hosted identity
decision outright — not just mis-attribute it.

## What changed

- `identity.rs` / `constitution.rs` / `types.rs`: added `acting_membership_role: &str` as the
  authorization input (server-derived, unspoofable — it comes from `RequestPrincipal::membership_role()`,
  never client input) and kept `decided_by`/`actor_id` purely for attribution. The gate now checks
  `acting_membership_role != "owner"`; the recorded decision still shows exactly who decided.
- `authority.rs::record_company_identity_decision`: added an `actor_id: &str` parameter, bound into
  the SQL insert instead of the literal `'owner'`.
- `approval.rs::grant/revoke/decline`: use the already-passed `principal` parameter for
  `authority.emit`'s actor_id instead of discarding it for the literal.
- `company.rs::recover`: added an `acting_actor_id: &str` parameter, threaded into its three
  `authority.emit` lifecycle calls.
- `owner.rs`: extracted `Extension<RequestPrincipal>` in the nine affected handlers and passed
  `principal.actor_id()` (and, for identity decisions, `principal.membership_role()`) through.

## What this does not claim

This is the set of violations a targeted audit + grep found, not a line-by-line audit of every
handler in the owner API. Rooms, Documents and company-principal handlers already used
`principal.actor_id()` correctly before this change and were left alone. A later pass should still
grep for any newly introduced `Some("owner")` / `"owner"` literal creeping into a fresh handler.

## Evidence

- `cargo test -p restlessd --bin restlessd` — 376 passed, 0 failed, 8 ignored (require live
  model/gateway fixtures, pre-existing).
- `cargo test -p restless-orgintel` — full identity/access/voice/culture/constitution/visual suites
  pass, including the existing negative tests (non-owner role cannot promote/decide).
- New regression test: `authority::identity_decision_attribution_tests::
  identity_decision_is_attributed_to_the_real_deciding_actor_not_a_literal` — proves a
  `human-{uuid}` actor id round-trips through `record_company_identity_decision` unchanged, and that
  it is never silently replaced with the literal `"owner"`.
- All tests run against a real, disposable local scratch Postgres database
  (`RESTLESS_TEST_DATABASE_URL`), created and dropped for this verification — not a mocked capability.

## Makes deletable

Nothing yet — this is additive correctness. Once C45-T3's `kind`→`actor_class` migration completes,
the remaining `kind == "owner"` bootstrap-actor lookups this ticket did not touch
(`company_bootstrap.rs::verify_company_substrate`, `org.active_actor("owner")`) become candidates for
deletion in favour of a first-class "who currently holds root Authority" fact (C45-T4).
