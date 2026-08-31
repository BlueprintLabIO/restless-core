# Sprint 23 — Product-native public surface

**Status:** Implementation and independent critique complete; awaiting owner review

**Date:** 28 August 2026

**Depends on:** Sprint 22's factual web-review capability and rejected Cloud commit `7e17ef0`;
the current rendered Bridge Light product in `web/src/lib/design/`; the Cloud-owned public source.

## Why this sprint exists

Sprint 22 fixed route depth, copy structure, standalone Blog routes, footer utility and mechanical
rendering defects. It still failed the visual outcome. The public site invented a warm editorial
identity that could belong to another company, used text to do work that product graphics should do,
and constrained long-form articles to a timid column. A clean manifest cannot make those choices good.

The primary reference was also wrong. Neon is useful for completeness, rhythm and finish, but the
actual Restless product is the source of truth for how Restless looks, moves and speaks.

## Outcome

One corrected Cloud website that feels like the public threshold of the Restless product:

- Bridge Light's materials, typography, geometry, semantic colour and motion vocabulary govern every
  public route;
- the homepage explains the company operating loop through memorable product-native visual encounters,
  not long runs of marketing prose;
- product, process, research and comparison pages use diagrams and interface artefacts appropriate to
  their subject;
- every Blog article is a substantial publication page with a generous readable measure and breakout
  evidence/figure structures; and
- the useful content depth, factual claims, Blog vocabulary, route completeness and professional footer
  from Sprint 22 remain intact.

No deployment, push or public replacement is authorised.

## Source-of-truth hierarchy

1. **Rendered Restless product / Bridge Light** — authoritative visual language and interaction model.
2. **Product source** — tokens and primitives in `web/src/lib/design/`, MatrixGlyph, WorkGraph,
   CompanyOffice, conversation, evidence and ReviewTarget surfaces.
3. **Restless research record** — authoritative claims, examples and vocabulary.
4. **Neon** — completeness and finish calibration only.
5. **Amicro, TFE, sv-animations and other component sources** — implementation mines only; every used
   behaviour must be restyled and semantically justified by the product.

## Design contract

### Product language

- IBM Plex Sans for interface/prose, IBM Plex Mono for evidence and labels, and Silkscreen only for the
  mark/wordmark.
- Pale blue-grey machine field `#e9edf3`, opaque light prose panes, subtle glass, dot-matrix texture,
  pane seams and one top-left light source.
- Semantic colour only: conversation blue `#2f6ca8`, work/feedback green `#237563`, direction/attention
  purple, authority amber, restrained success and danger. No decorative acid palette.
- Compact 4/6/8px geometry, bevelled controls, quiet borders and bounded shadows. No inflated marketing
  cards or giant editorial slabs alien to the product.
- Motion explains press, disclosure, state transition, flow and acknowledgement. It stops at rest and
  has a complete reduced-motion equivalent.

### Required visual encounters

- **Signature hero:** an orchestrated owner-intent pulse travels through Work, Attempt, evidence and a
  prepared decision using faithful product panes and semantic state colour.
- **Native product window:** a recognisable Restless review/cockpit surface, not a dark invented mock.
- **Organisation view:** product-native company/people or office topology showing responsibility.
- **Evidence view:** source → observation → accepted fact expressed as a diagram or interactive strip.
- **Work view:** actual Work/Attempt lineage or graph with state changes and clear ownership.
- Supporting routes must each contain at least one subject-specific figure, interface fragment or
  diagram; decoration alone does not satisfy this requirement.

### Blog layout

- Desktop article prose uses a visibly generous editorial measure around 760–840px, with body copy
  large enough for long reading and without an additional narrow nested column.
- Article header, metadata, source locator, figures, pull quotes, evidence bands and related navigation
  may break out to the wider 1040–1160px publication grid.
- Mobile collapses to one padded column without horizontal scrolling. Code/data overflow intentionally
  within its own scroll container.

## Acceptance contract

1. A product-language dossier maps every public token and repeated component back to the current
   product source or names the deliberate public-site extension.
2. The old cream/ink/acid identity and overly narrow Blog rule are absent from final computed styles.
3. Desktop and mobile captures show the five required native visual encounters and route-specific
   visuals on Product, How it works, Research and Compare.
4. The signature interaction works with pointer/keyboard where interactive, settles at rest, and has a
   reduced-motion state containing the same information.
5. All five Blog posts remain standalone and demonstrate the wider editorial grid, breakout structures
   and coherent related navigation.
6. A fresh critic compares the exact candidate first with the actual product and only second with Neon;
   “mechanically clean” is insufficient for acceptance.
7. The exact final commit passes portable build, route/link, desktop/mobile/reduced-motion and supervised
   preview checks with no hidden authored content, overflow, off-viewport control or browser error.
8. The owner receives a new ReviewTarget tied to the revised attempt and can accept or request changes.

## Tickets

Status lives only here.

- [x] [S23-T0 — Freeze product-language truth and rejected deltas](sprint-23/t00-product-language.md)
- [x] [S23-T1 — Extend web review for design-language evidence](sprint-23/t01-review-evidence.md)
- [x] [S23-T2 — Build product-native public visuals and interaction](sprint-23/t02-public-visuals.md)
- [x] [S23-T3 — Rebuild Blog publication layout](sprint-23/t03-blog-layout.md)
- [x] [S23-T4 — Critique, verify and return revised owner review](sprint-23/t04-final-review.md)

## Layer slices

| Concern | Owner | Sprint responsibility |
| --- | --- | --- |
| Visual and interaction truth | Restless product + Cloud | Derive public system from Bridge Light and product-native artefacts |
| Factual capture | Company Runtime | Reuse the Sprint 22 tool; add only observations needed to make comparison inspectable |
| Revision lineage | OrgIntel | Preserve the owner's rejection, new attempt, producer/critic responsibility and exact ReviewTarget |
| Final taste | Owner | Judge the rendered revised site; deterministic gates cannot accept it |

## Entry, stop and exit

**Entry:** the owner explicitly rejected Sprint 22's candidate and named the three gaps. The Cloud host
checkout is clean at `7e17ef0`, one unpublished commit ahead of origin.

**Stop:** stop for source corruption, unauthorised public effect, inability to preserve the current
candidate, or evidence that the product source itself is ambiguous. Ordinary revision is expected.

**Exit:** the product-language mapping is explicit; the revised site visibly uses it; graphics,
interaction and Blog measure pass native review; a fresh critic accepts the exact candidate; and a
healthy revised ReviewTarget opens for the owner.
