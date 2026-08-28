# AI game development playbook — evidence-led first loop

**Status:** Initial playbook, seeded by Swift Arrival Dogfood 4.  Extend it only with observed runs.

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
multi-player scale, Steam integration, automated exports, or whether a game is fun.  Treat those
as later scenarios with their own evidence rather than extrapolating from a local two-player loop.

## Source

The source guidance is [Swift Arrival’s AI-first development plan](../dogfood/swift-arrival/swift-arrival-ai-development-plan.md).
The observed run is [Dogfood 4](../dogfood/swift-arrival/dogfood-4.md); its after-action is added
only once the final founder review is recorded.
