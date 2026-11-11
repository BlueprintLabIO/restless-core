# Experiment 1: Do AI reviewers actually check?

**Status:** plan, not started · **Can run without the owner:** yes, except about 110 owner checks (about 1.5 hours)

## Question

When one AI agent reviews another's finished work, how many real mistakes does it catch, and what
makes it catch more?

## Why it matters

Every agent team, including Restless, uses review as a safety net: a worker finishes, someone checks,
the owner sees "done". If the check approves work without really checking it, the safety net is
decoration and costs money. EXP-17 never tested this because its tasks had no mistakes to catch.

## What we expect, and what would prove us wrong

- A reviewer that can run things (tests, open links, recompute numbers) catches clearly more than one
  that only reads. Wrong if the gap is under 10 points.
- A reviewer on a different model family catches more than one on the same model. Wrong if under 5
  points.
- Showing the reviewer the worker's own summary ("all links checked") lowers the catch rate. Wrong if
  catch rate does not drop.
- Finding one mistake makes reviewers stop looking: on items with three mistakes, the second and third
  are caught less often than single mistakes.

## The work being reviewed

200 finished pieces of work across four kinds (50 each), built from the shared task bank:

| Kind | Example | How mistakes can be checked |
| --- | --- | --- |
| Code change | a bug fix with tests | a test fails, or a planted bug a hidden test catches |
| Research note | a sourced answer to a question | a number differs from its cited source; a link is dead |
| Resale appraisal | an item price with comparable sales | wrong comparable, wrong condition, maths error |
| Customer email | a reply to a client (anonymised) | breaks a house rule, wrong date or amount, promises something not allowed |

Each piece starts as clean work, checked clean by script (and by the owner for the 30 emails). Then:

- **30% stay clean (60 items).** These measure false alarms: a reviewer that rejects everything must
  not win.
- **45% get one planted mistake (90 items). 25% get three (50 items, 150 mistakes).** That is 240
  planted mistakes, about 120 of them serious.
- **For the "stop looking" test,** each of the 150 mistakes in three-mistake items is also planted on
  its own in a separate copy of the same base item. The same mistake, alone or with company, is then
  compared directly. These copies are extra items and are not counted in the 200.
- Mistake types, spread evenly: a wrong fact, something broken, a house-rule break, a claim with no
  evidence ("verified" when nothing was verified), and a missing required part. Each is labelled
  serious or minor in advance.
- **Problems nobody planted.** Base work may contain real mistakes the script missed, and the best
  reviewers will find them. Every reported problem that matches no planted mistake is judged by the
  grader (family named below) as real or not; the owner checks a sample of 40 of those judgements. A
  real one counts as a catch, not a false alarm.
- **Real mistakes subset:** about 20 items use real mistakes Restless agents made and the owner caught
  later (unverified "confirmed" leads, unchecked prices, em dashes in public copy). These anchor the
  result in reality.

**Realism check before counting:** a grader from a different model family sees 100 items (half with
planted mistakes) and guesses which were altered, judging only whether the work looks edited. The
check passes only if the top of the 95% range for its accuracy is below 60%. Otherwise the planted
mistakes look artificial and get reworked.

## Setups

Every item is reviewed under every setup (paired), 3 times each.

| Setup | Reviewer model | Can run tools | Sees worker's summary |
| --- | --- | --- | --- |
| A. Read only (baseline) | same as worker | no | no |
| B. Can check | same as worker | yes | no |
| C. Different model | different family | no | no |
| D. Persuaded | same as worker | no | yes |
| E. As shipped | the real Restless lead review path in a sandbox company, on one pinned build | as that build is configured (recorded) | as that build is configured (recorded) |

**Models, named before the pilot:** the worker and same-model reviewer are one frontier model (family
X). Setup C uses a frontier model of similar strength from family Y, so the comparison is about family,
not strength. The grader is from a third family Z, used nowhere else in this experiment.

**The main comparison, named in advance:** A against B (reading against checking). The others are
supporting comparisons, adjusted for multiple tests.

Reviewers get the same brief in every setup: the original task, the work and the house rules. They
must answer approve or send back, and list each problem with where it is.

## Measures

- **Main:** share of serious planted mistakes caught, per mistake.
- False-alarm rate: share of clean items sent back, or flagged with a problem that isn't there.
- Catch rate by mistake type, and for the second and third mistake on three-mistake items.
- Real-mistakes subset catch rate, reported separately.
- Cost and time per review.

**Matching found problems to planted ones:** reviewers report the location. A script matches by
location and type. Unclear matches go to a different-family grader. The owner checks 40 matches; if
agreement is below 80%, matching is redone by hand for the headline numbers.

## Sample size

The independent sample for the main measure is about 120 serious mistakes in about 110 items. The 3
repeat reviews per setup reduce noise within each mistake; they are not extra samples. Analysis
resamples items, not reviews.

The pilot (16 items, setups A and B only) checks the harness and measures how much catch rates vary
between items. The main run is sized from that, targeting 80% power for a 10-point gap between A and
B. If the pilot says 120 serious mistakes is not enough, items are added before counting starts.

## Budget

About 350 items (200 plus the 150 single-mistake copies) × 5 setups × 3 reviews = about 5,250 reviews.
Estimated cost equivalent of $300 to $800 on the shared subscription. Ceiling $800.
Run overnight, 3 at a time.

## Decision rules

- If checking (B) beats reading (A) by 15 points or more: Restless reviews always get tools to check
  claims, and the setting says why.
- If a different model (C) beats the same model (A) by 10 points or more: Restless defaults to a
  different-family reviewer when one is connected.
- If the worker's summary (D) lowers catch rate by 10 points or more: reviewers stop seeing the worker's
  claims until after their own pass.
- If "as shipped" (E) is worse than the best setup, that is a bug to fix before publishing.

## What we publish

- **Title idea:** "We planted 240 mistakes in AI work. Here's how many the AI reviewer caught." (Use
  the final count.)
- **The one chart:** catch rate (up) against false alarms (across), one dot per setup, with ranges.
- **Public kit:** 40 items with planted mistakes, the answer key and a scoring script, so anyone can test
  their own reviewer.

## Limits to state

- Planted mistakes are not the same as natural ones. The real-mistakes subset is the check on this.
- Four kinds of work, mostly short. Long projects may differ.
- Results are for the models tested on the date tested.
