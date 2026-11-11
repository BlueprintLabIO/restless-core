# Experiment 5: Searching for the best AI org chart

**Status:** plan, not started · **Can run without the owner:** mostly; each stage needs the owner's
go-ahead because this is the most expensive experiment · **Expected length:** about 5 months

## Question

With the model held fixed, how much does the shape of the organisation change results, what shape
wins for which kind of work, and does the best shape get flatter as models get stronger?

## Why it matters

The best-received agent research on Hacker News this year held the model fixed and changed only the
harness around it. This holds the model fixed and changes only the organisation. It is also the fair
version of the question EXP-17 was quoted as answering (see
[the archive note](../archive/2026-11-coordination-programme/ARCHIVED.md)): EXP-17 used tasks a single
worker already did perfectly, so no organisation could show value.

## What we expect, and what would prove us wrong

- **Shape matters on hard work.** On tasks a single agent often fails, the best organisation beats a
  single agent by 10 points or more. Wrong if the difference's 95% range sits inside ±5 points.
- **Shape barely matters on easy work.** On tasks solo already passes, extra structure only adds cost
  (EXP-17's observation, repeated properly).
- **The Coase prediction:** the benefit of extra layers and handoffs shrinks as the model gets
  stronger. Tested across all shapes as described in stage 4. Wrong if the benefit of depth is the same
  or larger at the strongest model.
- The search finds shapes better than any we design by hand, on tasks it was never tuned on. Wrong if
  the best hand-built shape matches the searched winner on the held-out tasks.

## Rules that apply to every stage

- **One pinned Restless build** for the whole experiment, recorded.
- **At most 3 agents running at once**, across all runs together. No shape may need more than 3
  agents at the same moment.
- **No leaks:**
  - Code tasks run on a copy of the repository with no git history past the task's parent commit and
    no network beyond a package mirror.
  - Restless memory and Library are switched off.
  - Forecast tasks use only auctions that are still open, taken from experiment 4's live batches, never
    ones whose price is already known.
- **Picking is separate from scoring.** The solo runs used to label task difficulty are never reused to
  score solo.
- **Hard spend ceiling: $6,500 equivalent for the whole experiment**, split by stage below. Crossing 80%
  of a stage's ceiling pauses for an owner check. This uses the shared subscription, so stages are
  scheduled around the companies' own work.

## Stage 0: a task suite that grades itself (2 to 3 weeks, ceiling $400)

| Family | Tasks | Graded by |
| --- | --- | --- |
| Code | 30 real Restless changes from October and November 2026 (after the models' training data), each frozen at its parent commit, 15 to 90 minutes of solo work | the change's own tests plus hidden tests |
| Research | 30 questions needing several sources, with answer keys built and checked in advance (e.g. "list every X with property Y") | answer key |
| Operations | 20 multi-constraint jobs from experiment 2, plus 20 live-auction forecasts from experiment 4 | constraint checks and final prices |

Each task gets 3 solo calibration runs to label how hard it is for one agent, plus a label for how
separable it is. **Hard** means solo passed at most 1 of 3. These calibration runs are not used again.

**The suite is split once, by script: 60 search tasks and 40 held-out tasks**, balanced by family and
difficulty. The held-out tasks are not touched until stage 3.

## Stage 1: hand-built sweep, including a fair EXP-17 retest (3 to 5 weeks, ceiling $1,600)

Six org charts designed by hand, each needing at most 3 agents at once:

1. one agent alone;
2. one agent plus a reviewer on a different model;
3. a lead with one worker (Restless today);
4. a lead with two parallel workers;
5. a specialist pipeline (plan, build, check);
6. a deliberately bad shape (four layers, summary-only handoffs) as a sanity check.

Each runs 3 times on the cheap model across all 60 search tasks, and 3 times on the frontier model
across the hard search tasks: about 6 × 3 × (60 + about 30) = 1,620 runs. Solo is scored on fresh runs,
not the calibration runs.

The EXP-17 retest is shapes 1 and 3 on the hard tasks.

**Gate (hard tasks only):** compute the paired difference between the best sensible shape and solo,
with its 95% range from resampling tasks.

- If the range sits entirely inside ±5 points, stop and publish "on this work, the org chart barely
  matters". That is a real result.
- If the range is entirely above +5, continue to stage 2.
- If it is in between, the owner decides whether to continue, with the numbers in front of them.

## Stage 2: automated search (3 to 4 weeks, ceiling $1,500)

An organisation is written as a settings file with:

- roles, and the model and effort for each;
- who reports to whom and who can message whom;
- what each role can see (the owner's words, a summary, the shared board, other workers' output);
- where reviews happen, and who reviews;
- how many workers run in parallel (at most 2, under the 3-agent limit);
- whether memory is shared;
- when to escalate to the owner.

**Search loop:** a designer agent reads the scoreboard and proposes changes (add a reviewer, merge two
roles, cut a layer, share memory), plus random changes for variety.

- About 120 shapes are each tried on 10 search tasks with the cheap model.
- The best third move up to 30 tasks.
- The best 10 run on all 60.

That is about 120 × 10 + 40 × 20 + 10 × 30 = 2,300 runs.

**Scoreboard:** quality, cost and owner interruptions. Keep every shape that is best for its cost
(nothing else is both better and cheaper), rather than one winner.

**Control:** a random search with the same budget, so we can tell whether the designer agent helps.

## Stage 3: confirm on held-out tasks (2 weeks, ceiling $1,500)

The top 3 searched shapes, the best 2 hand-built shapes and solo run on the 40 held-out tasks, 3 runs
each, **on both the cheap model and the frontier model** (about 1,440 runs). Running both means a failure
can be traced to over-tuning (the winner fails even on the cheap model) or to not carrying over between
models (it fails only on the frontier model).

**The main comparison, named in advance:** best searched shape against best hand-built shape, on
held-out tasks, frontier model. Before stage 3 starts, stage 1's variation between tasks is used to
check whether 40 held-out tasks can detect a 10-point difference. If they cannot, new held-out tasks are
written and frozen first. The equivalence margin for "the search didn't help" is ±5 points.

**Only held-out results are headline results.** The cost of the search itself is reported.

## Stage 4: what the winners tell us (4 to 6 weeks, ceiling $1,500)

- **Coase test, registered now:** all six stage 1 shapes plus the top 3 searched shapes run at three
  model strengths (small, mid, frontier) on the 30 hard search tasks, 3 runs each (about 2,400 runs). The analysis models pass rate
  against number of layers, model strength, and the two combined, across all shapes. The prediction is
  that the combined term is negative: the benefit of depth shrinks as models get stronger. This uses
  every shape, so it doesn't rest on which single shape happens to win.
- **Transfer:** does the best shape for code also win for research and operations?
- **What a handoff costs:** using experiment 2's measurements, tokens spent and information lost per
  handoff in each winning shape.
- **Live check (descriptive only):** run the winning shape in one real company for two weeks and compare
  with its previous two weeks. Not a controlled result and not presented as one.

## Measures

- **Main:** held-out pass rate (share of tasks fully passing their checks).
- Cost per task, time per task, owner interruptions per task.
- Quality and cost by task difficulty and separability.
- Layers, handoffs and parallel width of each winning shape.

## Run time and budget

| Stage | Runs | Calendar | Ceiling |
| --- | --- | --- | --- |
| 0 | 300 | 2 to 3 weeks | $400 |
| 1 | about 1,620 | 3 to 5 weeks | $1,600 |
| 2 | about 2,300 (mostly short, cheap model) | 3 to 4 weeks | $1,500 |
| 3 | about 1,440 | 2 weeks | $1,500 |
| 4 | about 2,400 | 4 to 6 weeks | $1,500 |
| **Total** | **about 8,000** | **about 5 months** | **$6,500** |

Calendar estimates assume runs average 20 to 40 minutes and 1 to 2 runs proceed at once under the
3-agent limit. Stage 0 measures real run times and costs, and every later stage is re-estimated before
its approval.

## Prior work to cite

Automated design of agent systems (ADAS, Hu, Lu and Clune, 2024), AFlow (2024), GPTSwarm (2024),
multi-agent network shapes (MacNet, 2024) and the taxonomy of multi-agent failures (MAST, 2025). What is
new here: longer business work, a cost and interruption scoreboard rather than accuracy alone, held-out
tasks graded by tests and the world, and a theory (Coase) with a prediction that can fail.

## What we publish

- **After stage 1:** the fair EXP-17 retest, as its own short post.
- **Title idea for the main result:** "Same model, different org chart."
- **The charts:** quality against cost for every shape tried, best ones highlighted; a diagram of the
  winning org; the benefit of depth at each model strength.
- **Public kit:** the settings-file format, the held-out tasks and the runner.

## Limits to state

- One product's runtime. Shapes may behave differently in other frameworks.
- The 3-agent limit caps how wide a shape can be; wide parallel teams are untested.
- The task suite is ours. Held-out testing guards against tuning to it, not against it being narrow.
- Model strength tiers come from one or two providers.
