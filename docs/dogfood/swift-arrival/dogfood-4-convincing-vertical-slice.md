# Dogfood 4 v0.6: Swift Arrival convincing vertical slice

**Status:** Draft for founder approval

**Date:** 31 August 2026

**Starting evidence:** EXP-15 player-invalid baseline and EXP-16 final mechanics candidate
`7964d772d53f1b414d92129b207a5a3bb7bb3617` / tree
`ce10b298455e206504c7338df2823133b7224f24`.

**Depends on:** Sprint 30 acceptance before autonomous repair loops begin. Local mechanics and rendered
repair loops do not depend on hosting; remote human acceptance uses Sprint 36 plus Cloud 14's bounded
published-service path rather than exposing the company Runtime.

## Mission

Produce one convincing, replayable 10-15 minute Swift Arrival workday that a source-blind visual player
can understand and complete through ordinary rendered controls, and that the founder explicitly accepts
as a vertical slice worth playing again.

The campaign tests autonomous value generation against a real product outcome. Passing headless gates,
adding content or accumulating Work does not count unless the rendered game becomes more playable.

## Slice contract

One workday contains all of the following in the ordinary game world:

1. Begin at the depot and understand the job without source knowledge.
2. Accept and secure cargo or a vampire passenger.
3. Enter and drive the truck using camera-relative, correctly labelled controls and solid collisions.
4. Read the route and receive timely feedback for interaction, obstruction, damage and recovery.
5. Encounter one robber behaviour with a visible intention, fair response window and meaningful
   success/failure consequence.
6. Experience one vampire-specific transport constraint or event that changes player judgement rather
   than merely changing flavour text.
7. Recover from one ordinary navigation, obstruction or interaction mistake without debug commands.
8. Reach the destination, complete delivery, see the outcome and return to an explicit end-of-workday
   state.

The workday must be authoritative in multiplayer-capable simulation even when evaluated with one
player. Debug overlays may supply evidence but cannot be required to play.

## Scope boundaries

In scope:

- first-person feel, camera-relative movement, mouse look, collision and interaction feedback;
- one depot, one complete route, one destination and a small coherent encounter set;
- golden-path NPC driving/delivery plus robber and vampire behaviours integrated into the rendered
  world;
- tuning of vehicle size, speeds, distances, snapping, timings, feedback and encounter pressure;
- the smallest architectural repair proven necessary by repeated player evidence; and
- deterministic, visual-agent and founder evaluation of the exact same candidate.

Out of scope:

- an MMO-scale world, progression economy, broad narrative campaign or production content pipeline;
- public hosting, monetization, purchases, marketing or live players;
- replacing stable simulation architecture merely for code cleanliness; and
- claiming fun from deterministic or vision-model evidence alone.

## Autonomous improvement loop

Each loop is event-driven:

1. Freeze one exact candidate and its build fingerprint.
2. Run deterministic mechanics and authority gates.
3. If green, give an opaque rendered build and public controls to a source-blind visual player.
4. Convert only decisive evidence into one ranked repair brief. The accountable lead chooses whether
   the issue is tuning, content, feedback, mechanics or architecture.
5. One worker owns the coherent repair end to end. Parallel workers are permitted only for disjoint
   locally gated assets or fixtures.
6. Re-run the affected gate and the full workday regression.
7. Preserve compact receipts and delete raw captures after findings are synthesized.

No cron wakes a healthy idle campaign. New candidate, terminal gate, visual verdict, founder feedback,
provider recovery or a declared scheduled follow-up is the wake signal.

## Evaluation layers

### L1: deterministic mechanics

Every candidate must pass exact gates for project start, controls, collisions, enter/exit, cargo or
passenger custody, route progress, obstruction recovery, robber authority, vampire constraint,
delivery, multiplayer authority and process cleanup.

### L2: source-blind rendered operator

The operator receives only the launch surface and the same controls/help available to a normal player.
It may observe screen and audio, issue ordinary inputs and provide timestamped textual findings. It may
not read source, private telemetry, scenario scripts or deterministic expected actions.

Use three released journeys with distinct seeds. The visual model and exact effort are frozen before
the first counted journey. A fallback model is admitted only in the frozen activation manifest; it is
not selected after seeing a result.

### L3: founder play sample

After L1 and L2 pass, the founder plays the exact candidate without coaching. The founder answers:

- Did you understand what to do?
- Did movement, driving and interaction feel coherent?
- Did the robber and vampire events create meaningful decisions?
- Could you recover from mistakes?
- Would you voluntarily play another workday?

Only an explicit positive answer to the final question supports the word `convincing`.

## Acceptance gates

| Gate | Requirement |
| --- | --- |
| V1 Exact build | One candidate commit, tree, build and gate digest across all layers |
| V2 Mechanics | All deterministic gates pass; zero authority violations, leaked processes or manual mid-run repairs |
| V3 NPC utility | Golden-path NPC completes 10/10 released workdays; adverse policies terminate boundedly and expose honest failures |
| V4 Visual completion | Source-blind operator completes at least 2/3 journeys without privileged help or manual rescue |
| V5 Experience | No unresolved blocker in controls, collision, objective clarity, encounter fairness or recovery |
| V6 Founder acceptance | Founder completes one workday and explicitly says the slice is convincing and worth another run |
| V7 Learning economy | Every paid repair maps to decisive evidence; no three-loop repetition of one unresolved structural cause |

## Evidence record

For each loop retain candidate coordinates, scenario seed, model route/effort, elapsed and active time,
spend, inputs/actions, gate receipt, visual findings, classified cause, repair coordinate, outcome and
cleanup receipt. Retain only decisive still-image digests and short text observations; delete PNGs,
video, engine caches, model homes and temporary builds after synthesis.

## Terminal outcomes

- **Convincing vertical slice:** V1-V7 pass on one exact candidate.
- **Mechanically playable, experience rejected:** V1-V3 pass but V4-V6 do not. Preserve the precise
  player boundary and stop claiming a vertical slice.
- **Foundation blocked:** three consecutive valid candidates fail for the same architectural cause.
  Stop autonomous loops and present the smallest architectural decision to the founder.
- **Harness invalid:** the evaluator cannot observe or control ordinary rendered play. Repair the
  evaluator once without changing the candidate, then replay symmetrically.
