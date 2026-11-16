# Sprint 118 observed evidence: it notices and repairs itself

Recorded 16 November 2026 AEDT (times below are UTC, 15 November 23:07 to 23:49) on branch
`feat/s118-notices-and-repairs`, on one local stack for this worktree with the scripted model.
No real model was used: the stack was a fresh dev profile whose only model connection is the
scripted one, no provider key is in the environment, and none was entered. Every sentence an
agent says below is the scenario's text. This shows that the sweeps fire, who is told, what they
are handed, that a notice changes nothing by itself, and that a question reaches the owner while
its Work goes on. It does not show that a real lead repairs well when told, or that a real
worker uses the question marker. Neither was tried.

## Setup

- `node tools/mock-model/server.mjs --port 18618 --scenario <scenario.json> --log <calls.jsonl>`
- `RESTLESS_ANTHROPIC_BASE_URL=http://127.0.0.1:18618/v1 RESTLESS_DEV_MODEL=anthropic/claude-sonnet-5-5
  RESTLESS_DEV_CREDENTIAL_REFERENCE=env:<a placeholder> RESTLESS_HEALTH_SWEEP_EVERY_SECS=20
  RESTLESS_STALLED_AFTER_SECS=90 RESTLESS_CHECKIN_EVERY_SECS=150 RESTLESS_CHECKIN_NUDGE_AFTER_SECS=450
  scripts/restless-dev s118_test` (profile `dev172673`; existing company and document images;
  `web/node_modules` borrowed from another worktree).
- What the shortened intervals stand for:

  | Real | In this run |
  | --- | --- |
  | Health sweep every 10 minutes | 20 s |
  | A stopped Work with jobs behind it is raised after 30 minutes | 60 s |
  | A lead may hold a question for the owner for 1 hour | 120 s |
  | Stopped Work is told to its lead, and gets an owner card, after 24 hours | 90 s |
  | A check-in an hour after Exec last wrote to an engaged owner | 150 s |

- The company: one goal ("A new menu, priced and printed"), a Menu team (lead `menu-ops`, shown
  to people as Aang; workers `menu-build` and `menu-check`) and, from the second case, a Web team
  (lead `web-ops`). Work was added from the host with the CLI as the lead or Exec.
- The company image holds an older `restless` CLI. That is enough: every command the scripted
  agents ran already existed, and the question marker is read by the daemon, which was built
  from this branch.
- Read from the company's own database (`events`, `messages`, `work`, `work_attempts`,
  `owner_handoffs`), the mock's call log, and `restless attention --summary` for the owner's
  Inbox. No browser was opened.
- The daemon was rebuilt three times during the session as the run found things (below). The
  healthy window and the three cases are from the third build; the last case was run again on
  the fourth. After that only wording in the operating rules changed.

## A healthy company with open Work: nothing

Two pieces of Work open: one blocked and parked until a time by its own worker, one waiting
behind it. No owner message, so no check-in was owed either.

| Window | Model calls | Messages | Health notices | Stalled notices | Check-ins |
| --- | --- | --- | --- | --- | --- |
| 23:20:42 to 23:26:24 (342 s, a sweep every 20 s) | 0 | 0 | 0 | 0 | 0 |

The first build showed the same over 23:09:24 to 23:15:06. A Work parked until a time with one
job behind it is not a shared blocker (someone already knows), and one block is not a pattern.

## 1. A stalled worker

A worker is given "Get the supplier price list". The supplier never answers.

| Time | What | Source |
| --- | --- | --- |
| 23:27:03 | Work added for `menu-build` | CLI, as its lead |
| 23:27:09 | Its Attempt ends blocked: "The supplier has not sent the price list, and nothing can be priced without it." | `work_attempts` |
| 23:27:09 | Its lead is told at once (the existing exception notice) and does nothing useful | 1 model call |
| 23:28:39 | 90 s on: **the lead is told once**, and the owner's Inbox holds one card | `work_stalled_notice` |
| 23:28:43 | The lead gives the Work to `menu-check` with a different way in, and resumes it | `work_reassigned`, `work_repaired` |
| 23:28:48 | Finished by the second worker; the owner's card is gone | `work_attempts` |

The notice, exactly (sent to `menu-ops`, the lead, not to the stuck worker):

> Work 2d3147ee-… ("Get the supplier price list") has been blocked since 15 Nov 23:27 UTC and nobody has picked it up: The supplier has not sent the price list, and nothing can be priced without it.. The owner now has a card for it. Fix the cause and run \`restless work resume --work 2d3147ee-… --reason <what changed>\`; if only the owner can unblock it, tell Exec exactly what is needed; if it is no longer wanted, abandon it.

What the owner would see, from 23:28:39 until the Work was resumed four seconds later:

> blocker · Stuck since 15 Nov: Get the supplier price list
> Recommendation: Once the cause is fixed, resume it; if it is no longer wanted, stop it for good. Either is on the Work's page, or ask its lead in chat.

This is the stalled-Work notice that already existed, not a new signal. What this sprint
changed in it is who it goes to (see "Found and fixed"). With real intervals the lead's first
word is immediate and this second one comes after a day, not an hour. The worker here was
stopped, not hung: the silence and crash watchdogs were not exercised.

## 2. Duplicate work

Two leads are each given "List twelve local prospects" under the one goal, both waiting behind
the parked Work so neither has started.

| Time | What | Source |
| --- | --- | --- |
| 23:30:01 | The same job added for `menu-ops` and for `web-ops` | CLI, as each lead |
| 23:30:20 | The sweep raises it with Exec: two leads, so nobody below Exec answers for both | `health_signal` |
| 23:30:24 | Exec stops the second one | `work_abandoned` |
| 23:30:26 | Exec tells the owner which it kept | message |

The notice, exactly:

> Health notice: Work 4f63d1b2-… ("List twelve local prospects", with menu-ops) and Work 8140bca8-… ("List twelve local prospects", with web-ops) are both open under the same goal and read like the same job. If they are, keep the one further along, fold in what the others have and stop the others for good, and say which you kept; if they are deliberately separate, carry on. Nothing has been paused or changed, and nothing waits on this notice. Check it and make the smallest repair inside your own outcome; if nothing is wrong, carry on. You will not be told again unless it changes.

What the owner was sent (scripted; it names the two leads by id, which a real Exec must not):

> Two teams had each been given the same job: listing twelve local prospects. I kept the one with menu-ops and stopped the one with web-ops. Nothing else changes.

Raised once. Between the notice and Exec's turn both jobs stayed as they were. One Exec turn,
two model calls.

## 3. A manager sitting on a question for the owner

Two workers have a question only the owner can answer.

- `menu-build` is briefed the old way: "Do not guess which printer to use: stop and report
  back." It stops and hands the question to its lead. The lead does nothing with it.
- `menu-check` has a question about tax. It picks the likely answer, finishes, and writes the
  question on its turn marker: `"ownerQuestion":"Should the menu show prices with or without
  tax?","assumed":"with tax, as on the old menu"`.

| Time | What | Source |
| --- | --- | --- |
| 23:30:52 | "Print the menu" added for `menu-build` | CLI |
| 23:30:57 | Its Attempt ends blocked; the question is a judgement waiting on its lead; the Work is blocked | `owner_handoffs` |
| 23:30:58 | The lead is told at once (existing notices) and does nothing useful | 1 model call |
| 23:31:03 | **The second worker's question is recorded from its marker**; its Work finishes a second later | `owner_question` |
| 23:31:25 | The owner says hello, so check-ins begin. Nothing more from them until asked | owner |
| 23:33:01 | 124 s on: **the held question is recorded for the owner, and the lead is told once** | `owner_question`, `health_signal` |
| 23:33:05 | The lead answers it on a stated assumption; the Work is released | `owner_handoffs` |
| 23:33:11 | "Print the menu" finishes on that assumption | `work_attempts` |
| 23:35:46 | **The next check-in carries both questions to Exec** | `exec_checkin` |
| 23:35:53 | Exec puts them to the owner, numbered, each with what the work is assuming | message |
| 23:37:15 | The owner answers the first in two words ("Cheaper online") | owner |
| 23:37:19 | **Exec passes the answer to the lead**, naming the Work; the lead reads it | message |
| 23:39:51 | The next check-in carries only the question still unanswered | `exec_checkin` |

Nothing was blocked by a question. The Work that stopped was stopped by its own brief, for
2 minutes 8 seconds, until the notice told its lead to let it go.

The notice to the lead, exactly:

> Health notice: You have held a question for the owner on Work 70f58439-… ("Print the menu") since 15 Nov 23:30 UTC: Which printer do we use: the one near the shop, or the cheaper one online? It now goes to the owner at Exec's next check-in, so you do not need to pass it on. Do not keep the Work waiting for the answer. If you can settle it yourself, do. Otherwise choose the most likely answer and resolve the handoff with it as a stated assumption (\`restless work resolve-handoff --handoff c95d2105-… --state resolved --resolution 'Assumed: <the answer you chose>'\`), so the Work carries on wherever the step can be undone. Exec tells you when the owner answers. Nothing has been paused or changed, and nothing waits on this notice. Check it and make the smallest repair inside your own outcome; if nothing is wrong, carry on. You will not be told again unless it changes.

What the check-in added for Exec, exactly, after its usual instructions:

> Questions from the team for the owner. Each takes one of your three question slots, ahead of your own. The work has not stopped: it is carrying on with the answer shown, so say that too.
> - From Aang (\`menu-ops\`) on "Price the menu": Should the menu show prices with or without tax? Meanwhile assuming: with tax, as on the old menu
> - From Aang (\`menu-ops\`) on "Print the menu": Which printer do we use: the one near the shop, or the cheaper one online? No answer is being assumed meanwhile.
> Put each to the owner in plain words with what they need to answer it in a word or two. When they answer one, tell its lead in that same turn with \`restless message --to <the id shown> '<the Work's title>: the owner says …'\`, so the work changes course if the assumption was wrong. Start with the title exactly as shown: that is what settles the question. If they wave one off, tell its lead that instead.

What the owner was sent (scripted):

> Quick check-in.
>
> The supplier prices are in and the menu is priced. The print order is drafted.
>
> 1. Which printer do we use: the one near the shop, or the cheaper one online? The team is going ahead with the one near the shop for now, because the proof can be checked in person. It can still be changed before the order is placed.
> 2. Should the menu show prices with or without tax? It is priced with tax for now, as on the old menu.
>
> In the next hour I am keeping the print order moving.

In the Inbox it is the existing chat-question item (read with the CLI, so the answer buttons
the scenario offered were not seen):

> conversation · The Exec needs you
> Requested: Should the menu show prices with or without tax?

(read at 23:39:02, after the first answer, when the open question was the second one).

The answer going back down (Exec to the lead, 23:37:19):

> Print the menu: the owner chose the online printer, not the one near the shop.

### The same case again on the last build

The run above showed "No answer is being assumed meanwhile" for a question whose lead had
already answered it on an assumption. That was fixed and the case run again with a new
question, on the build that has every change to behaviour in this pull request.

| Time | What |
| --- | --- |
| 23:43:28 | "Plan the specials board" added, briefed to stop and report back |
| 23:43:35 | The worker stops; its question waits on its lead |
| 23:43:45 | The owner answers the question still open from before ("With tax") |
| 23:43:52 | Exec passes it to the lead, naming "Price the menu"; that question is settled |
| 23:45:49 | 134 s on: the new question is recorded for the owner and the lead is told once |
| 23:45:59 | The Work finishes on the lead's assumption |
| 23:48:34 | The next check-in carries the one open question, with the assumption |
| 23:48:44 | Exec puts it to the owner; the Inbox shows it |

The line the check-in carried, exactly:

> - From Aang (\`menu-ops\`) on "Plan the specials board": A chalk board or a printed card for the daily specials? Meanwhile assuming: a chalk board, because the specials change every day.

Across both runs three health problems were raised (one duplicate, two held questions), each
once, and three questions were recorded, each once.

## What each automatic wake cost here

| Wake | Model calls in this run | Notes |
| --- | --- | --- |
| Healthy company, 342 s of sweeps | 0 | |
| Stalled-Work notice to the lead | 3 | one lead turn: two commands and a reply |
| Duplicate work to Exec | 2 | one Exec turn: one command and a reply |
| Held question to the lead | 2 | one lead turn: one command and a reply |
| A question travelling to the owner | 0 of its own | it rode a check-in that was owed anyway |
| Passing the answer down | 0 of its own | one command in Exec's own answer to the owner; then one lead turn to read it |

## Found and fixed during the run

1. **The stalled-Work notice went to the stuck worker, not its lead.** `stalled_work` called
   the Work's owner its lead. For Work a worker owns, that told the worker it was stuck. It now
   goes to the lead of the owner's team (the owner itself when it has no team). The first case
   above is after this fix.
2. **Answering one of a lead's two questions settled both.** A question was settled by any
   message from Exec to its lead after the owner wrote. It is now settled only by one that
   names the Work, which is what the check-in tells Exec to write. The 23:39:51 check-in above,
   carrying only the tax question, is after this fix.
3. **A held question travelled without its lead's assumption** (above). A judgement its holder
   settles for real, rather than on an assumption, is also no longer carried to the owner.
4. **Many copies of one job would have been one notice per pair.** Not seen in the run; found
   reading the rule. Matching Work is now one group and one notice.
5. A sentence dropped from the operating rules to make room was one a test asserts. It is back,
   and two other passages say the same in fewer words.

## Found and not fixed

- **With real intervals the question a lead sits on reaches the owner in up to two hours, not
  one**: an hour with the lead, then the next check-in, and a result report from Exec in
  between moves that check-in on. A worker's own question waits only for the check-in. In this
  run both took about five minutes against a 150 s check-in.
- **A question rides check-ins and nothing else.** A check-in never posts into silence, so a
  question waits for an owner who has gone quiet, and with check-ins switched off it is never
  carried. It stays recorded. It is not an Inbox item of its own.
- The Inbox item for the question is the chat-question item, which lasts only until Exec next
  writes to the owner (found in Sprint 117). The daemon keeps carrying an unanswered question
  into the next check-ins, three at most.
- The answer goes back down only if Exec does what the check-in tells it. Nothing checks that
  it did, beyond keeping the question open until a message naming the Work reaches its lead.
- The stalled-Work notice reads "…without it.. The owner now has a card" when the block's own
  words end in a full stop. Existing wording.
- The lead's reply to a notice is not a message anyone sees; only what it does is recorded.

## Not shown here

- A real model, anywhere.
- A review loop, one blocker holding several jobs, the same failure twice, and cost rising with
  no result were not injected on the stack. The first three are in the database scenario tests;
  cost is in unit tests only, with a made-up ledger (the scripted model's turns were not
  checked for a charge).
- The signals that go to Exec for the whole company (spend with nothing completed, one effect
  class failing, a goal whose team is gone) were not raised live.
- The holds (spend limit, draining plane, stopped computer), the owner being away, the day's
  bound of twelve, and a notice for Exec waiting out a check-in were not exercised on the
  stack. The bound and the wait behind an unread owner message are in the database tests.
- A hung or crashed worker. Those watchdogs were not touched.
- The browser: the Inbox was read with the CLI, and no page was opened.
- What happens at rollout, when every company's existing problems are seen for the first time.

## Teardown

Recorded in the pull request.
