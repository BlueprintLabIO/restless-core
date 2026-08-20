# Sprint 09 run report

**Run date:** 20 August 2026  
**Synthetic companies:** `sprint08_graph_test` and `sprint08_ui_test`  
**Selected real company:** `aris`  
**Current status:** T0–T3 and T5 are implemented and locally verified. T4 remains open for founder visual
review at the three specified viewport widths, full-reload control-continuity review in a rendered
browser, and a fresh prepared-handoff/outcome-native-review run. The configured in-app browser
reported no available browser, so this report does not turn headless HTTP, transport or production
build evidence into visual sign-off.

## Implemented outcome

- Company is the canonical fourth primary area. Its stable rail is Company charter, Authority &
  limits, Resources & access, External actions, Company computer and Company doctor. The former
  `/authority` page is only a compatibility redirect to `/company/authority`.
- One source-aware Company projection composes Authority, OrgIntel and Runtime reads without owning
  their records. Source failure remains explicit and does not become an empty charter, inventory or
  consequence history.
- The charter keeps durable Authority-owned purpose separate from linked current OrgIntel direction.
  An explicit borderless Markdown edit state saves only through the version-checked owner Authority
  action; ordinary chat and Exec narration remain unable to rewrite it. Authority & limits answers
  independent, owner-approved and prohibited work before exposing spend and grant detail.
- Resources are generic projection rows carrying source, state, observation time and optional live
  probe detail. No provider or installed-application navigation registry was added.
- External actions show governed consequences and preserve `provider_confirmed`,
  `authority_recorded`, `self_attested`, `reconciled` and `legacy_unverified` evidence instead of
  interpreting successful-looking text as provider confirmation.
- Company computer is a full-canvas threshold with one centred entrance. It reuses the existing
  authenticated desktop ticket, private transport and one-controller lease. Desktop focus collapses
  the Company rail while keeping the executive rail available; return restores the Company surface.
  A per-tab client identity survives reload without being inherited by a duplicated tab.
- Company doctor is a separate page for live diagnostics, evidence and the current bounded recovery
  action. It reads the same source-owned state as the computer without becoming a second writer.

## Contrasting projection evidence

Both synthetic companies returned HTTP 200 for all five original Company routes with the same route
order and semantics. The later visual-design follow-up added Company doctor as the sixth stable route;
the final six-route smoke check is recorded below.

- `sprint08_graph_test` had a stopped Runtime. Its doctor reported degraded, while its charter and
  limits remained readable. Its resource rows retained the configured model route, credential
  reference and timestamped Airwallex observation rather than becoming empty or live-claimed.
- `sprint08_ui_test` had a current running Runtime. The same shell showed its model route and five
  live supervised Runtime services. Its doctor reported healthy with no proposed action.

The projection tests also prove that an unavailable primary source cannot yield overall `healthy`,
and that words resembling success cannot upgrade legacy or self-attested evidence.

## Doctor and recovery run

The bounded recovery run used only `sprint08_ui_test`:

1. The Runtime began current and healthy with all five supervised services running and the browser
   available and unclaimed.
2. `restless down` stopped only its replaceable Runtime shell and preserved its named company volume.
3. The Company doctor observed `Stopped`, reported degraded and proposed only `start`.
4. The owner recovery endpoint accepted `start`, wrote an Authority lifecycle `requested` record,
   invoked the existing Runtime start path, re-probed the result and wrote `succeeded`.
5. The resulting Runtime was `Running`, image reconciliation was current, all services and the
   browser/desktop path became available, and the doctor proposed no further action.

No snapshot, workflow engine or second diagnostic state was introduced. Start, restart and reconcile
remain the only owner recovery operations, and an operation is rejected unless it is the current
live doctor recommendation.

## Real Company computer observation

The `aris` probe was read-only apart from issuing an ephemeral owner attach ticket:

- Authority, OrgIntel and Runtime sources were available.
- The existing container, six supervised services, browser, automation and desktop transport were
  running. The browser controller remained `agent`.
- The image needed reconciliation, so both `restless doctor -c aris` and the Company doctor honestly
  reported degraded and proposed only `reconcile`.
- An authenticated owner attachment followed the short-lived ticket redirect to the protected noVNC
  page and returned HTTP 200. The verification did not request owner control.

The real company was deliberately not reconciled: the sprint authorised implementation, not an
unrequested replacement of its running shell. No simulated capability, provider state or synthetic
receipt entered `aris`.

## Automated verification

Observed against the final implementation:

- `cargo test -p restlessd` — 94 passed;
- `cargo test -p restless -q` — 2 passed;
- `cargo fmt --all -- --check` — passed;
- `cargo clippy -p restlessd --all-targets -- -D warnings` — passed;
- `npm run check --silent` — zero Svelte errors and warnings, including the raw-type-size guard;
- `npm run build --silent` — production static build passed;
- scoped Prettier check over the Sprint 09 Company components, routes, models and styles — passed;
- direct desktop transport — first owner acquired control, a competing second client received 409,
  and return succeeded in `sprint08_ui_test`;
- all ten original Company routes across the two contrasting `_test` companies — HTTP 200;
- `restless doctor -c sprint08_ui_test` after recovery — live with no proposed action.

## Visual-design follow-up

The Company area now uses the cockpit's existing Bridge palette and type ramp with a consistent
recessed chassis, raised working surfaces, bevelled controls and focus states. The treatment follows
Beautiful UI's compact state density and executive composition, Cult UI's sculpted geometry and
continuous computer transition, and Origin UI Svelte's restrained semantic controls. No component
source or second visual system was imported. Svelte AI Elements remains an interaction reference for
the existing executive conversation; Motion Core, SvelteBits and Canvas UI remain deliberately
unused.

The latest implementation check observed:

- `npm run check --silent` — zero Svelte errors and warnings;
- `npm run build --silent` — production static build passed and emitted the Company doctor route;
- all six Company routes in `sprint08_ui_test` — HTTP 200;
- an authenticated owner attachment followed the short-lived ticket to the protected desktop and
  returned HTTP 200;
- `restless doctor -c sprint08_ui_test` — degraded with only `reconcile` proposed, because the
  frontend source change made the running image digest stale.

The synthetic company was not reconciled merely to make this follow-up green. The Runtime and all
supervised services remained available; the status accurately records that the running image no
longer matches the edited source.

### Aris operating-charter follow-up

The live Aris Authority configuration was re-authored through `restless company set` as a readable
operating charter: purpose, customers and value, business model, strategic intent, operating model
and operating principles. Repository paths, Work-graph syntax, test commands, effect-runner syntax,
credential mappings, campaign approvals and handoff mechanics no longer compete with the durable
company intent; their existing source owners remain unchanged.

The Company page now presents that charter as the primary reading surface, with current OrgIntel
direction and safe company identity in a separate context rail. The rail stacks beneath the document
as width narrows. The live API returned the new canonical charter and retained the current direction;
the persistent `/company/mission.md` projection was reseeded without replacing or restarting the
running container; and `restless doctor -c aris` remained live with no proposed action. The route
returned HTTP 200, `npm run check --silent` reported zero errors or warnings, and the production build
passed. Rendered viewport review remains part of the open founder evidence because the configured
in-app browser still reported no available browser.

### Versioned charter editing follow-up

The founder explicitly amended Sprint 09 to include direct owner editing. The implementation keeps
`CompanyConfig.mission` canonical and adds no Charter database or rich-text document model:

- the Company projection returns a SHA-256 revision of the exact UTF-8 Markdown;
- edit mode is a borderless, spellchecked Markdown manuscript with explicit Save and Cancel, an
  unsaved-state indicator and a before-leave guard;
- the same-origin local owner endpoint requires the opened base revision, serialises concurrent tab
  writes, rejects empty/oversized/NUL content and preserves a rejected browser draft;
- the Authority-owned action records owner-attributed `requested` and `succeeded` mandate evidence,
  atomically replaces the company config and refreshes `/company/mission.md` without restarting a
  running Runtime;
- Runtime projection failure is distinct from canonical-save success, and a stopped or absent
  Runtime is reported as deferred.

The live write proof used only `sprint08_ui_test`. A temporary exact-text revision returned
`evidence_status=recorded` and `runtime_projection=updated`; replaying the stale base revision
returned HTTP 409. The test then restored the original bytes and original revision
`sha256:19d63bedb30036c4563d38e4acfe1f8e526b7e9e5ebb783cd3f0a5c189cb3923`. The restored Runtime
`/company/mission.md` hash matched that revision exactly. Authority held the owner-attributed
requested/succeeded pairs. Empty input returned HTTP 400 in the earlier boundary probe, and the
bounded validator remains covered by the 94-test daemon suite.

Aris was reconciled during this explicitly authorised implementation pass after the source changed.
Its replaceable shell was rebuilt and replaced while the named company volume was preserved;
`restless doctor -c aris` then returned live with no actions. No synthetic mandate or provider state
was written to Aris.

Sprint 05's retained boundary suite remains the authority for invalid, expired and wrong-company
ticket refusal, protected nested assets, WSS transport and lease expiry. Sprint 09 reused that code
path rather than copying it.

## Remaining owner evidence

The in-app browser setup and its one permitted retry both returned no available browser. Per the
browser-verification contract, no unrelated automation stack was substituted. Before T4 can close, a
founder still needs to:

1. review all six Company pages, the executive rail and desktop focus/return at 390, 768 and 1440
   CSS pixels;
2. perform the owner lookup task for purpose, limits, resources, latest consequence and computer
   health;
3. reload while viewing and controlling, confirm the valid lease reconnects, and confirm a duplicate
   tab receives a separate identity;
4. exercise one freshly prepared desktop handoff through observable resume and one native review
   target that correctly bypasses the desktop;
5. review the new read/edit/save/cancel charter states, including the narrow layout and a stale-edit
   conflict presentation.

These are owner/rendered-product judgements or live-company authority decisions, not hidden green
checks. T4 therefore remains unchecked.
