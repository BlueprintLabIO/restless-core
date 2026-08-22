# ADR 0004 — Vendor the office engine; own the company-floor experience

**Status:** Accepted

**Date:** 20 August 2026

## Context

The calm Attention state can use the otherwise empty main surface to show that the company continues
working inside the authority the owner already set. A small animated company floor makes that truth
more immediate and endearing than another status dashboard, provided it does not invent activity.

Restless already vendors Pixel Agents 1.4.1 at pinned commit
`3537e140c2094761beae748592aeb92ece8edfdd`. The vendored subsystem supplies its renderer, sprite and
furniture catalogues, tile map, pathfinding, seat placement, pets and character state. The source and
art are MIT-licensed; the bundled character art has the separate CC0 provenance recorded in
`web/src/lib/vendor/pixel-agents/NOTICE.md`.

Restless already owns a meaningful layer above that engine:

- `OfficeCanvas.svelte` composes the engine with owner interaction and the live company projection;
- `officePlan.ts` derives a company-shaped floor from actual Teams and People;
- `pixelAssets.ts` adapts browser assets and registers Restless-authored additions;
- Work and runtime observations determine whether a person is observed working, waiting, available,
  stale or unavailable;
- custom visual studies in `scratch/dream-office-demos/` explore materially different office hearts,
  amenities and movement patterns without changing production architecture.

The alternatives are unattractive at opposite ends. Consuming an opaque package would make local
adaptation and provenance harder while adding a release dependency. Rebuilding rendering,
pathfinding, collision, character animation and asset tooling would spend weeks recreating a solved
game-engine slice. Editing the vendored engine freely until it becomes an undocumented fork would
make upstream fixes and attribution progressively harder.

The company floor is also a presentation projection, not an organisational simulation. Restless has
no product need for hunger, morale, schedules, inventory, room booking or a durable virtual-world
state machine merely because an animated office can depict people taking breaks.

## Decision

Keep Pixel Agents as a **pinned, vendored implementation foundation** and keep the company-floor
experience **Restless-owned above that boundary**.

### Vendored foundation

The vendored Pixel Agents subtree remains the source for generic pixel-office mechanics:

- tile and furniture rendering;
- pathfinding and obstacle-aware movement;
- character and pet animation state;
- seat placement and hit testing;
- generic sprite, carpet, wall and furniture catalogues.

It is not installed as an npm runtime dependency. The exact upstream version, commit and licences
remain recorded with the vendored source and served artwork.

Changes inside the vendor subtree require a demonstrated engine limitation. Such a change must stay
narrow, identify its upstream base and remain distinguishable from Restless product code. An
upstream refresh is a deliberate diff-and-probe exercise, never an automatic overwrite of local
adaptations.

### Restless-owned experience

Code outside the vendor subtree owns:

- projecting actual Teams, People, Work, Attempts and runtime observations into one office view;
- generating a legible irregular open campus with team neighbourhoods and shared circulation;
- choosing and placing amenities, restorative spaces and whimsical landmarks;
- Restless-authored assets, the retained visual language, animation overlays and interaction affordances;
- click-through to the canonical People or Work detail rather than a second task system;
- owner preferences that are already presentation state, without promoting office objects into
  OrgIntel or Authority entities;
- accessibility, responsive framing, reduced motion, visibility pausing and owner-surface styling.

New Restless-authored artwork belongs outside the vendored asset directory. Furniture or animation
metadata should use a thin Restless-side catalogue or adapter unless a live interaction proves the
generic engine catalogue itself must change.

### Truth and behaviour boundary

The office never becomes a writer of company truth.

- An observed current activity may animate a person working and may show a bounded current-step cue
  with its observation time.
- `active` Work without a current process signal may read as in motion but not as executing now.
- A waiting person may use a waiting area only when source state says they are waiting.
- An available person may walk, read, drink tea, stretch, socialise or visit a garden as explicitly
  ambient restorative behaviour. That motion makes no claim about Work progress.
- Stale, unknown and unavailable remain distinct. They do not inherit a working animation from an
  earlier observation.
- Environmental animation such as leaves, pets, steam, light or a seasonal installation is
  non-semantic ambience and must not be presented as a runtime heartbeat.

Amenity use and walking state remain in-memory, disposable presentation state. Reloading or
rebuilding the projection may choose another safe restorative action; no durable office simulation,
event history or scheduling protocol is introduced.

### Canonical campus and authored scenes

The retained exterior is one **sunlit top-down lakeside campus**. Team pavilions, quiet care spaces
and a south activity terrace form an irregular C around a central zen court; a glass conservatory,
open project table and dock face the lake. The floor is a real tile mask containing voids, not a
rectangle covered by scenic paint. The Restless-owned backdrop uses the same overworld perspective
and the same camera projection as the office for meadow, topographic tree contours, shoreline,
water, rocks and planting, and is clipped through the voids between pavilions. It is world geometry,
not a viewport-anchored wallpaper: pan and zoom move the campus and office together. There is no
theme selector, beach mode, weather state or second renderer. Real Team count controls which north
and south pavilion plates exist, so a small company does not inherit a huge empty floor.

The cared-for `lush` density is the versioned production default. The smaller `calm` density and
owner-placed decoration list remain local presentation preferences only; neither creates another
layout canon or company record. A standalone `/office-demo` route is retained strictly as a visual
harness over the same renderer. Company ids always resolve through the real owner shell and source
APIs; no fixed demo route may shadow a company route.

Downtime is composed from authored presentation scenes. A scene contains fixed reachable approach
and pose points around an amenity; the existing engine still performs every route and interruption.
The retained catalogue covers reading, sofa conversation, pool, cards/puzzles, sketching, co-op
arcade, garden lunch, hammock rest, music, aquarium, greenhouse tending, open-table collaboration,
dock birdwatching, pet and unicorn-zen moments. Small companies receive one pose from different
scenes before large companies fill another seat in the same scene.

The calmness budget is explicit: at most four interior amenity scenes animate, at most three people
temporarily leave their authored resting scene for another amenity, and at most one ambient speech
bubble appears. The exterior has exactly three slow channels—water, leaves and one butterfly—and
reduced motion freezes all of them. Amenity selection avoids immediately repeating the person's last
visit where another reachable kind exists.

A tiny celebration may appear only when source-owned Work changes to `completed`, or on first load
when that completion's source timestamp is less than two minutes old. It never follows elapsed
office time, character proximity or an inferred mood.

### Branch, run and purge

Non-obvious layout, art and interaction choices are compared in scratch against representative team
shapes and real owner use. Sprint work retains one production composition and removes losing runtime
paths, dependencies and adapters. A rendered comparison and short decision may remain as evidence;
the product does not ship a permanent theme laboratory merely because several candidates were built.

## Risk dispositions

| Risk                                                                        | Disposition   | Reason                                                                                                                                                             |
| --------------------------------------------------------------------------- | ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Attractive motion implies work that was never observed                      | **Invariant** | Semantic activity requires a source observation and time; ambient behaviour is explicitly non-semantic.                                                            |
| Restless drifts into an undocumented Pixel Agents fork                      | **Guarded**   | Pin provenance, keep product work outside the vendor subtree and require a demonstrated engine limitation for vendor edits.                                        |
| Upstream stops maintaining the engine                                       | **Accepted**  | The pinned MIT snapshot already contains the small mechanics Restless needs. Revisit only when an observed browser or platform defect cannot be repaired narrowly. |
| A new upstream version overwrites local behaviour                           | **Guarded**   | Refresh through an inspected diff and repeat asset, pathfinding, interaction and owner-surface probes.                                                             |
| Amenities become first-class OrgIntel entities or a durable game simulation | **Invariant** | Office state remains a disposable read projection; Work, People and runtime sources remain authoritative.                                                          |
| Richer art becomes visually inconsistent with the owner cockpit             | **Guarded**   | Restless owns one cool, sunlit top-down art contract and accepts assets only through rendered comparison at owner sizes.                                           |
| Pathfinding and amenity interactions expand without limit                   | **Guarded**   | Add one interaction verb after a representative run needs it; reuse engine movement and authored interaction points.                                               |
| Scratch alternatives accumulate into permanent production modes             | **Guarded**   | Sprint exit chooses one canon and deletes losing runtime code and dependencies.                                                                                    |
| The office consumes excessive CPU while hidden or for reduced-motion users  | **Guarded**   | Pause ambient work while the document is hidden, bound entity count and provide a stable reduced-motion rendering.                                                 |
| A scenic exterior competes with Work and people                             | **Guarded**   | One softer, lower-contrast top-down campus surrounds and passes through the floor voids; three slow environmental channels are the hard ambience budget.           |
| A completion celebration launders ambience into progress                    | **Invariant** | It is gated by a source-owned Work completion transition or a recent source timestamp and has no ambient trigger.                                                  |

## Consequences

- Restless can extend the office quickly through layouts, assets and small behaviour adapters without
  owning a general-purpose game engine.
- The production dependency remains inspectable and locally runnable, with explicit licence and
  commit provenance.
- Most visual additions are cheap; convincing new interaction verbs still require path, reservation,
  facing, animation and interruption work and must earn that complexity through dogfood.
- Pixel Agents remains an implementation detail. It never becomes a source of Work, People,
  presence, progress or authority truth.
- A future engine replacement is possible behind the Restless projection boundary, but package
  fashion, rendering novelty or a hypothetical scale problem is not sufficient reason to do it.
- Sprint 10b is the first implementation slice governed by this decision.
