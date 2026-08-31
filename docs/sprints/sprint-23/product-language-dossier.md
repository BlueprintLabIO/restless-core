# Sprint 23 product-language dossier

**Frozen:** 28 August 2026

## Authority

This dossier is derived from the current rendered Restless product and its source. It supersedes the
Sprint 22 marketing palette. Neon remains a maturity/completeness reference and does not supply visual
identity.

## Exact product sources

| Product quality | Source |
| --- | --- |
| Type, colour, spacing, radius, surface, shadow and motion tokens | `web/src/lib/design/tokens.css` |
| Type roles and hierarchy | `web/src/lib/design/type.css` |
| Controls, focus and held authority action | `web/src/lib/design/primitives.css` |
| Product material and shell | `web/src/lib/design/cockpit.css`, `chrome.css`, `surfaces.css` |
| Motion grammar | `web/src/lib/design/motion.css` |
| In-house pixel marks | `web/src/lib/primitives/MatrixGlyph.svelte` |
| Work/Attempt lineage | `web/src/lib/work/WorkGraph.svelte` and `layout.ts` |
| Company as an observable organisation | `web/src/lib/office/CompanyOffice.svelte`, `OfficeCanvas.svelte` and pixel assets |
| Owner/Exec relationship | `web/src/lib/components/AppShell.svelte`, `ExecutiveRail.svelte` and conversation primitives |
| Prepared judgement | Attention outcome folio in `web/src/routes/[companyId]/+page.svelte` |

## Product design grammar

### Colour

- `Machine field #e9edf3` — the product's ambient substrate.
- `Pane #fafbfd / Rail #fcfdff` — opaque reading and working surfaces.
- `Ink #171b24` — primary operational text.
- `Conversation #2f6ca8` — owner/agent communication and required handover.
- `Work #237563` — production, revision return and accepted feedback.
- `Direction #6447a6` / `Attention #6f58a8` — mandate and judgement.
- `Authority #8f5c16` — bounded consequential action.
- `Success #2f7752`, `Danger #ad4944` — observed outcome states only.

Colour answers “what kind of thing is this?” It must never be used merely to make a section louder.

### Type

- IBM Plex Sans: body, headings and controls.
- IBM Plex Mono: evidence, state, timing, source locators and compact system labels.
- Silkscreen at 16px: Restless wordmark only.
- Product headings are compact and precise. Public display type may extend the scale, but must retain
  Plex, sentence case and the same operational tone; no Georgia editorial italic or unrelated serif.

### Material and geometry

- A quiet dot matrix and very low-opacity semantic radial fields sit behind the work surface.
- Panes are separated by 4px machine channels, with 4/6/8px radii and a top-left bevel.
- Depth is shallow: one-pixel edges, subtle glass and bounded lift. Surfaces compose into one machine;
  they are not a gallery of floating marketing cards.
- Controls feel physical through bevel, press depth and state tint.

### Motion

- 110ms press, 180ms local state, 320ms disclosure, 650ms acknowledgement, 1400ms working pulse.
- The product is quiet at rest. Motion explains continuity, flow, state or a recorded consequence.
- Reduced motion collapses animation while preserving the complete state and topology.

## Rendered product observations

Reference captures live under `/company/inputs/s23-product-reference/` in the Sprint 23 Runtime:

- `attention-all-clear.png` — full product shell plus live CompanyOffice and Exec rail;
- `work-lineage.png` — requires/revises topology, stateful Work nodes and evidence labels; and
- `people-transcript.png` — organisation list, actor identity and full-width conversation.

The distinctive combination is not “light SaaS UI.” It is an observable company presented as a pale,
precise instrument: an inhabited pixel organisation below, accountable state/topology in the middle,
and human judgement returning through calm operational panes.

## Rejected Sprint 22 delta

| Rejected choice | Why it drifted | Required correction |
| --- | --- | --- |
| Cream `#f4f1e9`, acid lime, orange and saturated royal blue | Generic editorial/AI-marketing palette; not semantic product colour | Use Bridge Light machine/pane/intent tokens |
| Georgia italic display | Introduced an unrelated publication identity | Use Plex scale and mono/Matrix accents |
| Giant 66–112px headlines with extreme negative tracking | Marketing spectacle dominates the product | One bounded public display scale with product-like precision |
| Dark invented outcome mock | Does not resemble the actual owner cockpit or ReviewTarget | Use faithful Bridge Light pane, folio and control construction |
| Flat text sections and coloured slabs | Content depth without product demonstration | Replace prose runs with office, Work, evidence and decision encounters |
| `article-body` at 720px plus 18% indent | Visibly timid and compounds unused whitespace | 760–840px reading measure on a 1040–1160px publication grid |
| Evidence strip as the only article visual | Still a prose page with one box | Add article-specific figure/callout/evidence breakout and related navigation |

## Revised public design plan

### Subject and job

The subject is an observable, accountable company for owner/operators. The homepage's job is to let a
visitor feel the owner relationship: intent enters a living organisation, Work becomes inspectable,
evidence changes state, and one prepared judgement returns.

### Layout

```text
[product-native shell / nav..............................................]
[thesis..................animated intent → Work → evidence → decision...]
[inhabited company office viewport.............what is actually alive...]
[Work lineage canvas............................responsibility + gates...]
[evidence instrument............................claim → observe → accept.]
[prepared ReviewTarget..........................owner boundary............]
[research / Blog publication entries...................................]
[product-native information footer.....................................]
```

### Signature

One “company pulse” begins as a purple direction glyph, follows a blue requires path into green Work,
records an evidence acknowledgement, and arrives at an amber owner boundary. It can be replayed by a
button, announces the current stage, and becomes a static complete topology under reduced motion.

### Supporting visuals

- an inhabited office viewport adapted from the real CompanyOffice visual;
- a faithful Work/Attempt graph with solid `requires` and dashed green `revises` paths;
- a three-stage evidence instrument using actual source/observation/accepted-fact vocabulary;
- a prepared-decision folio with product controls and authority semantics; and
- route-specific diagrams, not repeated feature cards.

### Self-critique

The risk is turning a compact operational product into a microscopic marketing-site facsimile. The
public site therefore enlarges product structures enough to read, retains generous whitespace around
the machine, and uses no 11px prose. The one aesthetic risk is the pixel office as a large public
brand moment; it is justified because it is a real, unique Restless product surface rather than an
imported marketing gimmick. Other motion and graphics stay disciplined.
