# S10b-T2 · Generate a walkable office from real Teams and People

**Layer:** Owner projection over OrgIntel Teams and People.

**Serves:** Sprint 10b criteria 2–6.

**Depends on:** S10b-T0, S10b-T1.

**Observed friction:** The current planner derives team rooms but has not proved that one open-plan
composition, shared amenities and restorative points remain reachable across representative company
shapes. A beautiful fixed plan would stop being truthful as soon as the organisation changes.

**Makes deletable:** fixed team labels/counts, fixed-fountain geometry, overlapping fallback
placement, duplicate scratch movement code and any second layout engine.

## Outcome

Generate one bounded open office from actual Teams, People and existing preferences. Team
neighbourhoods, seats, waiting/restorative points and first-wave amenities remain connected and
legible from an empty company through the supported representative size.

## Work

- Adapt the T0 spatial grammar to 0/1/2/4/8 Teams and 1/6/20 visible people.
- Define neighbourhoods with material and low furniture rather than enclosing walls.
- Place shared amenities around an obvious circulation network and expose engine interaction points.
- Bound the visible population and degrade deliberately when the company exceeds the proved shape.
- Rename camera/home concepts around the retained floor rather than a particular monument.

## Verification

- Every spawn, seat, work, waiting, restorative and amenity point is walkable and connected.
- Furniture footprints do not overlap or seal a route.
- Source ids, names, teams and canonical links survive layout rebuilding.
- A Team/person update rebuilds only when the plan signature changes, never per animation frame.
- Geometry tests assert invariants and reachability, not exact decorative snapshots.

## Deletion

Delete the fixed centrepiece assumptions, losing layout branch, redundant geometry helpers and any
custom path code that the vendored engine already supplies.
