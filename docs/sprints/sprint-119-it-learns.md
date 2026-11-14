# Sprint 119: it learns

**Status: planned, not started (14 Nov 2026 AEDT). Depends on Sprints 116 and 117.**

The company gets better only when the owner corrects it, and only because a prompt says to record
the correction. One incident becomes a standing rule in the same turn. Nothing records what a piece
of Work was expected to do against what it did. Lessons are never pruned; the owner's runs saw
1,600 words and growing. The owner's Brand reaches workers and not the people who brief them.

Founding lines this restores:

- `docs/specs/orgintel.md` §3.1: "One incident should usually create a hypothesis, not a permanent
  rule."
- `docs/specs/orgintel.md` §5.3: "Promote information upward only when it is likely to remain
  useful."
- `docs/specs/orgintel.md` §10.4: "the organisation records one useful learning that changes a
  later choice".
- `ARCHITECTURE.md` §4.2, Self-building: "Evaluate whether new organisational mechanisms improve
  outcomes" and "Remove mechanisms that add overhead without value".

## What the owner decided

- The company reviews its own shape when the owner has given it enough new signal, "not a clock"
  (12 Nov). Part D keeps that test and only lets the review happen without waiting for an owner
  turn.
- Shrinking the company needs no approval; the owner is told in one line (12 Nov).
- Act then tell (14 Nov, see Sprint 116).
- **For the owner to settle before part B starts.** An explicit standing instruction from the
  owner ("from now on, always") should stay a rule at once, because it is a ruling and not an
  incident. A single correction or a send-back becomes an observation first. This spec assumes
  yes.

## What exists to build on

- Lessons: `crates/restless-orgintel/src/lessons.rs` (`record_lesson`, `mark_lesson_recurred`,
  `retire_lesson`, `lessons_block`), shown to every agent they apply to, capped at `LESSONS_SHOWN`
  (20) and `LESSONS_BLOCK_CHARS` (2,000). That cap cuts the list off; it does not choose what to
  keep.
- The team review: `crates/restless-engine/src/org_review.rs` (`due`, `scorecard`, `exec_block`)
  and the stored reviews in `crates/restless-orgintel/src/org_review.rs` (`record_org_review`,
  `list_org_reviews`). It is added to an owner turn that was happening anyway.
- Work already states an expected artifact (migration 0006) and an outcome. Attempts end with a
  state, and reviews can end `changes_requested`. `restless spend` gives cost per actor and model.
- The Company Constitution compiler: `compile_constitution` in
  `crates/restless-orgintel/src/constitution.rs`, called for Staff Attempts only, in
  `crates/restless-engine/src/staff.rs`, with a 32 KB bound.
- The `observation` statement kind from migration 0047, and Sprint 117's writer.

## A. An outcome record on every finished piece of Work

**Problem.** D1. Everything else in this sprint needs it.

**Build.**

1. Each finished Work has one record: what was expected, what happened, what it cost, and the
   verdict (accepted, sent back, failed, abandoned).
2. Build it as a read over rows that already exist wherever it can be: the expected artifact and
   outcome, the Attempt results, the review state, the owner's decision. Add a stored field only
   for what cannot be derived.
3. Cost per Work: whether spend can be attributed to one Work today is to confirm during
   ticketing (the same question as Sprint 118 part B). Where it cannot, the record says "unknown";
   it never guesses.
4. Shown on the Work's own page as labelled values in a fixed-height row. No new page.

**Done when.** Thirty finished Work on the Reno-scale seed each show the four values, and a Work
with no cost data shows "unknown".

## B. A short review when work is sent back or fails

**Problem.** D2 and D3.

**Build.**

1. When Work is sent back or fails, the lead's next wake for it (the exception or feedback wake
   that already happens) asks for a review of three lines: what was expected, what went wrong,
   what it would do differently. No extra turn.
2. The review writes one observation, scoped to the team or actor. An observation is not shown as
   a rule.
3. An observation becomes a lesson only when it repeats: a second observation of the same problem.
   `mark_lesson_recurred` already counts repeats; whether observations live as lessons in a
   not-yet-a-rule state or as `observation` statements is to confirm during ticketing. Choose the
   one that adds no table.
4. Change the Exec prompt's lesson section (`context.rs`, "What the owner has ruled") to match: a
   standing instruction is recorded as a rule; a single correction is recorded as an observation.
5. No debrief template, no review meeting, no state machine. Three lines in a turn that was
   already happening.

**Done when.** One send-back writes an observation and no rule. A second send-back for the same
problem makes it a lesson, and the next brief carries it.

## C. Lessons are capped and pruned

**Problem.** D4.

**Build.**

1. A lesson that has not been used or repeated in a stated number of weeks is retired with the
   reason recorded (`retire_lesson`). The owner can restore it (`restore_lesson`).
2. Near-duplicates are merged at the team review, by the model, with the owner told in one line.
3. Observations expire by themselves if they never repeat.
4. The 1,600 words the owner saw: which file or block grew (lessons, Exec's plan or its journal)
   is to confirm during ticketing, and that one gets a stated cap.

**Done when.** A seeded company with forty lessons, ten of them stale, ends with the stale ones
retired with reasons, and what each agent reads on a wake is under the cap without cutting a
lesson in half.

## D. The team review can happen without an owner turn

**Problem.** D5.

**Build.**

1. The test for "due" does not change: enough new owner signal and enough finished Attempts
   (`due` in the engine's `org_review.rs`).
2. When a review is due and no owner turn has carried it, it is added to the next daily review
   turn (Sprint 116). At most once a week. No wake of its own.
3. With no new evidence there is no review, whatever the date.

**Done when.** A test shows a due review carried by a daily review with no owner message, none the
next day, and none in a week with no new signal.

## E. Exec and leads get the company's Brand

**Problem.** D6. With no release, nobody gets anything.

**Build.**

1. Exec and leads receive a bounded part of the released Brand, truth and voice first, on turns
   that write to the owner or commission identity-bearing Work.
2. The Exec prompt has a size cap with a test. The bound and where the block goes (the wake
   message or a skill, before the fixed prompt) are to confirm during ticketing. The compiler is
   bound to one Work today; whether it can compile without one is also to confirm.
3. With no release, nothing is added and the existing Inbox nudge to set up Brand stays.

**Done when.** An Exec turn that writes to the owner in a company with a release carries the Brand
block within the cap, and a company without one carries none and invents nothing.

## F. The Friday brief says what changed

**Build.** Once a week the morning brief (Sprint 117) adds a short part: what the company changed
about how it works. It lists lessons added and retired, team changes from the stored reviews, and
skills added or retired, each in one line with its reason. With nothing changed it says nothing.

**Done when.** A scripted week with one new lesson and one retired specialist shows both lines.

## Tickets

| | Ticket | Layer | Closes | Reuses | Makes deletable |
| --- | --- | --- | --- | --- | --- |
| [ ] | 119-A Outcome record | OrgIntel, owner surface | D1 | Expected artifact, Attempt results, reviews, spend | Nothing |
| [ ] | 119-B Review on send-back or failure | Prompt, OrgIntel | D2, D3 | Exception and feedback wakes, `lessons.rs` | The prompt line that turns one correction into a rule |
| [ ] | 119-C Cap and prune | OrgIntel | D4 | `retire_lesson`, `restore_lesson` | Cutting the list off at 20 as the only limit |
| [ ] | 119-D Team review without an owner turn | Scheduler | D5 | Engine `org_review::due`, daily review | Nothing |
| [ ] | 119-E Brand for Exec and leads | Prompt, OrgIntel | D6 | `compile_constitution` | The "inspect `restless identity show`" reminder, if the block replaces it |
| [ ] | 119-F What changed, weekly | Prompt | D5, D9 in part | Morning brief, `list_org_reviews` | Nothing |
| [ ] | 119-G Two related briefs | Verification | D1 to D6 | `_test` company, real model | Nothing |

## Slice per layer

- **Kernel.** No change. A lesson grants no authority and gates nothing.
- **OrgIntel.** A read for the outcome record, an observation state, pruning. A new table only if
  ticketing shows no existing row can carry an observation.
- **Runtime.** No change.
- **Owner surface.** Four labelled values on the Work page and one weekly part in the brief.

## Cost

No new wake. The review after a send-back rides the lead's existing wake. The team review rides a
daily review, at most once a week, and only with new evidence. The Brand block adds tokens to some
Exec and lead turns; its size is stated in the ticket and tested. Pruning runs without a model
except the weekly merge.

## Done when

- **Two related briefs, a day apart**, in a `_test` company with a real model. The first is sent
  back for a stated mistake. The second, a related job, avoids that mistake with no correction
  from the owner. The transcript shows the observation, the lesson and the later brief that
  carried it. This is the spec's own line: "records one useful learning that changes a later
  choice".
- A single send-back alone creates no rule.
- No extra model turns against the same run before this sprint, apart from the stated Brand block.

## Out of scope

- Improvement records with a prediction, baseline and revert (D7).
- The full ladder from observation to skill to default (D8), and skills tied to outcomes (D11).
- Evolution beyond team shape (D10), competence evidence per person (D12), a knowledge curator or
  historian (D13), learning from how the owner handles Inbox items (D14).
- An experiment state machine, a debrief workflow, or scoring of individual workers.
