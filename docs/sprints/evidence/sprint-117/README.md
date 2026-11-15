# Sprint 117 observed evidence: the right questions

Recorded 14 November 2026 (times below are UTC) on branch `feat/s117-right-questions`, on one
local stack for this worktree with the scripted model. No real model was used: the stack was a
fresh dev profile with no model connection, and none was entered. Every sentence the scripted
Exec says is the scenario's text. This shows that the wakes fire, what Exec is handed, that an
answer reaches Exec and where the brief shows. It does not show that a real model asks questions
worth the owner's time. That is the sprint's own acceptance week, and it has not been run.

## Setup

- `node tools/mock-model/server.mjs --port 18617 --scenario <scenario.json> --log <calls.jsonl>`
- `RESTLESS_ANTHROPIC_BASE_URL=http://127.0.0.1:18617/v1 RESTLESS_DEV_MODEL=anthropic/claude-sonnet-5-5
  RESTLESS_DEV_CREDENTIAL_REFERENCE=env:<a placeholder> RESTLESS_DAILY_REVIEW_EVERY_SECS=120
  RESTLESS_DIRECTION_SESSION_EVERY_SECS=300 scripts/restless-dev s117q_test` (profile
  `dev172673`; existing local company and document images; `web/node_modules` borrowed from
  another worktree, so Vite refused the font files and the screenshots use fallback fonts).
  Check-ins were left at their real interval (an hour), so none fell inside the run.
- Scenario rules matched `Daily review due` (first review: two `restless commission` calls, one
  with no assumptions in its outcome and one ending `Assumptions:` … `Least sure of:` …, then a
  three-part brief; later reviews: one commission with assumptions, then a brief), the
  commissioned Work, `Completed Work result…`, `Weekly direction review due` (two questions,
  with `ownerNeed` and `ownerReplies` in the intent marker), and the owner's two answers.
- The company image holds an older `restless` CLI. That is enough here: assumptions are text in
  the Work outcome, and the daemon, which was built from this branch, reads them.
- Read from the company's own database (`events`, `messages`, `work`), the mock's call log, and
  headless Playwright against the stack's Vite server at 1440 and 390 wide.

## What happened

| Time | What | Source |
| --- | --- | --- |
| 02:56:57 | Goal "Three paying clients" set; one owner message; Exec answers | owner, 1 model call |
| 02:58:57 | **Daily review 1**, 120 s after the goal | `exec_daily_review` |
| 02:59:01, 02:59:02 | Exec commissions "Draft the one-line offer" (no assumptions) and "List twelve local prospects" (two assumptions, one key uncertainty) under the goal | `work_commissioned` |
| 02:59:04 | Exec writes the **brief** | message |
| 02:59:07 to 02:59:11 | Both pieces of Work complete; Exec reports | result notices |
| 03:00:57 | Daily review 2: one more Work with assumptions, a second brief at 03:01:03 | `exec_daily_review` |
| 03:01:57 | **Direction session 1**, 300 s after the goal was set and 60 s after the review | `exec_direction_session` |
| 03:02:01 | Exec posts two numbered questions; the first is the tap question | message |
| 03:02:57, 03:04:58 | Daily reviews 3 and 4 | |
| 03:05:16 | The owner answers the first question in words ("Clients first"); Exec acts at 03:05:17 and offers the second as the tap question | owner turn |
| 03:05:29 | **The second answer is tapped in the browser** ("Local only"); it reaches Exec as an ordinary owner message quoting the question; Exec acts at 03:05:31 | owner turn |
| 03:06:57 | **Direction session 2**, 300 s after the first. The owner never answers it | `exec_direction_session` |
| 03:07:27 | Daily review 5, 30 s after the session: the settling time, not the same scan | |
| 03:09:28 to 03:17:28 | Daily reviews 6 to 10, each commissioning Work | 6 more `work_commissioned` |
| 03:06:57 to 03:17:52 | **No third session.** 655 s watched, more than two intervals, while the company stayed busy: sessions stayed at 2, Work went from 5 to 11, model calls from 28 to 58 | `events` |

The owner last wrote at 03:05:29. Session 2 was asked after that and went unanswered, so the
rule left the owner alone although a session interval passed twice and Work was commissioned
six times in between. The owner returning was not exercised live (the unit and database tests
cover it).

### The direction session wake, exactly

> Weekly direction review due. The owner has not asked for this. Once a week the company brings them the questions about direction; the owner decides direction, not you. From the company's own records since the last one:
> Open goals:
> - Three paying clients: nothing in motion
> What the work rests on, as written when it was commissioned:
> - Write to the next three prospects (toward Three paying clients): assumes A short email beats a long one.; least sure of who reads the shared inbox.
> - List twelve local prospects (toward Three paying clients): assumes Local firms answer cold email.; assumes Twelve prospects is enough to learn from.; least sure of whether price or timing decides it.
> Finished: Write to the next three prospects; List twelve local prospects; Draft the one-line offer
> Started by daily reviews: Write to the next three prospects; List twelve local prospects; Draft the one-line offer
> Write the owner one short message in plain words, with no ids and no commands, holding at most 3 questions about direction: which goal matters most, whether to continue, pivot or stop a line of work, what the company is for, who it serves, what to give up. For each question give three short lines: what the company knows, what it is assuming, and what would change with each answer. Offer two or three answers to each and do not choose for them; you may add which way you lean in a few words. Never ask for status, never ask what you could find out yourself, and never repeat a question they have answered. Number the questions so each can be answered in a word. Make the one that matters most answerable by a tap: in your reply's intent marker put that question in "ownerNeed" and its answers in "ownerReplies", a list of two or three short strings. When the owner answers, act on it in that turn, record a changed goal, and if another of these questions is still open, offer it the same way. Fewer than 3 is fine. If there is no real direction question, stay quiet. You will not be asked again until the owner has engaged, so keep the work moving on the goals as they stand.

Every line above "Write the owner" is computed from rows, with no model. "Draft the one-line
offer" appears under Finished and Started but not under what the work rests on, because its
outcome carried no assumptions. The second session's facts listed only what had been written
since the first: the two later pieces of Work, not the first three.

### The questions as the owner saw them (scripted)

> Two questions about direction this week.
>
> 1. Which comes first: signing local clients, or building the online shop?
> What we know: twelve local prospects are listed and none has been written to.
> What I am assuming: local firms answer cold email.
> What changes: with clients first I keep outreach going and park the shop; with the shop first, outreach stops after this batch.
>
> 2. Local firms only, or anyone who fits?
> What we know: every prospect so far is in the city.
> What I am assuming: twelve prospects is enough to learn from.
> What changes: local only keeps the list small and personal; anyone who fits means a wider list and less specific emails.
>
> I lean towards clients first, but it is your call.

It arrived as one Inbox item, "The Exec needs you", with the first question and the answers
"Clients first", "Shop first", "Both, slower" as buttons in the item and in the chat
(`question-open-1440.png`, taken before the pane-width fix below). The tapped answer was stored
as:

> \> Exec: Local firms only, or anyone who fits?
>
> Local only

### The daily review wake now asks for the brief

> Daily review due. The owner has not asked for this; it is the company moving on the goals they set. Since the last review: nothing finished. Goals that need a next step:
> - Three paying clients: nothing in motion
>
> For each one, start the next step now: commission it to an accountable lead as Work under that goal. … End the outcome of what you commission with what it rests on: a line \`Assumptions:\`, up to three lines each starting \`- \`, and one line \`Least sure of: …\`. … Then write the owner today's brief: one short message in plain words, with no ids and no commands, in three parts. What moved since the last brief. What you are starting today unless they say no, and why; they can tell you to undo it. What you need from them: at most 3 things, each answerable in a word. …

(The middle is elided here; the full text is in `daily_review.rs`.)

### Missing assumptions are noted once and block nothing

The first commission's outcome had no assumptions. Its tool result, from the mock's call log,
carried: "This Work was created, but its outcome does not say what it rests on. From the next
one on, end the outcome with a line \`Assumptions:\`, up to three lines each starting \`- \`, and
one line \`Least sure of: …\`, so the company can bring them to the owner as questions about
direction. Nothing is blocked." The Work was created and completed. The second commission, which
carried them, got no note.

## The landing page

The brief said the brief row belongs in the slot `DirectionToday.svelte` holds in
`InboxList.svelte`. With nothing waiting, that list is folded away: the landing page is the
office with a "No decisions" pill over it. So the brief shows in two places, from one cached
read:

| State | What the owner sees | File |
| --- | --- | --- |
| Nothing waiting, no goal | Pill: "No goals yet. Tell Exec what you want done." | `calm-no-goal-1440.png` |
| Nothing waiting, a goal, no brief yet | Pill: "1 goal open · nothing in motion yet" | `calm-1440.png`, `calm-390.png` |
| Nothing waiting, a brief | Pill: "Today Nothing had moved since yesterday." | `brief-1440.png`, `brief-390.png` |
| The pill opened | The whole brief on a card over the office | `brief-open-1440.png` |
| Something waiting | "Today" row at the top of the Inbox list, one line, cut with an ellipsis | `list-row-1440.png` |
| After the tapped answer | The queue is clear again; the pill carries the brief | `answered-1440.png` |

Measured on every animation frame from navigation for five seconds, counting frames after the
page is revealed:

- The pill was 34 px high at 1440 and 44 px at 390 in every frame, with the calm line and with
  a brief. Its top read 74 then 73: that is its existing slide-in, not a change of size.
- The list row was 36 px high in every frame (303 frames at 1440), the list under it stayed at
  one position, and the layout-shift score was 0. Opening the brief left the row at 36 px.
- Layout shift was 0 in every run but one: 0.0014 on the 1440 run with a brief in the pill.
  What moved was not identified.
- `web/scripts/capture-surfaces.mjs` on `attention` and `attention-all` at 1440 and 390:
  nothing clipped, nothing scrolling sideways, no raw ids or commands. Its console errors
  were the five refused font files.

## Found and fixed during the run

1. **"No decisions" is not in the Inbox list.** It is the pill on the company page. The first
   build put the brief only in the list row, where a clear queue hides it. The pill now says it.
2. **The row widened the list pane.** A long one-line brief pushed the pane under the item pane
   (visible in `question-open-1440.png`: the "Inbox" heading is cut). The row's text now has
   zero intrinsic width and grows to fit; `list-row-1440.png` is after the fix.

## Found and not fixed

- **A direction question's Inbox item lasts only until Exec next writes to the owner.** The
  Inbox reads an open question from the latest message of a conversation
  (`latest_human_agent_messages`). At 03:03:00 the next daily review's brief replaced the
  question as the latest message, and the item and its buttons went away; the question stayed
  in the chat. With real intervals the brief comes a day later, and any result report does the
  same sooner. This is how every chat question already behaves. Keeping an unanswered question
  open until the owner writes is a change to Attention and to the rail, and was not made here.
- The Inbox item for a chat question says "Work is paused until you answer". For a direction
  question that is wrong: the work carries on. Existing wording, not changed.
- On a phone the existing brand card's button is clipped ("et up with Exe"). Not from this
  sprint.
- In the wake message an assumption that ends in a full stop reads "…a long one.; least sure
  of…". Only Exec reads it.

## Not shown here

- A real model: whether its questions are about direction, whether it writes assumptions worth
  reading, whether its brief is useful.
- The notification intent. The delivery endpoint answers only on a hosted plane with a delivery
  token; a local stack has neither. A unit test covers the intent's kind, identity, link and
  when it is withdrawn. Whether Cloud's email reads well under the `personal_attention` kind
  was not seen.
- The holds (spend limit, draining plane, stopped computer, owner away) and the Check-ins
  toggle switching the session off were not exercised live.
- The owner coming back after an unanswered session.
- The list row at phone width after the pane-width fix, a member's view, dark mode,
  `verify-motion.mjs` and the zero-jank gate (`scripts/multiplayer-smoke --jank`).
- A check-in, a daily review and a session all due in one scan. The run showed a session and a
  review due in the same second being asked 30 s apart.

## Teardown

Recorded in the pull request, since the machine was restarted between the run and the cleanup.
