# Natural-lead screen — blind artifact review

- Review date: 23 August 2026.
- Reviewer: fresh `gpt-5.6-terra`, high reasoning, read-only, artifact-only.
- Allocation seed: `EXP-01:NATURAL-LEAD:G-WORLD-R2:blind:2026-08-23`.
- Allocation SHA-256: `6ce4039432d062ed0dbf4212918a14712151bc45304491af31e0f57ebbf6748d`.
- Frozen permutation: A = fresh B0, B = forced B1, C = repaired natural N1.
- Topology, model, timing and arm identities were withheld until after the report.
- Reviewer inputs: the three clean candidates, frozen scenario and evaluator evidence, and fresh
  Chromium regression traces. Screenshots were not credited as outcome proof.

## Scores

- C — 8.2/10
- B — 6.4/10
- A — 4.5/10

All three pass the frozen owner journey with zero browser errors. The fresh regression evidence
materially separates them.

## A

### Defects

1. High — Existing battle regression: fresh post-run `verify-battle.mjs` records no projectile
   damage, no burn, no weakened state, then a null `activeIndex` error. This violates preservation of
   the established battle loop.
2. High — At 390×844 the fresh Prism check found the interaction prompt absent, failing the responsive
   requirement. This conflicts with the frozen evaluator's later pass, so the mobile outcome is not
   stable.
3. Low — `bridge.text` becomes empty after returning to the Basin despite retained powered state,
   weakening the promised persistent semantic state outside the cavern.

### Strengths

- The bridge is the clearest genuine restoration: a contiguous hidden route replaces visible broken
  ledges, and the movement gate prevents crossing before power.
- Best textual guidance: entrance, console, bridge state, authored encounter, and return are unusually
  explicit and coherent.

## B

### Defects

1. High — The powered bridge does not visibly restore a continuous span. The source leaves a roughly
   3m central gap between the near deck and far deck; power only reveals far-deck rails/conduits and
   hides fragments. Yet the snapshot calls it traversable when those cosmetic parts are visible. This
   is a weaker interpretation encoded into the proof, not a genuinely solid restored route.
2. Medium — The actual traversal block is a progress-plane rule between console and encounter, rather
   than collision grounded in the restored bridge geometry. It works as a gameplay gate, but does not
   validate the visual claim.

### Strengths

- Strongest atmospheric treatment beyond C: fog transition, ceiling, portal animation, animated
  crystals, differentiated materials, and spatial staging make it feel authored.
- Fresh post-run checks pass battle, combat, evolution, Prism journey, reload, and phone viewport
  coverage.

## C

### Defects

1. Low — Entering the cavern does not retune the Basin fog or lighting; the cavern's distinction
   relies on its enclosing geometry and materials rather than a complete lighting transition.
2. Low — The powered bridge is segmented with small visible joins rather than one uninterrupted deck,
   although it spans the chasm and is backed by the same runtime gate logic.

### Strengths

- Best contract fidelity: an enclosed rock-and-ceiling room, 15 crystals, real authored Nullix entity,
  actual visibility separation, and bridge geometry/state all derive from the live scene.
- The unpowered clamp prevents crossing at the chasm; powering simultaneously reveals the route and
  removes that restriction. This is a credible physical traversal change, not merely a snapshot flag.
- Clean fresh evidence: all regression suites and responsive checks pass, with zero browser errors.

## Blind ranking

1. **C**
2. **B** — material difference from C: B's "solid and traversable" bridge remains visibly
   discontinuous and its proof masks that fact.
3. **A** — material difference from B: A has a better bridge, but fresh evidence shows broken core
   combat and unstable phone presentation.

## Smallest overturning evidence

- For C vs. B: one fresh native traversal capture or scene-bounds trace proving B's powered geometry
  creates a continuous deck over its central gap.
- For B vs. A: a fresh repeat of A's battle and 390×844 checks passing consistently, with no
  null-state error.

`BLIND_REVIEW_COMPLETE`

## Identity reveal

| Blind identity | Arm | Run | Candidate |
|---|---|---|---|
| A | fresh lead alone (B0) | `exp01-natural-gworld-r2-b0` | `b61902eb3ebfac660262171a4cd5ca60c1a1dfe1` |
| B | forced ordinary team (B1) | `exp01-w01-gworld-r2-b1-terra` | `2a628b3675dfe4161547bc7bfb7317c2b0437c1f` |
| C | natural lead + optional Staff (N1 repaired) | `exp01-natural-gworld-r2-n1-r2` | `dbab2a7daf534aea14dc5874c3db7dc960f65995` |
