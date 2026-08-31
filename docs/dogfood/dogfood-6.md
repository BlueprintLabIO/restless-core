# Dogfood 6 — Restless interaction foundry

**Status:** Active — founder aligned; catalogue freeze and company commissioning in progress

**Version:** 0.2

**Type:** Company-level design-engineering dogfood with a real reusable-library and public-site outcome

**Operating phase:** Source, port, govern, integrate and independently review

**Company:** `restless_interaction_foundry_test` (or an equivalent dedicated `*_test` company frozen
at launch)

**Depends on:** [Sprint 24](../sprints/sprint-24.md); the exact founder-reviewed Sprint 23 Cloud site;
the rendered Bridge Light product language; and the canonical Cloud site at `restless-cloud/site/`.

## Scenario identity and purpose

This scenario asks whether Restless can turn a large external interaction catalogue into a coherent,
reusable Svelte capability while improving its own public site. The economically useful output is not
"more components." It is a governed interaction foundry and a visibly better Restless site whose
motion, graphics and editorial surfaces remain unmistakably Restless.

The run is a dogfood because Restless must perform the same design-engineering work the capability is
meant to support: freeze changing upstream sources, divide a broad catalogue into locally complete
families, port behavior rather than syntax, preserve licensing and provenance, reject unsuitable
patterns, maintain a visual system, and return a native result for owner taste.

This is not a causal experiment and does not claim that a large agent team outperforms one designer.
It is an acceptance and diagnosis run for one substantial design-engineering outcome.

## Starting mission shown to Exec

> Build a complete internal Svelte interaction registry from Cult UI's frozen public component
> catalogue, prioritising the motion and shader ideas that provide distinctive design value. Port every
> in-scope component idiomatically to Svelte, with provenance, accessibility, reduced-motion,
> responsive, performance and native-demo evidence. Then use only the small subset that genuinely
> improves the Restless public site: replace the hero graphic with a product-native Liquid Metal
> interpretation, make selected supporting surfaces feel more alive, resolve uncanny pseudo-product
> UI by making each encounter either faithful or deliberately abstract, and make every Blog article
> fully standalone for a public reader. Preserve Bridge Light as the visual source of truth. Do not
> push, publish, deploy, buy a licence, contact an upstream maintainer or make another external effect.

Exec receives the mission once. The evaluator freezes the catalogue and acceptance contract before
production; it does not prescribe team topology, component ownership or implementation order beyond
the motion-first outcome priority.

## Desired owner outcome

The founder can inspect one prepared result and directly observe that:

- every component in the frozen free/public Cult UI catalogue has one accountable registry entry and
  an idiomatic Svelte port, or a precise source/licence blocker that prevents a valid port;
- the highest-value motion, shader, morphing, animated visual, carousel and text-effect families were
  completed and reviewed first rather than waiting behind low-value static utilities;
- the registry is usable as an internal design-engineering tool, with live demos, source/provenance,
  dependency cost, accessibility, reduced-motion and Restless-fit guidance;
- the public site uses a deliberately small, coherent subset instead of becoming a catalogue collage;
- the hero contains a recognisable Liquid Metal idea translated into Restless's product language;
- product snippets are either faithful enough to trust or abstract enough not to impersonate a real
  product state;
- every Blog article makes sense on its own and exposes no unreadable Markdown filename or repository
  locator to the visitor; and
- a fresh critic has reviewed the exact rendered site and exact registry rather than accepting build
  health as design quality.

## Frozen upstream scope

At run entry, create a manifest from the current [Cult UI component catalogue](https://www.cult-ui.com/docs/components/)
and exact [Cult UI repository](https://github.com/nolly-studio/cult-ui) commit and licence. Cult UI is
currently presented as an MIT-licensed React/Next.js, Tailwind and shadcn-compatible source library;
the run must verify the exact licence and dependency terms from the frozen source rather than relying
on this sentence.

For this scenario, **all components** means every free component linked under the public Components
navigation at the frozen timestamp, across:

- marketing and heroes;
- buttons;
- expandable widgets;
- cards and surfaces;
- frames and mockups;
- textures and overlays;
- visual systems;
- navigation and floating UI;
- inputs and decision UI;
- AI and productivity widgets;
- media; and
- typography and text effects.

Paid Cult Pro blocks, templates, promoted AI-agent patterns and unrelated linked products are outside
scope unless the founder separately adds them with valid access and licence authority. Upstream
components added after the frozen manifest do not silently expand the run.

## What counts as a port

A registry row, copied JSX file or visually similar screenshot is not a completed port. Each component
must have:

1. an idiomatic Svelte 5 implementation with no React or Next.js runtime dependency;
2. a stable public API expressed through Svelte props, snippets, events, actions or stores as
   appropriate rather than mechanically mirroring hooks and React composition;
3. a live isolated demo covering its defining behavior, not only its resting frame;
4. keyboard, focus, pointer and touch behavior appropriate to the component;
5. complete reduced-motion semantics and a useful static state;
6. responsive behavior at the catalogue's declared supported sizes;
7. bounded dependency and rendering cost, including offscreen/hidden suspension for continuous canvas
   or shader work;
8. exact upstream URL, frozen source identity, licence and material adaptation notes;
9. a Restless design translation or an explicit warning that the primitive is not visually native;
10. independent verification of behavior and rendered quality.

Ports may use appropriate Svelte-native motion or shader foundations. They must not keep a React
compatibility island merely to claim catalogue completeness. This registry acceptance rule does not
ban the Astro public site from using a scoped React island: the founder explicitly permits original
Cult UI React implementations on the site when direct integration produces the strongest result with
less translation risk.

## Motion-first production order

The company owns sequencing, but the outcome priority is fixed:

1. **Shader and hero systems:** Liquid Metal, dithering, heatmap, animated SVG, lens/blur, grid and
   ambient visual systems.
2. **Spatial motion:** morphing and expandable surfaces, carousels, floating panels, docks and
   direction-aware navigation.
3. **Micro-interactions:** animated buttons, disclosures, hover/touch card behavior, polls and
   acknowledgement transitions.
4. **Typography and media motion:** text animation, typewriter, numbers, pixel text, video and loading
   experiences.
5. **Static and utility primitives:** useful catalogue completion after the distinctive behavior
   families are reviewable.

Shift Card is a high-quality reference, not a required site motif. No individual component receives
special treatment merely because the founder mentioned it.

## Restless visual-selection rule

The internal foundry may faithfully preserve upstream behavior in isolated demos. The Restless public
site may adopt a component only after it passes a second translation test:

- it answers a real narrative or interaction need;
- it speaks Bridge Light through Restless type, semantic colour, geometry and material;
- it is either a faithful product encounter or a clearly abstract brand graphic;
- it does not simulate inspectable product state that the visitor cannot actually operate;
- it does not duplicate an existing primitive with no material quality gain; and
- the complete page becomes better, not merely more animated.

Astro's framework interoperability is intentional. A selected public-site effect may use the frozen
React implementation directly through an Astro React island instead of waiting for its Svelte port.
That choice does not count as a completed Svelte registry port. Hydration must remain scoped to the
interactive surface, and the same accessibility, reduced-motion, responsive and lifecycle evidence
still applies.

The site must remain quieter than the registry. Registry completeness is not site-component density.

## Hero Liquid Metal contract

The hero uses the interaction idea from Cult UI's
[Hero Liquid Metal](https://www.cult-ui.com/docs/components/hero-liquid-metal), not its example copy,
brand or layout wholesale. Its frozen React implementation may be used directly as the Astro island
when that preserves the effect best.

- The graphic is an abstract Restless brand moment, not a fake product screen.
- Its mask, tint, field and surrounding composition derive from Restless marks and semantic palette.
- Text contrast and CTA hierarchy remain stable while the shader moves.
- Mobile receives a specifically tuned variant, not a cropped desktop canvas.
- Reduced motion presents an intentional static frame with the same composition.
- Continuous rendering pauses when hidden or offscreen and respects a documented pixel/work budget.
- The effect is independently judged in the complete hero at desktop and mobile; a shader demo alone
  cannot pass the outcome.

## Standalone Blog contract

Markdown may remain the internal authoring format, but it must be invisible as an implementation detail.
Every public Blog route must include enough rendered context to be read without the repository, another
article or prior Restless knowledge:

- an intelligible title, deck and thesis;
- definitions and setup needed to understand the claim;
- rendered figures, evidence explanations and limits;
- human-readable source labels and public destinations where available;
- no raw `.md` filename, local path, source locator or experiment-file reference as the only way to
  understand or verify a statement;
- a graceful explanation when underlying evidence is not publicly available;
- related navigation that is useful but not required for comprehension.

The evaluator opens every article directly in a fresh browser context. Index-first reading is not
accepted as a substitute.

## Actor and organisation constraints

- Begin with Exec and the smallest justified accountable design-engineering boundary.
- Exec does no production. Leads supervise, reconcile the system and judge complete returned units;
  they do not privately port components or repair the site.
- Component families are valid locally closing production units, but the evaluator does not prescribe
  one worker per family or a fan-out quota.
- Shared Svelte foundations, registry schema and design tokens have named ownership; parallel work may
  not fork them silently.
- The site integrator selects from accepted ports only and owns whole-page coherence.
- A fresh critic who did not produce the port or site judges behavior and rendered outcomes.
- The owner decides irreducible taste, including whether the hero and overall motion density feel right.

## Authority and resource envelope

**Allowed:** read the frozen public source; edit isolated Core/Cloud worktrees; create local Svelte
ports, demos, registry records, screenshots and build artifacts; install compatible open-source local
dependencies after exact inspection; run browsers, builds, accessibility and performance probes; make
local commits and prepare a ReviewTarget.

**Requires owner approval:** push, merge, publish, deploy, purchase or access paid components, contact
upstream authors, change public licensing statements, or make an externally attributable claim.

**Prohibited:** copying paid/private source without authority; presenting React wrappers as Svelte
ports; hiding missing catalogue items; publishing the registry by implication; inventing performance,
accessibility or browser-support claims; or exposing private Restless evidence through Blog links.

The run has no semantic time deadline. Budget and model/provider envelopes are frozen at entry and
reported, not inferred from this draft.

## Success contract

### Catalogue and foundry

1. **Frozen completeness.** A machine-readable manifest enumerates every in-scope public component,
   category, upstream URL, source identity and licence; registry coverage has zero unexplained omissions.
2. **Real Svelte ports.** Every in-scope entry satisfies the port definition or ends in one explicit,
   evidence-backed source/licence blocker. A blocker prevents an `accepted` run classification.
3. **Motion first.** The distinctive motion families are independently reviewable before catalogue
   completion, and their shared foundations do not require React.
4. **Usable registry.** A developer can browse by family, behavior, cost, accessibility, motion and
   Restless fit; every accepted port has a live demo and exact usage record.
5. **Truthful provenance.** Licence, attribution, upstream changes and material adaptations remain
   traceable without turning public site copy into implementation notes.

### Behavior and quality

6. **Defining behavior preserved.** Native browser operation demonstrates the interaction that makes
   each component worth porting; still-frame resemblance is insufficient.
7. **Svelte-native composition.** APIs are coherent with Svelte 5 and shared foundations are smaller
   than family-by-family copies.
8. **Accessible alternatives.** Keyboard/focus/touch and reduced-motion behavior are complete for the
   relevant component types, with named limits rather than blanket compliance claims.
9. **Bounded performance.** Shader/canvas/continuous components suspend appropriately and remain usable
   on the accepted desktop/mobile profiles without hidden runaway animation or obvious input jank.

### Site outcome

10. **Better, not busier.** The public site uses a small justified subset and retains coherent Bridge
    Light identity, hierarchy and reading rhythm.
11. **Hero earns the effect.** Liquid Metal is recognisably high-quality, product-native, responsive,
    accessible and intentionally abstract.
12. **No uncanny pseudo-product.** Every UI-like encounter is demonstrably faithful or deliberately
    abstract; the critic records no ambiguous middle category.
13. **Standalone Blog.** Every article passes direct-entry comprehension and exposes no unreadable
    Markdown/repository dependency to a visitor.
14. **Exact native review.** A fresh critic accepts the complete registry and exact rendered site;
    deterministic green checks remain supporting evidence rather than the design verdict.

## Required evidence bundle

- frozen upstream manifest, repository commit, licence and dependency inventory;
- registry coverage report with accepted/blocked/rejected state for every entry;
- per-family port commits, demos, API notes and shared-foundation decisions;
- motion/reduced-motion, keyboard/focus/touch and responsive observations;
- shader/canvas lifecycle and performance observations with stated devices/profiles;
- before/after site captures at desktop, mobile and reduced motion;
- direct-entry review of every Blog article and its public evidence destinations;
- independent critic report and exact owner ReviewTarget;
- owner intervention, spend, elapsed/active time, rework and rejected-component accounting; and
- an after-action at `docs/dogfood/dogfood-6-after-action.md` created only after a run exists.

## Stop conditions and classification

Stop for licence ambiguity that could make continued copying unsafe, private/paid source exposure,
source or branch corruption, uncontrolled publication, secrets/privacy risk, spend breach or founder
stop. Ordinary port failure, design rejection, performance repair and upstream incompatibility are run
evidence rather than automatic stops.

Classify the run as:

- `accepted` — the complete frozen catalogue, foundry, site and Blog contracts pass and the founder
  accepts the native outcome;
- `rejected` — the exact outcome is reviewable but materially fails quality or usefulness;
- `inconclusive` — evidence cannot support a reliable judgement;
- `product-invalid` — Restless cannot sustain the company outcome without rescue; or
- `source-invalid` — upstream/licence facts prevent a valid all-components port under the frozen scope.

## After-action questions

1. Did the registry become a usable design-engineering capability or an inventory graveyard?
2. Did motion-first ordering return distinctive value early?
3. Which shared Svelte abstractions emerged from repeated ports, and which abstractions were premature?
4. Where did fidelity to upstream behavior conflict with Restless design language?
5. Which site adoptions materially improved comprehension, feeling or product truth?
6. Did any pseudo-product encounter remain in the uncanny middle?
7. Could a public reader understand every Blog article without source-repository access?
8. What should be maintained, deleted, rejected or offered upstream after the run?
