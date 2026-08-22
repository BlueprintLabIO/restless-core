# Sprint 10b run report — Lakeside Courtyard Campus

**Run date:** 20 August 2026
**Retained path:** Lakeside Courtyard Campus on the pinned Pixel Agents engine
**Machine-verifiable office work:** Complete
**Attention integration:** Complete and browser-verified
**Founder visual acceptance:** Accepted 21 August 2026; production integration requested

## Verdict

The retained clear-Attention experience is one source-derived, sunlit, irregular company campus with
a queue-sensitive Attention rail. It has no theme or layout switcher, fountain-centred camera semantics,
second renderer, custom pathfinder, office ontology or persisted movement state. The production
planner owns composition; the vendored engine still owns rendering, collision, seats and pathfinding.

Tickets T0–T5 are complete. Chrome DevTools first verified the full Attention rail iteration; the
settled founder decision removes decision history from Attention and smoothly collapses an empty
rail into a subtle top-left `All clear` refresh control at desktop and mobile sizes. The founder
described the retained campus as beautiful and asked
for it to be integrated into the actual application. The real-company checks proved live source
degradation, a real waiting Work route and a fresh real
Exec session that correctly remained `unknown` because it was not associated with a running Work
Attempt. No live associated Work observation occurred, and the run did not wake an actor or write
simulated evidence merely to manufacture one.

## Retained implementation

- **Canon:** Lakeside Courtyard Campus. Open Team pavilions form the variable north and south arms of
  a C-shaped floor around a zen court; a west care pavilion and east project/dock deck share paired
  circulation lanes. Rugs, cool floor material, plants and low furniture provide identity without
  room walls.
- **Care baseline:** tea/snacks/water, focus nook, recovery nook, garden library, commons seating,
  coats/recycling, greenery, glass conservatory, open project table, lakeside dock, optional pet
  corner and one enlarged animated unicorn landmark.
- **Dynamic topology:** actual Team ids/names/membership generate bays of at most six seats. The
  visible projection is bounded to 20 people; larger organisations deliberately retain a proved
  20-person view.
- **Truth grammar:** `observed`, `in-motion`, `waiting`, `available`, `stale`, `unknown` and
  `unavailable`. Only a fresh session observation associated with the same actor and a running
  Work/Attempt carries a current step or semantic work treatment.
- **Ambient grammar:** every available person leaves their workstation for a unique reachable
  leisure pose in the shared commons. The retained 20-person scene includes sofa seating, grouped
  conversations, four pool positions and four table-game seats. One short conversation bubble is
  permitted at a time. Up to three people may temporarily visit another presentation-only amenity,
  then return to their lounge group. New source truth interrupts ambience; observed and in-motion
  Work return to desks. Nothing is persisted or written back.
- **Interaction:** pointer/hover, keyboard person traversal, Enter to canonical Work/People, Escape
  close, compact one-action detail, camera pan/zoom and decoration preferences.
- **Attention framing:** the left rail remains full width while active owner items exist. Once the
  source proves the queue clear, CSS grid, transform and opacity transitions yield its width to the
  office and reveal a compact top-left `All clear` refresh control. Recent decisions do not appear on
  Attention; company decision history remains on its canonical Company surface.

## 21 August campus and activity enrichment

The retained path now has one Restless-authored top-down lakeside campus around and through the
office. Its meadow, contour-like forest ridge, trees, rocks, planting, shoreline and water all use
the same overworld projection as the people and furniture. The office is a real irregular tile mask:
voids between its C-shaped pavilions reveal the landscape instead of a scenic image merely sitting
behind a rectangle. Actual Teams add only the north and south pavilion plates they need. No
beach/theme setting, image dependency or second renderer was added.

A finite material pass shifts the authored furniture structures from dominant brown into Restless
slate, lake blue, teal and violet while preserving small warm wood, food, flower and game accents.
Vendor desks, chairs, sofas and bookshelves used by the planner receive the same cool furniture tint;
team plaques use a pale mint surface rather than another cream/brown card.

The lounge grammar now contains authored quiet, social, playful, whimsical and outdoor-restorative
scenes: reading, sofa conversation, pool, cards/puzzles, sketching, co-op arcade, garden lunch,
hammock rest, records/headphones, aquarium, greenhouse tending, open-table collaboration, dock
birdwatching, pet and unicorn-zen moments. The planner interleaves one pose from each scene before
filling second seats, so smaller teams receive variety and 20-person teams form convincing groups.
Every pose and approach remains unique and reachable in the same 45-topology matrix.

The deterministic calmness contract caps four animated interior scenes, three temporary amenity
visitors, one ambient speech bubble and three slow environmental channels (`water`, `leaves`,
`butterfly`). Ambient visit selection avoids the person's immediately previous amenity kind when
another reachable kind exists. Reduced motion and hidden documents pause the loops.

Completion confetti is gated by source-owned Work: an observed status transition to `completed`, or
a first-load completion timestamp less than two minutes old. Five deterministic cases reject stale,
already-completed and non-completed inputs.

Observed on the retained tree:

```text
cd web && npm run check
svelte-check found 0 errors and 0 warnings
type: no raw font sizes outside the ramp

cd web && npm run verify:office
Office geometry verified: 45 shapes, max 72x52, 15-16 reachable amenities.
Every visible person has a unique reachable authored scene pose; 20-person plans retain at least
nine quiet, social, playful, whimsical and outdoor activities.
The top-down lakeside campus retains exactly three non-semantic motion channels.
The calmness budget caps four animated scenes, three ambient visitors and one bubble; completion
confetti is source-gated.

cd web && npm run build
Vite/SvelteKit production build completed and wrote the static site.
```

The connected `_test` owner API returned 21 people, eight Teams, OrgIntel `available`, Authority
`available` and Runtime `running`; the owner shell and cockpit API both returned HTTP 200. The doctor
initially reported only a stale test-runtime image. `restless-dev sprint10b_office_test --reconcile`
rebuilt and replaced that disposable `_test` container while retaining its volume; the repeated
doctor then returned `status: live` with no actions.

The accepted composition is now the versioned production default (`OFFICE_PLAN_VERSION` 7 with lush
decor and pets enabled). The actual company route, not a fixture page, owns
`/sprint10b_office_test`: it loads Attention and cockpit APIs, projects the source's 21 people and
eight Teams into the bounded 20-person canvas, and opens canonical Work or People routes. The visual
harness remains separately at `/office-demo` and imports the same `OfficeCanvas`; no alternate
renderer or demo data enters the company route.

Before the irregular-campus refinement, Chrome DevTools inspected the prior rectangular enrichment
at 1440×1000 and 390×844. The desktop Fit view
showed the whole campus and office; one zoom step produced the closer activity-readable view without
changing source state. The mobile view retained the subtle clear rail above a full-width, pannable
office and had no page-level horizontal overflow. In the settled 20-person sample every source
projection was `available`, zero people were at desks, nine were visibly seated, three were playing,
and the canvas reported 11 distinct activities across all five tones. The same sample retained four
animated interior scenes, three ambient reservations, no completion celebration and zero frames over
32 ms; the longest observed frame was 25.30 ms. Chrome reported no console errors or warnings before
the supported launcher exited on the already-recorded image-reconciliation diagnosis. This remains
historical interaction evidence rather than evidence for the retained C-shaped composition.

The current Chrome DevTools MCP pass exercised the real company route after the fixed-name demo
route was removed. At 1440×1000 it retained the desktop Exec sibling; at 390×844 the Exec overlay now
starts closed so the clear-state control and full-width pannable office are immediately visible. The
page had no document overflow, decision history was absent from Attention, and all office/API/asset
requests completed without console warnings or errors. Keyboard selection opened the compact Exec detail,
and its one action navigated to `/sprint10b_office_test/people?person=exec`, where the source's
accountable People and Team records loaded. The canvas reported 20 bounded members and zero observed
live sessions, matching source truth rather than ambient animation.

The final clear-state interaction pass also exercised the two truthful queue shapes directly. Aris
retained one 336 px desktop rail for its one active owner decision; the `_test` company exposed no
rail or decision history and gave the office the entire available main surface. A DevTools timing
probe observed the clear transition at 336, 207, 47, 5 and 0 px across 420 ms while the office grew
from 1,078 to 1,418 px; the rail opacity reached zero before its final track collapse. The mobile
equivalent moved its 186 px queue row through 116, 26 and 3 px to zero without document overflow.
The `All clear` control triggered a fresh 200 Attention read, remained keyboard-visible, and a
settled-state Lighthouse snapshot scored Accessibility 100 and Best Practices 100.

A subsequent camera-coupling correction removed the last viewport-space seam: the lakeside backdrop
now projects every shoreline, ridge, tree, rock, flower, wave and ambient sprite from the same office
origin and tile scale as the floor. The solid meadow fill still covers the viewport, but it carries no
independent feature position. Chrome compared the whole-floor render at device zoom 2 with a closer
render at device zoom 3: floor plates, shoreline, ridge bands, exterior trees and source-pixel detail
scaled and panned as one world. The deterministic office probe also doubles a projected shore point
under a 2× camera scale and applies the new camera origin, guarding against a viewport-anchored
regression.

## Deterministic evidence

### Office geometry and truth

Command:

```text
cd web && npm run verify:office
```

Observed:

```text
Built dynamic catalog with 61 assets
Office geometry verified: 45 shapes, max 72x52, 15-16 reachable amenities.
Truth grammar verified across 8 source states. Every visible person has a unique reachable commons
leisure pose; 20-person plans retain sofa, pool, table-game and conversation groups. Decoration
routes, bubble avoidance and viewport clamping verified.
```

Inputs cover 0, 1, 2, 4 and 8 Teams crossed with 1, 6 and 20 people in default-lush/pets-on,
calm/pets-on and calm/pets-off presentation states. Every generated floor passed footprint bounds,
furniture overlap, irregular floor voids with no walls, protected paired campus circulation,
home/zone connectivity, seat capacity, every committed care category and interaction-point
reachability. A separate 31-person input retained exactly 20 visible people and also passed the same
validator. Decoration probes proved that a clear plant placement survives planning while a
placement over a required amenity approach or paired circulation lane is rejected before it can break
the office routes.

The truth cases cover fresh associated observation, active-but-unobserved Work, source waiting,
stale observation, fresh unassociated session, unavailable Runtime, unavailable OrgIntel and
availability. Only the fresh associated case retained `semanticActivity` and the Attempt summary;
the waiting case retained the canonical Work URL. Pure placement probes also proved that a bubble
moves away from a nearby furniture obstacle when clear space exists and remains inside its viewport
margin at an edge.

### Build and invariant checks

All of the following were executed on the retained tree:

```text
cd web && npm run lint
cd web && npm run check
cd web && npm run build
cargo fmt --all -- --check
cargo check -p restlessd -p restless
cargo test -p restless-orgintel
cargo test -p restlessd
```

Observed results:

- Prettier: all files matched.
- `svelte-check`: 0 errors and 0 warnings; type-ramp check found no raw font sizes.
- Vite/SvelteKit production build completed and wrote the static site.
- Rust formatting/check completed.
- OrgIntel: 13 tests across unit/integration/doc targets passed.
- Daemon: 101/101 tests passed.

## Source-owned topology run

The local fixture `sprint10b_office_test` was created through the product CLI with a zero-dollar
ceiling and a mission forbidding wakes, providers, live evidence and external effects. It contains
20 visible company people (Exec plus 19 Staff) and eight actual Team records. No Work, wake, model
call or external effect was created.

Source probe:

```text
curl -fsS http://127.0.0.1:7788/api/companies/sprint10b_office_test/cockpit
```

Observed: 20 non-owner people, eight Teams, zero running sessions; OrgIntel available and Runtime
running. `restless doctor -c sprint10b_office_test` returned `status: live` with no actions.

## Chrome DevTools browser evidence

Chrome DevTools MCP drove the final application at 390×844, 768×900 and 1440×1000 CSS pixels.

Observed at all three sizes:

- an active queue retains the full-width Attention rail;
- a proved-clear queue collapses the rail and gives the office the full available width;
- a subtle `All clear` refresh control remains at the office's top-left;
- Recent decisions is absent from Attention;
- the whole 72×52 dense office remains reachable through Fit/pan/zoom;
- the 20-person and compact real-company floors retain readable team labels and open circulation;
- selection detail remains inside the viewport at 390 px;
- no permanent person labels or dashboard legend surround the floor.

Interaction observations:

- Browser pointer input through the production canvas handler selected the real Aris Exec at its
  sprite and opened the same compact detail as keyboard selection.
- Arrow navigation selected the Exec; Escape closed; Enter opened the canonical real Work route
  `/aris/work/8f3e8058-9b9a-4549-a7dc-f9c6d3605e52?lens=map` from the source-owned waiting state.
  Browser Back restored `/aris` with the office ready, no retained selection and the same seven
  source-derived person presences.
- Hover showed one bubble attached to the Exec. After a camera pan and a zoom-out, the bubble was
  redrawn at the moved sprite and remained within the canvas. The retained placement scorer compares
  above/left/right anchors and minimises overlap with visible furniture and other people rather than
  always covering the same screen rectangle.
- Aris exposed two source-owned decision continuations. Expanding the collapsed section, opening
  one, and using Back to Attention all worked. Its Inspect Work action resolved to
  `/aris/work/3cca6f4a-c045-426e-a908-b010ed4f9969?lens=map`.
- On `sprint10b_office_test`, Decorate accepted a route-safe plant at tile 1,1, persisted it in the
  versioned company presentation preference and rebuilt the layout once. Undo removed it, persisted
  an empty decoration list and rebuilt once more; no company or OrgIntel state was written.
- On the 20-person clear-state floor, all 20 source projections were `available`, the live canvas
  reported `data-available-at-desks="0"`, and the rendered desk bays were empty while the people
  occupied the authored lounge. A normal-motion sample reported 13 seated, all eight pool/table-game
  positions occupied and one active conversation bubble; the standing people faced game tables,
  companions or shared landmarks rather than random coordinates.
- Chrome's browser-only offline emulation changed every real Aris person to `unavailable` and showed
  the explicit `Source signal unavailable` badge while retaining the last clear Attention result.
  Removing emulation cleared the badge on the next poll and restored the source-derived states; the
  test did not stop or mutate the company runtime.
- Lighthouse snapshot: Accessibility 100, Best Practices 100. Chrome reported no console errors,
  warnings or issues for the office route.

### Motion, visibility and performance

Final 20-person normal-motion observation after 900 retained frames:

| Observation             |             Value |
| ----------------------- | ----------------: |
| Layout rebuilds         |                 1 |
| Ambient reservations    |                 3 |
| Frames over 32 ms       |                 0 |
| Longest retained render |          22.80 ms |
| Used JS heap            |  65,647,490 bytes |
| Total JS heap           | 120,777,886 bytes |
| World                   |             60×44 |

This performance table records the earlier rectangular baseline. The retained 72×52 irregular
campus passed the deterministic animation budget and Chrome interaction checks above; no newer
profiling numbers are claimed here.

A Chrome navigation trace on the same 20-person route observed LCP 402 ms and CLS 0.00 without CPU
or network throttling. The DOM contained 317 elements. Its one reported initial 57 ms layout event
was attributed to application navigation/composer autosizing, not the office animation loop, and
Chrome estimated no savings.

Reduced-motion emulation held the final canvas hash stable for 3.2 seconds with one layout rebuild,
zero ambient reservations and zero >32 ms frames. A separate hidden-document probe held the canvas
hash stable while `documentVisible=false`; after visibility returned the canvas changed normally.
The engine already clamps the first resumed delta, so the simulation did not fast-forward.

## Asset and licence evidence

- Pinned Pixel Agents static asset directory: 404 KB, 88 files.
- Actual final browser request set: 60 vendor asset requests, 101,564 decoded-body bytes from the
  warm local route.
- Restless-authored amenity plus unicorn source: 10,861 bytes in `amenities.ts`.
- Runtime fountain/unicorn concept-image requests: 0.
- Removed generated concept sheets: fountain 802,362 bytes and unicorn 1,139,443 bytes, saving
  1,941,805 static bytes.
- Restless authorship is recorded in `web/static/office/README.md`; upstream code/assets and notices
  remain in the pinned vendor paths. No Pokémon or commercial-game art is included.

## Real-company evidence

Aris rendered seven real people and one real Team. Keyboard selection of its source-owned waiting
Exec opened the exact Work route above. After the final company-image rebuild made reconciliation
required, the first attempt correctly refused replacement because Aris's Exec was genuinely
running. The run did not stop that actor. Once it finished naturally, `restless up -c aris
--reconcile` replaced the container while retaining its volume and moved it to the final source
image. The immediate doctor caught the normal service-starting window; supervisor inspection then
showed all seven desktop/browser services running, and the repeated `restless doctor -c aris`
returned `status: live` with no actions.

During that live session the cockpit API returned `session_running=true` and a fresh
`session_observed_at` equal to the aggregate `refreshed_at`. The office rendered the Exec as
`unknown`, not observed working, because the session was a free Exec conversation with no associated
running Work/Attempt. This is the intended truth boundary. No fake Work was added to satisfy the
screenshot or manufacture an associated observed session.

## Narrow vendor patch

Pixel Agents autonomously wanders every inactive character, which prevents the host from making
ambient movement source-sensitive. The retained patch adds one `ambientAllowed` boolean and
`setAgentAmbient(id, allowed)` method. It disables upstream autonomous target choice; all actual
routes still use the engine's `walkToTile`, collision map and assigned work seats. No renderer,
pathfinder or collision code was forked.

## Purge

Removed from the production path:

- fountain-specific asset, centre/home naming and fixed atrium geometry;
- generated unicorn concept sheet and its client-side crop/resample decoder;
- office theme picker and alternate theme/layout branches;
- permanent liveness legend, oversized surrounding cards and default-selected person detail;
- status inferred from elapsed animation time;
- unobserved current-step prose and semantic monitor treatment;
- autonomous upstream wander as a host policy;
- concept-only runtime asset references.
- three generic gallery pictures whose footprints intruded into the protected garden ribbon under
  compact topology; the cared-for amenities carry more meaning and the removal improves circulation.

Scratch concept demos remain quarantined under `scratch/` as comparison evidence and are not imported
by production.

## Success-contract disposition

| #   | Disposition | Authoritative evidence                                                                                                         |
| --- | ----------- | ------------------------------------------------------------------------------------------------------------------------------ |
| 1   | Achieved    | One Garden Ribbon Commons planner remains; no production theme/layout switch or fountain path remains.                         |
| 2   | Achieved    | The eight-Team `_test` source generated its labels, memberships and lead flags; production contains no company names.          |
| 3   | Achieved    | All 45 plans passed the no-interior-wall check and retain a protected two-tile shared ribbon.                                  |
| 4   | Achieved    | 0/1/2/4/8 Teams × 1/6/20 people × three preference states passed; 31 people bounded to 20.                                     |
| 5   | Achieved    | The validator requires nourishment, focus, recovery, reading, social, practical, belonging and garden care.                    |
| 6   | Achieved    | Amenity/interaction data exists only in the web planner/catalogue; no backend amenity state was added.                         |
| 7   | Achieved    | Eight-state truth fixtures retain semantics/current step only for a fresh associated running Attempt.                          |
| 8   | Achieved    | Available people occupy unique authored lounge poses; conversations, games, amenity visits and cooldown are presentation-only. |
| 9   | Achieved    | Waiting fixture and real Aris Exec both came from blocked Work and opened the same canonical Work route.                       |
| 10  | Achieved    | Pointer and keyboard share one selection/detail path; overlay contains person, state, outcome and one canonical action.        |
| 11  | Achieved    | Pan/zoom probe retained the attached bubble; deterministic placement avoids obstacles when possible and clamps the viewport.   |
| 12  | Achieved    | Engine work idle, generic amenity dwell, pet motion and restrained environment frames share one scale/timing grammar.          |
| 13  | Achieved    | Settled source and Chrome prove the active-queue rail and clear-state transition to a subtle refresh control.                  |
| 14  | Achieved    | Reduced-motion and hidden-document probes held the canvas stable and resumed without fast-forward.                             |
| 15  | Achieved    | The recorded 20-person run had one layout rebuild, zero >32 ms frames, LCP 402 ms and CLS 0.00.                                |
| 16  | Achieved    | Pinned vendor attribution and Restless authorship remain; generated concept sheets and commercial reference art are absent.    |
| 17  | Achieved    | `_test`, real Aris, passing doctors and canonical People/Work navigation exercised the real application path.                  |
| 18  | Achieved    | Founder called the retained direction beautiful and explicitly requested production integration.                               |

Criterion 17 was not manufactured: a fresh real Exec session did occur, but without a running
Work/Attempt association it correctly rendered `unknown`. Founder acceptance closes criterion 18;
it does not weaken that source-truth boundary.

## Remaining risks

- At 1440 px with both the full Attention rail and Exec conversation open, a 72×52 office starts at
  whole-floor Fit and therefore reads smaller. Camera zoom is available; whether the initial scale
  is satisfying is a founder judgement.
- The enlarged code-authored unicorn is intentionally a single restrained landmark. Richer landmark
  art is a later art pass, not grounds for restoring megabyte concept sheets.
- Pair conversation, meal moments, gatherings, milestone transformations, weather and richer pet
  interaction remain ranked post-baseline candidates; none was accumulated into this sprint.
- Founder review must name the least useful retained accessory. Until then, T5 remains unchecked.
