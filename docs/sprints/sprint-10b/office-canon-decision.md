# Sprint 10b office canon — Garden Ribbon Commons

**Engineering selection:** 20 August 2026

**Founder visual acceptance:** Pending the retained-path review in T05

## Decision

Retain one **Garden Ribbon Commons** composition for production. The company floor is a sunlit,
open plan organised around a broad shared circulation ribbon. Team neighbourhoods sit along its
edges and are separated by rugs, planting, furniture and presentation surfaces rather than rooms.
An off-centre planted canopy gives the floor a memorable silhouette; food, hydration, reading,
focus, recovery, social and practical-care amenities are distributed along the ribbon.

This combines the useful qualities of the Tree Commons and Garden Court studies without making a
fountain, tree or other fixed monument the office's organising rule. The signature is the cared-for
garden route and the life around it. A whimsical landmark may change without changing the plan.

## Comparison result

| Direction       | Retained quality                          | Reason it is not the production canon                          |
| --------------- | ----------------------------------------- | -------------------------------------------------------------- |
| Tree Commons    | memorable greenery and shade              | one tree should not become a compulsory centrepiece            |
| Hearth Lounge   | warmth and sociability                    | reads more like a lounge than a working company at dense sizes |
| Greenhouse Café | nourishment and sunlight                  | café vocabulary can overwhelm team identity                    |
| Garden Court    | open circulation and distributed planting | weakest single point of memory on its own                      |
| Library Forum   | quiet focus and reading                   | too formal and event-like as the everyday floor                |
| Seasonal Studio | change and delight                        | seasonal variation is a later layer, not the structural plan   |

The retained composition takes Garden Court's circulation, Tree Commons' memorable canopy,
Greenhouse Café's nourishment, and Library Forum's quiet corner. It deliberately leaves the other
spatial grammars in `/scratch` as comparison evidence rather than runtime modes or feature flags.

## Campus and activity enrichment — 21 August 2026

The canon now sits on one forested hill campus overlooking a lake and soft mountain ridge. A warm
garden terrace and path replace the infinite blue implementation grid around the office. The setting
is quieter and less saturated than the floor so people and source-owned Work remain the focal point.
Beach, weather and theme variants remain rejected runtime branches rather than settings.

Available people now occupy an authored scene catalogue spanning quiet, social, playful, whimsical
and outdoor-restorative moments: reading, sofa conversation, pool, cards/puzzles, sketching, co-op
arcade, garden lunch, hammock rest, records/headphones, aquarium, pet and unicorn scenes. These are
fixed reachable poses plus the existing engine's movement—not a durable leisure simulation. Real
Work and waiting state interrupt them immediately.

Interior animation is capped at four scenes, temporary amenity visitors at three, ambient speech at
one bubble, and exterior motion at clouds, water and one butterfly. Completion confetti is permitted
only from source-owned Work completion.

## Production rules

- There is one daylight palette and one layout grammar.
- Actual Teams and People determine neighbourhood count and seat capacity.
- The executive is integrated into the floor rather than placed in a private room.
- Shared circulation stays at least two walkable tiles wide through the main ribbon.
- Amenities are thin presentation data. No room, amenity, mood, need or booking state enters the
  company model.
- The vendored Pixel Agents engine remains the sole renderer, pathfinder and character simulation.
  Restless owns planning, truth projection, behaviour policy, overlays and authored assets outside
  the vendor tree.
- Ambient amenity visits are decorative and interruptible. Only a fresh associated observation may
  light a workstation or claim current Work.
- Available people occupy authored, grouped scenes across quiet, social, playful, whimsical and
  outdoor-restorative activity rather than random open tiles. These poses are presentation-only and
  are interrupted immediately by source-owned Work or waiting state.

## Purge implied by the decision

The production theme switcher, fountain-specific home semantics, fixed atrium geometry and alternate
layout branches are deletable. The fountain study asset is not part of the retained vocabulary.
Scratch demos remain outside production and are not imported by the application.

## Risks and dispositions

- **The canon may still feel too busy beside Attention:** pending founder review; invalidate the
  premise if the retained clear state is less calm.
- **Dense team shapes may reduce legibility:** guarded by deterministic 0/1/2/4/8-Team and
  1/6/20-person geometry checks plus browser review.
- **Ambient movement may imply productivity:** guarded by explicit source states and a behaviour
  controller that permits restorative movement only for `available` people.
- **One authored asset style may drift from the vendor pack:** accepted for this wave; keep the
  16-pixel grid, hard edges, restrained palette and recorded authorship.
