# Experiment 2: The telephone game

**Status:** plan, not started · **Can run without the owner:** yes

## Question

How much of what the owner asks for survives each layer of AI management, and where is it lost: in
the message passed down, or in the work itself?

## Why it matters

Every handoff is a chance to drop something. The owner says "under $200, British spelling, don't
email before 9am"; three layers later the worker never saw the 9am rule. This is the most direct
measure of an organisation's information flow, and it gives Coase's "cost of coordinating" a number.
Restless also claims the owner's goal "stays pinned" through the company. This tests that claim.

## What we expect, and what would prove us wrong

- Each layer loses a share of constraints, and the losses add up (the curve keeps falling). Wrong if
  loss stops after the first handoff.
- Most loss happens in the message (the manager leaves it out), not in the work (the worker ignores
  it). Wrong if more than half of losses are constraints that were in the worker's brief.
- Constraints buried in the middle of a long message, and ones that sound minor, are lost more.
- Passing the original message along unchanged, or pinning it where the worker can see it, cuts the
  loss by at least half. Wrong if it cuts less than a quarter.

## Tasks

40 realistic owner requests in the same four kinds as experiment 1 (code, research, resale, customer
email), each 5 to 15 minutes of work for one agent. Each request carries 10 constraints that a script
can check:

- a number limit (price ceiling, word count);
- a format rule (a table, a heading, a file name);
- a word or phrase that must or must not appear;
- a spelling or tone rule a script can detect;
- an action rule visible in the tool log (don't contact X, don't touch file Y, nothing before 9am).
  Contacts are sandbox stand-ins, and time rules use a simulated clock set in the task, so they don't
  depend on when an overnight run happens to start;
- something that must be included (a link, a caveat, a named person).

Constraints belong to their task (a price ceiling only exists in a resale task), so position and
phrasing are balanced within each task: every task is written in 4 versions that rotate which
constraints sit in the middle versus at the start or end, and which are phrased as important versus as
asides. Each run gets one version at random, recorded.

## Setups

| Setup | Path from owner to worker |
| --- | --- |
| Depth 0 | owner message goes straight to the worker |
| Depth 1 | owner, Exec, worker |
| Depth 2 | owner, Exec, lead, worker |
| Depth 3 | owner, Exec, lead, sub-lead, worker |
| Depth 2 and 3, unchanged message | same chains, but each manager must pass the owner's message along word for word, plus their own notes |
| Depth 2 and 3, pinned goal | same chains, with the original message pinned where the worker can always read it (Restless as designed) |

Same model in every role, on one pinned Restless build. **Goal pinning is switched off in every setup
except "pinned goal"**, so the plain chains measure handoffs alone. Every handoff message is saved.
In pinned runs, a constraint counts as reaching the worker whenever it is in the pinned message, so a
loss there is always a loss in the work.

**The main comparison, named in advance:** loss at depth 2 against depth 0.

## Measures

- **Main:** share of constraints the final work satisfies, by depth.
- Where each lost constraint went missing: which handoff message first left it out, or whether the
  worker had it and didn't follow it. Checked by script where the wording is exact, otherwise by a
  grader from a model family not tested anywhere in this experiment (owner checks 40).
- Loss by position and by importance phrasing.
- Overall work quality on a short rubric, so a fix that keeps constraints but ruins the work is caught.
- Tokens and cost per handoff (the cost side of Coase).

## Sample size and analysis

40 tasks, 10 constraints each, 8 setups, 3 runs each: 960 runs. The independent sample is the 40
tasks; the 400 constraint checks per setup are related within a task and are analysed that way. Each
constraint is modelled as kept or lost, with depth, position and phrasing as factors and task as a
grouping factor. Ranges come from resampling tasks.

Pilot: 8 tasks, depths 0 and 2. It checks the harness and measures the depth 0 loss rate. **If depth 0
already keeps 95% or more, the tasks are made harder before counting** (more constraints, longer
messages), so the depth curve has room to show. The halving test is stated as an absolute drop in
loss, with its range, not only as a ratio.

**Second model family:** depth 0 and depth 2, all 40 tasks, 3 runs each (240 runs), to check the
direction holds.

## Budget

960 short runs plus 240 for the second family. Each run is a chain of up to 4 agents. Estimated $300
to $800 equivalent. Ceiling $800.

## Decision rules

- If pinning cuts loss by half or more: keep pinning, and the product shows it with the finding.
- If pinning doesn't help but unchanged forwarding does: change Restless handoffs to forward the
  owner's words, not a summary.
- If loss is mostly in the work, not the message: the fix is in worker briefs, not handoffs.

## What we publish

- **Title idea:** "Every layer of AI management loses X% of what you said."
- **The one chart:** share of instructions kept (up) against layers of management (across), one line
  per setup.
- **Public kit:** the 40 tasks with their constraint checkers.

## Limits to state

- Checkable constraints are a slice of intent. Taste and judgement are not measured here.
- Chains are linear. Real organisations also talk sideways.
- The tasks are realistic but constructed.
