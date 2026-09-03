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
| `matrix-glyph` | Native | `web/src/lib/primitives/MatrixGlyph.svelte` | In-house 5×7 marks for identity and state | Wordmark, route/state marks and diagram nodes; never sentences |
| `machine-field` | Native | `web/src/lib/design/tokens.css` | Establishes the observable-company substrate | Pale blue-grey field, faint semantic radials and 14px dot matrix |
| `pane-machine` | Native | `tokens.css`, `cockpit.css` | Composes one instrument from bounded work regions | Large product encounters with 4px seams, top-left bevel and restrained lift |
| `physical-control` | Native | `primitives.css` | Makes a bounded action feel pressable and consequential | CTAs, tabs and replay controls with 110ms press and 180ms state response |
| `tab-arrive` | Native | `cockpit.css`, `motion.css` | Punctuates a newly selected surface | One 650ms semantic sweep after selection; never ambient looping |
| `acknowledge` | Native | `motion.css`, `HoldApprove.svelte` | Confirms that a state was recorded | Evidence attached, decision prepared or hold completed |
| `work-lineage` | Native | `web/src/lib/work/WorkGraph.svelte` | Shows requires/revises responsibility and current state | Enlarged outcome topology; solid blue requires, dashed green revision return |
| `inhabited-office` | Native | `web/src/lib/office/` | Makes the company and active responsibility visible | One large pixel-world brand moment or route-specific organisation figure |
| `conversation-band` | Native | conversation primitives, `chat.css` | Distinguishes owner, agent and context before reading | Product explanation, annotated transcript or role handoff |
| `outcome-folio` | Native | Attention folio in `[companyId]/+page.svelte` | Returns evidence and one bounded judgement | Faithful ReviewTarget demonstration and final conversion boundary |
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

## Rejected patterns

| Pattern | Reason |
| --- | --- |
| Cream/ink/acid editorial identity | Contradicts Bridge Light and was explicitly rejected in Sprint 23 entry review |
| Georgia italic display voice | Creates an unrelated publication brand |
| Generic neon glow / spotlight field | Reads as AI-landing-page shorthand rather than observable-company behaviour |
| Text reveal as a substitute for content design | Animation does not turn prose into a product encounter |
| Screenshot-exists acceptance | Mechanical presence is not design judgement |
| Narrow nested article column | Wastes the publication grid and was explicitly rejected by the owner |

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
