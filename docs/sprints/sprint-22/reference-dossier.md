# Sprint 22 reference dossier

**Frozen:** 28 August 2026

## Candidate before-state

| Item | Exact reference |
| --- | --- |
| Cloud integration baseline | `restless-cloud@d3c405a` (clean worktree at sprint entry) |
| Failed Restless-produced candidate | `restless-cloud@627d8ca0a7c42e5d8deda1413a00fd192bcd7867` |
| Original rendered evidence | `/company/worktrees/work-24062841e873-r1/artifacts/{desktop,mobile}-*.png` in company `restless_cloud_design_test` |
| Scroll-aware recapture | `/company/outputs/s22-before-web-review/manifest.json` and adjacent PNGs in company `restless_cloud_design_test` |

The original full-page capture never scrolled the candidate. IntersectionObserver sections remained
transparent and count-up values remained zero, creating large false-empty regions. The new capture
exercised every page before capture and exposed the content, while preserving further deterministic
faults:

- mobile Blog and Compare navigation links sit beyond the 390px viewport on every route;
- the Blog marquee contains focusable links outside the viewport;
- the mobile comparison table extends beyond its visible frame;
- every listed Blog link resolves to `/blog`, so no standalone article exists;
- the homepage rotating word abuts the preceding word (`yourresearch`, `yourrefactor`, or equivalent);
- the Product orbit calculation positions satellites relative to their own box and collapses them at
  the centre; and
- the public footer devotes 344 characters to implementation sources/licensing instead of useful
  customer navigation.

The capture tool does not claim that route height or text quantity is good or bad. The critic judges
the substantive problem: the homepage contains three movements and each primary subpage is mostly one
introduction plus one visual/list, leaving the product underexplained.

## Primary maturity reference

**Reference:** [Neon](https://neon.com/), current public homepage observed 28 August 2026.

Observed page movements:

1. ownership/product context and a decisive product thesis;
2. a broad but concrete product surface;
3. a hands-on setup/product encounter;
4. an advanced-autoscaling capability story with operational meaning;
5. instant-branching mechanics and user benefits;
6. integrated authentication;
7. production-grade assurances and concrete operational features;
8. scale/company credibility;
9. customer proof; and
10. a complete closing conversion path.

## Principles to transfer

| Mature quality | Restless-specific expression |
| --- | --- |
| Product thesis immediately reinforced by a product encounter | Intent enters the real cockpit and becomes attributable Work, evidence and a returned decision |
| Long-form narrative rhythm alternates explanation and proof | Alternate operating-loop demonstrations, actual interface fragments, workflows, evidence and objections |
| Capability sections explain mechanism and consequence | Companies, Work, Attempts, gates and owner decisions each show what changes for an operator |
| Trust is supported by operational facts | Show bounded authority, observable failures and prepared owner judgement without making safety the main pitch |
| Proof appears at several altitudes | Use exact public experiment evidence and product-native examples; never invented counters |
| Responsive completeness | Every primary route remains reachable and legible at 390px without horizontal clipping |
| Footer closes information architecture | Product, How it works, Research, Blog, Compare, Docs/GitHub where real, company/contact, status and legal destinations |

## Explicit non-copy boundary

Do not copy Neon's brand, green treatment, typography, illustrations, section order, component geometry,
copy, product taxonomy, customer claims or conversion language. Restless's signature is a visible
company operating loop: intention enters, responsibility forms, evidence accumulates and a prepared
decision returns. The mature reference sets the completeness and finish bar only.

## Restless design plan before production

**Subject and audience:** Restless is the operating system for owner/operators who want a company to
perform work rather than merely discuss it. The homepage's single job is to make that difference
concrete enough that a serious operator wants to inspect how it works.

**Palette:** Carbon `#0B0D10`, Paper `#F1F3F4`, Instrument `#A9B1B7`, Signal `#65F2C2`, Decision
`#D7B8FF`, Evidence `#F2C66D`. Signal green is reserved for the active operating loop, not sprayed
across every heading.

**Type:** a characterful engineered grotesk for display, a highly readable humanist sans for body,
and a restrained mono face for real system observations. The existing all-purpose Inter/system stack
does not carry enough identity.

**Layout:** a wide operating surface with alternating full-bleed demonstrations and disciplined
reading columns. Text width follows reading function; diagrams and product surfaces use the full grid.

```text
[navigation...........................................action]
[thesis......................live operating encounter......]
[proof rail / real observations............................]
[capability narrative..............product surface.........]
[workflow gallery...........................................]
[evidence + trust.............objections / comparison.......]
[research + blog............................................]
[closing decision...........................................]
[professional information footer...........................]
```

**Signature:** one orchestrated “company pulse” that moves an owner intent through real Work,
Attempt, evidence and decision surfaces. Other motion remains quiet.

**Self-critique:** near-black plus bright green is a common AI-site default. Restless keeps the dark
instrument-panel base because it belongs to the product, but removes the generic glowing-card and
rotating-word collage. The unique work is the operating loop and real product state, not the palette.
