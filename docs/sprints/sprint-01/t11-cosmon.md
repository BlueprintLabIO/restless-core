# T11 · Cosmon — playable browser loop (the skeleton is built here)

**Layer:** All three. This is the vertical slice the skeleton is built against (§16.2, §16.9).
**Serves:** The first economic objective (§10.2), deliberately shrunk. Cosmon is the only one of the three companies that needs **no external effects at all** — local build, local serve, local play — which is why the skeleton gets built here.
**Makes deletable:** Nothing yet — first sprint.
**Depends on:** T1, T3, T4, T5, T6, T7, T9, T10.

## Build

- Company config: mission, granted capabilities, staff shapes.
- The owner directive.
- **Ban the art pipeline.** three.js primitives or 2D, procedural only. Original 3D assets are the slowest, weakest agent loop and are not what this sprint measures. Original art is a later milestone.
- Avoid premature MMO infrastructure (§10.7.1). The first success is a coherent loop, not a backend.

## Pass bar

A playable local browser build of a minimal **exploration → encounter → capture** loop, committed to Git under `/company/repos`.

## Acceptance

A Playwright script drives the served build through the loop's key transitions: move → encounter fires → capture resolves. **Playable is a claim; the script is the proof** (CLAUDE.md → "Never report green without running it"). Manual play supplements this, it does not replace it.

## Note

**This is where the skeleton is actually built.** T12 and T13 should then be config-shaped. If they are not, that is the sprint's most valuable finding — see T15.

---
Sprint spec: [`../sprint-01-walking-skeleton.md`](../sprint-01-walking-skeleton.md)
