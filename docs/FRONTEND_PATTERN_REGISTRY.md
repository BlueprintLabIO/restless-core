# Restless frontend pattern registry

This is the internal source-of-truth registry for reusable product, landing-page and publication
patterns. It answers three questions before an agent invents or imports a component:

1. Does Restless already have the mechanism?
2. What semantic job does it perform?
3. If an upstream implementation saves work, how must it be adapted to Bridge Light?

The registry records patterns, not screenshots. The rendered product and `web/src/lib/design/` remain
authoritative. External libraries never supply palette, type, geometry or product vocabulary.

## Status vocabulary

- **Native** — exists in the product and may be reused or faithfully enlarged.
- **Public-ready** — adapted for a public site and verified responsively.
- **Candidate** — useful upstream mechanism; inspect source, licence and accessibility before use.
- **Rejected** — conflicts with the product or failed rendered review.

## Restless-native visual and interaction patterns

| ID | Status | Product source | Semantic job | Public use |
| --- | --- | --- | --- | --- |
| `matrix-glyph` | Native | `web/src/lib/ui/glyph/MatrixGlyph.svelte` | In-house 5×7 marks for identity and state | Wordmark, route/state marks and diagram nodes; never sentences |
| `machine-field` | Native | `web/src/lib/ui/style/tokens.css` | Establishes the observable-company substrate | Pale blue-grey field, faint semantic radials and 14px dot matrix |
| `pane-machine` | Native | `tokens.css`, `cockpit.css` | Composes one instrument from bounded work regions | Large product encounters with 4px seams, top-left bevel and restrained lift |
| `physical-control` | Native | `primitives.css` | Makes a bounded action feel pressable and consequential | CTAs, tabs and replay controls with 110ms press and 180ms state response |
| `surface-indicator` | Native | `chrome.css`, `AppShell.svelte` | Shows where the owner moved between surfaces | One indicator slides to the selected tab (320ms spring); phones use a bottom dock. Replaced the per-tab `tab-arrive` sweep |
| `acknowledge` | Native | `motion.css`, `HoldApprove.svelte` | Confirms that a state was recorded | Evidence attached, decision prepared or hold completed |
| `work-lineage` | Native | `web/src/lib/work/WorkGraph.svelte` | Shows requires/revises responsibility and current state | Enlarged outcome topology; solid blue requires, dashed green revision return |
| `inhabited-office` | Native | `web/src/lib/office/` | Makes the company and active responsibility visible | One large pixel-world brand moment or route-specific organisation figure |
| `conversation-band` | Native | conversation primitives, `chat.css` | Distinguishes owner, agent and context before reading | Product explanation, annotated transcript or role handoff |
| `outcome-folio` | Native | Attention folio in `[companyId]/+page.svelte` | Returns evidence and one bounded judgement | Faithful ReviewTarget demonstration and final conversion boundary |
| `artifact-navigation` | Native | `web/src/lib/ui/navigation/ArtifactSidebar.svelte` | Finds and opens company Docs/Sheets with calm icon/title rows | Company work navigation; data and permissions remain page/model-owned |
| `evidence-chain` | Public-ready | Sprint 23 dossier | Separates source, observation, accepted fact and decision | Interactive or static strip using semantic state colours and source locators |
| `company-pulse` | Public-ready | Sprint 23 homepage | Explains intent → responsibility → evidence → judgement | The one orchestrated public signature; complete static reduced-motion state |

## Publication patterns

| ID | Status | Contract | Use |
| --- | --- | --- | --- |
| `publication-grid` | Public-ready | 1040–1160px article grid; 760–840px prose measure | Standalone Blog and finding routes |
| `evidence-breakout` | Public-ready | Wider than prose, source-located, green work/evidence semantics | Supporting records without squeezing them into paragraph width |
| `article-figure` | Public-ready | Subject-specific diagram with caption and same information in text | One or more genuine visual explanations per article |
| `article-navigation` | Public-ready | Previous/index/next with meaningful titles | Keeps every entry standalone but connected |
| `reading-rail` | Candidate | Sticky only when it adds real orientation; disappears on mobile | Status, experiment/source scope and section position |

## Upstream implementation mines

These entries are discovery routes. Before copying code, record the exact component URL/commit,
licence, dependencies, keyboard/touch behaviour, reduced-motion behaviour and the Restless pattern it
implements. If no native semantic job can be named, do not import it.

| Source | Status | Mechanisms worth mining | Required Restless translation |
| --- | --- | --- | --- |
| [Amicro](https://amicro.enisdev.com/) | Candidate | magnetic/press response, tilt, cursor response, text reveal, carousel mechanics | Use only for `physical-control`, a bounded product viewport or one signature; remove boutique-demo styling |
| [sv-animations](https://sv-animations.vercel.app/) | Candidate | copyable Svelte reveals, borders, surfaces and motion sequences | Bind to Bridge motion roles and semantic tokens; no glow collage |
| [TFE Svelte Templates](https://tfe-svelte-templates.vercel.app/) | Candidate | restrained motion primitives, ambient surfaces, layout and data visualisation | Use for Work/evidence/organisation mechanics with product typography and geometry |
| [Aceternity UI Svelte](https://aceternity.sveltekit.io/) | Candidate | spotlight and 3D implementation references | Rarely appropriate; only if it explains a product threshold, never as ambient hero decoration |
| [SveltoUI](https://sveltoui.dev/) | Candidate | broad component search when a precise mechanic is missing | Treat as a code index; Origin/native primitives win for ordinary controls |
| [Origin UI Svelte](https://originui-svelte.pages.dev/) | Candidate | polished conventional controls and interaction details | Preserve accessibility, replace visual styling with Bridge controls |
| [Svelte Animations source](https://github.com/SikandarJODD/animations) | Candidate | inspectable implementation source | Pin the exact file/commit before adaptation |

## Sprint 63 adoption — Apps

Pattern ID: `apps-surface`

Outcome/route: `web/src/routes/[companyId]/apps/` (the surface, `AppTile.svelte` and the app page)
and the app-request branch of `InboxDetail.svelte`.

Native semantic source: Bridge Light `Item` rows for anything the company already has or that
needs the owner; one restrained tile grid only for Browse, where comparison across many unowned
apps is the job. The Sprint 38 launcher rejected an app-store grid for launching owned artifacts;
here the grid is limited to discovery, and owned apps stay in rows.

Beautiful UI reference: [Recommendation Card](https://www.beautifului.dev/), inspected 6 October
2026. The agent's suggestion leads with the ask and offers one primary action; Restless keeps that
order for Exec's app requests (reason first, **Add** last) and drops the confidence meter, because
an app request is a sign-in step, not a judgement. No source copied.

Cult UI reference: [Minimal Card](https://www.cult-ui.com/docs/components/minimal-card), inspected
6 October 2026. Borrowed the restraint: one border, a soft shadow only on hover or focus, no
gradient or texture. The Agent Suggest Card Stack was rejected as decorative motion for a
rarely-used surface.

Keyboard/touch observation: tiles and rows are native links and buttons; category chips are
buttons with `aria-pressed`; the chip row scrolls sideways on phones instead of stacking; every
action is reachable without hover.

Reduced-motion observation: the tile's hover transition is removed under
`prefers-reduced-motion`; the Apps dot reuses the existing badge entrance.

Desktop/mobile evidence: inspected live on a seeded `_test` plane on 6 October 2026 at desktop
width and 375 px (Inbox app request, Apps, an app page with permission levels, Browse).

Elements removed during final restraint pass: built-in know-how from the main In use list (now
folded under "Comes with Restless"), a requested app's duplicate tile in Browse, "Sign in again"
on apps that were never signed in, and the search box from the title row on phones.

## Rejected patterns

| Pattern | Reason |
| --- | --- |
| Cream/ink/acid editorial identity | Contradicts Bridge Light and was explicitly rejected in Sprint 23 entry review |
| Georgia italic display voice | Creates an unrelated publication brand |
| Generic neon glow / spotlight field | Reads as AI-landing-page shorthand rather than observable-company behaviour |
| Text reveal as a substitute for content design | Animation does not turn prose into a product encounter |
| Screenshot-exists acceptance | Mechanical presence is not design judgement |
| Narrow nested article column | Wastes the publication grid and was explicitly rejected by the owner |
| Standalone company-creation form as the primary path | It makes the owner configure a company before explaining the business. The intended primary path is a conversation that helps shape the mandate, then prepares creation for explicit owner approval. The account plane still performs the creation and authority checks; a company Exec does not receive host-root authority. Keep a direct form only as a secondary, deliberate shortcut if it earns its place. |

## Adoption record template

```text
Pattern ID:
Outcome/route:
Native semantic source:
Upstream source + pinned commit (if any):
Code copied or mechanism reimplemented:
Licence:
Keyboard/touch observation:
Reduced-motion observation:
Desktop/mobile evidence:
Elements removed during final restraint pass:
```

## Sprint 38 adoption — owner artifact launcher

Pattern ID: `artifact-launch-rail`

Outcome/route: `web/src/routes/[companyId]/company/resources/+page.svelte`

Native semantic source: Bridge Light `pane-machine`, `physical-control` and the existing resource
evidence table.

Beautiful UI reference: [Task Rows and Records Table](https://www.beautifului.dev/), inspected 3
September 2026. The calm one-row identity/state/action rhythm informed the launch rail; no source code
was copied.

Cult UI reference: [Expandable Screen](https://www.cult-ui.com/docs/components/expandable-screen),
repository commit `3b855612fb524cb042cc91b65f0cd575057471cc`, MIT. The useful mechanism was a
single explicit trigger revealing a larger usable surface. Restless reimplemented only that state
transition in native Svelte; it rejected the full-screen morph, scroll lock, React and Framer Motion.

Svelte source-first reference: [Interactive Hover Button](https://sv-animations.vercel.app/magic/docs/components/interactive-hover-button)
and its public registry source, inspected 3 September 2026. The duplicated hover label, expanding dot
and hover-only transformation were rejected. Restless retained its existing keyboard-visible,
pressed-state `physical-control`, so no dependency or inaccessible hover behavior entered the bundle.

Keyboard/touch observation: Open is a native button; unavailable states use `disabled`; the viewer has
a named close button; Company Computer uses normal navigation.

Reduced-motion observation: no ambient or layout animation was imported. Existing press motion is
removed by the product-wide reduced-motion rules.

Desktop/mobile evidence: pending final rendered acceptance in Sprint 38 T6.

Elements removed during final restraint pass: app-store card grid, duplicated provider metadata,
full-screen morph, gradient/glow decoration, hover-only copy replacement and automatic iframe load.

## Streaming Markdown

`web/src/lib/primitives/Markdown.svelte` uses `svelte-streamdown` 4.2.0
([source](https://github.com/beynar/svelte-streamdown/tree/085b97daaa046d1ddd5b73354322e6f16c7b6628), MIT).
It serves `conversation-band`: stable completed blocks while the live tail changes,
partial tables/fences, and highlighted code with an accessible copy control.
Restless owns typography and geometry; no Tailwind runtime or upstream visual theme is imported.
Raw HTML rendering and model-supplied components are disabled; links and images go through
Streamdown's URL checks. Animations are disabled, including for reduced-motion users.
Only the visible live-reply surface is rendered. Static documents use the same component
without incomplete-Markdown repair. Mermaid and math remain lightweight source fallbacks.

Regression check against a running local cockpit:
`RESTLESS_TEST_SOURCE_COMPANY=<existing-company> npm run verify:markdown` from `web/`.
Install Playwright Chromium first (`npx playwright install chromium`) if needed.
The check proxies GETs only and supplies conversation/SSE fixtures to a browser-only
`markdown_renderer_test` path; it does not create or write a company. It verifies DOM
identity across updates, partial tables and code, highlighting/copy, unsafe HTML/URL
handling, final-message handoff, and narrow/mobile overflow. The temporary SSE server
and browser are closed on success or failure.

## Shared document header simplification — 22 September 2026

Native `physical-control` and document editor surfaces, using the browser's top-layer popover for
secondary document actions. Beautiful UI's compact task/action hierarchy, Cult UI's explicit reveal,
and Origin UI Svelte's ordinary compact controls were consulted; no upstream code or dependency was
copied. One title and sync state replace stacked request/editor headers; type selectors and the
recovery subtitle are removed. Required actions remain keyboard/touch accessible, Escape and
outside click dismiss the popover, and it stays within the desktop/mobile viewport without animation.

## Shared Docs/Sheets navigation — 2 October 2026

`artifact-navigation` combines native `physical-control` and `pane-machine`: one compact artifact
switcher, creation action, search, recent/name ordering and a scrollable icon/title list. Rows remain
native links with modifier-click and full-title tooltips. Arrow keys enter and move within the list;
Home/End move between rows while retaining native search-input caret behavior. Enter opens, Escape
clears search then restores focus, and `/` focuses search only from within navigation. Editors and
the global company command menu retain their own shortcuts. Mobile opens a full-width browser list
and transfers focus to the visible editor/back control.

Beautiful UI's Sidebar Nav/Search informed density and alignment; Cult UI's Side Panel informed calm
inset surfaces; Origin UI Svelte's compact input/navigation controls informed keyboard-visible
chrome. Linear's official search interaction documentation informed clear temporary filtering and
keyboard escape behavior. No external source, runtime or visual theme was copied. Restless tokens
own type, color and spacing; no ornamental animation was added.

The component owns transient presentation only. Page scope keys reset search/order/focus on company
or verified-principal changes. Docs search belongs to `lib/model`, uses the existing permission-aware
named-version search endpoint, and cancels/discards stale completions across query, company,
principal and access changes. Its titles/snippets stay in memory. Sheets keeps its existing accepted
revision and access-epoch boundary. Search pagination and ordering refer to the loaded results.

## Cockpit inbox and Company settings — 2 October 2026

`AttentionInbox` replaces the old multi-line navigation cards with 52px native links, grouped by
Decide, Approve, Review and Fix setup. Neutral selection, icon-only category color, relative time,
unread state, filtering, ordering and one row action menu carry the hierarchy. Right click opens
that same menu; J/K, arrows, C, H, Cmd/Ctrl+period and `?` stay scoped away from text inputs.
Mobile pushes from a full-height inbox to a detail page with a Back link. Device-local snoozing
has Undo; authoritative decisions keep their existing revision and approval boundaries.

`SettingsHeader`, `EmptyState`, `RelativeTime` and `ActionMenu` consolidate Company page chrome.
The menu uses the browser top layer to escape scroll clipping, remains within the viewport,
closes on selection/outside click/Escape, supports arrow/Home/End focus, and has 44px touch actions.
Company uses one grouped spine, a compact mobile picker and existing type tokens. Secondary detail
stays behind disclosure. No ambient animation was added; existing motion respects reduced motion.

Polish references inspected: [Beautiful UI](https://www.beautifului.dev/) for compact task/sidebar
hierarchy; [Cult UI Expandable Screen](https://www.cult-ui.com/docs/components/expandable-screen)
for a single explicit reveal; and
[Origin UI Svelte dropdown-08 source](https://github.com/max-got/originui-svelte/blob/main/src/lib/components/dropdowns/dropdown-08.svelte)
for grouped native actions and shortcut alignment. Only those qualities were reimplemented.
No upstream source, theme, React runtime or dependency was copied. The full-screen morph,
hover-only behavior and decorative surfaces were rejected.

Verification: 16 cockpit routes at 1440, 390 and 320px, plus mobile detail and Doctor disclosure
(50 captures), with no horizontal overflow or browser page errors. Browser-only fixtures use
read-only GETs with writes rejected; they verify presentation, not live provider availability.
Keyboard navigation, snooze/Undo, stale-link feedback and mobile Back were exercised. Real isolated
`cockpit_ux_test` API checks separately verified schedule create/pause/resume/cadence and stale-write
refusal, Vault creation without returning secret values, charter A/B/A history and stale revision
refusal, and agreement between catalog and Doctor on missing intelligence. Schedule controls and
schedule appliance integration tests passed, as did the declared-schema/migration invariant.
The web check reports zero errors/warnings; the production web and daemon builds passed.

Additional interaction acceptance passed: desktop context-menu focus/Escape; unchanged rail DOM
and scroll across Company navigation; skill search and agent disclosure; touch menu placement and
44px actions; mobile picker; fresh loading/error/retry and blocked empty states. The real test
company rendered all three charter revisions (one current), the E edit shortcut, and Doctor's
missing-provider action. Browser page errors remained empty.
