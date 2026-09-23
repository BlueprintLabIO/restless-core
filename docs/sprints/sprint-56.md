# Sprint 56 — Company members, one issuer for Core and Cloud

**Status:** Core tickets done; Cloud half (C56-T7) open in `restless-cloud`
**Programme:** bounded company collaboration (ADR 0008)
**Decision:** [ADR 0012 — One identity issuer, two hosts](../adr/0012-one-identity-issuer-two-hosts.md)
**Depends on:** ADR 0009 entry and membership-control verifier (built), `services/self-hosted-identity`
(`1a55aff`). The Cloud half is a paired `restless-cloud` sprint (C56-T7).

## Outcome

An owner opens **Company → Members** in the cockpit. They see who has access, invite a colleague,
watch the invitation become a person, change that person's role and remove them. The same page and
the same steps work on a self-hosted install and on Cloud. The only difference between the two is
which host runs the one identity issuer.

A company that began in local mode turns on network entry and keeps its owner: the owner's history,
private documents and conversations stay theirs. In local mode, Members says honestly that the
company is local-only and how to change that.

When someone joins, Exec is told, and gives them a place in the organisation.

## Observed friction

Each item was read on 23 September 2026 from `dev` at `70093e1` and `restless-cloud`
`personal-cloud-dogfood` at its 19 September head.

| Friction | Evidence |
|---|---|
| The cockpit has no members surface in any mode | `web/src/routes/[companyId]/company/` has no members route. `human_principal_actor_bindings` is never shown |
| Local mode doesn't explain why nobody can be invited | Loopback entry has one `owner` principal and no issuer; the UI is silent |
| Two Better Auth issuers | `services/self-hosted-identity/src/server.mjs` (Core) and `apps/fleet-web/src/lib/server/auth-core.ts` + `services/fleet-api/src/handoff.rs` (Cloud) |
| Two People screens, both outside the cockpit | `services/self-hosted-identity/public/app.js` and `fleet-web/src/routes/account/company/[organizationId]/people` |
| The self-hosted issuer has constant membership versions | `server.mjs` `signed()`: `membership_version: control ? 2 : 1`. A role change never advances the version |
| The self-hosted issuer can't suspend, and removal delivery isn't durable | `revoke()` does one synchronous POST. On failure the owner sees **Retry removal**. Cloud has `fleet_outbox` with a reconciler and an entry barrier |
| A local company can't be converted | Self-hosted README: "does not convert an existing local-owner company". `access.rs` always creates a new `human-…` Actor on first entry |
| Exec isn't told a human joined | `consume_human_access_context` inserts the Actor as "Company member" with no event, Attention or wake |

## Design stance

- **Core is a relying party. The issuer owns membership.** Nothing here makes Core a membership
  writer or the issuer a holder of company state (ADR 0012, Invariants).
- **Converge, don't add.** This sprint builds no third issuer. It turns the self-hosted service into
  the canonical library and moves Cloud's better semantics into it.
- **The read side comes from Core, the write side from the issuer.** The cockpit merges them on one
  page and calls the issuer directly with the issuer session.
- **Probe, never guess.** The cockpit shows only the capabilities the configured issuer publishes.
  Local mode is the null issuer.

## Scope by layer

### Identity issuer (`services/identity`)

- Rename `services/self-hosted-identity` → `services/identity`. Split it into the issuer library and
  a self-hosted host (`main.mjs`).
- Placement port and Mail port (ADR 0012 §2). The self-hosted host implements both from its
  generated config.
- Versioned membership source: `restless_membership_state`, advanced in the same transaction as
  every role, suspend, reinstate and remove change. Handoffs sign the current version.
- Durable membership-control outbox with Cloud's delivery semantics (stable `jti`, backoff, lease,
  entry barrier until a verified receipt). This replaces `revoke()` and `restless_membership_removals`,
  with a one-time migration of existing rows.
- Membership admin API: list members and invitations; invite; cancel an invitation; change role;
  suspend; reinstate; remove. CORS allowlist is exactly the cockpit origins from Placement.
- `/.well-known/restless-issuer` metadata document.
- `configure.mjs` refuses issuer and cockpit origins that aren't same-site.
- Delete `public/app.js` People screen. Keep sign-up, sign-in, verification, reset, invitation
  acceptance and **Open company**.

### OrgIntel

- Owner claim: the first verified `owner`-role context for a company with local history binds to the
  existing `owner` Actor. A second claim, or a claim from a non-owner role, is refused.
- First-entry fact: a human's first accepted entry records a fact addressed to Exec, carrying the
  Actor, the verified display name and the membership role. It is recorded once per Actor.
- Members projection read: bound humans with role, status, first and last entry, and Actor.

### Kernel / Authority

- Record the owner claim as an Authority bootstrap fact. The claim transfers no capability, mandate
  or root ownership.
- No other new governed state. Invariant: no membership role or status change alters grants,
  budgets or effect authority.

### restlessd

- Fetch, bound and cache the issuer metadata document with the JWKS client. Return it, or local
  mode, from `GET /api/companies/{company}/members`.
- Accept `suspended` controls from the canonical issuer. The verifier already supports them; this
  proves the path end to end.

### Owner cockpit

- **Company → Members** (`web/src/routes/[companyId]/company/members`), gated to owner and admin:
  - the Core projection merged with issuer members and pending invitations;
  - invite, cancel, role, suspend and remove actions, enabled only by capabilities the issuer
    publishes;
  - removal shows **Ending access…** until Core's receipt arrives, and is never shown as done earlier.
- Local-mode state: the owner row, one statement and a link to the setup guide. It has no disabled
  invite button.
- Tooltips, not subtitles, for role meaning and removal consequences. Follow
  `docs/FRONTEND_DESIGN_REFERENCES.md` in the final pass.

### Cloud (paired sprint in `restless-cloud`)

- `fleet-web` mounts the issuer library, pinned through the Core release lock. Fleet implements
  Placement from its topology snapshot and Mail from its adapter.
- Delete the duplicated membership hooks, the `people` route, and, once a real removal has passed
  through the library outbox, `handoff.rs`, `membership_control.rs` and
  `membership_control_reconciler.rs`.

## Acceptance

1. **Local honesty.** In a local `_test` company, Company → Members shows the owner and the local-only
   statement. No issuer request is made.
2. **Self-hosted journey, two real browsers.** On a fresh self-hosted `_test` install with two
   independent browser profiles:
   - the owner invites a colleague from the cockpit and receives a real delivered email, not a
     loopback capture;
   - the colleague accepts, enters and appears in Members with their own Actor;
   - Exec receives the first-entry fact and acts on it in the next wake;
   - the owner changes the colleague to admin, and the next entry carries the new role and a higher
     `membership_version`;
   - the owner suspends them: the open session and document connection end, and fresh entry is
     refused;
   - the owner reinstates them, then removes them. Attribution remains on everything they did.
3. **Durable removal.** Removing while Core is stopped blocks fresh entry at once. The outbox then
   completes delivery after Core restarts, with no owner retry. The cockpit shows **Ending access…**
   until the receipt arrives.
4. **Existing company keeps its owner.** A local `_test` company with owner-authored private documents
   and conversations switches to network entry. The owner's first network entry resolves to the
   `owner` Actor, and those documents and conversations open. A second owner-role claim is refused.
   Grants and budgets are unchanged.
5. **Same page on Cloud.** The same scenario 2 passes on a Cloud `_test` company with no Core diff.
   Only the issuer host differs. Record the Core commit, cockpit build digest and both issuer host
   versions.
6. **Adversarial.**
   - A member-role session can't call admin endpoints.
   - A cockpit origin outside the allowlist is refused.
   - A replayed handoff, and one signed with an older `membership_version`, are refused.
   - A client-supplied Actor or role in any request body is ignored.
7. **Cleanup.** Every `_test` company, issuer database, container and supervisor program created is
   removed, and `restless-reap --check` is clean.

## Ticket outline

- [x] C56-T0 — canon: ADR 0012 accepted; ADR 0009, cross-layer contract §2.4 and
      `docs/self-hosted-network-entry.md` amended ([ticket](sprint-56/c56-t0-canon.md))
- [x] C56-T1 — issuer library/host split, Placement and Mail ports, `services/identity`
      ([ticket](sprint-56/c56-t1-issuer-library.md))
- [x] C56-T2 — versioned membership, suspend/reinstate, durable control outbox, legacy removal
      migration ([ticket](sprint-56/c56-t2-versioned-membership.md))
- [x] C56-T3 — membership admin API, `/.well-known/restless-issuer`, same-site setup check
      ([ticket](sprint-56/c56-t3-admin-api.md))
- [x] C56-T4 — owner claim with Authority fact, Exec announcement, members projection
      ([ticket](sprint-56/c56-t4-owner-claim.md))
- [x] C56-T5 — issuer metadata probe and `GET /api/companies/{company}/members`
      ([ticket](sprint-56/c56-t5-issuer-probe.md))
- [x] C56-T6 — Company → Members; issuer People screen deleted
      ([ticket](sprint-56/c56-t6-members-page.md))
- [ ] C56-T7 — Cloud mounts the library and purges its duplicate issuer (paired `restless-cloud`
      sprint; [ticket](sprint-56/c56-t7-cloud.md))
- [x] C56-T8 — journeys, adversarial checks and cleanup, self-hosted half
      ([ticket](sprint-56/c56-t8-journeys.md))

### Evidence (23 September 2026)

- `npm test` in `services/identity`: 4/4 configuration checks, including the same-site rule.
- `npm run test:issuer` against a scratch PostgreSQL: metadata; the admin API answers only the
  cockpit origin; invitation link and email; members cannot manage; owner-only role changes advance
  the version; suspension reaches a signature-checking Core stand-in and blocks entry, and
  reinstatement is newer; removal during an outage completes from the outbox across an issuer
  restart with one stable `jti`. Negative control: a stand-in that echoes the wrong
  `requested_version` leaves the control pending and the run fails.
- `cargo test -p restless-orgintel --test access`: 11/11, including the owner claim, the
  refusal of a second claim, the pre-existing-owner case, and one Exec announcement per new human.
  The full OrgIntel suite passed except two `actors_and_teams` tests, which fail only inside another
  session's uncommitted edits to that file.
- `npm run test:core` with the built `restlessd`, the pinned company and Documents images, and
  Chromium (`RESTLESS_BROWSER_EXECUTABLE`): 16 PASS steps on a `_test` company. They cover a
  local-owner private document still opening after network entry; the owner mapped to `owner`; one
  Exec announcement; the Core members view naming the issuer; role change, suspension and
  reinstatement against real Core; removal during a Core outage delivered by the outbox with no
  retry; cross-origin and issuer-origin admin refusals; and, in the browser, the local-only
  Members state, the owner inviting and cancelling on desktop (1440) and mobile (390) with no
  horizontal scroll, and a member being refused. The runner removed its containers, volume,
  databases and roles.
- Not proven: Exec acting on the announcement in a live wake (the `_test` company had no model);
  real public SMTP delivery (mail was captured locally); two simultaneous independent
  browsers (the owner and member contexts ran one after the other); the Authority
  `owner_actor_claimed` row, which the journey could not read from the cell database; the Cloud
  host.

## Deletion

- `services/self-hosted-identity/public/app.js` People screen, `revoke()`'s synchronous path, the
  **Retry removal** state, and the constant membership versions.
- In `restless-cloud`: the duplicated Better Auth membership hooks, the `people` route, and Fleet's
  handoff signer, control and reconciler, after acceptance 3 passes on Cloud.
- The "does not convert an existing local-owner company" caveat in the self-hosted README.

## Exclusions and stop rules

This sprint does not build:
- SSO or OIDC federation into the issuer;
- custom roles, per-person permissions or an ACL editor;
- seat billing;
- presence;
- a user directory spanning companies;
- multi-owner companies;
- a network-to-local mode switch.

Membership roles stay `owner | admin | member`.

Stop and return to design if:
- the issuer needs any Actor, Room, Work or Authority state to do its job;
- a membership change alters grants, budgets or effect authority;
- the Cloud host needs a claim, audience or delivery rule the self-hosted host doesn't, because
  that is a fork of the signer contract;
- Cloud's production origins can't be same-site and the fallback would make Core relay admin calls.

## Open questions for founders (recommended answers)

1. **Signing-key custody → the Node host, with the library** (ADR 0012 Q1). This keeps one signer in
   both hosts. The key comes from a mounted secret file and rotates through JWKS `kid`.
2. **Role changes → owner only** (ADR 0012 Q3). Admins invite and remove members, never admins.
   This narrows today's self-hosted behaviour, where an admin can invite an admin.
3. **SMTP stays mandatory. Add "Copy invitation link" as a convenience.** Without mail there's no
   email verification, so an invited address would be unproven. The copied link still requires the
   invitee to sign up with, and verify, the invited address. It only helps when an email lands in
   spam.
4. **No owner Attention on first entry.** The owner sent the invitation, so they already know.
   Exec receives the fact and raises Attention only when it needs the owner's judgement, for
   example what the person is for.
5. **Cloud origins → same-site by construction** (ADR 0012 Q2). C56-T0 records the production
   hostname plan. Published company services go on a separate registrable domain.
