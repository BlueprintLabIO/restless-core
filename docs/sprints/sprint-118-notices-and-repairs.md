# Sprint 118: it notices and repairs itself

**Status: planned, not started (14 Nov 2026 AEDT). Part C uses Sprint 117's assumption record.**

The company can see some of its own problems and cannot act on them. The health signals are
computed only inside an Exec wake that something else caused, so they give awareness and never a
wake. Four signals the spec lists are missing. And leads swallow questions: in the owner's own
small runs on 12 Nov (not a published result), an agent working alone asked a necessary question
24 times out of 24, and 3 times out of 24 under managers, because the managers' briefs said "stop
and report back".

Founding lines this restores:

- `ARCHITECTURE.md` §4.2, Self-healing: "Detect stalled, duplicated or contradictory work",
  "Restart, reassign, simplify or replan", "Escalate only when the organisation cannot recover
  autonomously".
- `docs/specs/orgintel.md` §7.2: the health observer "should trigger awareness or a wakeup, not
  become a universal blocker".

## What the owner decided

- Act then tell (14 Nov, see Sprint 116). A lead may repair inside the outcome it owns without
  asking.
- Blocked work is looked at again without the owner saying "try now" (12 Nov).
- No dead ends (3 Nov): a blocker with no way through is a top-priority bug.

## What exists to build on

- `health::organisational` (spend with nothing completed, one effect failing three times, blocked
  Work) and `health::team_shape` (idle members, one lead across several goals) in
  `crates/restless-engine/src/health.rs`. Both are called once, in `exec.rs` when Exec's context is
  assembled, and shown under "What looks wrong".
- Sweeps that already owe a wake, each bounded and recorded as an event:
  `tell_leads_of_stalled_work` (told once per revision, `stalled_work.rs`),
  `recheck_external_waits_at` (`block_recheck.rs`), `signal_stale_owner_handoffs_at` (at most three
  times per item, `stale_handoffs.rs`). `stalled_work_loop` in `schedule.rs` runs the first.
- Terminal exceptions already go to the accountable lead.
- The check-in counts `waiting_on_owner`, and its message asks for "one of their Inbox items that
  has sat too long".
- Work revisions (`WorkRow.revision`), Work edges (`list_work_edges`), and an Attempt limit per
  Work (`attempt_limit`).

## A. A health signal can wake the accountable lead

**Problem.** C1.

**Build.**

1. Compute the signals in the scheduler sweep as well, from the same rows Exec's snapshot reads.
   One function, two callers, so the sweep and "What looks wrong" cannot disagree.
2. A signal about a piece of Work or a team owes one wake to its accountable lead. A signal about
   the whole company (spend with nothing completed) goes to Exec.
3. At most once per problem. The key is the signal kind, its subject and the Work revision,
   recorded as an event, the way stalled Work is told "once per revision". A problem that stays
   the same is never raised twice. A new revision or a new subject is a new problem.
4. The wake message states the observation and asks the lead to check it and choose the smallest
   repair. It never changes Work state itself and never blocks anything. The lead may answer that
   nothing is wrong.
5. Held under the same conditions as the check-in, except Away mode: repair continues while the
   owner is away.

**Done when.** A test seeds each existing signal and sees exactly one wake to the right actor, none
on the next sweep, and one more after the Work's revision changes.

## B. Four missing signals

**Problem.** C3. Each must be computable from rows already kept, with no model.

| Signal | First test | Goes to |
| --- | --- | --- |
| Review loop | A Work past its third revision with a review still sending it back | Its lead |
| One blocker holding several jobs | One blocked or abandoned Work that two or more open Work require | The blocker's lead |
| Cost rising without output | Spend on one Work past a stated amount since its last produced result | Its lead |
| Duplicate work | Two open Work under one goal that look like the same job | Exec |

**Build.**

1. Review loop: the `running-the-business` skill already says to stop "past its third revision".
   This makes it a detector instead of advice.
2. Shared blocker: count the edges. `stalled_work` already finds Work that requires abandoned Work.
3. Cost without output: whether spend can be attributed to one Work is to confirm during ticketing.
   If it cannot, use spend per actor since that actor's last finished Attempt.
4. Duplicate work: a cheap test without a model is to confirm during ticketing. Start with the same
   goal, the same expected artifact and overlapping titles. If that proves noisy, show the lead the
   goal's other open Work at commissioning instead, and do not build a detector.
5. Thresholds start loose. A signal that fires on normal work is noise, as the comment in
   `health.rs` says.

**Done when.** Each signal has a test with a seeded true case and a seeded near miss that does not
fire.

## C. A lead's question travels; the work does not stop

**Problem.** C2.

**Build.**

1. Doctrine, in the lead contract (`crates/restless-engine/src/staff/context.rs`) and operating
   rules §6: a brief never tells a worker to stop and report back on an open question. The worker
   states the question, picks the most likely answer, writes it down as an assumption (Sprint 117
   part A), and carries on wherever the step can be undone.
2. A question only the owner can answer is recorded once as an item waiting on the owner. It then
   appears in the next check-in through the existing `waiting_on_owner` count. No new channel.
3. A lead who has held a worker's question for more than a stated time is one more signal under
   part A: the question goes up to Exec.
4. The Work stops only when the next step cannot be undone or acts outside the company. That is
   the existing owner handoff, unchanged.

**Done when.** In a scripted run a worker hits a necessary question. The question is in the next
check-in, the Work has moved on with the assumption recorded, and nothing was blocked.

## D. Replan, not retry

**Problem.** C4. Only replan is built here. Reassign and simplify stay the lead's judgement.

**Build.**

1. When two Attempts on the same Work end failed or blocked for the same reason, the lead gets one
   message asking for a replan: change the approach, split the Work, or drop it and say why.
2. "The same reason" is read from the typed block kind where one exists. For free-text results the
   test is to confirm during ticketing.
3. The existing Attempt limit still bounds retries. This adds the question before the limit is
   reached, not a new limit.

**Done when.** Two same-cause failures produce one replan message, and a third Attempt runs only
after the lead has changed the Work.

## Tickets

| | Ticket | Layer | Closes | Reuses | Makes deletable |
| --- | --- | --- | --- | --- | --- |
| [ ] | 118-A Signals owe a wake to the lead | OrgIntel, scheduler | C1 | `health.rs`, the once-per-revision pattern | The second copy of signal logic, if any is found |
| [ ] | 118-B Four signals | OrgIntel | C3 | Work revisions, edges, spend | The "third revision" advice line in the skill |
| [ ] | 118-C Questions travel to the check-in | Prompt, OrgIntel | C2 | Check-in, owner handoffs, assumption record | "Stop and report back" in any brief template |
| [ ] | 118-D Replan after two same failures | OrgIntel, prompt | C4 (replan) | Typed block kinds, Attempt limit | Nothing |
| [ ] | 118-E Three injected failures | Verification | C1 to C4 | `_test` company, mock model | Nothing |

## Slice per layer

- **Kernel.** No change. A signal never grants or removes authority.
- **OrgIntel.** Signals as reads over existing rows, one event per raised problem. No new table
  unless the event log cannot hold the once-per-problem key (to confirm).
- **Runtime.** No change. Crash and restart recovery are already built and are not touched.
- **Owner surface.** None. Health reasons shown to the owner are B10, after the first owners.

## Cost

A healthy company adds no model turns. Each problem costs at most one lead turn, or one Exec turn
for a company-level signal. A daily cap on repair wakes per company is set during ticketing so a
broken detector cannot spend freely. A question that travels adds no turn: it rides the check-in.

## Done when

Three failures from `ARCHITECTURE.md` §10.6 are injected into a `_test` company, one at a time:

| Injected | Expected within the hour, with no owner action |
| --- | --- |
| A stalled worker | The lead is woken once and reassigns from the preserved work, or the owner has one card |
| Duplicate work | Exec merges, compares or stops one of the two, and says which |
| A manager sitting on a question | The question reaches the owner's next check-in and the Work has moved on |

- Each is repaired or reaches the owner within the hour. Run with the mock model at minimum and a
  real model where a connection exists. Keep the transcripts.
- No signal blocks Work. No problem is raised twice.

## Out of scope

- Drift and "busy but producing nothing" (C5), a recovery session with a handover note (C6),
  degraded states shown to the owner (C7).
- Reassign and simplify as mechanisms.
- Any detector that needs a model to run.
- A health score, a health page, or a rule about team size.
