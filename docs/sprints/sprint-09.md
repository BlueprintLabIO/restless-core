# Sprint 09 — A stable Company surface over the flexible company computer

**Status:** Active. Founder-authorised for implementation on 20 August 2026. Sprint 05 already
selected and proved the remote-desktop stack; this sprint must reuse that path rather than branch
another desktop, attach or controller system.
**Date:** 20 August 2026
**Spec refs:** `ARCHITECTURE.md` §2.1 / §3 / §5 / §6 / §9.2 / §16,
`owner-cockpit` §2 / §3 / §8 / §12.5 / §14,
`company-runtime` §2 / §3 / §12 / §14,
`authority-plane` §2 / §4 / §6 / §9,
`cross-layer-contract` §3 / §5 / §7 / §8,
Sprint 05 T1/T5 and Sprint 08 success contract 13

---

## Observed product gap

The current owner surface exposes **Authority** as a primary area, then places a long owner mandate
beside raw command and effect output. That is the wrong information architecture for an ordinary
owner:

- durable company purpose, enforceable authority, live Runtime state and diagnostic activity compete
  in one view;
- the mandate is rendered as one narrow wall of prose rather than a usable company charter;
- routine shell and Git failures receive more space than owner decisions or real external outcomes;
- provider, budget and Runtime categories appear fixed even though two companies may use entirely
  different tools, accounts, services and working conventions;
- the flexible Linux Runtime risks being copied into a growing cockpit taxonomy, recreating a
  universal domain model for arbitrary company work;
- the existing persistent desktop can be opened from prepared handoffs, but there is no calm,
  owner-initiated Company entrance that explains its state before attachment.

The architectural revelation is the opposite of a configurable admin dashboard:

> **Keep the owner-facing Company shell stable; let the company computer remain arbitrary.**

Linux already provides the open-ended substrate. Restless should not turn every discovered tool,
directory, provider or local application into a navigation item or a new platform concept. The
cockpit needs a small set of durable owner concepts, honest projections over source-owned state, and
one generic visual door into the real company computer when the abstraction is insufficient.

Sprint 05 already proved the latter mechanism:

- one persistent visible browser/desktop;
- authenticated short-lived attach tickets;
- private Runtime desktop endpoints behind the owner gateway;
- one deterministic controller lease;
- focus mode in which the desktop receives the main canvas;
- contextual conversation and return to the originating owner item.

Sprint 09 does not reopen that stack decision. It gives the proven capability a coherent place in
the product and tests it across materially different company shapes.

## Outcome

> **The primary navigation contains a Company area whose left rail remains recognisable across every
> company: Company charter, Authority & limits, Resources & access, External actions, Company
> computer and Company doctor. Each page projects the real company-specific state without hard-coded provider or tool
> assumptions. The charter has its own readable page and one explicit, versioned owner edit path
> through Authority; ordinary chat and Exec narration still cannot rewrite it. The Company computer page
> is a full-canvas threshold with one centred **Enter computer** action and honest controller state.
> Company doctor owns source health, diagnostics and prepared recovery controls. Entering the
> computer opens the existing persistent desktop in a full-canvas mode when direct inspection or
> takeover is useful. Outcome-native review still opens
> the exact site, document, media or prepared browser state first. The owner is never expected to
> hunt through an empty Linux desktop for work Restless could have prepared.**

The user-facing primary navigation becomes:

```text
Attention    Work    People    Company
```

The Company rail is deliberately stable:

```text
Company charter

Authority & limits
Resources & access
External actions
Company computer
Company doctor
```

Budget categories belong inside **Authority & limits**. Runtime lifecycle and recovery belong inside
**Company doctor**. A company-specific application, provider or tool may appear within Resources
& access or the real desktop, but it does not silently become permanent product navigation.

## Success contract

The sprint passes only when one observed end-to-end run, plus contrasting company projections,
demonstrates all of the following:

1. **Company replaces Authority as the primary product area.** The four primary areas are Attention,
   Work, People and Company. There is one canonical Company destination. The previous standalone
   Authority page is removed or redirects to the Company area's Authority & limits page; it does not
   remain as a competing surface.
2. **The secondary navigation is stable across companies.** Company charter, Authority & limits,
   Resources & access, External actions, Company computer and Company doctor remain in the same order and retain the
   same meaning for materially different company shapes. The UI does not generate navigation from
   installed packages, provider names, directories, credentials, Work types or model output.
3. **The charter is a real page, not a sidebar prompt.** It leads with the company's durable
   authorised purpose and may present owner-authored principles, intended customer and offer where
   the source contains them. It shows effective/revision context. Current direction is a clearly
   linked OrgIntel/Work projection and never silently becomes part of the durable charter.
4. **The charter has one deliberate owner write path.** The page offers an explicit edit mode and
   save action backed by the existing owner-controlled company configuration. Each save requires the
   revision the owner opened, records Authority evidence, atomically replaces the canonical config,
   and refreshes the Runtime's read-only projection when it is running. A stale revision is refused;
   neither casual chat nor Exec narration can rewrite the mandate. Sprint 09 does not create a second
   Charter store, strategy schema, collaborative editor or universal document lifecycle.
5. **Authority is owner-readable.** Authority & limits answers what the company may do independently,
   what requires owner approval and what it cannot do. Budget and approval categories come from
   current Authority state. Raw policy implementation, escaped JSON and ordinary command output are
   absent from the default view.
6. **Arbitrary resources remain arbitrary.** Resources & access renders the actual grants,
   connections, productive resources and Runtime-observed services available to the selected
   company. Unknown provider types remain displayable through generic source metadata. Adding a new
   company tool does not require a new global enum, route, sidebar item or provider-specific owner
   component.
7. **Capability claims are live and honest.** Runtime availability, browser/desktop health,
   connection state and resource usability come from live probes or timestamped source observations.
   Unavailable and stale remain distinct from absent, disconnected or revoked. The UI never infers a
   working capability from configuration alone.
8. **External actions contain consequences, not activity exhaust.** The page shows brokered effects,
   provider outcomes, confirmation/attestation distinction, unknown/reconciliation state and useful
   receipt evidence. Research, file edits, builds, shell commands and Git failures remain with the
   related Work/Attempt diagnostics unless they produced a governed external consequence.
9. **Company computer and Doctor have one job each.** Company computer is an immersive threshold with
   honest browser/controller state, relevant prepared handoff context and one centred **Enter
   computer** action. Company doctor is its own destination: it checks Authority, OrgIntel, Runtime
   persistence, image reconciliation, supervised services and browser/desktop state, keeps unknown
   distinct from healthy, and proposes only bounded source-owned repairs. Runtime identifiers and
   generations remain supporting detail.
10. **Desktop attachment reuses the Sprint 05 door.** Owner-initiated attachment from Company computer
    uses the same authenticated gateway, opaque attach reference, short-lived ticket, private desktop
    transport and deterministic controller lease as prepared Attention handoffs. No second remote
    desktop, direct Runtime address, durable browser credential, shell/filesystem API or provider-
    specific interaction path is added.
11. **The desktop gets a real focus mode.** Opening Company computer collapses the Company rail and
    gives the persistent desktop the main canvas. Exec chat and the current Attention item remain
    available on demand without permanently squeezing or covering the desktop. Returning restores the
    prior Company page and navigation state.
12. **Control ownership is explicit.** The surface distinguishes viewing, owner control and named
    actor control. V0 permits one controller of the shared browser/desktop at a time. Refresh/reconnect
    preserves the valid lease; owner takeover and return do not resolve Work or approvals by
    themselves.
13. **The prepared last mile remains the default.** A live site opens at the useful route and state; a
    document opens rendered; media opens in its player; an approval opens the exact bounded decision.
    Desktop takeover is used when the native outcome is the company desktop/browser or when ordinary
    bounded company-browser work or repair requires it. Provider-root administration, financial
    accounts, identity/KYB, MFA and initial credential issuance remain in the owner's external browser
    outside Company Runtime, per ADR 0002. The owner is not dropped onto an empty desktop to locate a
    file, service or tab Restless already knows.
14. **Return and resume are observable.** A prepared desktop handoff names the exact owner action and
    observable resume condition. When the external condition can be observed, Restless resumes
    without asking the owner to report completion. Sprint 08's causal continuation remains visible
    until the responsible actor, successor Work/Attempt and observed outcome or blocker are clear.
15. **The right rail remains one coherent executive surface.** Company pages use the same compact
    Exec chat / Attention switcher as the rest of the cockpit. Owner and agent messages remain
    visually distinct, message actions remain available, and raw activity does not compete with the
    reply or decision.
16. **Source ownership remains intact.** Authority owns the mandate, grants, limits, budgets, effects,
    receipts and Runtime lifecycle authority. OrgIntel owns current direction, Work, Attempts, actors
    and prepared handoff meaning. The Runtime owns files, browser/desktop state, tools, services and
    process reality. The cockpit composes projections and invokes source-owned actions; it never
    becomes another writer.
17. **Variation is proved rather than asserted.** At least two deliberately contrasting `_test`
    companies exercise the same Company shell with materially different resource/provider/service
    inventories, and one real selected company live-probes its Company computer and completes an
    authenticated owner attachment. Simulated capabilities never enter a live company.
18. **The layout remains useful at owner sizes.** Founder visual review at 390, 768 and 1440 CSS
    pixels covers every Company page, the Exec/Attention rail and desktop focus/return. No long charter
    column, giant unexplained gap, clipped navigation, raw log wall or permanently cramped desktop is
    accepted. Keyboard navigation and visible focus work through the rail, page actions and sidebar
    switcher.
19. **Partial failure never masquerades as emptiness.** Authority unavailable is not “no grants” or
    “no actions”; OrgIntel unavailable does not hide Authority or Runtime truth; Runtime offline does
    not blank the charter or limits. Each observation names its source and time. Confirmed success,
    confirmed failure, self-attested, unknown, reconciled and legacy-unverified outcomes remain
    distinguishable wherever that evidence exists.
20. **Company is not a second Attention queue.** Pending approvals and owner handoffs retain one
    canonical resolution in Attention. Company pages may show a count, context or link, but they do
    not duplicate decision state or offer a competing resolution path.
21. **The information architecture works as an owner task.** In the rendered product, a founder can
    locate the authorised purpose, independent/approval/prohibited boundaries, currently usable
    resources, latest consequential outcome and current computer/controller health without reading
    raw JSON, logs or opening the desktop. Any failed lookup or misleading answer is recorded as
    sprint friction rather than argued away from screenshots.
22. **Typography stays quiet and explanatory detail stays available.** Pages use headings and space,
    not decorative eyebrows or repeated subtitles. Concise hover explanations are also reachable by
    keyboard focus and accessible naming. No raw technical label is promoted merely to fill hierarchy.

## Existing baseline and scope decisions

| Capability                                                     | Current evidence                                                                           | Sprint 09 treatment                                                                                  |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------- |
| Persistent desktop, attach tickets, proxy and controller lease | Sprint 05 implementation and automated `_test` proof                                       | Reuse; add the owner-initiated Company entrance and rerun the boundary probes                        |
| Runtime, supervisor and browser diagnosis                      | `runtime::doctor` and `restless doctor`                                                    | Compose into one owner-facing company doctor; do not build a second diagnostic engine                |
| Durable mandate                                                | `CompanyConfig.mission`, owner-set and seeded into Runtime                                 | Add a versioned owner edit action over the existing source; do not add a second Charter store         |
| Runtime recovery writes                                        | start/stop/reconcile exist behind the daemon; owner BFF does not expose them               | Add bounded start, restart and reconcile actions with explicit consequences and lifecycle receipts   |
| Snapshot restore                                               | Architectural target; no proved owner action in the current build                          | Display no fictional control; defer until a real snapshot exists                                     |
| Resources                                                      | Credential references, provider observations, model access and supervised Runtime services | Compose generic presentation rows with source/freshness; no provider registry or new source of truth |
| External action evidence                                       | Generic effect intents/receipts plus provider-confirmed finance state                      | Preserve attestation, confirmation, unknown and reconciliation distinctions in the projection        |

One record has one primary owner-facing home:

| Information                                                            | Primary Company page | Cross-reference rule                                                         |
| ---------------------------------------------------------------------- | -------------------- | ---------------------------------------------------------------------------- |
| Mandate and safe legal/display identity                                | Company charter      | Current OrgIntel direction is a link, never folded into the charter          |
| Grants, prohibitions, approval policy, budgets and freeze              | Authority & limits   | Resources may reference the grant that makes access possible                 |
| Accounts, credentials, model/compute access and observed services      | Resources & access   | Health never implies broader authority than the referenced grant             |
| Effect intents, provider consequences, receipts and reconciliation     | External actions     | Pending owner decisions link to Attention rather than forming a second queue |
| Runtime lifecycle, persistence, doctor, browser/desktop and controller | Company computer     | Prepared handoffs retain their Work/Attention context                        |

## Product and information architecture

### Stable concepts, dynamic content

The rail is stable because it describes owner concepts rather than company implementation:

| Stable owner concept | Dynamic source-owned content                                                   |
| -------------------- | ------------------------------------------------------------------------------ |
| Company charter      | authorised purpose, owner principles and current mandate revision              |
| Authority & limits   | grants, prohibitions, approval thresholds and company-specific budgets         |
| Resources & access   | accounts, providers, compute, capabilities and Runtime-observed services       |
| External actions     | consequential intents, outcomes, receipts and reconciliation state             |
| Company computer     | persistent desktop/browser, current controller, prepared handoffs and recovery |

This is not a universal company ontology. A restaurant, game studio and education publisher may have
almost no tools in common. Their Resources & access pages may therefore look very different while
their route, hierarchy, state semantics and escape hatch stay familiar.

Company-specific tools may be pinned inside the Company computer or listed as resources after a live
probe. They do not automatically earn primary navigation. Promote a new stable product concept only
after repeated real companies reveal the same owner need.

### Company charter

**Company charter** is the owner-facing name for the durable owner mandate and its readable purpose,
not a second entity alongside it. The implementation reads and edits the smallest useful body
already owned by the host company configuration. Browser editing is an explicit owner Authority
action with optimistic revision checks and durable evidence; the cockpit never owns a copy.

The page may organise source content into readable sections such as mission and owner principles and
may show the Authority-owned safe legal identity where present. That presentation does not make each
heading a required global field. Missing content is omitted or explained honestly rather than invented.

The current top-level objective appears as a compact link to Work. Strategy and current objectives
change through OrgIntel; they are not smuggled into the mandate merely because the Company page can
display both.

### Authority & limits

This page answers three questions before it exposes implementation detail:

```text
What can the company do without me?
What will it ask me before doing?
What can it never do?
```

Budget envelopes, freeze state and exceptional approval requirements support those answers. The page
may offer explicit source-owned changes, grants, revocations and freeze/resume actions. Every control
states its practical consequence before execution.

### Resources & access

Resources & access is a generic inventory, not a provider catalogue. It may combine:

- Authority grants and connection metadata;
- scoped provider/account display information;
- productive resource grants;
- Runtime/Bridge capability and service probes;
- last successful use or observation time where source-owned and useful.

The projection may group items for comprehension, but the underlying authority and health state
remain source-owned. An Exec explanation can interpret an unfamiliar resource; model prose cannot
grant access or turn an unprobed capability into a working one.

### External actions

This page is intentionally much smaller than the current activity feed. It answers what happened in
the world under company authority. Its default rows show plain-language consequence, responsible
actor, time, outcome and the strongest evidence. Technical receipt payloads and redacted invocation
detail are progressive disclosure.

Ordinary Runtime failures remain inspectable through Work. A failed `git` command is not an Authority
event merely because it was recorded somewhere.

### Company computer

The Company computer page is both a status/recovery page and the universal visual escape hatch.

Before opening the desktop it shows only what helps the owner decide whether to enter:

- running, stopped, restoring or unavailable;
- browser/desktop live-probe result and freshness;
- who currently controls the shared session;
- the prepared session or handoff, when one exists;
- attach, start, restart and reconcile actions with consequences;
- external-authority freeze state where relevant.

**Open company computer** enters the existing Sprint 05 desktop focus mode. It is not an embedded
thumbnail surrounded by dashboards. The desktop is actual Runtime state, not a simulated file tree,
terminal transcript or cockpit-owned copy of the filesystem.

The user may open Company computer for direct inspection without an Attention item. Prepared owner
handoffs use the same door but pre-position the exact browser tab, application, file or external step
and retain their resume context.

## Layer slices and ownership

### Authority Plane

- Continue to own the mandate, limits, budgets, grants, provider/account scope, effects, receipts,
  freeze state and Runtime lifecycle authority.
- Expose generic owner reads plus the bounded Runtime start/restart/reconcile writes required by the
  Company doctor, and one versioned owner mandate revision action over `CompanyConfig.mission`. Do
  not add a Charter database, collaborative document service or snapshot lifecycle.
- Reuse the existing attachment authority and controller boundary; do not add a generic desktop
  command algebra.
- Do not semantically inspect the Runtime's arbitrary filesystem, applications or ordinary network
  work.

### OrgIntel

- Supply the current direction link/summary, responsible actors, Work/Attempt context and prepared
  handoff/resume meaning.
- Preserve Sprint 08's decision continuation after an owner step.
- Do not own grants, capabilities, provider outcomes, Runtime health or charter authority merely
  because they appear beside organisational context.

### Company Runtime and Runtime Bridge

- Continue to own actual files, Git, processes, browser/desktop state, applications and services.
- Report real process/desktop/browser health and controller state through the existing narrow bridge
  and attach mechanisms.
- Reuse the persistent desktop, private endpoints and imported supervision selected in Sprint 05.
- Do not add a Cockpit filesystem API, package inventory ontology, per-click RPC surface or a new
  service for every company application.

### Owner surface

- Rename and restructure the current Authority area into Company.
- Render one stable rail and source-aware dynamic pages.
- Offer one explicit Markdown edit/save mode for the charter; carry the opened revision to the
  source-owned write and preserve the owner's draft when a conflict or failure occurs.
- Keep the existing Exec/Attention rail coherent across those pages.
- Give the desktop the main canvas in focus mode and restore the prior page on return.
- Use progressive disclosure for receipts, diagnostics, Runtime ids and raw technical evidence.

The owner surface may aggregate these reads through the existing BFF/projection layer. That layer is
not authoritative state and should not gain a new Company database.

## Problem classification

This sprint combines deterministic state with open-ended company reality:

**Deterministic:** primary/secondary navigation, route identity, source health, attach-ticket
exchange, controller lease, Runtime lifecycle, Authority decisions and effect/receipt state.

**Judgement:** how Exec explains an unfamiliar company resource, which native ReviewTarget best
represents an outcome, and whether a repeated company-specific need deserves later product
promotion.

Do not solve the judgement half with a global provider/application enum. Do not solve the
deterministic half with model narration.

## Verification sequence

### 1. Source and projection checks

Create contrasting `_test` companies whose source-owned inventories differ materially—for example a
software/product company with repository and local services and an operations company with documents,
browser sessions and external accounts. Verify:

- identical rail items and route semantics;
- no UI allowlist for provider names, resource kinds or actor ids;
- unknown resource metadata remains renderable;
- stale/unavailable probes remain visibly distinct;
- Company pages invoke only source-owned write operations.
- concurrent charter saves reject the stale writer, successful revisions appear in Authority, and
  the Runtime projection is updated or honestly reported as deferred;
- source outages render unavailable/stale rather than empty inventories;
- confirmed, failed, self-attested, unknown, reconciled and legacy-unverified external action rows
  remain distinct;
- pending owner actions link to the one Attention item rather than duplicating resolution controls.

These projections demonstrate UI generality only. They do not claim simulated providers or services
exist in a live company.

### 2. Real Company computer probe

Against one selected real company:

1. Start through `restless-dev <company>` and pass `restless doctor -c <company>`.
2. Observe current Runtime, browser, desktop and web-transport health through the real APIs.
3. Open Company → Company computer and request owner attachment.
4. Verify one-time ticket exchange, authenticated WSS proxy, controller state and the absence of a
   direct public Runtime/desktop endpoint or durable credential in the page.
5. Reload the full SPA while observing and while controlling, reconnect within the lease, and return
   to the same Company page. A new tab does not inherit another tab's control.

### 3. Prepared last-mile and resume run

Prepare one real owner handoff whose best target requires the persistent browser/desktop. Confirm
that it opens on the exact useful state, keeps requester/Exec context available, distinguishes viewing
from control, observes the owner condition where possible and retains the causal continuation until
the responsible actor's next observed state appears.

Also open at least one outcome whose better native target is not the desktop. Confirm that Sprint 09
does not route every review through Linux merely because a desktop now has a Company page.

### 4. Owner visual review

Capture and review the following at 390, 768 and 1440 CSS pixels:

- Company charter;
- Authority & limits;
- materially different Resources & access inventories;
- External actions with an empty state and a consequential receipt state;
- Company computer before attachment;
- desktop focus mode with Exec chat and Attention available on demand;
- return from desktop to the prior Company page.

Visual inspection supplements, but does not replace, projection, source-action and attach-boundary
checks.

### 5. Purge

Before closing the sprint, remove:

- the old standalone Authority navigation destination or duplicated page;
- the long mandate/sidebar presentation;
- raw command/activity rendering from Authority/Company defaults;
- fixed provider, budget or capability UI lists not justified by source contracts;
- duplicated Runtime attach or controller code;
- obsolete CSS and fixture branches belonging only to the rejected layouts.

## Risks and dispositions

| Risk                                                           | Disposition              | Sprint treatment                                                                                            |
| -------------------------------------------------------------- | ------------------------ | ----------------------------------------------------------------------------------------------------------- |
| Different companies make navigation unpredictable              | **Guarded**              | Five stable owner concepts; company-specific tools remain content or desktop applications                   |
| Stable navigation hides a useful company-specific tool         | **Accepted**             | It remains reachable in Resources & access or Company computer; promote only after repeated dogfood         |
| Company becomes another dense admin dashboard                  | **Guarded**              | One page question at a time, outcome/limits first, progressive technical disclosure                         |
| Desktop becomes the primary owner workflow                     | **Guarded**              | Outcome-native targets first; desktop only for inspection, takeover or genuinely native desktop work        |
| Owner is dropped into unprepared Linux state                   | **Guarded**              | Handoffs pre-position the exact useful state and preserve observable resume context                         |
| Runtime capability is claimed from stale config                | **Guarded**              | Live probe or timestamped observation; unavailable and stale stay explicit                                  |
| Doctor becomes a second orchestration engine                   | **Guarded**              | It composes existing checks and invokes only named start/restart/reconcile operations                       |
| Owner and agent control the shared browser simultaneously      | **Guarded**              | Reuse the deterministic single-controller lease and visible control state                                   |
| Remote desktop becomes publicly reachable or leaks credentials | **Invariant**            | Reuse Sprint 05 private endpoints, owner gateway, scoped tickets and authenticated WSS proxy                |
| Company projection becomes a second source of truth            | **Invariant**            | Source-owned reads/actions only; no Company store or copied filesystem/provider state                       |
| Two open charter editors overwrite one another                  | **Guarded**              | Save requires the exact base revision; stale writes return a conflict without discarding the local draft    |
| Rich-text conversion changes authorised wording                | **Guarded**              | Edit canonical Markdown directly; no beta Markdown↔rich-document round trip or per-keystroke server writes  |
| Renaming Authority obscures the safety boundary                | **Accepted and watched** | Authority & limits remains explicit in the rail and global authority state remains visible                  |
| Arbitrary Runtime state is messy or partially broken           | **Accepted**             | The Runtime is recoverable productive state; Work diagnostics, snapshots and repair handle ordinary failure |

## Explicitly out of scope

- per-company configurable primary or secondary navigation;
- arbitrary owner-created dashboards or widgets;
- turning installed applications, packages, files or providers into global product entities;
- a provider marketplace, capability registry or universal provider interface;
- a new Charter database, strategy ontology or business-model schema;
- real-time collaborative charter editing, presence or CRDT infrastructure;
- a Cockpit file manager, terminal, IDE or per-command Runtime API;
- a second desktop, browser farm, parallel profile or replacement remote-desktop stack;
- rebuilding Sprint 05 attachment, ticket, proxy or controller mechanics;
- snapshot creation or snapshot restore before a real snapshot implementation exists;
- making the desktop the review target for every outcome;
- storing browser credentials, desktop passwords or raw provider secrets in the owner projection;
- redesigning Attention, Work or People beyond the shared navigation/right-rail integration required
  by Company;
- multi-owner collaboration or cross-company administration.

## Tickets

| ✓   | Ticket                                                                                                                | Layer                               | Evidence served                                                                                                                             | Depends        |
| --- | --------------------------------------------------------------------------------------------------------------------- | ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | -------------- |
| [x] | [**S09-T0 · Freeze the Company projection and doctor contract**](sprint-09/t00-company-contract.md)                   | Cross-layer + Owner BFF             | The draft promised versioned mandate and recovery paths the current APIs do not own, while source failures can still render as empty arrays | —              |
| [x] | [**S09-T1 · Replace Authority with one stable Company shell**](sprint-09/t01-company-shell.md)                        | Owner surface over all sources      | Purpose, limits, resources, consequences and Runtime health currently compete in one provider-shaped page                                   | S09-T0         |
| [x] | [**S09-T2 · Put a general doctor at the Company-computer door**](sprint-09/t02-company-doctor.md)                     | Runtime + Authority + Owner surface | A company can be stopped, stale or partially broken, but the owner must reconstruct recovery from CLI output and raw Runtime details        | S09-T0, S09-T1 |
| [x] | [**S09-T3 · Reuse the persistent desktop as a true Company focus mode**](sprint-09/t03-company-computer.md)           | Runtime + Owner surface             | The proved desktop exists only behind prepared Attention items and full-page reload/control continuity is not explicit                      | S09-T1, S09-T2 |
| [ ] | [**S09-T4 · Prove variation, comprehension and purge the old Authority surface**](sprint-09/t04-dogfood-and-purge.md) | All touched layers                  | Generality, calm hierarchy and the new recovery door are assertions until contrasting companies and one live computer exercise them         | S09-T1–T3      |
| [x] | [**S09-T5 · Let the owner revise the charter without creating a second writer**](sprint-09/t05-charter-revision.md)   | Authority + Runtime + Owner surface | The readable charter still forces character-level owner changes through the CLI and offers no conflict-safe revision context in the cockpit | S09-T0, S09-T1 |

## Exit evidence

The sprint closes with:

- the aligned ticket checklist and completed ticket evidence;
- source/projection checks for contrasting `_test` company shapes;
- a real `restless doctor` result and authenticated Company-computer attachment;
- an owner-facing doctor result plus one bounded recovery action whose lifecycle receipt and
  post-repair probe agree;
- security probes for invalid, expired and wrong-company tickets plus direct-endpoint refusal;
- one prepared desktop handoff with observed return/resume;
- one outcome-native review that correctly bypasses the desktop;
- 390/768/1440 founder-reviewed captures of the Company pages and focus mode;
- the exact old Authority/log/layout paths deleted during purge;
- aligned `owner-cockpit` and `cross-layer-contract` wording naming Company as the canonical fourth
  primary area;
- one `_test` company charter revision proving Authority evidence, stale-write refusal, canonical
  config persistence and Runtime projection behaviour;
- a run report that distinguishes source observations, Runtime reality, provider confirmation and
  owner judgement.

Current observed evidence and the remaining founder-review gap are recorded in
[`sprint-09/run-report.md`](sprint-09/run-report.md).
