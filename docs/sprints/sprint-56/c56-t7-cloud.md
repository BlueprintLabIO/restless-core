# C56-T7 — Cloud mounts the issuer library

**Layer:** `restless-cloud` (paired sprint)

**Friction served:** Cloud's `fleet-web` + `fleet-api` duplicate the issuer.

**Change, for the paired sprint:**
- Mount `services/identity`, pinned through the Core release lock.
- Fleet implements Placement from its topology snapshot and Mail from its adapter.
- The Node host holds the signing key.
- Plane hostnames live under the account site.

**Makes deletable, after one real Cloud removal has passed through the library outbox:**
- `fleet-api` `handoff.rs`, `membership_control.rs` and `membership_control_reconciler.rs`;
- the duplicated Better Auth membership hooks in `auth-core.ts`;
- the `account/company/[organizationId]/people` route.

Pushing to `restless-cloud` is owner-only, so this ticket is not implemented from Core.

**Update, 23 September 2026:**
- The founders chose a Store port over a live data migration (ADR 0012 §2 amendment).
- `fleet-web` mounts `@restless/issuer` from the `restless-identity` image, with Fleet Placement and a
  Store that wraps Cloud's existing versioned controls and Fleet projection.
- Cloud's People page is deleted in favour of the cockpit Members page.
- Migrating Cloud onto `membership-sql.mjs` and deleting the Rust delivery remains a separate
  decision.
