# AI game development playbook — evidence-led first loop

**Status:** Observed playbook through Swift Arrival EXP-14. Extend it only with observed runs.

## Purpose

Use this when a Restless company is asked to build a new game mechanic or the first playable
loop.  It keeps agents on a native, runnable outcome instead of treating code, screenshots, or
task completion as the game.

## Proven in Dogfood 4

The following are observed practices, not generic promises:

1. Start with the smallest complete player outcome.  For Swift Arrival this was: host, join,
   pick up one crate, drive one bounded route, unload, and observe a host-owned delivery result.
2. Establish a one-command, two-process probe before expanding gameplay.  It must prove the
   real selected transport and must retain distinct host and client logs.
3. Validate game scripts before spawning processes.  This converted repeated Godot parse hangs
   into a fast, attributable failure.
4. Bound every automated game run.  The Swift Arrival probe killed both processes at 90 seconds,
   stamped both logs as failed, and returned nonzero when a run did not complete.  The owner
   stopped being the timeout mechanism.
5. Let logs name the smallest failing mechanism.  The successful repair followed evidence that
   the host resolved a grab while `crate_holder` had not propagated to the joining client.
6. Keep a review target native to the game: a rendered host/client session plus outcome
   screenshots.  Source diffs and model reports remain supporting evidence.
7. Checkpoint the harness separately when it becomes an independently useful seam.  It made
   failure detection, evidence capture, and gameplay production independently accountable.
8. Keep a local physical-input replay alongside engine-level probes. In the v0.4 correction, an X11
   replay (`xdotool` + `scrot`) drove the rendered Godot client, retained its key trace and frames,
   and required host/client completion markers. Its failures found a real interaction problem that
   the in-engine scripted probe bypassed.
9. Keep production and independent play in separate contexts. EXP-11 repeatedly showed that a
   deterministic or producer-side pass can coexist with a fresh player's failure.
10. Make transient feedback capture atomic. If a banner lasts seconds but a model tool turn takes
    longer, issue the input and capture from one bounded native action, then inspect the resulting
    pixels in the same model session.
11. Treat exact window pixels as stronger evidence than titles, focus success, filenames, or process
    logs. Overlapping X11 windows repeatedly produced CLIENT-labelled routes with HOST pixels.
12. Test the happy path and the economically important bypass separately. Swift Arrival first passed
    the intended journey while still accepting an on-foot route-zero delivery.
13. Stop at the first conclusive independent blocker. Twenty numbered evaluator arms and USD 122.28
    of evaluation/harness spend were far too costly. Reuse the validated protocol and enforce one
    aggregate experiment envelope.
14. Make launch and attachment one atomic operation. EXP-14's native session returned one opaque
    exact CLIENT handle and removed title enumeration, focus guessing and second-channel discovery.
15. Give each independent player one native terminal claim. Combining a negative shortcut test and a
    complete positive journey caused a valid journey to fail its Work contract. Separate those runs.
16. Require player-visible evidence, not only state rejection. An in-zone route-zero attempt remained
    incomplete but gave no explanation; a fresh blind player found what the mechanical gate missed.
17. Treat an evaluator's pass label as a proposal. The final negative evaluator pressed E outside the
    destination zone and proved the wrong guard. Semantic supervision rejected the claim.
18. Pin one ACP runtime generation across host and company. Version skew produced a malformed native
    stream, consumed a message and poisoned metering before gameplay began.
19. Admit models by a valid protocol envelope, not HTTP status. A mixed GLM route returned HTTP 200
    with `{code,msg,success}` and no completion.
20. After a provider refusal, record one shared cooldown and sleep. The repaired EXP-14 path made one
    exact GLM attempt and generated no hidden retries.
21. Treat fun as a configured search surface over stable mechanics. Vehicle feel, hand assistance,
    snapping, cargo behaviour, pacing and recovery should branch as named presets with fixed scenario
    seeds before a playtest result is allowed to trigger core simulation refactoring.
22. A tuning win is not a count of changed constants. Freeze the candidate, preset and seed; run the
    native scenario; retain one canonical choice; and refactor only when the available parameter space
    cannot express legible intention, reliable multiplayer state or an enjoyable central interaction.

## Tight-loop architecture

Use this order for each material candidate:

1. Freeze an immutable candidate and exact launch profile.
2. Run project load and deterministic positive, negative and recovery gates.
3. Give a fresh source-blind player one opaque target handle and one outcome.
4. Stop at the first decisive semantic failure and retain at most 12 frames.
5. Permit one evidence-driven repair, rerun every cheap gate, then restart fresh-player acceptance
   from zero.
6. Ask a human to judge feel and fun only after independent completion succeeds.

The loop is a funnel, not a fan-out. More visual agents do not compensate for ambiguous target
identity, a bad Work contract or a missing mechanical gate.

## Run protocol

1. Write one scenario with a player-visible start, interaction, success condition, and clean
   shutdown.  State the explicit non-goals.
2. Live-probe the actual engine, headless mode, local transport, display/capture route, Git,
   and launch command.  Record commands and outputs.
3. Build placeholder visuals first, but make the network authority real from the first loop.
   A locally simulated second client is not evidence of multiplayer.
4. Put the probe behind these minimum contracts:

   - syntax/load validation before process spawn;
   - separate host and client logs;
   - bounded wall time with process cleanup and nonzero failure;
   - positive success markers on both sides; and
   - a clean-shutdown observation.

5. When the same failure repeats, change the method rather than adding another patch:
   separate harness from gameplay, add discriminating logs, or narrow the scenario.
6. Before review, run the scenario freshly, checkpoint the game and evidence, and prepare the
   playable desktop target plus capture fallback.
7. Split automated playtesting into honest lanes: a deterministic OS-input replay for mechanics;
   a separately live-probed vision-capable Staff worker for bounded visual observations; and founder
   review for taste. Do not call a screenshot or a replay a judgement of fun.
8. Freeze the candidate read-only but seed a separate writable evidence root. Bind every critical
   image and report to the exact current Work and Attempt.
9. Give a blind downstream player an ordering dependency without automatically attaching upstream
   evidence. Sequencing and evidence flow are different contracts.
10. Require two fresh completions only after one strict run proves the complete protocol. A conclusive
    blocker ends the candidate; it does not justify more evaluator retries.

## Evidence standard

For every scenario, preserve:

- engine version, commands, source commit, and scenario version;
- host and client logs from the exact run;
- a failure record when the run does not pass, including any owner rescue;
- captured native checkpoints for review; and
- a concise limitation statement.

`PASS` is credible only when it follows the actual end state on both processes.  A window,
network connection, screenshot, or test exit by itself is not enough.

## What this does not yet prove

Dogfood 4 does not establish Internet multiplayer, adversarial latency, physics stability,
multi-player scale, Steam integration, automated exports, or whether a game is fun. EXP-11 also did
not establish that passing deterministic probes makes a loop independently playable; its final
candidate passed mechanics and still failed the fresh route-end delivery. Treat all later claims as
separate scenarios with their own evidence.

## Source

The source guidance is [Swift Arrival’s AI-first development plan](../dogfood/swift-arrival/swift-arrival-ai-development-plan.md).
The observed run is [Dogfood 4](../dogfood/swift-arrival/dogfood-4.md); the strict autonomous frontier
result is [EXP-11](../../experiment/coordination/experiments/EXP-11/RESULTS.md). The repaired loop and
its incomplete final acceptance are [EXP-14](../../experiment/coordination/experiments/EXP-14/RESULTS.md).
