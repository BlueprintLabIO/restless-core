# C45-T6 — Self-hosted and hosted end-to-end proof

**Layer:** restlessd (entry + OrgIntel composition)
**Status:** Self-hosted composition proof added; full HTTP-router proof and hosted (Cloud) proof
remain open.

## Observed friction this closes

Every layer Sprint 45 touches already had focused tests: `entry.rs`'s own test module thoroughly
covers JWKS fetch/rotation/signature/replay/expiry in isolation, and `restless-orgintel`'s
`tests/access.rs` thoroughly covers actor binding, replay safety and monotonic membership control at
the database layer. Nothing proved they compose correctly the way production code actually chains
them: `consume_entry_assertion` calls `network.verify(...)` and then `resolve_entry_company` /
`consume_human_access_context`, and the request middleware later calls
`human_session_membership_is_current` on every request. A change to any one seam (a field rename, an
argument order swap) could pass every existing test while breaking the real journey.

## What changed

Added `entry::tests::network_entry_maps_and_reidentifies_one_durable_actor_then_revocation_blocks_it`,
which runs the real sequence:

1. Start a real HTTP server serving a real JWKS document (Ed25519 public key) — not a mock, an actual
   `axum::serve` on a loopback port, reusing the same pattern the existing
   `stale_jwks_is_refreshed_before_a_known_key_can_authorise_entry` test already established.
2. Sign a real assertion with `AssertionClaims` + Ed25519, matching a company/cell pair bound in a
   real scratch Postgres OrgIntel schema.
3. `NetworkEntry::verify` the assertion against the live JWKS endpoint.
4. `OrgIntel::consume_human_access_context` maps the verified context to a durable `human-{uuid}`
   Actor.
5. A second, later assertion (new `jti`, same subject) is signed, verified and consumed again — the
   test asserts the **same** Actor id comes back, proving durable identity rather than a fresh actor
   per login.
6. `OrgIntel::apply_external_membership_control` records the membership as removed.
7. `OrgIntel::human_session_membership_is_current` — the exact check the request middleware performs
   on every call — now returns `false` for the old membership tuple, without the Actor or its history
   being erased.

## Why this stops short of the full HTTP router

`consume_entry_assertion` takes `State<OwnerState>`, and `OwnerState` has grown fields
(`native_documents_proxy`, `capacity_activity`, `plane_readiness`) that belong to the session's other,
unrelated in-flight work (the Open Company Runtime programme). Building a full-router test today would
mean either standing up those components with no test constructors of their own yet, or reaching into
unfamiliar, unfinished code to add them — exactly the entanglement risk flagged when that WIP was first
discovered this session. This test instead exercises the real production functions in the real
sequence, one layer below axum routing, which is the actual seam Sprint 45 cares about (JWKS → actor
mapping → revocation), without depending on unrelated unfinished surface.

## What this does not claim

- **No full HTTP-router proof.** `POST /entry` → session cookie → authenticated API call has not been
  exercised end-to-end through the real axum app.
- **No hosted (Cloud) proof.** This is the *self-hosted* half of "self-hosted and hosted end-to-end
  proof." The hosted half needs a real Cloud/Better Auth fixture issuing the assertion, which lives in
  `restless-cloud`, not this repository.
- **A real, reproducible test-infrastructure hazard was found and worked around, not fixed**: running
  the full suite with `--test-threads=4` against a freshly created scratch database hung twice (of
  ~5 attempts) on `AuthorityStore::connect`'s single fixed advisory-lock bootstrap. Flagged as a
  separate task (not this ticket's scope); verification here used `--test-threads=1`, which never
  reproduced it.

## Evidence

- `cargo test -p restlessd --bin restlessd -- --test-threads=1` — 381 passed (up from 380), 0 failed,
  8 ignored, against a real disposable scratch Postgres database.
- The new test alone: `cargo test -p restlessd --bin restlessd entry::tests::network_entry_maps_and_reidentifies... ` — passes in isolation.
- Verified as a standalone commit: staged diff isolated from the session's unrelated pre-existing WIP
  via `git stash --keep-index`, rebuilt and retested (333 passed alone), confirming this slice is
  correct on its own.
