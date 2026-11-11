# How every OI experiment is run

Each experiment plan in this folder follows these rules. A plan may be stricter but never looser.

## 1. Write it down first

Before the first counted run, the plan states and dates:

- the question, in one sentence;
- what we expect to happen and what result would prove us wrong;
- the setups being compared, and the one thing that differs between them;
- the main measure (one), the supporting measures, and how each is scored;
- the sample size and why it is enough (see section 5);
- the decision rule: which result leads to which product or publishing decision;
- the stop rules and the budget.

The plan is committed before running. Its commit hash goes in the results. Any later change is an
amendment file that says what changed, why, and whether any counted data had been seen.

## 2. Freeze the inputs

- Task text, starting files, data snapshots and graders are frozen and hashed.
- The exact model name, effort setting, tools and runtime are recorded per run.
- Setups run in an interleaved, shuffled order, so time of day and provider load don't line up with
  one setup. The shuffle seed is recorded.
- Each setup gets the same total budget unless budget is the thing being tested. Budgets are defined as
  token and tool-call caps, and actual spend is recorded per run.
- **One pinned Restless build per experiment.** Restless ships about twice a day; an experiment runs
  on one recorded build hash from first counted run to last. A build change mid-experiment is an
  amendment.
- Anything that depends on the real world while running (the clock, live websites, other people) is
  either simulated or frozen into a snapshot taken before the run.

## 3. Prefer checks a machine or the world can do

In order of preference:

1. **Known answers:** planted mistakes, constraints that can be checked by code, tests.
2. **The world:** outcomes that happen after the prediction (auction prices).
3. **Model graders:** only when 1 and 2 are impossible. Use a different model family from the one
   being tested, give graders no hint of which setup produced the work, and check the grader against
   the owner on at least 40 items. Report the agreement. If agreement is below 80%, the grader is not
   used for headline numbers. Each plan names its grader's model family, and it must be outside every
   family being tested in that plan.
4. **Agreement checks on fresh items.** If a rubric is revised after an agreement check, agreement is
   checked again on new items, not the ones used to revise it.

## 4. Keep failure types apart

Every failed run gets one label before anyone looks at which setup it came from:

- **the work was wrong** (counts against the setup);
- **the organisation failed** (lost information, duplicated work, conflict; counts against the setup);
- **the harness broke** (our tooling; run is invalid and replayed once for both sides);
- **the provider failed** (rate limit, outage; invalid and replayed once for both sides).

All invalid runs are listed in the results with their reason. Nothing is silently dropped.

## 5. Enough data to say something

- **Pilot:** about 10% of the full run, to find harness bugs and estimate spread. Pilot data is never
  part of the headline result.
- **Size from the pilot.** Use the spread seen in the pilot to size the main run so the smallest
  difference we care about would be detected most of the time (80% power). Rough guide for comparing
  two rates near 50%: about 100 items per setup to detect a 20-point gap, about 400 for a 10-point gap.
  Paired designs (the same item under every setup) need fewer.
- **Count the right thing.** Repeat runs of the same task are related, so they are not extra samples.
  The sample size for power is the number of independent tasks or items. Each plan states this number
  for its main comparison.
- **Repeat runs.** At least 3 runs per task per setup where model randomness matters. They reduce
  noise within a task; they do not add tasks.
- **"No difference" needs a margin.** A claim that two setups are the same requires a margin named in
  advance and a formal equivalence test (two one-sided tests). Failing to find a difference is not
  evidence of no difference.
- **Don't pick and score with the same runs.** If tasks are selected by how a setup did (e.g. "hard for
  one agent"), the runs used to select them are not reused to score that setup.
- **Report ranges.** Every headline number comes with a 95% range from a bootstrap over tasks (resample
  tasks, not runs, because runs on the same task are related).
- **Several comparisons:** when testing more than three setups, say so and adjust (Holm correction), or
  name one main comparison in advance.

## 6. Check it holds elsewhere

A headline claim needs at least one of:

- a second model family giving the same direction of result;
- a second task family giving the same direction;
- a held-out task set the design was not tuned on.

If none holds, the claim is published as "on X, with model Y" and nothing wider.

## 7. Budget and machine limits

- Runs go on restless-dev in sandbox companies, never on prod companies.
- At most 3 heavy agents at once (shared subscription limit). Long runs go overnight.
- Dev Postgres struggles past about 20 test companies; reuse and tear down sandbox companies.
- Each plan states a spend ceiling. Crossing 80% of it pauses the run for an owner check.
- Grading costs count. EXP-17 spent more on blind review than on the runs themselves.

## 8. Harness checks before counting (lessons from EXP-17)

- Grader bundles are readable by the grader's user (EXP-17 lost reviews to file permissions).
- Concurrent graders do not share one identity or meter.
- Every run records tokens, spend and wall time for every agent, including supervisors. EXP-17 had no
  token counts for supervised runs.
- Both setups use the same worker runtime and the same brief. If one side's worker spends more on the
  same task, investigate before counting.

## 9. What every result ships with

- `RESULTS.md`: the finding in plain words, the one chart, the range, the limits, all invalid runs;
- the raw data (anonymised), and the analysis script that turns it into the chart;
- a public kit where possible: tasks plus a script that runs on someone else's agents;
- the plan's commit hash and every amendment.

## 10. Privacy

Real company material (Reno clients, marketplace sellers, owner messages) never leaves the machine
unless anonymised. Task sets built from real work are rewritten so no person or business can be
identified before anything is published.
