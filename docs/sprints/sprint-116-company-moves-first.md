# Sprint 116: the company moves first

**Status: planned (14 Nov 2026 AEDT). Being implemented on `feat/s116-company-moves-first`. First of
five sprints (116 to 120) from the 14 Nov audit of the founding documents against the code.**

The audit found that the company never moves first. Every automatic wake follows something already
in motion: an owner message, a finished Attempt, a schedule the owner approved. A new company with
a goal and a quiet owner does nothing. The owner's judgement: it feels like chatting to one agent,
when the product is meant to be "more alive and intelligent".

This sprint gives every company with an open goal one turn a day of its own, in which Exec starts
the next step toward each goal that has nothing moving.

Founding lines this restores:

- Helm's constitution (`helm-core`, predecessor repo): "The default message direction is toward the
  member, not from them".
- `docs/company-identity/restless-company-truth.md`: "Your company should keep working when the
  chat ends."
- `ARCHITECTURE.md` §4.2, Self-running: "Maintain active goals and next actions", "Wake the right
  actors without waiting for a human prompt", "Plan at daily, project and strategic horizons".

## What the owner decided (14 Nov 2026)

- **Act, then tell.** Exec may add its own follow-ups and schedules and start work on a goal the
  owner already set, inside the spend limit, without asking. It tells the owner, with an Undo.
  Spending more, installing or connecting things, acting outside the company and changing the
  charter still need the owner's yes.
- **One Exec turn a day for every company with an open goal is accepted**, provided it stops by
  itself when it has nothing to do. This relaxes the earlier rule that nothing changed means no
  wake. A company with no open goal is still never woken.

## What exists to build on

- The hourly check-in (PR #752): `crates/restless-orgintel/src/checkin.rs` reads facts into
  `CheckinState`, decides with the pure `next` and `due`, writes the wake's instructions in
  `message`, and records an event when Exec is asked (`signal_checkin_at`). The scheduler calls it
  from `check_in_with_owner` in `crates/restless-engine/src/schedule.rs`, and `next_due_delay`
  wakes the sweep in time. The daily review copies this shape.
- The holds the check-in already uses: `daemon.lifecycle.tasks_paused()` (draining for an update),
  `daemon.spend.budget_state(config).is_available()` (spend limit), `computer_runs_on_demand`
  (computer stopped by the owner), and an unread owner message to Exec.
- Goals with a finish line and a due date: `GoalRow.done_when` and `due_on`, `list_goals` in
  `crates/restless-orgintel/src/goals_work.rs`. Nothing reads `due_on` in the scheduler today.
- Commissioning to a lead in one call (`restless commission`, Exec prompt in `context.rs`).
- Applied changes with Undo (`crates/restless-engine/src/direction.rs`, `restless direction
  applied`, `web/src/lib/components/AppliedChanges.svelte`).
- Interval overrides for local stacks (`checkin_rhythm` reads `RESTLESS_CHECKIN_EVERY_SECS`).

## A. The daily review

**Problem.** Nothing looks at open goals (A2). Goal due dates are stored and shown and nobody
watches them (A4). A new company is seeded with no turn of its own (A1). Daily planning is a line
in a prompt (A7).

**Build.** One new mechanism, next to the check-in and shaped like it.

1. Once a day, for a company with at least one open goal, the daemon owes Exec a wake. The default
   interval is 24 hours since the last daily review. Whether it is pinned to the owner's morning
   is to confirm during ticketing; Sprint 117's morning brief needs that.
2. The wake message is computed without a model. It lists each open goal with no Work in motion,
   and each goal that is overdue or due within three days. A goal can appear under both.
3. The message tells Exec to start the next step for each listed goal by commissioning it to a
   lead, unless the goal is truly blocked on the owner, and then to post one short message saying
   what it started. Instructions live in the wake message, not the fixed Exec prompt, so they cost
   tokens only on this turn and the prompt size cap is untouched.
4. Held, not dropped, while the spend limit is reached, the plane is draining for an update, the
   computer was stopped by the owner, or an owner message to Exec is unread.
5. **Not** held by Away mode. The company keeps working while the owner is away. **Not** switched
   off by the Check-ins toggle in Company › Notifications, which controls check-ins only.
6. The daily review is a daemon message. Like the check-in, it is not owner activity for the
   blocked-work recheck and not a signal for the team review.
7. No separate goal sweep and no separate deadline watcher. Both are inputs to this one message.

**Done when.** With the interval shortened, a company with one open goal and no Work in motion gets
one Exec wake whose message names that goal. A goal due in two days is listed as due. A company
with no open goal gets none. Each hold is shown by a test, and so is "not held while away".

## B. It stops by itself

**Build.**

1. After three daily reviews in a row that started nothing, with no owner engagement and no goal
   change in between, the daily review goes quiet.
2. It resumes when the owner returns (the same engagement facts the check-in reads) or a goal is
   added, changed, closed or reopened.
3. "Started nothing" is read from Work rows created or started since that review, not from what
   Exec said.

**Done when.** A test shows three empty reviews, then silence, then one review after a goal's due
date changes, and one after an owner message.

## C. Act, then tell, in Exec's rules

**Problem.** `crates/restless-engine/skills/running-the-business/SKILL.md` lists "adding standing
schedules or other recurring commitments" among changes that need the owner's yes, and allows
routine delegation only "inside an outcome the owner already asked for" (A3).

**Build.**

1. Rewrite that section to the decision above. Exec may add its own follow-ups and schedules and
   start work toward an open goal without asking. The list that still needs a yes is: spending
   more, installing or connecting things, acting outside the company, changing the charter.
2. Each such change is told to the owner in one line with an Undo. Use the existing applied-change
   path where it fits. Whether Exec's own follow-ups fit `restless direction applied`, which was
   built for changes made on another person's request, is to confirm during ticketing.
3. Check three neighbouring lines for conflict and reword only what conflicts: "After dispatch,
   quiesce" in the Exec prompt (`context.rs`), `OPPORTUNITY_AUTONOMY_GUIDANCE` in `schedule.rs`,
   and operating rules §6 on not renewing a schedule "merely to keep the company active".

**Done when.** The prompt tests assert the new wording. A scripted run shows Exec adding a
follow-up without an approval card, and the owner sees one line with a working Undo.

## D. Continue, pivot or stop

**Problem.** Exec reports an outcome and stops. The founding cockpit spec asked for a
recommendation to "continue, branch, repair, pivot, or stop" (A6).

**Build.** When Exec reports a finished outcome to the owner, it ends with one line: continue (and
the next step it is starting), pivot, or stop, with the reason. Under act then tell, "continue"
means it has already started that step. The instruction goes where it costs least: the result wake
or the `running-the-business` skill before the fixed prompt.

**Done when.** A scripted outcome report ends with that line, and "continue" has a commissioned
Work behind it.

## E. See the hourly check-in work

**Problem.** PR #752 says it was "not run on a live stack and not run with a model" (A5).

**Build.** Run it end to end on a local stack with the recipe in that PR: `tools/mock-model` with a
rule matching `Check-in due`, short intervals through the two override variables, a company with a
goal, one owner message. Then once with a real model where a connection already exists. Fix the
wording until the questions are worth answering. Check the toggle in the browser as owner, and
that a member cannot see it.

**Done when.** A check-in, its nudge and the silence after it are each seen on a local stack, and
the transcript is kept under `docs/sprints/evidence/`.

## Tickets

| | Ticket | Layer | Closes | Reuses | Makes deletable |
| --- | --- | --- | --- | --- | --- |
| [ ] | 116-A Daily review wake and message | OrgIntel, scheduler | A1, A2, A4, A7 (daily) | `checkin.rs` shape, `list_goals`, scheduler sweep | The need for a goal sweep or a deadline watcher |
| [ ] | 116-B Goes quiet after three empty reviews | OrgIntel | Cost bound | Check-in engagement facts | Nothing |
| [ ] | 116-C Act then tell in `running-the-business` | Prompt | A3 | Applied changes with Undo | The yes-first lines for follow-ups and schedules |
| [ ] | 116-D Continue, pivot or stop line | Prompt | A6 | Proactive result wake | Nothing |
| [ ] | 116-E Check-in verified live | Verification | A5 | PR #752 recipe, `tools/mock-model` | Nothing |
| [ ] | 116-F The idle test | Verification | A1 to A7 | `scripts/restless-dev`, mock model | Nothing |

## Slice per layer

- **Kernel (Authority).** No change. The spend limit and Undo are read, not altered.
- **OrgIntel.** One module for the daily review, with its event. No new table unless the event log
  cannot hold the count of empty reviews (to confirm during ticketing).
- **Runtime.** No change.
- **Owner surface.** None beyond Exec's one short message and the Undo line. The "today" brief on
  the landing page is Sprint 117.

## Cost

At most one Exec turn per company per day. Zero for a company with no open goal. Zero after three
empty reviews until the owner returns or a goal changes. Work that the review starts is paid for
under the existing spend limit. The Sprint 109 idle check (zero model calls in a quiet company)
must still pass for a company with no open goal.

## Done when (the idle test)

- **Input.** A new company with one goal. One owner message on day one, then silence. Intervals
  shortened by an environment override.
- **Expected.** The next daily review starts Work toward the goal with no owner input, and Exec
  posts one short message saying what it started. Three further empty reviews, then no more wakes.
- Run with the mock model at minimum, and with a real model where a connection already exists.
  Keep the transcript. A sprint that fails this test is not done.
- No new jank, and no model call in a company with no open goal.

## Out of scope

- The morning brief on the landing page and by email (Sprint 117).
- Health signals that wake a lead (Sprint 118).
- Any learning from outcomes (Sprint 119).
- Mandates, strategic planning and a resource allocator (A7 strategic, A8, A9).
- A workflow engine, a planner entity or a goal state machine. The daily review is a message.
