# Sprint 120 observed evidence (Core): an owner can get in, and it can be measured

Recorded 16 November 2026 AEDT (times below are UTC, 16 November 01:36 to 02:01) on branch
`feat/s120-owners-can-get-in`, on one local stack for this worktree with the scripted model. No
real model was used: the dev profile's only model connection is the scripted one, no provider
key is in the environment, and none was entered. Every sentence an agent says below is the
scenario's text.

**What a scripted model proves here.** The plumbing: the first-run screens and what they say,
that one tap sets a goal and reaches Exec, that the daily review fires and what Exec is handed,
that stopped Work is noticed and who is told, that two send-backs from the Work page make a
lesson and the lesson is in the next job's context, and that the attention report counts what
the owner did. **What it does not prove.** That a real Exec interviews well, starts a sensible
first step from a starter, writes assumptions worth reading, or heeds a lesson; that a real lead
repairs stopped Work; or that a non-technical person understands the screens. Nobody but the
script and the driving browser took part.

Everything the owner did was done in the web UI by headless Playwright (hidden browser panes
freeze Svelte on this machine). Work for the second and third smoke tests was added from the
host with the CLI as the lead or Exec, as in Sprints 118 and 119; each place is marked.

## Setup

- `node tools/mock-model/server.mjs --port 18620 --scenario <scenario.json> --log <calls.jsonl>`
- `RESTLESS_ANTHROPIC_BASE_URL=http://127.0.0.1:18620/v1 RESTLESS_DEV_MODEL=anthropic/claude-sonnet-5-5
  RESTLESS_DEV_CREDENTIAL_REFERENCE=env:<a placeholder> RESTLESS_HEALTH_SWEEP_EVERY_SECS=20
  RESTLESS_STALLED_AFTER_SECS=90 RESTLESS_DAILY_REVIEW_EVERY_SECS=150
  RESTLESS_DIRECTION_SESSION_EVERY_SECS=3600 scripts/restless-dev <company>` (profile
  `dev172673`; existing company and document images; `web/node_modules` borrowed from another
  worktree, so Vite refuses the font files and the screenshots use fallback fonts).
- What the shortened intervals stand for:

  | Real | In this run |
  | --- | --- |
  | A daily review of open goals every 24 hours | 150 s |
  | Health sweep every 10 minutes | 20 s |
  | Stopped Work is told to its lead, and gets an owner card, after 24 hours | 90 s |
  | Weekly direction session, hourly check-in | left long; none fell in the run |

- Three test companies. The dev script writes a stock purpose into a new company; a company made
  with "Start a company" has none. So each company's purpose was blanked from the host
  (`restless company set mission ''`) before its first screen, which is the state the button
  leaves. "Start a company" itself was not pressed: it would make a company whose name does not
  end in `_test`.

  | Company | Used for | Daemon build |
  | --- | --- | --- |
  | `s120a_test` | first run at 1440, then the three smoke tests | second build from 01:43 (see "Found and fixed") |
  | `s120b_test` | first run at 390, the skip path | final |
  | `s120c_test` | first run at 1440 once more, for the word probe | final |

- Read from the company's own database, the scripted model's call log, files in the company
  computer, `restless work outcome`, and `restless org diary` (the attention report, part F).

## The first run, as a brand-new owner

Final build, `s120c_test`, 1440 wide. Owner actions are in bold.

| Time | What the owner saw or did | File |
| --- | --- | --- |
| 02:00:12 | "Hi, I'm Exec… What should we call it?" **Typed a name, pressed Continue** | `first-run-1-name-1440.png` |
| 02:00:14 | "So, tell me about this idea of yours." **Typed one sentence** | `first-run-2-idea-1440.png` |
| 02:00:23 | Exec's question as a card with three answers and "Something else…". **Tapped "Less paperwork"** | `first-run-3-question-1440.png` |
| 02:00:29 | The read-back, and under it "Want me to start on one of these first?" with three first jobs, then "Let's do it". **Tapped "Reply to every enquiry within a day"** | `first-run-4-readback-offer-1440.png` |
| 02:00:30 | The goal exists, set by the owner through the ordinary goal call | `events` (`goal_created`, by `owner`) |
| 02:00:35 | The tour opens over the company; the chat is in the rail | `first-run-5-after-tap-1440.png` |
| 02:00:36 | Exec starts the first step as Work under that goal, with a lead | `events` (`work_commissioned`) |
| 02:00:45 | The Work is done; Exec says so in the chat | `first-run-7-landing-1440.png` |
| | Work, then the item, then "Open the full brief": the result line | `first-run-8-work-1440.png`, `first-run-9-result-1440.png` |

Four owner actions, 33 seconds from the name to the first result. The 16 seconds from the tap
to the result are the script's speed and say nothing about a real one.

What the tap sent, exactly:

> Let's do it. Start with this: Reply to every enquiry within a day. It is done when every
> customer enquiry has a reply drafted for me to approve within a day. First, draft replies to
> this week's enquiries for me to approve.

and the goal it made (title on the first line, as the owner goal call takes it):

```text
Reply to every enquiry within a day
Done when: every customer enquiry has a reply drafted for me to approve within a day.
```

The scripted Exec looked the goal up by its title and ran `restless commission … --goal <id>`
with an outcome ending in `Assumptions:`. A real Exec is told the same thing only by the
owner's sentence above and its standing rules; whether it then starts Work was not tried. If it
does not, the goal has nothing in motion and the next daily review asks Exec to start it (the
database test `a_starter_outcome_is_an_ordinary_goal_that_the_daily_review_moves_on`).

**Skipping the offer** (`s120b_test`, 390 wide): one tap on "Let's do it" with none chosen. The
read-back was saved, no goal was added, Exec answered, and the owner landed in Exec's chat
(`first-run-skip-390.png`). This company's first pass was spent by a slip in the driving script,
which clicked a first job instead of scrolling. Its saved purpose was blanked again from the
host and Exec was asked to read it back again, so the skip was pressed on a second read-back.
The button and what it sends are the ones a first read-back has.

**At phone width** (`first-run-1-name-390.png`, `first-run-3-question-390.png`,
`first-run-4-readback-offer-390.png`): each first job is one line, 44 px high. The offer and
"Let's do it" sit below the read-back, so on a phone they are below the fold until scrolled.

### Words

The seven words (charter, Vault, API key, compute, revision, harness, runtime identity) were
looked for in the visible text of every screen above: none on the home, name, idea, question,
read-back, after-tap, tour, Inbox, Work list and Library screens at 1440 on the final build,
and none on the four phone screens. **One is left:** a piece of Work opens beside its lead's
thread in People, and that thread shows Exec's whole note to the lead, which says "team
charter". So "charter" is on the screen where the owner reads their first result
(`first-run-8-work-1440.png`, `first-run-9-result-1440.png`). `web/scripts/capture-surfaces.mjs`
now carries the same probe for its `welcome`, `welcome-tour` and `attention` surfaces:

| Company | Surfaces | 1440 | 390 |
| --- | --- | --- | --- |
| `s120b_test`, before anything was typed | `welcome` | none | none |
| `s120c_test`, after the first run | `welcome-tour`, `attention` | none | none |
| `s120a_test`, first run on the earlier build | `welcome-tour`, `attention` | **charter** | none |
| `s120b_test`, after the unscripted first job | `welcome-tour`, `attention` | **Charter** | none |

The two hits are the probe working. The first is the note in the owner's chat that Exec
appointed a lead ("You now lead Front of house. Read the current team charter…"), written by
the earlier build; the line now starts with what the lead does, and the third company shows it
without the word. The second is the scripted model answering a message it had no rule for by
echoing Exec's own context into the chat, which a real model does not do. Its only other
findings on every surface were the refused font files.

Words still on the path that a café owner may not know, seen in the screenshots and left alone:

- "Claude Sonnet 5.5" on the composer, from the first question on.
- "Ask, redirect, or make a judgement…" in the chat box.
- On a piece of Work: "Done · unverified", "Estimate", "Evidence", "Run details", "Open the
  full brief".
- Exec's whole note to the lead, shown beside any piece of that lead's Work: "team charter",
  "Staff", "seam", "blockers". It is written for the lead, who needs the exact term, and was not
  reworded. The probe does not cover People or Work.
- The Company step of the tour now reads "what it is for, its brand and voice, spending limits
  and the passwords it keeps"; the Company pages themselves keep their names (Charter, Vault).

### Layout

On the read-back screen after a reload, sampled every 50 ms for 3 s: the offer was 131 px high
at 1440 and 167 px at 390 in every sample, at one position, and the layout-shift score was
0.0000 at both widths. The row with "Let's do it" was 44 px at 390 throughout. While the
interview is live the thread moves as Exec's reply and its question card arrive (a score of
0.23 at 390 from page load to the card); that is the chat growing, and until this branch the
card was not shown at all (below).

## Three smoke tests (`ARCHITECTURE.md` §10.6)

All on `s120a_test` after its first run. The attention report was run over each window.

### 1. A vague mission becomes a bounded first step, started without being asked

| Time | What | Source |
| --- | --- | --- |
| 01:43:26 | **The owner types: "I'd like the café to be busier on weekdays."** | owner |
| 01:43:30 | Exec offers a goal with a finish line; tap answers "Make it a goal", "Just this once" (`smoke-1-goal-offer-1440.png`). **The owner taps "Make it a goal"** | owner |
| 01:43:33 | Exec records the goal and starts nothing | `events` (`goal_created`, by `exec`) |
| 01:43:35 | The owner's last action | |
| 01:45:33 | **A day later:** the daily review lists the goal with nothing in motion | `events` (`exec_daily_review`) |
| 01:45:37 | Exec starts "Find out which weekday hours are quiet" under it | `events` (`work_commissioned`) |
| 01:45:44 | Done | `work` |
| 01:45:56 | The brief is in the chat and on the Inbox pill (`smoke-1-brief-1440.png`) | browser |

The step's outcome record (`restless work outcome`, no model):

```text
Find out which weekday hours are quiet
  Expected: S120-WEEKDAY: one page on which weekday hours are quietest, from the last four weeks of till records, saved at /company/outputs/quiet-hours.md Assumed: The till records the hour of each sale. Four weeks is enough to see the pattern.
  What happened: The quiet weekday hours are written up from four weeks of till records. (1 attempt)
  Cost: $0.02
  Verdict: Finished, not reviewed
```

The scripted Exec was written to record the goal and wait, so that the review's start could be
seen apart from the owner's tap. That the wish became one bounded step and not a plan is the
script's choice of words.

Attention report, 01:43:17 to 01:46:00: **2** times stepped in (1 answered the company, 1 not
told by the records: the wish itself), 0 chasing, no accepted result.

### 2. Stopped Work is noticed, reaches the owner, and is repaired

A worker is given "Get the supplier price list" (from the host, as its lead). The supplier
never answers.

| Time | What | Source |
| --- | --- | --- |
| 01:46:19 | The worker's Attempt ends blocked | `work_attempts` |
| 01:48:03 | 90 s on: the lead is told once, and the owner has one card | `events` (`work_stalled_notice`) |
| 01:48:10 | The owner's Inbox: "Stuck since 16 Nov: Get the supplier price list", with "Resume it" and "Stop it for good" (`smoke-2-card-1440.png`) | browser |
| 01:48:48 | The lead gives it to a second worker with a different way in, and resumes it | `work_reassigned`, `work_repaired` |
| 01:48:54 | Finished | `work_attempts` |
| 01:49:07 | The card is gone (`smoke-2-after-1440.png`) | browser |

The owner did nothing. The scripted lead waited 40 seconds before repairing so the card could be
photographed; in Sprint 118's run it was there for four seconds.

Attention report, 01:46:14 to 01:49:08: **0** times stepped in.

### 3. Sent back, the same mistake again, and the lesson in a later job

Two priced documents (added from the host as the lead; the scripted worker leaves GST out).

| Time | What | Source |
| --- | --- | --- |
| 01:54:18 | **The owner opens "Price the lunch menu" in Work, presses "Request changes…" and writes "Prices are shown without GST."** (`smoke-3-send-back-1440.png`) | owner |
| 01:54:19 | One observation; no lesson | `events` (`work_observation`), `company_lessons` (0) |
| 01:54:29 | Redone with GST | the file |
| 01:54:45 | **The owner sends "Write the catering quote" back: "The quote shows prices without GST."** (`smoke-3-second-sent-1440.png`) | owner |
| 01:54:45 | The second of its kind: a lesson, for the team, naming both observations | `events` (`lesson_recorded`, by `daemon`) |
| 01:54:55 | Redone with GST | the file |
| 01:54:56 | A later job: Exec gives the lead "Price the dinner menu" (from the host, as Exec) | CLI |
| 01:55:05 | Priced with GST on the first attempt; nothing sent back | the file, `work_attempts` |

The lesson, as the owner can read it under Company (`smoke-3-lesson-1440.png`):

> Work has been sent back twice for the same reason: "The quote shows prices without GST". Check
> for it before handing work in.

and in the lead's context for the dinner menu, 3,360 bytes ahead of its brief (from the
scripted model's call log):

```text
# What the company has learned [work went wrong this way more than once; follow each one unless the owner has ruled otherwise]
- Work has been sent back twice for the same reason: "The quote shows prices without GST". Check for it before handing work in. (team Front of house, lesson ec3d3267)
```

As in Sprint 119, the scripted lead prices with GST when those words are in its context and
without otherwise. That rule is the run's, not a model's judgement.

Attention report, 01:49:15 to 01:55:16: **2** times stepped in, both "sent work back or
corrected it", 0 chasing, no accepted result.

## The attention report over the whole session

`restless org diary -c s120a_test --from 2026-11-16T01:36:37Z`, exactly:

```text
Owner attention, 16 Nov 01:36 to 16 Nov 01:55 (UTC)

Results the owner accepted: 0
The owner stepped in: 9 times
No result was accepted in this time, so there is no number for each result.
Of those, chasing for status: 1 time. The company should never need this.

By kind:
  Accepted a result: 0
  Sent work back or corrected it: 2
  Answered the company: 3
  Approved or refused an outside action: 0
  Did a step only a person can do: 0
  Gave new direction: 1
  Stepped into the work: 0
  Chased for status: 1
  Not told by the records: 2

Each time, oldest first:
  16 Nov 01:36  Not told by the records: I run a small café near the station and I want more regulars.
  16 Nov 01:38  Answered the company: Less paperwork
  16 Nov 01:39  Gave new direction: Set a goal: Reply to every enquiry within a day
  16 Nov 01:39  Answered the company: Let’s do it. Start with this: Reply to every enquiry within a day. It is done when every customer enquiry has a reply dr…
  16 Nov 01:43  Not told by the records: I’d like the café to be busier on weekdays.
  16 Nov 01:43  Answered the company: Make it a goal
  16 Nov 01:54  Sent work back or corrected it: Sent back: Price the lunch menu
  16 Nov 01:54  Sent work back or corrected it: Sent back: Write the catering quote
  16 Nov 01:55  Chased for status: progress?
```

The last line is one "progress?" typed into the chat after the smoke tests, to see it counted.
The owner API (`GET /companies/{company}/company/attention-report`) returned the same totals,
and a start it cannot read ("yesterday") was refused with 400.

Read it knowing:

- **No number came out.** None of the scripted Work asked the owner to review its result, so no
  result was accepted and the headline reads "no number". The arithmetic is shown in the
  database test (two accepted results, ten interventions, 5.00 for each; chasing 0.50).
- One tap on a first job counts twice: the goal it set and the sentence it sent.
- The owner's first sentence in setup answers "tell me about this idea", but no record says
  so, so it is "not told by the records". So is the vague wish, which was new direction.
- Naming the company, and looking without doing, are not counted. Nothing records reading.

### What the records can and cannot tell apart

Against `evaluation-dogfood` §6.3:

| §6.3 kind | In the report |
| --- | --- |
| Product/taste judgment | "Accepted a result"; "Sent work back or corrected it" |
| Capital or authority decision | "Approved or refused an outside action" (Authority approvals, payment confirmations) |
| External relationship/human-only action | "Did a step only a person can do" |
| Mandate/strategy judgment | "Gave new direction" (a goal set, changed, closed or reopened by the owner); most of "Answered the company" |
| Agent chasing/status request | "Chased for status" |
| Correcting false or unsupported claim | **Not told apart** from any other send-back or correction |
| Ordinary task decomposition | **Not told apart**: "Stepped into the work" (paused, stopped, restarted, reassigned) or an unprompted message |
| Repairing harness failure | **Not told apart**: the same two places |

An unprompted message is never guessed into a kind from its words. The one exception is the
closed list of nudges ("progress?", "try again", …) the team review already uses.

## The three-day away test (the owner's to run)

1. Pick one of your own companies with at least one open goal and work under way. Write down
   the time.
2. For three days, open it only when a notification asks you to, and do whatever is needed in
   the product as you normally would. Do not keep notes: the records are the diary.
3. Afterwards run `restless org diary -c <company> --from <start> --to <end>` (a day as
   `2026-11-16`, or a time as `2026-11-16T09:30:00Z`; both are UTC) and save what it prints.
4. For each line under "Not told by the records" and "Stepped into the work", write beside it
   which of the eight kinds it was. That is the only part done by hand.
5. The number is the one it prints for each accepted result, with chasing apart. If it says
   there is no number, no result was put to you for review in those days, which is itself the
   finding.
6. Note what it cannot see: time spent reading, anything done on Cloud's own pages, and
   anything said to the company outside the product.

## Found and fixed during the run

1. **The question card never showed during the interview.** On the first company the owner's
   first answer (01:36:43) was followed by Exec's words and nothing else: no card, no tap
   answers, no thinking dots. Since the commit that offers the owner's own
   ChatGPT or Claude account in setup (`b38cf5b7`, on main), that offer sat in the middle of
   the chain that chooses what to show under the thread, and hid the question card, the
   thinking dots and every "I couldn't reply" line for the whole interview. It is now its own
   block. The rest of the first company's walk, and both later ones, are after the fix.
2. **"charter" in the owner's chat.** Above.
3. **The report counted a send-back three times.** A review or a send-back leaves the feedback
   as messages in the owner's name to the worker and its lead; those were counted as things the
   owner typed. Found by the database test, before the run.
4. **The report would have missed every chase typed in the browser.** The cockpit adds a note
   about the page the owner was on after their words; "progress?" arrives as 94 characters. The
   report strips the note before reading the words.

## Found and not fixed

- **The team review does not see what this run did.** After the two send-backs and the
  "progress?", `restless org scorecard` still read `sent_back: 0, nudges: 0`. It reads only
  messages of 60 characters or fewer (the browser's note makes every message longer), and
  counts as sent back only a declined review, not Work sent back from its page. Sprint 119's
  rule that a team review rides a daily review "when the owner has given enough new signal"
  rests on those numbers. Not this sprint's code; not changed.
- **What Exec is told to do when the owner confirms is gone by the time they confirm.** The
  welcome instructions (make the goals, start the first Work) are in Exec's prompt only while
  the company has no saved purpose, and the screen saves it before sending "Let's do it". Read
  from the code; a real model was not tried. The first-job sentence carries its own
  instruction for this reason.
- The stuck-Work card is headed "Setup" and says "Work is paused until you answer".
- After the first job finished, the Inbox pill read "1 goal open · nothing in motion yet".
- The first result shows as one line with "Evidence: nothing is linked yet". The scripted lead
  wrote a file and linked nothing; whether a real lead links it was not tried.
- One web unit test fails on this branch and on main alike (`inline-cards.test.ts`, the
  `open-integrations` action).

## Not shown here

- A real model, a real person, and Cloud. The admit command, the restore drill and a real
  person getting in are not Core's part of this sprint.
- An accepted result, so no number per accepted result from a live run.
- The model step of setup ("Exec needs a model"), where the key wording changed: this profile
  has a model, so the step never showed. Nor the Inbox items whose wording changed (a key
  request, the review actions): none was raised.
- A member's view, dark mode, `verify-motion.mjs` and the zero-jank gate.
- Smoke tests from §10.6 other than these three (external purchase, human takeover of the
  company computer, and the rest).

## Teardown

Recorded in the pull request.
