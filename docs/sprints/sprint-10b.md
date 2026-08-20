# Sprint 10b — A cared-for company floor that stays honest

**Status:** Draft for founder alignment

**Date:** 20 August 2026

**Decision:** [ADR 0004](../adr/0004-vendored-office-engine-owned-company-floor.md)

**Spec refs:** `ARCHITECTURE.md` §2.6 / §4.4 / §9.2 / §16,
`owner-cockpit` §2 / §3 / §7 / §12 / §14,
`orgintel` actor, Team, Work and Attempt projections,
`cross-layer-contract` source-ownership rules,
Sprint 08 T10 liveness contract

---

## Observed product gap

The empty Attention state now has a promising company-floor direction: it can quietly show that the
company remains active without surrounding the view with another dashboard. The existing production
office already reuses Pixel Agents pathfinding, characters and furniture and projects actual Teams,
People and Work into the canvas.

The current floor still looks like an implementation demo rather than a company someone has cared
for over time:

- its shared centre is overly tied to one fountain/atrium composition;
- team neighbourhoods and amenities do not yet form one convincing open office;
- the furniture pack covers desks and a few generic objects but not a complete care layer;
- movement proves pathfinding but does not yet produce a small, legible vocabulary of working,
  waiting, restoring and interacting;
- speech/status cues can collide with sprites or become text-heavy at the exact moment the office
  should feel calm;
- dynamic team layout, amenity reachability and dense-company behaviour have not been exercised
  together;
- a visually delightful idle loop could accidentally imply live Work when the source is stale,
  unavailable or merely active without a current runtime observation.

Six runnable scratch studies now branch the spatial question: Tree Commons, Hearth Lounge,
Greenhouse Café, Garden Court, Library Forum and Seasonal Studio. They demonstrate different office
hearts and amenity vocabularies, but their authored walking loops are comparison evidence, not a
second production engine and not evidence that one layout has won.

## Outcome

> **The clear Attention state opens onto one canonical, sunlit, Pokémon-like company floor derived
> from the actual Teams and People. It feels comprehensively cared for through food, hydration,
> focus, recovery, reading, social and pet-friendly amenities without becoming cluttered. People
> move between reachable work and restorative interaction points using the vendored engine. Current
> Work cues appear only from observed source state; ambient life remains visibly non-semantic.
> Clicking a person reveals one compact current summary and leads to the canonical Work or People
> detail. The office remains the experience itself, with only the persistent Attention rail and
> minimal in-canvas controls around it.**

Sprint 10b selects a spatial grammar, not one compulsory centrepiece. A tree, garden, café, hearth,
forum or changing installation may win or be combined after the comparison run. A fountain has no
special architectural status.

## Success contract

The sprint passes only when all of the following are observed:

1. **One office canon survives.** The six scratch directions are compared against the same company
   shapes, owner sizes and truth states. One production composition and asset vocabulary remain;
   losing production branches, feature flags and renderer experiments are removed.
2. **The floor derives from real organisation.** Team neighbourhoods, accountable leads, members and
   unassigned people come from the existing source-owned projection. No actor names, team labels or
   counts are hard-coded into production layout code.
3. **Open plan remains legible.** Teams are separated through flooring, rugs, furniture, planting,
   low shelving and presentation surfaces rather than full-height boxes. Shared circulation remains
   visually obvious and no fixed central monument blocks it.
4. **Representative shapes remain walkable.** Layouts with 0, 1, 2, 4 and 8 Teams and 1, 6 and 20
   visible people produce reachable spawns, seats, team zones, shared amenities and return paths.
   Oversized companies degrade to a deliberate bounded view rather than overlapping furniture or an
   unbounded canvas.
5. **A care baseline is present.** Every generated floor contains appropriate nourishment/hydration,
   focus support, restorative space, comfortable social seating, practical storage and greenery.
   Pet amenities appear only when pets are enabled. At least one whimsical or seasonal landmark
   gives the floor memory without dominating its plan.
6. **Amenities are composition data, not company ontology.** Assets and interaction points live in a
   thin Restless-side presentation catalogue or planner. No amenity, room, mood, hunger or booking
   table enters OrgIntel, Authority or the Runtime contract.
7. **Observed work stays honest.** Only a fresh session/process observation associated with the
   person and Work/Attempt may claim a current step or receive the semantic working treatment.
   Active-but-unobserved, stale, unknown, unavailable and waiting render distinctly.
8. **Ambient behaviour is bounded and harmless.** Available people may choose reachable restorative
   actions such as tea, reading, stretching, gardening, social seating or pet interaction. These
   actions are labelled and implemented as ambient presentation, never Work progress or inferred
   wellbeing.
9. **Waiting has causal meaning.** A waiting cue appears only from source-owned waiting/blocker/
   owner-handoff state and links to the same canonical Work or Attention item. It is not generated
   from elapsed animation time or lack of movement.
10. **Interaction stays small.** Hover/focus identifies a person without permanent nameplates.
    Selection opens one compact overlay with person, honest state, current outcome when known and
    one `Open Work` or `Open person` action. Raw logs, tools, prompts, token counts and long status
    prose stay out of the office.
11. **Cues remain attached and readable.** Speech/thought/status bubbles anchor to their sprite after
    pan, zoom, device-pixel scaling and movement; they avoid important furniture and viewport edges.
    At most one selected detail and a bounded number of transient cues compete for attention.
12. **Animation has one grammar.** Walking, seated/standing idle, work station use, amenity use, pets
    and environmental ambience share consistent timing and sprite scale. Semantic and ambient
    animation are distinguishable without relying on colour alone.
13. **The office respects the rest of the product.** It uses the sunlit owner-cockpit palette and
    quiet framing. Attention remains a persistent left rail; recent decisions remain collapsed by
    default; the clear state is subtle and vertically centred rather than a competing hero card.
14. **Reduced motion is complete.** Reduced-motion users receive a stable, attractive floor with the
    same truth and selection affordances. Ambient loops and autonomous walking stop while the
    document is hidden; returning does not fast-forward a large simulation delta.
15. **Performance is measured on the retained path.** A representative 20-person floor remains
    responsive at the supported desktop size, performs no layout rebuild per animation frame and
    records frame timing, asset weight and memory observations in the run report. A visual screenshot
    alone is not performance evidence.
16. **Art and licence provenance are intact.** Pixel Agents remains pinned and attributed. New
    Restless assets live outside the vendor directory with recorded source/licence or explicit
    Restless authorship. No asset is copied from a reference game or an unlicensed sprite sheet.
17. **The real path is exercised.** One `_test` company covers contrasting topology/truth fixtures.
    One selected real company passes `restless doctor -c <company>`, renders its live people and Teams
    and proves that clicking an observed person reaches the matching real Work detail.
18. **The office earns the empty state.** In founder review, the clear Attention experience feels
    calmer and more informative with the office than with the prior empty treatment. If it distracts
    from Attention or miscommunicates work, the run may invalidate the premise rather than forcing a
    decorative result into production.

## Committed first wave

The first wave is a balanced baseline, not a catalogue-completion exercise.

| Care need | Production vocabulary | First behaviour | Truth boundary |
|---|---|---|---|
| Nourishment | stocked tea/snack and hydration point, communal table | an available person may take a short tea route | no hunger, break compliance or productivity claim |
| Focus | team desks, low-screen focus nook, nearby whiteboard | fresh observed work prefers the accountable team/work zone | a lit monitor is semantic only when driven by fresh observation |
| Recovery | planted quiet nook with stretch mat or comfortable chair | an available person may choose one restorative point | no inferred stress, health or happiness state |
| Reading | small library shelf and reading seat | an available person may pause and read | no claim about the document or research being performed |
| Social | open pair/commons seating | compatible available people may briefly share the area | proximity is ambience, not evidence of collaboration |
| Practical care | coat/storage point, waste/recycling and clear circulation | static in this wave | no inventory, facilities or booking model |
| Belonging | pet bed/bowl when enabled, personal plants and one whimsical landmark | pet idle/wander and optional available-person visit | pets and décor never function as liveness signals |

The exact visual objects are chosen in T0/T1. Every row need not become a unique engine interaction
verb; several may share `walk → face → bounded idle → release`.

## Behaviour grammar

Sprint 10b implements the smallest useful presentation grammar:

```text
fresh observed work  → route to work/team point → work idle → release when source changes
source-owned waiting → route/stay at waiting point → waiting cue → canonical detail
available            → choose reachable restorative point → ambient idle → return/wander
stale                 → neutral retained presence + explicit stale cue; no semantic work animation
unknown/unavailable   → neutral or unavailable presence; never infer idle or working
```

The grammar is interruptible. A new source projection replaces an ambient choice; an ambient
animation never delays a Work, waiting or selection update. Interaction reservations are local and
ephemeral so two people do not occupy the same chair or appliance, but they are not durable locks or
OrgIntel leases.

## Animation and interaction wave

### Required in Sprint 10b

- character walk and directional facing inherited from the engine;
- one seated/standing work idle treatment tied to fresh observation;
- one generic amenity-use idle shared across the first-wave interaction points;
- waiting and stale/unknown treatments that do not rely on motion alone;
- pet idle/wander;
- two or three restrained environmental loops such as steam, leaves, soft light or a seasonal
  hanging object;
- bubble/selection anchoring through camera pan, zoom and responsive resizing;
- keyboard selection, visible focus, Enter to inspect and Escape to close;
- reduced-motion equivalents for every meaningful state.

### Ranked candidates after the baseline run

These remain candidates, not Sprint 10b acceptance criteria:

1. two-person conversation facing and a shared social idle;
2. café preparation and communal-meal moments;
3. library/forum gathering for a real team event;
4. seasonal or milestone transformations backed by actual company outcomes;
5. employee-selected personal desk decoration;
6. weather/daylight ambience that is clearly decorative and locally deterministic;
7. richer pet interaction and varied sitting poses.

Promote one only when dogfood shows that it materially improves comprehension, belonging or delight
without inventing company truth.

## Layer slices and ownership

### Owner surface

- Own the office composition, presentation catalogue, ephemeral behaviour controller, custom assets,
  overlays, controls and accessibility.
- Keep the Attention rail and Work/People navigation canonical.
- Reuse the existing Pixel Agents renderer and engine state; do not introduce Pixi, Phaser or a
  second canvas engine during this sprint.

### OrgIntel

- Supply source-owned Teams, People, Work/Attempt state, accountable actor and bounded current
  activity observations already justified by the Work/liveness contract.
- Add no office, amenity, mood, room, schedule or animation entities.
- Remain the only writer of organisational and Work state reached from office selection.

### Runtime and Runtime Bridge

- Supply only observed runtime/session/process state already needed to substantiate current work.
- Do not receive office movement, décor or amenity events and do not treat the canvas as a process
  supervisor.

### Authority Plane

- Untouched. The office does not grant capabilities, resolve approvals, mutate budgets or infer
  authority from a person's location or animation.

## Problem classification

**Deterministic:** asset decoding, footprint/collision masks, path reachability, interaction-point
reservation, camera projection, bubble clamping, state freshness, reduced motion and source-owned
detail links.

**Judgement:** which office composition feels cared for, which amenities improve belonging, whether
an animation is delightful or distracting and which scratch direction should become canon.

Run deterministic checks to convergence. Resolve the judged choices through rendered comparison and
founder use, then purge; do not encode aesthetic selection as a scoring function.

## Verification sequence

### 1. Choose the composition

Render the six existing scratch concepts with equivalent staff/team populations and the product's
real surrounding Attention layout at 390, 768 and 1440 CSS pixels. Compare:

- open-plan legibility and circulation;
- team identity without box rooms;
- perceived care without clutter;
- character/furniture scale and occlusion;
- relationship to the sunlit cockpit palette;
- room for source cues and selection;
- how well 1, 2, 4 and 8 Team projections adapt;
- implementation and asset cost.

Record the decision before implementing the final production plan. The result may combine a small
number of proven ideas, but must name one spatial grammar and one signature—not retain six modes.

### 2. Geometry and source-truth checks

- Generate representative 0/1/2/4/8-Team and 1/6/20-person layouts.
- Assert every spawn, seat, work point, waiting point and amenity interaction point is on a walkable
  tile and reachable from its actor's starting component.
- Assert furniture footprints do not overlap and critical circulation remains connected.
- Feed fresh observed, active-unobserved, waiting, stale, unknown and unavailable fixtures through
  the same projection used by Attention, Work and People.
- Assert only fresh observed state receives semantic working/current-step treatment and every click
  resolves to the canonical source id and route.

These are invariant/geometry tests, not pixel snapshots of exact furniture positions.

### 3. Browser and animation checks

- Run the real local cockpit stack rather than Vite alone and pass `restless doctor -c <company>`
  before describing the real company floor as live.
- In a browser, exercise pan, zoom, pointer selection, keyboard selection, focus/close, route opening,
  recent-decision expansion and return.
- Capture bubble anchors before/after movement and at viewport edges; verify no detached or clipped
  cues at supported zoom/device-pixel ratios.
- Compare frame hashes over time for standard motion, then prove reduced motion and hidden-document
  modes remain stable.
- Record representative 20-person frame time, long-frame count, asset bytes and rebuild count; inspect
  the browser console for asset, canvas and accessibility errors.

### 4. Founder comprehension review

Using clear, active, waiting and degraded Attention states, ask the founder to identify:

- whether the company is observed working or merely ambient;
- which person/team owns the current outcome;
- who is waiting and what opens next;
- whether the space feels calm, cared for and worth revisiting;
- which visual element should be removed.

Misread liveness or inability to find the canonical Work path is a failed run regardless of visual
quality.

### 5. Purge

Before closing the sprint, remove:

- the fixed-fountain assumption from layout and camera control naming;
- losing production layout branches, themes, feature flags and custom edge/path experiments;
- amenities that do not survive the rendered founder comparison;
- duplicated movement or collision code that bypasses the vendored engine;
- status-only motion, fixture-only busy cues and unobserved progress text;
- obsolete CSS, custom assets and catalogue entries belonging only to rejected directions.

Retain the pinned vendor subtree, its notices and the smallest Restless adapter that serves the
winning office.

## Risks and dispositions

| Risk | Disposition | Sprint treatment |
|---|---|---|
| Delightful animation invents liveness | **Invariant** | Fresh source observation gates semantic current-work cues; ambient life has no Work meaning. |
| The sprint turns into a general game or life simulation | **Invariant** | One disposable behaviour grammar; no needs, moods, schedules, inventory, rooms or persistent world state. |
| Amenities make the floor cluttered and reduce comprehension | **Guarded** | Commit care categories, not every object; founder review must name and remove the least useful accessory. |
| A fixed signature makes dynamic teams fit poorly | **Guarded** | Compare representative team shapes and permit a compositional family around one spatial grammar. |
| Scratch alternatives become permanent product modes | **Invariant** | One production canon and no shipped layout switcher at sprint exit. |
| Vendor modifications make upstream provenance unclear | **Guarded** | Product changes remain outside vendor; any necessary vendor patch names its limitation and base. |
| Pathfinding passes simple rooms but fails dense layouts | **Guarded** | Component reachability and 20-person runs cover furniture and interaction points before visual acceptance. |
| Available people socialising looks like fake collaboration | **Guarded** | Ambient actions are sparse and carry no current-step or Work claim; founder truth review includes this misread. |
| Unknown/stale people make the cheerful office feel broken | **Accepted** | Honest degraded state matters more than uninterrupted charm; art direction should make uncertainty calm, not hide it. |
| Mobile cannot show a useful explorable map and a full rail | **Accepted for this sprint** | Preserve truth, person selection and canonical detail access; do not build a second mobile game layout without observed owner need. |
| Asset creation dominates the sprint | **Guarded** | Recolour/recombine the licensed base where appropriate and author only the small winning first-wave pack. |

## Explicitly out of scope

- replacing Pixel Agents with Pixi, Phaser, another engine or a custom renderer;
- a hunger, energy, morale, happiness, health or productivity simulation;
- room booking, desk booking, schedules, shifts, attendance or virtual-office analytics;
- persisted office movement/event history or employee-surveillance playback;
- RPG movement controls, quests, collectibles, currency or owner-controlled avatars;
- multiplayer cursors or employees controlled by separate browser clients;
- continuous physics, crowd steering or a general navigation-mesh editor;
- an arbitrary floor-plan editor, user-uploaded sprite pipeline or asset marketplace;
- first-class office, room, amenity, furniture or decoration entities in OrgIntel or Authority;
- runtime-generated art, copying commercial game art or unlicensed third-party packs;
- milestone parties, all-hands assemblies, weather and day/night systems before the baseline run;
- exposing raw Work logs, prompts, tools, cost or runtime traces in office overlays;
- redesigning the Work graph/board, People detail or Attention decision flow beyond canonical links
  and shared liveness truth required by this surface.

## Tickets

| ✓ | Ticket | Layer | Evidence served | Depends |
|---|---|---|---|---|
| [ ] | [**S10b-T0 · Select one cared-for spatial grammar**](sprint-10b/t00-select-office-canon.md) | Owner surface research | Six runnable concepts exist, but no representative rendered comparison has chosen a production canon | — |
| [ ] | [**S10b-T1 · Author the first-wave amenity kit**](sprint-10b/t01-amenity-kit.md) | Owner presentation/assets | The current furniture pack does not communicate nourishment, focus, recovery, practical care and belonging | T0 |
| [ ] | [**S10b-T2 · Generate a walkable office from real Teams and People**](sprint-10b/t02-dynamic-office-plan.md) | Owner projection over OrgIntel | Current placement has not proved amenity reachability or representative 0–8-Team shapes | T0, T1 |
| [ ] | [**S10b-T3 · Give people a small honest behaviour grammar**](sprint-10b/t03-honest-behaviour-grammar.md) | Owner projection + OrgIntel/Runtime reads | Movement exists, but working, waiting, restoring and degraded truth are not yet one tested grammar | T2 |
| [ ] | [**S10b-T4 · Polish animation, bubbles and inspection**](sprint-10b/t04-animation-and-interaction.md) | Owner surface | Sprite interaction and cues do not yet meet product-level anchoring, restraint, accessibility or responsive quality | T1–T3 |
| [ ] | [**S10b-T5 · Dogfood the clear Attention floor and purge**](sprint-10b/t05-dogfood-and-purge.md) | Full owner-facing slice | A charming demo is not evidence that the office is truthful, calm, performant or better than the prior clear state | T0–T4 |

Ticket status lives only in this checklist.

## Exit evidence

- ADR 0004 and a recorded T0 selection naming the retained spatial grammar and signature;
- rendered comparison at 390, 768 and 1440 CSS pixels with equivalent source fixtures;
- the small attributed first-wave asset/amenity catalogue;
- geometry/reachability results for representative team and people counts;
- fresh/active/waiting/stale/unknown/unavailable truth-table results;
- pointer, keyboard, pan/zoom, bubble-anchor, reduced-motion and hidden-document evidence;
- recorded representative 20-person frame/asset observations;
- one `_test` topology run and one live selected-company Work-detail path after a passing doctor;
- founder comprehension notes, including the least useful accessory removed;
- exact losing runtime paths, assets, dependencies and fixed-fountain assumptions deleted;
- `sprint-10b/run-report.md` separating deterministic checks, source observations and founder taste.

## Salvage

Reuse the pinned Pixel Agents engine and assets through ADR 0004, plus the existing Restless
`OfficeCanvas`, `officePlan`, `pixelAssets` and People/Work projection seams. Revalidate rather than
assume:

- pathfinding reaches new amenity interaction points in dense representative layouts;
- upstream sprite/seat assumptions still hold after Restless composition changes;
- the source projection does not retain stale semantic animation;
- every retained asset has its recorded licence or Restless authorship;
- any vendor-subtree change is necessary, narrow and identified in the run report.
