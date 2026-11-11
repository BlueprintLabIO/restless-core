# Workload C-SL-01 — late guard

Status: matched pair closed; B0 lead-alone win

## Frozen success contract and native review target

The exact contract is `scenario.md`. The native target is the browser battle on the frozen Cosmon seed;
the external evaluator drives the same keyboard and game loop a player uses.

## Domain, artifact type and why the work is real

Coding/product. The output is a playable interaction, not a code-only fixture. Timing, combat state,
damage, feedback and discoverability must agree in one running artifact.

## Starting artifact/ref and scenario hash

- Repository seed: `514b7b3d0a65e093af608b08ca142344412181f4`.
- Scenario: `scenario.md`; SHA-256 is frozen in each run manifest at preparation.
- Evaluator: `evaluate.mjs`; SHA-256 is frozen outside actor-visible context in each run manifest.

## Size prediction — judge 1

- Expected strong-lead wall time/tool calls: 6–12 minutes; roughly 8–18 tool calls.
- Evidence for prediction: one existing battle module owns input, telegraph, damage and battle HUD;
  existing Playwright fixtures already reach battle deterministically.
- Small / medium / large judgement: small.

## Structural feature judgement — judge 1

| Feature | Observation and locator | Judgement |
|---|---|---|
| Independently acceptable artifact seams | Logic and microcopy can be split, but neither is a useful accepted outcome alone. | Low |
| Dependency width / critical-path depth | Input edge → telegraph window → next-hit damage/reward → feedback/help. | Narrow, serial |
| Shared mutable-state surface | `Battle`, active combatant, enemy AI telegraph and HUD all meet in `js/battle.js`. | High relative to scope |
| Interface stability | Existing internal methods are stable, but no Perfect Guard seam exists. | Medium |
| Independent verifiability | Whole interaction is externally testable; a partial producer artifact is not. | High outcome verifiability, low sub-artifact verifiability |
| Specialist diversity | Product timing and presentation judgement are adjacent rather than distinct specialties. | Low |
| Tool/external latency available to overlap | Browser checks take time, but implementation has no independent external wait. | Low |
| Breadth uncertainty / coherence requirement | Little discovery breadth; high need for one timing model to stay coherent. | High coherence |
| Bad-contribution detection and repair cost | External evaluator catches most faults; overlapping edits in the one battle module are costly. | Medium |

## Candidate decomposition without assigning an arm

One contributor could own battle-state semantics while another changes help/feedback presentation, but
the second piece is too small to be independently valuable and both must agree on exact timing and
wording. This is deliberately a low-potential-parallelism cell, not a task made artificially indivisible.

## Judge 1 / Judge 2 disagreement and reconciliation

Judge 2 was a fresh read-only `gpt-5.6-terra` session. It saw the frozen seed and `scenario.md`, not
Judge 1's card. It independently classified the workload as small, low useful parallelism and high
coupling. Its evidence was the single `Battle` state surface joining held-key input, enemy windup,
telegraph, damage, energy and HUD feedback; it judged the one secondary test seam dependent on the
state protocol rather than independently acceptable.

The only material disagreement was size buffer: Judge 2 estimated 10–16 minutes and 10–20 tool calls,
versus Judge 1's 6–12 minutes and 8–18 calls, because fresh-key-edge semantics, projectile/leak edge
cases and fixed-step verification can take longer than the local code change. Reconciled prediction:
**small, 8–16 minutes, 9–20 tool calls; low potential producer parallelism; high coupling.** The raw
judge session used 39,074 tokens; this is experimental design cost, not an arm outcome cost.

## B0/B1 arm order and fairness controls

Order will be generated once from a recorded seed before either run. Both arms receive the same seed,
scenario bytes, lead model/reasoning, tools, wall envelope and evaluator hash. B1 alone receives one
`experience-presentation` producer on one pinned free model and one Staff slot.

## Evaluator and blind-review contract

`evaluate.mjs` is frozen before both runs, stored outside actor mounts, hash-checked and executed only
against isolated commit exports. Existing repository checks also run. A later blind product review may
rank candidates, but cannot override a contract failure.

## Post-run correction of predicted size and parallelism

The pre-run classification held. B0 reached its accountable completion decision in 371.6 seconds
(6.2 minutes), inside the reconciled 8–16 minute range and faster than predicted. B1 reached its
decision in 660.4 seconds (11.0 minutes). The code change was small; native browser-environment
discovery and proof dominated both arms' tool use.

B1 realised substantial overlap: its one producer worked for 430.3 seconds while the lead owned the
combat state and proof. The producer's one-file presentation commit was accepted without modification.
That overlap was not beneficial for the complete outcome. Both actors independently rediscovered and
adapted browser fixtures, the lead still owned all coupled timing/reward semantics, and integration
added a callback wake plus combined verification. The hidden evaluator rejected B1 10/11 because its
energy result was about 18.073 beyond matched natural regeneration rather than exactly 18. B0 passed
11/11.

Corrected observation: **small, high coupling, one independently acceptable but non-critical
presentation seam; potential and realised parallelism existed, but beneficial parallelism did not.**
This is a B0 win for this cell, not proof that the worker caused the lead's exactness defect.
