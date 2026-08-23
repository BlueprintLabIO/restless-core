# Workload C-SH-01 — research cairn navigation

Status: frozen for matched run

## Frozen success contract and native review target

The exact contract is `scenario.md`. The native target is the running Sunleaf Basin: an animated
world-space cairn beacon and a live accessible HUD distance/bearing marker.

## Domain, artifact type and why the work is real

Coding/product. The output changes the playable exploration experience in two modalities rather than
only adding fixtures or prose.

## Starting artifact/ref and scenario hash

- Repository seed: `514b7b3d0a65e093af608b08ca142344412181f4`.
- Scenario SHA-256: `336755d437ade5ddb6897f7614b5dab1f187c1034ffee74a87b61c6d6cb54731`.
- Evaluator SHA-256: `a27128d2092f93e2b9523f005936a3bb8e3bccbbc45bde0451b581f287dd63d3`.

## Size prediction — judge 1

- Expected strong-lead wall time/tool calls: 7–14 minutes; roughly 10–22 tool calls.
- Evidence for prediction: the cairn and world update already live in `js/world.js`; the HUD has a
  compact constructor/setter surface in `js/hud.js`; the game loop already exposes
  `world.props.cairn.position` and an exploration-only update point.
- Small / medium / large judgement: small.

## Structural feature judgement — judge 1

| Feature | Observation and locator | Judgement |
|---|---|---|
| Independently acceptable artifact seams | Animated world beacon in `world.js`; live navigation marker in `hud.js` + `game.js`. Either improves navigation alone. | High for two seams |
| Dependency width / critical-path depth | Both consume the frozen existing cairn position; neither requires the other's implementation. | Wide/shallow |
| Shared mutable-state surface | Read-only cairn/player positions are shared; writes are in distinct modules and DOM/scene objects. | Low |
| Interface stability | `world.props.cairn`, `HUD`, player position and the main exploration loop already exist. | High |
| Independent verifiability | Beacon presence/animation and HUD distance/bearing can be probed separately in the browser. | High |
| Specialist diversity | World/3D presentation versus HUD/navigation logic and accessibility. | Medium |
| Tool/external latency available to overlap | Both seams can be implemented and checked in parallel; final browser regression remains shared. | Medium |
| Breadth uncertainty / coherence requirement | Little discovery breadth; only naming, location and visual fit must agree. | Low coherence burden |
| Bad-contribution detection and repair cost | Each seam has an explicit browser handle and does not mutate the other's files. | Low |

## Candidate decomposition without assigning an arm

One contributor can build and animate the diegetic beacon under the existing cairn while another adds
the exploration-only accessible HUD marker and live eight-way bearing. They share only the already
observable cairn position and frozen compass convention. Integration should be an artifact acceptance,
not an interface negotiation.

## Judge 1 / Judge 2 disagreement and reconciliation

Judge 2 was a fresh read-only `gpt-5.6-terra` session. It saw only the exact seed and frozen scenario,
not Judge 1's card or evaluator. It also classified the work as small and judged the two seams
independently acceptable, the existing interfaces stable, browser verification strong, coherence and
repair costs low, and the high-separability label broadly justified.

There were two useful disagreements. Judge 2 estimated 35–60 minutes and 8–14 tool calls, versus Judge
1's 7–14 minutes and 10–22 tools, because it reserved substantial visual-polish and regression time.
It also called overlap low at feature-code level and moderate at integration, reasoning that lifecycle
visibility converges on `Game.mode` and the main loop. Judge 1 sees more direct implementation overlap:
the beacon can remain wholly inside `buildWorld`/`world.update`, already called by the loop, while the
marker alone touches HUD/game lifecycle.

Reconciled prediction: **small, 8–30 minutes, 10–22 tool calls; two high-value parallel implementation
seams with a short shared integration/proof tail; low coupling, not zero coupling.** The observed C-SL
lead timings justify discounting Judge 2's 60-minute upper bound without erasing it. Judge 2 used
240,609 input tokens (194,816 cached), 2,405 output tokens and 530 reasoning tokens; this is design cost,
not arm cost.

## B0/B1 arm order and fairness controls

Order seed: `C-SH-01:v24:ordinary-crossover:2026-08-23`; SHA-256
`a42b2a8b298829f8f531243041812c176d0ab8757dc80024df2fb347dcd1831f`. The same first-32-bit parity
rule used by C-SL selects **B1 → B0**. Both arms receive the same seed, scenario bytes, Sol lead, tools,
1,200-second envelope and evaluator hash. B1 alone receives one `world-content` producer on pinned
`stealth/ox-alpha` and one Staff slot.

## Evaluator and blind-review contract

`evaluate.mjs` is frozen before either run, stored outside actor mounts, hash-checked and executed only
against isolated commit exports. It probes the two seams independently, then checks their combined
running state and browser errors. Enumerable contract failure cannot be overridden by preference.

## Post-run correction of predicted size and parallelism

The pre-run structure judgement was substantially right: both arms found the same world/HUD split,
and B1 realised useful overlap. The worker produced the world beacon as clean commit
`70d580445c7d7ef5d96a28167c5d79713cca54be`; the lead inspected and integrated it without editing the
worker's code while completing the HUD seam. The final candidate contained both independently useful
affordances and passed the frozen evaluator 11/11.

The prediction overstated the economic value of that overlap. B1 reached its decision in 907.3
seconds and used 4,981,085 reported tokens plus 116 tool calls across all turns. B0 reached its
decision in 567.4 seconds with 2,392,349 tokens and 31 tool calls. Both arms independently rediscovered
the host's Chrome/CDP verification path; B1 also paid for the worker's container-side Playwright
discovery, a long self-invented pixel-luminance proof, artifact inspection and final reintegration.
Potential and realised parallelism were high enough to be observable, but beneficial parallelism was
not established.

The frozen evaluator then exposed a validity defect rather than a clean outcome difference. B0's
native proof observed animation in the rendered beacon's child ring, beam and core, but the evaluator
watched only scale/rotation of the static outer `cairnBeacon` review handle. It therefore scored B0
10/11 despite the scenario requiring the visible beacon, not specifically its root transform, to be
animated. The immutable score is retained. It is not post-hoc rewritten into a pass, and it is not
treated as evidence that B0 failed the owner outcome. A corrected subtree/render-observable evaluator
and actor-visible runtime capability capsule require a fresh matched replicate.

Post-run classification: **small; high potential and realised separability; large ordinary-handoff
tax; beneficial parallelism unresolved because the frozen outcome proxy was narrower than the owner
contract.** Codes: `R2`, `R4`, `S4`, `E5`.

## Confirmatory replicate freeze

The scenario and seed remain byte-identical. Corrected evaluator `evaluate-r2.mjs` has SHA-256
`a9d2ccbc5e1357c800c4ea1adc23b49f63dcbc46c4fe9392387c487de76cf871`. It observes transforms and
material/light properties throughout the rendered `cairnBeacon` subtree rather than requiring motion
on its outer handle, waits 300ms after marker moves, and accepts only `ARRIVED` or a standalone `0m`.

Three pre-run controls passed: the seed failed the same nine absent-feature checks with zero browser
errors; the original B0 and B1 candidates each passed 11/11 when evaluated sequentially. The first
draft of r2 is retained in scratch evidence but was rejected before counted runs because its `0m`
regex also matched `20m` and concurrent browser probes exposed stale marker state.

The runtime revision passed 38/38 baseline-isolation checks and 22/22 artifact/runtime checks,
including execution of the first-party native adapter against a live mounted workspace. Order seed
`C-SH-01-R2:v25:ordinary-crossover:2026-08-23` has SHA-256
`7c38716eb919fca4942df467b16053a4d453859fa36211c11d4abfda721de253`; even first-32-bit parity
selects **B0 → B1**.

## Confirmatory post-run result

The runtime correction removed much of the accidental tax but did not reverse the result. B0 reached
its decision in 318.8 seconds with 1,270,246 reported tokens and 21 tools. B1 reached its decision in
449.4 seconds with 1,644,569 combined tokens and 55 tools. Thus B0 remained about 29% faster, used 23%
fewer tokens and 62% fewer tools.

B1's useful worker Attempt fell from 562.5 to 224.3 seconds; its turn used 15,144 tokens and 21 tools,
down from 32,308 and 57. The team produced one scoped `js/world.js` commit, while the lead completed
and committed the HUD seam during the overlap. The worker no longer rediscovered browser
infrastructure or delayed handoff for a pixel-luminance test. The lead still paid a second model turn,
exact artifact inspection, integration and combined proof, which exceeded the saved production time.

The r2 evaluator passed B0 11/11 and scored B1 10/11 because B1 placed rendered beacon geometry above
the cairn while leaving the containing group's origin at cairn height. A post-run evaluator with hash
`cd0334c91203339579b18e3a15fca47fdf53b6f66631d5898414c145ac6029de` inspected rendered mesh world
positions: the seed failed and both r2 candidates passed with zero browser errors. B1 also passed its
own combined 14/14 feature proof plus 12/12 battle, 7/7 combat and 29/29 roster/evolution checks. The
frozen 10/11 remains immutable, but the narrower root-origin proxy is not treated as an owner-outcome
defect.

Final cell classification: **small, highly separable work can realise clean useful overlap without
becoming beneficial teamwork. Runtime capability capsules reduce waste substantially; briefing,
artifact acceptance and whole-outcome proof remain the irreducible handoff tax at this size. B0 wins
the cell on efficiency at outcome parity.**
