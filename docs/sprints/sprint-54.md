# Sprint 54 — Released collaboration candidate

**Status:** Draft for founder alignment
**Programme:** [Core company collaboration](company-collaboration-programme.md)
**Paired Cloud sprint:** Cloud 21
**Depends on:** Sprints 45–53

## Outcome

One immutable, self-identifying Core release contains the complete collaboration subsystem and passes
the same self-hosted and hosted acceptance corpus. Cloud can pin and deploy it unchanged; no local
patch, mock service or operator-created hidden state is required.

## Scope

- Reconcile `ARCHITECTURE.md`, ADRs, Core specs, generated contracts and release manifest into one canon;
  archive or mark superseded proposal language instead of retaining contradictory active rules.
- Package OrgIntel/chat, Core cockpit/PWA, native Doc sidecar, migrations, readiness, backup/export and
  compatibility metadata as one released cell/account-plane candidate.
- Publish machine-readable access, command/query/event, collaboration-token, sidecar health and release
  contracts consumed by Cloud without hand-maintained copies.
- Run migration/rollback, backup/restore, revocation, two-client, Runtime-suspension, context-restart,
  document convergence and mobile performance suites against exact artifacts.
- Prove a local/self-hosted identity adapter with no Cloud dependency and a Cloud fixture against the
  same Core API semantics.
- Verify all temporary `_test` companies, processes, sidecars, services, volumes and caches are removed
  after the run.
- Produce a native review package with exact images/digests/revisions, limits, known failures and Cloud
  handoff—not only test logs.

## Acceptance

1. The programme's twelve acceptance conditions pass against the exact release artifact.
2. Fresh install and supported upgrade preserve Actor attribution, Work, Messages, Docs, versions,
   outbox cursor and Authority history.
3. Backup/restore recovers Chat and Docs with the cell and does not repeat an already completed effect.
4. Runtime stop/replacement leaves lightweight collaboration available and later wakes the correct
   durable Actor from queued Attention.
5. Self-hosted use requires no Fleet/Better Auth dependency; Cloud use requires no Core source patch.
6. Generated contract compatibility fails closed on a mismatched Cloud consumer or sidecar version.
7. Release identity is visible in cockpit, API, events and Doc diagnostics without logging private
   message/document bodies.
8. A final deletion audit finds no duplicate DTOs, owner shortcuts, blanket polling, competing Doc
   truth, obsolete assertion verifier or full-company prompt assembly.

## Ticket outline

- [ ] C54-T0 — canon and migration freeze
- [ ] C54-T1 — release manifest, sidecar and generated contracts
- [ ] C54-T2 — upgrade/rollback and backup/restore corpus
- [ ] C54-T3 — full self-hosted acceptance
- [ ] C54-T4 — Cloud compatibility and adversarial corpus
- [ ] C54-T5 — purge, native review package and immutable release

## Exclusions and stop rules

This sprint does not authorize deployment, public launch, billing, shared multi-company compute or
market-demand claims. Stop release for cross-company access, assertion replay, lost document content,
unbounded revocation, membership-as-Authority, unreconciled external effects or an acceptance path that
depends on manual database edits or mutable artifacts.
