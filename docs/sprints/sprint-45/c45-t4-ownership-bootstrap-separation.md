# C45-T4 — Ownership/company bootstrap separation

**Layer:** restlessd (Authority Plane) + restless-orgintel (read-only membership projection)
**Status:** Core mechanism implemented and tested; consumer migration (replacing every
`membership_role == "owner"` check that actually means "root Authority" with the new fact) not
started — see "What this does not claim" below.

## Observed friction this closes

`company_bootstrap.rs::verify_company_substrate` and every owner-only handler gate on one thing:
membership_role (or the legacy singleton `kind == "owner"` Actor). ARCHITECTURE.md decision #28 is
explicit that membership ownership, organisational responsibility and root Authority ownership are
**separate facts** that an explicit bootstrap may *initially* bind to one human Actor — implying they
can later diverge. Before this ticket, there was no way for them to diverge at all: there was exactly
one "owner" concept, and no operation existed to move root Authority to a different Actor while
leaving company membership untouched (or vice versa).

## What changed

- `restless_orgintel::OrgIntel::current_membership_owner_actor_id` — a narrow read of which durable
  Actor currently holds the external `membership_role = 'owner'` binding. This is Cloud/Better-Auth
  truth; Core never writes it. It exists purely so the Authority-ownership fallback below has
  something principled to default to.
- `AuthorityStore::current_authority_owner` / `AuthorityStore::transfer_authority_owner` — a new
  governance-kind record (`authority_ownership_transfer`) in the existing Authority store. Absence of
  any record means "never explicitly transferred," not "unowned." A transfer is race-safe (advisory
  lock + re-check under the lock) and requires the caller to already know who currently holds
  ownership — see the scoping note below.
- `owner.rs::effective_authority_owner` — the single place that computes "who holds Authority right
  now": the latest explicit transfer if one exists, else the current membership owner (network mode)
  or the local singleton `"owner"` (local mode). This is the only trusted source for
  `expected_current_owner`; it is never taken from client input.
- Two new endpoints, both owner-membership-gated by the existing middleware (same boundary as
  archive/restore/recover):
  - `GET /api/companies/{company}/company/authority-owner` — read the current holder and its source.
  - `POST /api/companies/{company}/company/authority-owner/transfer` — only the actor who
    `effective_authority_owner` currently names may call this; it moves Authority to a new Actor with
    an attributed rationale, independent of membership.

## Why the store cannot verify a *first* transfer alone

`AuthorityStore::transfer_authority_owner` cannot, by itself, prove that a caller claiming to be the
bootstrap owner actually is one — it has no view of OrgIntel's membership data. This is why the
authorization check (`principal.actor_id() == current.actor_id`) lives in the handler, computed from
`effective_authority_owner` (server-derived, unspoofable), and the store's own guarantee is narrower
but still load-bearing: once *any* transfer has happened, only the exact resulting owner can move it
again, closing the race/stale-retry window. This is deliberately documented in both the function's doc
comment and a test (`transfer_refuses_a_stale_expected_owner_once_a_transfer_chain_exists`) so a future
reader doesn't mistake the store for a self-sufficient authorization boundary.

## Evidence

- `cargo test -p restlessd --bin restlessd` — full suite (380 tests including 4 new ones) passes
  against a real, disposable scratch Postgres database, verified as a standalone commit (temporarily
  set aside the session's unrelated pre-existing WIP with `git stash --keep-index` to confirm this
  exact staged diff builds and passes alone, not merely "on top of everything else").
- New tests in `authority::authority_ownership_transfer_tests`: no-transfer-yet reports `None`;
  transfer moves ownership and is scoped per-company; a stale `expected_current_owner` is refused once
  a chain exists; empty destination, empty rationale and self-transfer are all refused.

## What this does not claim

- **No consumer migration.** Every existing owner-only check in the codebase still gates on
  `membership_role == "owner"` (or the legacy `kind == "owner"` Actor), not on
  `effective_authority_owner`. The mechanism exists; nothing has been told to prefer it yet. Doing that
  broadly is a follow-up, ideally paired with C45-T3's `kind`→`actor_class`/role migration so both
  changes don't churn the same call sites twice.
- **No combined "transfer both" UX.** AC6 asks for "the combined case when both are intended" to be
  presented clearly; today an owner wanting to hand off both membership and Authority makes two
  separate calls (membership transfer via Cloud's existing external-membership-control path; Authority
  transfer via the new endpoint here). A single guided combined operation is not built.
- **No UI.** This is API-only; the owner cockpit has no surface for it yet.

## Makes deletable

Nothing yet — additive. Once consumers migrate to `effective_authority_owner`, the direct
`membership_role == "owner"` checks that were really asking "who holds root Authority" (as opposed to
"who may administer membership") become deletable in favour of it.
