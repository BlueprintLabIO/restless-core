# Sprint 24 — Dogfood 6: Svelte interaction foundry

**Status:** Active — Dogfood 6 launched against the exact frozen baselines below

**Date:** 29 August 2026

**Scenario:** [Dogfood 6 — Restless interaction foundry](../dogfood/dogfood-6.md) v0.1

**Frozen Cloud baseline:** `a812052e3252d2c546562ffe6447e07809a6f5ee`

**Frozen Cult UI baseline:** `3b855612fb524cb042cc91b65f0cd575057471cc` (MIT; 77 public
component documentation entries, pending T0 manifest reconciliation against source registry)

**Depends on:** founder disposition of Sprint 23's exact public-site candidate; the canonical Cloud
site at `restless-cloud/site/`; the rendered Bridge Light product language; and a frozen, licence-
verified Cult UI public-component source manifest.

**Primary upstream references:** [Cult UI components](https://www.cult-ui.com/docs/components/) ·
[Cult UI repository](https://github.com/nolly-studio/cult-ui) ·
[Hero Liquid Metal](https://www.cult-ui.com/docs/components/hero-liquid-metal) ·
[Shift Card](https://www.cult-ui.com/docs/components/shift-card)

---

## Why this sprint exists

Sprint 23 corrected the Restless public site's identity, visual depth and Blog measure by grounding the
surface in the actual product. The resulting site is substantially better, but two limits remain:

1. Enlarged product-like snippets can sit in an uncanny middle: too literal to read as illustration,
   not faithful or operable enough to read as the product.
2. Restless has no maintained Svelte interaction foundry from which a design engineer can select
   high-quality motion, shader and micro-interaction ideas without repeatedly rediscovering and
   translating React libraries.

The founder also identified a concrete editorial defect: standalone Blog routes still expose Markdown
or repository locators that a public reader cannot open or interpret. A route existing independently
is not the same as an article being independently comprehensible.

This sprint turns those observations into one useful outcome: a complete, governed Svelte port of the
frozen free/public Cult UI component catalogue, plus a restrained site revision that demonstrates the
foundry's value without sacrificing Restless's product language.

## Founder's hypothesis

> Restless can build and govern a large reusable design-engineering capability while retaining whole-
> site taste: motion-rich components become idiomatic Svelte primitives, the best few improve the
> public narrative, pseudo-product UI becomes faithful or abstract, and Blog articles stand alone for
> readers who cannot access the repository.

This is a **company dogfood and acceptance sprint**, not a component-count benchmark. Catalogue
completeness is required, but the site is judged by selection and restraint rather than adoption count.

## Outcome

Using Restless itself as the operating company:

- freeze every component in Cult UI's current free/public Components navigation with exact source,
  licence and dependency identity;
- create a canonical internal registry and idiomatic Svelte 5 port for every frozen component;
- complete and review shader, motion, morphing, animated visual, carousel and text-effect families
  before low-value static catalogue completion;
- provide isolated live demos, APIs, provenance, accessibility, reduced-motion, responsive,
  performance and Restless-fit evidence for every accepted port;
- revise the Cloud site with a small selected subset, led by an abstract product-native Liquid Metal
  hero and only those supporting interactions that improve the complete page;
- make every product encounter either faithful or deliberately abstract;
- remove visitor-facing Markdown filenames and repository-only evidence dependencies from every Blog
  article; and
- return one exact locally hosted candidate and registry for fresh independent and founder review.

No push, merge, publication, deployment, paid-component access or upstream contact is authorised.

## Source and scope contract

The catalogue is mutable, so Sprint 24 begins by freezing:

- upstream repository commit and clean-source assertion;
- exact public documentation navigation and component URLs;
- exact licence and per-component third-party dependencies;
- screenshots or recordings of defining behavior where source alone is ambiguous;
- current Restless registry, product-language dossier and public-site commit;
- public Blog routes and every visitor-visible evidence/source reference.

"All Cult UI components" means all free components in the frozen public Components catalogue. It does
not include Cult Pro blocks, templates, promoted agent patterns or components added after the freeze.
The frozen manifest, not a remembered count, is the acceptance authority.

## Design direction

### Motion is the priority

Production returns value in this order:

1. shader and hero effects;
2. animated visual systems and SVGs;
3. morphing, expandable and spatial surfaces;
4. carousels, floating navigation and media motion;
5. buttons, cards, disclosures and micro-interactions;
6. text effects; then
7. static/utility completion.

This ordering is outcome priority, not a prescribed actor topology. Locally complete component
families may proceed in parallel once shared foundations and acceptance fixtures are stable.

### Selection, not spectacle

- Shift Card is evidence of a high-quality interaction, not the default card treatment.
- The public site uses only components with a clear narrative job.
- The Astro site may use frozen Cult UI React components directly as scoped islands when that is the
  highest-quality, lowest-friction implementation; the internal registry still requires genuine
  Svelte ports for catalogue completion.
- Bridge Light remains the source of visual truth; Cult UI supplies behavior and implementation ideas.
- A faithful product excerpt uses real structures, states and semantics. An abstract visual makes no
  claim to be an inspectable product screen. Nothing remains halfway between them.
- Motion density is evaluated at the page and whole-site level, not only component by component.

### Liquid Metal hero

Translate the defining shader behavior into a Restless brand abstraction. The public Astro hero may
use the original React implementation as a scoped island and must not wait for the separate Svelte
registry port.
It must have purpose-built desktop/mobile composition, stable content contrast, a static reduced-motion
state, bounded canvas cost and hidden/offscreen suspension. It does not copy Cult UI's example copy,
badges, identity or React composition.

### Standalone Blog

Every article is opened directly in a fresh context. It must define its claim, render necessary
evidence and limits, and offer readable public source destinations. Markdown paths may remain in build
metadata but may not appear as a reader obligation. Non-public evidence receives a human explanation,
not a dead local locator.

## Acceptance contract

Sprint 24 passes only when all of the following are true in the same versioned run:

1. **Catalogue manifest is complete.** Every frozen public component has a category, URL, exact source,
   licence, dependencies and registry identity with no unexplained omissions.
2. **Every in-scope component is ported.** Each has an idiomatic Svelte 5 implementation and defining-
   behavior demo; no React/Next runtime island counts as completion.
3. **Motion-first value is visible early.** All high-value motion families reach independent review
   before the static/utility tail defines completion.
4. **Registry is operational.** Components are searchable by family, behavior, cost, accessibility,
   reduced motion and Restless fit; state is exact (`candidate`, `ported`, `verified`, `rejected` or
   `blocked`).
5. **Behavior is native-tested.** Keyboard, pointer/touch, focus, responsive and reduced-motion
   behavior is observed where relevant; screenshots alone cannot verify interaction ports.
6. **Continuous work is bounded.** Shader/canvas effects pause when hidden or offscreen and stay within
   documented rendering limits on the accepted desktop/mobile profiles.
7. **Site remains coherent.** The final site uses a restrained subset, retains Bridge Light identity,
   and is judged better as a whole rather than merely richer in effects.
8. **Hero is accepted.** The Liquid Metal adaptation is high-quality, responsive, accessible,
   product-native and unmistakably abstract rather than fake UI.
9. **No uncanny excerpts remain.** A fresh critic classifies every UI-like public encounter as faithful
   or abstract, with no consequential ambiguous middle.
10. **Blog is genuinely standalone.** All articles pass direct-entry comprehension; no visitor needs a
    `.md` file, repository path or inaccessible source locator to understand the article.
11. **Portable and native gates pass.** Exact builds, routes, links, overflow, focus, console, motion,
    reduced-motion and local ReviewTargets are healthy for the registry and public site.
12. **Independent and owner review remain distinct.** A fresh critic judges exact behavior and rendered
    quality before the founder receives one bounded accept/request-changes ReviewTarget.

## Slice per layer

| Concern | Sprint slice |
| --- | --- |
| Kernel/product | No new coordination primitive is pre-authorised. Change only repeated product friction exposed by the dogfood. |
| OrgIntel | Preserve accountable family ownership, shared-foundation decisions, revision lineage, independent critique and one exact owner handoff. |
| Runtime | Provide isolated Svelte worktrees, browser/visual/shader tooling, supervised demo and site ReviewTargets, and exact build/runtime evidence. |
| Authority | Enforce no push, publication, paid access or upstream contact; local source and dependency use remains within the frozen licence boundary. |
| Restless Cloud | Own the canonical `site/`, internal registry/demo surface, Svelte ports and selected public integration. |
| Evaluation | Freeze the catalogue, inspect native behavior and complete pages, record interventions and classify the run without a component-count score. |

## Problem classification

**Deterministic and enumerable:** frozen catalogue membership; registry coverage; source and licence
identity; presence of Svelte ports/demos; React runtime absence; build/routes/links; raw `.md` or local
path exposure; animation lifecycle; reduced-motion state; focus/overflow/console facts.

**Judgment and open-ended:** Svelte API quality, interaction fidelity, shared abstractions, which ports
belong on the public site, whether the hero earns its prominence, whether an encounter is faithful or
abstract, and whether motion improves the whole experience. Static scoring cannot accept these.

## Proposed ticket decomposition

Founder alignment was received on 29 August 2026. Status lives only in this checklist; ticket files
record scope and closure evidence, not a second status system.

| Status | Proposed ticket | Slice | Outcome or friction served | Prior machinery made deletable |
| --- | --- | --- | --- | --- |
| [~] | **S24-T0 · Freeze catalogue, licence and accepted baselines** | Evaluation + Cloud | Mutable upstream scope makes "all" unverifiable | Remembered component counts and mutable browsing prompts |
| [ ] | **S24-T1 · Establish registry schema and Svelte demo harness** | Cloud + Runtime | Ports otherwise become disconnected files with no native review path | Ad hoc component notes and screenshot-only review |
| [ ] | **S24-T2 · Port and verify motion-first families** | Cloud | Distinctive behavior is the primary reusable value | Repeated one-off React translation and runtime islands |
| [ ] | **S24-T3 · Complete remaining public component families** | Cloud | A partial mine cannot satisfy the requested internal foundry | Untracked copy-paste fragments and catalogue ambiguity |
| [ ] | **S24-T4 · Art-direct the hero and selected site interactions** | Cloud + product design | Current snippets sit between faithful product and abstraction | Uncanny pseudo-product mockups and decorative over-adoption |
| [ ] | **S24-T5 · Make every Blog route independently readable** | Cloud + editorial | Raw Markdown/repository locators break the standalone promise | Reader dependence on internal source files or index context |
| [ ] | **S24-T6 · Independently critique, verify and return owner review** | Evaluation + OrgIntel | Mechanical completeness can hide poor motion, APIs or whole-site taste | Clean-build-as-design-acceptance and producer self-approval |
| [ ] | **S24-T7 · Classify Dogfood 6 and purge losing machinery** | Evaluation + touched layers | A large port can leave abstraction and dependency debris | Unused wrappers, duplicated motion engines and rejected site effects |

## Evidence package

The run creates evidence only after launch, including:

1. frozen source/catalogue/licence/dependency manifest;
2. complete registry coverage and per-entry state report;
3. Svelte APIs, demos and defining-behavior observations by family;
4. shared-foundation decisions and React-runtime absence check;
5. accessibility, reduced-motion, responsive and rendering-lifecycle observations;
6. before/after hero, UI encounter and full-page captures;
7. direct-entry audit of every Blog route and source destination;
8. exact registry and site commits, supervised URLs and portable gates;
9. independent critic and founder decisions; and
10. Dogfood 6 after-action, attention/spend/rework accounting and deletion record.

## Risks and dispositions

| Risk | Disposition | Treatment |
| --- | --- | --- |
| Upstream scope changes during the run | Guarded | Frozen manifest and commit define acceptance; later additions become a new revision |
| Licence or paid-source ambiguity | Invariant | Stop affected copying, preserve evidence and request exact authority/source clarification |
| Port count overwhelms quality | Guarded | Motion-first locally complete families, live demos and independent behavior review |
| Shared abstraction is designed before repeated evidence | Guarded | Extract only after at least two ports expose the same stable seam |
| Public site becomes a component collage | Invariant for acceptance | Separate registry completeness from site selection and judge whole pages natively |
| Continuous shader work harms mobile/accessibility | Invariant for acceptance | Static reduced-motion state, offscreen suspension, device-profile observation and bounded cost |
| Faithful product language becomes generic Cult styling | Invariant for acceptance | Bridge Light mapping and faithful-or-abstract critic classification |
| Blog exposes private/internal evidence | Invariant | Human-readable public evidence boundary; no private source is copied to satisfy a link |
| The company needs founder art direction | Accepted owner work | Prepare exact alternatives or native candidate; do not mechanise taste or ask for implementation rescue |

## Non-goals

- using every port on the Restless site;
- preserving Cult UI's React API, example copy, branding or Tailwind composition verbatim;
- importing Cult Pro or paid templates without separate access and licence authority;
- claiming universal accessibility, device performance or browser support;
- building a generic multi-framework compiler or automatic React-to-Svelte translator;
- publishing the registry, deploying the revised site or contacting upstream maintainers; and
- treating component count, animation count or visual novelty as product quality.

## Entry, stop and exit

**Entry:** founders approve this spec, exact source scope, model/spend envelope and current site baseline;
Sprint 23 has an explicit owner disposition; the Cloud `site/` checkout and dogfood company are isolated;
and the upstream licence/source probe is healthy.

**Stop:** stop for unsafe licence ambiguity, private/paid source exposure, uncontrolled publication,
credential/privacy crossover, branch corruption, spend breach or founder stop. Ordinary failed ports,
review rejection and design revision remain inside the run.

**Exit:** the frozen catalogue has no unexplained gap; every in-scope port and demo meets the contract;
motion-first families and selected site integrations pass independent native review; Blog routes are
standalone; the owner receives the exact registry/site ReviewTarget; and Dogfood 6 is classified
`accepted`, `rejected`, `inconclusive`, `product-invalid` or `source-invalid`.
