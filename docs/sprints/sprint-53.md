# Sprint 53 — Mobile judgement console

**Status:** Draft for founder alignment
**Programme:** [Core company collaboration](company-collaboration-programme.md)
**Paired Cloud sprint:** Cloud 20
**Depends on:** Sprints 45–52

## Outcome

On a supported phone, a repeat visitor sees safe cached personal Attention immediately, reads/replies
in Chat, reviews Work/Artifacts/Doc proposals, answers one judgement request and backgrounds/resumes the
app without losing route, scroll or drafts. The client remains responsive and canonical rather than
becoming an offline company database.

## Scope

- Turn the existing Core Svelte cockpit into an installable PWA with a small authenticated shell,
  manifest, service worker and exact-object deep links.
- Make phone navigation `Attention`, `Chat`, `Work`, `People`, `More`; keep search/Ask Exec global and
  open Docs/Artifacts contextually. `Work` is a projection, not a new state machine.
- Persist selected TanStack Query state and drafts in IndexedDB, partitioned by user/company/schema;
  clear it on logout, membership removal, company switch or incompatible release.
- Render cached safe state before revalidation, reconcile on resume from event cursor and preserve
  current route/scroll/keyboard/composer state.
- Add safe optimistic Chat, Work and Attention actions with canonical reconciliation. Authority effects
  show pending until an authoritative receipt; destructive/conflict-prone offline actions do not queue.
- Lazy-load Docs, Runtime, diff, large Artifact and diagnostics bundles only when opened. Paginate and
  virtualize unbounded lists; reserve layout dimensions.
- Apply the three-level context ladder (glance, understand, inspect) and review-first mobile Docs and
  Artifacts.
- Enforce provisional performance budgets with stage instrumentation and real devices/poor networks.

## Acceptance

1. Repeat launch renders useful cached Attention in the budgeted posture and reconciles without a blank
   shell or full SPA reload.
2. A common tap paints within 50–100 ms; cached route changes target under 150 ms; same-region mutation
   acknowledgement and cross-client visibility are measured against the programme targets.
3. Backgrounding for ten minutes preserves route, scroll and drafts, then reconnects from cursor or
   performs a bounded resync.
4. Sending a Message inserts it locally, returns one canonical ID and never duplicates on timeout retry.
5. Opening Attention does not load editor/terminal/diff bundles; opening a Doc loads collaboration only
   then and reconnects safely.
6. Membership removal invalidates the persisted company cache and every further protected request.
7. Offline drafts are explicit; Authority, ownership transfer, destructive lifecycle and conflicted
   Work actions are never shown as submitted while offline.
8. Keyboard, screen-reader, reduced-motion and supported mobile-browser journeys pass the scoped audit.

## Ticket outline

- [ ] C53-T0 — performance baselines and bundle budgets
- [ ] C53-T1 — PWA shell, cache partition and logout/revocation purge
- [ ] C53-T2 — mobile information architecture and progressive disclosure
- [ ] C53-T3 — optimistic commands and resume reconciliation
- [ ] C53-T4 — capability splitting, pagination and stable layouts
- [ ] C53-T5 — accessibility, poor-network and real-device proof

## Deletion and exclusions

Delete full-screen default spinners, full-app event refetches, memory-only repeat-visit state and
desktop controls squeezed onto phones. Do not build a native app, global client domain store,
ElectricSQL/offline multi-master replica, service-worker mutation queue or mobile IDE/terminal parity.
