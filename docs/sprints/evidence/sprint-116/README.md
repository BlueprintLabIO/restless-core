# Sprint 116 observed evidence: the idle test

Recorded 14 November 2026 (times below are UTC) on branch `feat/s116-company-moves-first`, on one
local stack for this worktree with the scripted model. No real model was used: the stack was a fresh
dev profile with no model connection, and none was entered. What the scripted Exec says is the
scenario's text, so this shows that the wakes fire, what they ask and that Work really starts. It
does not show that a real model writes good questions or picks a good next step.

## Setup

- `node tools/mock-model/server.mjs --port 18611 --scenario <scenario.json> --log <calls.jsonl>`
- `RESTLESS_ANTHROPIC_BASE_URL=http://127.0.0.1:18611/v1 RESTLESS_DEV_MODEL=anthropic/claude-sonnet-5-5
  RESTLESS_DEV_CREDENTIAL_REFERENCE=env:<a placeholder> RESTLESS_CHECKIN_EVERY_SECS=60
  RESTLESS_CHECKIN_NUDGE_AFTER_SECS=180 RESTLESS_DAILY_REVIEW_EVERY_SECS=150
  scripts/restless-dev s116idle_test` (profile `dev172673`; the company image and document image
  were existing local ones, named by `RESTLESS_COMPANY_IMAGE` and `RESTLESS_NATIVE_DOCUMENTS_IMAGE`,
  so nothing was built; `web/node_modules` and `web/build` were borrowed from the #752 worktree).
- Scenario rules matched `Check-in due`, `Check-in nudge`, `Daily review due` (one `bash` step:
  `restless commission --lead sales-outreach … --goal <the goal> …`, then one line to the owner),
  the commissioned Work, and `Completed Work result for Exec delivery consideration`.
- Input: `restless goal add --title 'Three paying clients' --done-when '3 signed clients'` and
  `restless tell 's116 owner hello: the goal is three paying clients'`, both at 01:36:20. Nothing
  else from the owner until 01:56:42.
- Read from the company's own database (`events`, `messages`, `work`) and the mock's call log.

## What happened

| Time | What | Source |
| --- | --- | --- |
| 01:36:20 | Goal created; owner message | owner |
| 01:36:24 | Exec answers | 1 model call |
| 01:37:24 | **Check-in** asked, 60 s after Exec's answer; Exec posts an update and one question | `exec_checkin` (checkin) |
| 01:38:50 | **Daily review 1**, 150 s after the goal was set and 86 s after the check-in | `exec_daily_review` |
| 01:40:24 | Check-in nudge, 180 s after the unanswered check-in; then no more check-ins | `exec_checkin` (nudge) |
| 01:41:20 | Daily review 2 | `exec_daily_review`, `quiet_reviews` 1 |
| 01:43:50 | Daily review 3 | `exec_daily_review`, `quiet_reviews` 2 |
| 01:43:54 | **Work "List twelve local prospects" commissioned by Exec** to a new lead, under the goal | `work_commissioned`, `work.goal_id` |
| 01:43:55 | Exec tells the owner what it started and that they can have it undone | message |
| 01:43:59 | The lead's Work completes; the result reaches Exec | result notice |
| 01:44:01 | Exec reports the result and ends with a "Continue: …" line | message |
| 01:46:20, 01:48:51, 01:51:21 | Daily reviews 4 to 6 (the goal has nothing in motion again). The scenario now ends each quietly: no message to the owner | `quiet_reviews` 0, 1, 2 |
| 01:51:21 to 01:56:42 | **Nothing.** No seventh review across two more intervals; model calls stayed at 15 | 320 s watched |
| 01:56:42 | The owner writes again; Exec answers at 01:56:46 | owner |
| 01:56:46 | Daily review 7, in the same second as Exec's answer | see "Found and fixed" |

Reviews 1 and 2 started nothing because of two mistakes in my scenario, not in the product: the
first `restless commission` carried a flag that does not exist, the second a lead name the Work
graph refuses (`outreach-lead`). Both commands failed, no Work was created, and the rule counted
both reviews as having started nothing, which is the "read from Work, not from what Exec said"
rule working: the scripted Exec had claimed "I started…" both times. Review 3 ran the corrected
command.

### The daily review wake, exactly

> Daily review due. The owner has not asked for this; it is the company moving on the goals they set. Goals that need a next step:
> - Three paying clients: nothing in motion
>
> For each one, start the next step now: commission it to an accountable lead as Work under that goal. Do not ask first; the owner set these goals and this stays inside the spend limit. Only when a goal is truly blocked on the owner, ask them one prepared question instead. Then write the owner one short message in plain words, with no ids and no commands: what you started and why, and that they can tell you to undo it. If nothing needs starting, stay quiet.

### The check-in wake, exactly (first live run of #752)

> Check-in due. The owner has not written; this is you coming to them. Work since your last check-in: 0 running now, 0 open. Look at the Work, then write them one short message in plain words, with no ids and no commands:
> 1. Update, two or three lines: what landed, what is running, what is next. Leave out what they already know.
> 2. Up to three questions whose answers would most improve the next hour: a choice between options, something only they know, or one of their Inbox items that has sat too long. Number them, make each answerable in a word or two, and give your recommendation with each.
> 3. One line on what you will do in the next hour without waiting for them. Then do it in this turn: start or steer that Work.
>
> Do not ask what you can decide yourself. If there is nothing new to tell and nothing to ask, stay quiet.

The nudge read "Check-in nudge. The owner has not answered your last check-in, a while ago. …"
("a while" because the shortened interval is under an hour).

### The result wake now asks for a recommendation

The completed-result message to Exec ended "…give the owner the result and a usable link in your
final reply without waiting for another owner message, and end it with one line recommending what
comes next and why: continue (name the next step, and start it now when it serves a goal the owner
set), pivot, or stop. …". The "Continue: …" line in Exec's report is scripted.

## Found and fixed during the run

Daily review 7 was asked at 01:56:46, the same second Exec answered the returning owner. The rule
now also waits the settling time after the owner's own message or a goal change. The daemon was
rebuilt and the stack restarted on the same company:

| Time | What |
| --- | --- |
| 02:02:21 | Owner writes; Exec answers at 02:02:22 |
| 02:02:59 | Daily review, 38 s later (the settling time at this interval is 37.5 s) |
| 02:03:22 | Check-in, 60 s after Exec's answer and 23 s after the review (its settling time is 15 s) |

## No open goal

On the restarted stack the scripted Exec again started nothing: reviews at 02:02:59, 02:05:29 and
02:07:59, then none. The goal was closed at 02:11:13 (`restless goal close`). A goal change is one
of the things that would bring the review back for an open goal. From 02:11:13 to 02:16:44 (331 s,
longer than the 300 s repair sweep): daily reviews stayed at 10, check-ins at 6 and model calls at
7. This is one company watched by hand; it is not a run of `scripts/verify-ai-idle.py`, the
Sprint 109 idle check, which was not run.

## Not shown here

- A real model. Whether it starts a sensible next step, and whether its check-in questions are
  worth answering, is untested.
- The holds (spend limit, draining plane, stopped computer) and "not held while away" were not
  exercised live. The unread-owner-message hold is in the Postgres scenario; the other three are
  the same three calls the check-in uses, and nothing tests them.
- The Check-ins toggle was not opened in the browser.
- Undo: the scripted Exec only says it can be undone. Nobody asked it to undo anything.
- The last change to the code (the four holds gathered into one struct) was made after the stack
  was stopped. It is covered by the unit and Postgres tests, not by this run.

## Teardown

`restless down -c s116idle_test --destroy`, then the daemon, Vite and the mock were stopped and the
`dev172673` profile removed with its four containers, three volumes and one network. Afterwards no
container, volume, network or image matched `dev172673`, and 59 containers were running, as before.
