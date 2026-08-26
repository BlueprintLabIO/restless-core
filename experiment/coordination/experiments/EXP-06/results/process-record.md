# EXP-06 process record

**Status:** Arms and repairs complete; owner blind decision pending.

This record intentionally does not map the neutral A/B labels to the producing arm before the owner
locks a native judgement.

## Frozen inputs

- Starting commit: `06c114fc2ef2244777df78c8a754386f50faeeef`.
- Owner prompt SHA-256: `d23aa8d83ad5473199ac54e96c92fd41205e0a5318c14da28ebcbb7355ee1d21`.
- Rubric SHA-256: `070e4dca5a1b428c4ecdebf71ae6d86308f25414d3d050a10beaee795291ca39`.
- Matched repair feedback SHA-256: `5273682a4f7a8bf084cc1aa4057607f5878f13a1939b48e861e0b3fe59ac29da`.
- Model and effort: GPT-5.6 Sol, medium, no fallback.
- Frozen order: Restless, then Codex.

Both prompt and rubric hashes still match their frozen files after the run.

## Arm R: Restless product path

- Topology: owner to Exec to standing non-producing lead to one attributable Staff producer.
- Exec and lead remained non-producing. Staff owned both productive Attempts.
- Initial candidate: `d580494869d32348e8432814edf7fcab3a03ad3c`.
- Repaired candidate: `5fec1f82331151143ab0f12599fb0729309bb46a`, amended to remain one commit after the baseline.
- Initial terminal time: 15m29s from owner wake to lead report.
- Matched repair time: 11m10s from feedback to lead report.
- Active closure time excluding evaluator/operator pauses: about 26m39s.
- Repair loops: one. The first repair passed the independent 20-case matrix.
- Model wakes: two Exec, four lead, and two Staff wakes. The lead and Exec sessions resumed hot; the
  repair Staff session was fresh.
- Sum of reported context snapshots: 407,875 tokens. This is observable context, not canonical token
  consumption. Estimated list-price deltas total about $9.15; subscription charge recorded as $0.
- Final change: 10 files, 1,347 insertions, 37 deletions; 1,192 KiB static output across 86 files.
- Dependency state: 353 installed packages and three high-severity audit findings after adding pinned
  `puppeteer-core` for a retained 143-line all-route responsive probe.

The lead framed a materially richer product demonstration and independently reviewed each Staff
result. It did not catch the inherited overflow on the first pass. On repair it identified the
headline min-content cause, fixed it, and added a durable regression probe, but did not challenge the
new dependency weight or audit findings.

## Arm C: one Codex producer

- Topology: one first-party Codex app actor working end to end; no supervisor or peer.
- Initial candidate: `12b3712e69586ce6c33a68d5f2a5cba474dc1ee8`.
- Repaired candidate: `440fc3b01b78c2a496340f4ee5096bd24c1f6da7`, amended to remain one commit after the baseline.
- Initial terminal time: about 8m28s.
- First repair: about 3m35s, but it produced a false-green result and did not fix document overflow.
- Second repair: about 2m43s and passed the independent matrix.
- Active closure time excluding evaluator/operator pauses: about 14m46s.
- Repair loops: two. Token usage is not exposed by the current app subagent surface.
- Final change: 8 files, 100 insertions, 13 deletions; 1,164 KiB static output across 86 files.
- Dependency state: 282 installed packages, no new dependency, and zero audit findings.

The actor made a smaller editorial, navigation, metadata, and evidence-linking pass. Its first repair
targeted only article prose, then incorrectly claimed the entire document was fixed. External
verification showed the header grid still widened to 411px. The second repair corrected the grid's
min-content behavior and the evidence note.

The standalone Codex CLI was updated from 0.149.0 to 0.149.1 during preflight, but full repository
sessions still rejected the Sol tool contract while simple Sol probes succeeded. The counted arm
therefore used the current authenticated first-party Codex app actor. This happened before productive
work and is a harness comparability limitation, not a candidate failure.

## Objective outcome

Both final commits now pass native verification and the same independent 20-case production browser
matrix. The first attempt from both arms missed the same inherited article overflow because their
internal visual checks did not reliably cover every content route after fonts settled.

The final blind reviewers disagree and the mean artifact score is effectively tied. Under the frozen
rubric, process evidence breaks a near tie only after the owner locks the native outcome judgement.
This single strong-baseline website task cannot establish a general solo-versus-team law.

## Product and harness findings

1. **Startup pool exhaustion, fixed.** Daemon bootstrap cached one PostgreSQL pool for every
   historical company and exhausted the server before startup. Commit `65e0c43` now drops each serial
   bootstrap pool; the daemon started with roughly 15 connections across 47 historical configs.
2. **Completion delivery is still broken.** Both Restless lead terminal reports remained only as
   `actor_wake_end` events. Completed Work had no handoff or native ReviewTarget, the Work graph ended
   with `handoffs: []`, and the owner inbox never received the final repaired artifact. Intermediate
   Exec acknowledgement did arrive. This directly violates the prepared-last-mile product promise.
3. **Internal responsive evidence was too weak.** The initial six-route evaluator and both actor
   checks allowed a linked route to stay broken. Native outcome review must discover the complete
   rendered surface and wait for assets that affect geometry, rather than trusting a fixed shortlist
   or an actor's prose claim.
4. **A plausible patch can still generate false evidence.** Codex's first repair and evidence note
   said 390/390 while a clean production probe measured 390/411. Independent outcome checks remain
   necessary at consequential acceptance boundaries.
5. **Supervision did not automatically remove first-pass defects.** The Restless lead improved scope
   framing and commissioned a correct repair after external feedback, but did not catch the original
   route failure or dependency audit regression. Lead value must be measured in accepted outcomes,
   not the presence of a review wake.
6. **Hot context is uneven.** Exec and lead resumed their sessions; repair Staff did not. That is a
   real cost and an explicit next target for context-continuity experiments.

