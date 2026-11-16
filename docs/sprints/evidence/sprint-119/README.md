# Sprint 119 observed evidence: it learns

Recorded 16 November 2026 AEDT (times below are UTC, 16 November 00:37 to 00:55) on branch
`feat/s119-it-learns`, on one local stack for this worktree with the scripted model. No real
model was used: the stack was a fresh dev profile whose only model connection is the scripted
one, no provider key is in the environment, and none was entered. Every sentence an agent says
below is the scenario's text.

This shows what the records do: one send-back writes an observation and no lesson; the same kind
of send-back on a second piece of Work writes a lesson that names both; that lesson is in the
lead's context for a related job a day later with nothing from the owner in between; the cap
holds; the weekly message lists what changed; a due team review rides a daily review. **It does
not show that a real model heeds a lesson.** The scripted lead prices with GST when the words
"What the company has learned" are in its context ahead of its brief and without GST otherwise.
That rule is this run's, not a model's judgement.

## Setup

- `node tools/mock-model/server.mjs --port 18619 --scenario <scenario.json> --log <calls.jsonl>`
- `RESTLESS_ANTHROPIC_BASE_URL=http://127.0.0.1:18619/v1 RESTLESS_DEV_MODEL=anthropic/claude-sonnet-5-5
  RESTLESS_DEV_CREDENTIAL_REFERENCE=env:<a placeholder> RESTLESS_HEALTH_SWEEP_EVERY_SECS=20
  RESTLESS_DAILY_REVIEW_EVERY_SECS=420 RESTLESS_DIRECTION_SESSION_EVERY_SECS=600
  scripts/restless-dev s119b_test` (profile `dev172673`; existing company and document images;
  `web/node_modules` borrowed from another worktree).
- What the shortened intervals stand for:

  | Real | In this run |
  | --- | --- |
  | A daily review of open goals every 24 hours | 420 s |
  | The weekly direction session every 7 days | 600 s |
  | Health sweep every 10 minutes (learned lessons are pruned in it) | 20 s |
  | A check-in an hour after Exec last wrote to an engaged owner | unchanged, so none fell in the run |

- The company: one goal ("A new menu, priced and printed") and a Menu team: lead `menu-ops`
  (shown to people as Aang), workers `menu-build` and `menu-check`. Work in the first brief was
  added from the host with the CLI as the lead. The second brief was commissioned by the scripted
  Exec, from the daily review.
- The company image holds an older `restless` CLI with no `lessons add --once`. This build's CLI
  was copied into the company computer as `/tmp/restless-s119` so the scripted lead could run
  that one command. Every other command the scripted agents ran used the image's own CLI. The
  daemon was built from this branch.
- Read from the company's own database (`events`, `messages`, `work`, `work_attempts`,
  `company_lessons`, `org_reviews`), the mock's call log, the files in the company computer, and
  `restless work outcome`, `restless lessons list` and `restless org scorecard` from the host.
  No browser was opened.
- 40 model calls in the whole run (24 replies, 16 tool calls). No health notice was raised.

## The run

T0 is the goal being set, 00:40:40.

| Time | T0+ | What | Source |
| --- | --- | --- | --- |
| 00:40:40 | 0 s | Goal set; the lead is commissioned and plans the work | `events`, `work` |
| 00:40:48 to 00:40:53 | | Brief one, first piece: `menu-build` prices the lunch menu, without GST | `work_attempts`, the file |
| 00:40:58 | 18 s | `menu-check` sends it back: "Prices are shown without GST." **One observation, no lesson** | `events`, `company_lessons` (0 rows) |
| 00:41:05 | | The lead resumes it and says what it would do differently (`lessons add --once --work`) | the observation's `next_time`, `noted_by` |
| 00:41:07 to 00:41:09 | | Redone with GST; the check passes on its second look (scripted) | `work_attempts` |
| 00:41:15 to 00:41:17 | 35 s | Second piece: `menu-build` writes the catering quote, again without GST | `work_attempts` |
| 00:41:20 | 40 s | `menu-check` sends it back: "The quote shows prices without GST." **The second of its kind: a lesson is recorded**, for the Menu team, naming both observations | `events` (`lesson_recorded`) |
| 00:41:28 | | The quote is redone and passes | `work_attempts` |
| 00:47:40 | 421 s | **A day later:** the daily review asks Exec to move on the goal. No owner message has been sent | `events`, `messages` |
| 00:47:48 | | Exec commissions "Price the dinner menu" to the lead | `events` (`work_commissioned`) |
| 00:47:49 | | The lead's context for it holds the lesson | the mock's call log |
| 00:47:51 | 431 s | The dinner menu is priced **with GST on the first attempt**; nothing is sent back | the file, `work_attempts` |
| 00:50:40 | 601 s | **A week later:** the weekly direction session; its ask lists what changed about how we work | `messages` |
| 00:51:08 | 628 s | The owner gives standing instructions; 25 are recorded as rules (from the host, as Exec) | `company_lessons` |
| 00:51:16 | | One more job for the lead: its context holds 20 of the 26 lessons, 1,968 bytes | the mock's call log |
| 00:54:41 | 841 s | The next daily review; **it carries the team review**, with no owner message unread | `events` (`org_review_carried`) |
| 00:54:44 | | Exec records the review | `org_reviews` |

## 1. One send-back is an observation, not a rule

The observation, exactly as stored (`events`, kind `work_observation`), read at 00:41:00:

```json
{
    "by": "menu-check",
    "key": "sent-back:7556a872-e4f4-4913-8cf1-9d19ced53d23:1",
    "kind": "prices are shown without gst",
    "source": "sent_back",
    "team_id": "0d320614-866d-4e47-b3e3-3d72519bd080",
    "work_id": "7556a872-e4f4-4913-8cf1-9d19ced53d23",
    "expected": "S119-LUNCH: every dish on the lunch menu has a price, saved at /company/outputs/lunch-prices.md",
    "happened": "Prices are shown without GST.",
    "next_time": null,
    "work_title": "Price the lunch menu"
}
```

Lessons on record then: 0. After the lunch menu was redone and finished: still 0. The second
piece of Work, started 17 seconds after the first was sent back, made the same mistake: an
observation is shown to nobody.

What the lead was sent, in the message it was already getting (the last two sentences are new):

> Review rejected Work 7556a872-e4f4-4913-8cf1-9d19ced53d23 owned by menu-build: Prices are
> shown without GST.. Change the failed mechanism, then resume the node. If you can say in one
> sentence what you would do differently next time, record it: `restless lessons add --once
> --work 7556a872-e4f4-4913-8cf1-9d19ced53d23 --rule '<one sentence>'`. It is noted, not a rule;
> it becomes one only if the same thing happens again.

The scripted lead answered with "Show every price with GST included, and say so at the top." It
was kept on the observation (`next_time`, `noted_by: menu-ops`) and made no rule.

## 2. The same kind a second time is a lesson

The second observation (00:41:20), on other Work and in other words:

```json
{
    "by": "menu-check",
    "key": "sent-back:a7d89591-dcfa-4c4b-85d1-95b1ec711025:1",
    "kind": "the quote shows prices without gst",
    "source": "sent_back",
    "work_id": "a7d89591-dcfa-4c4b-85d1-95b1ec711025",
    "expected": "S119-QUOTE: a priced quote for forty covers, saved at /company/outputs/catering-quote.md",
    "happened": "The quote shows prices without GST.",
    "work_title": "Write the catering quote"
}
```

The two reasons are not the same sentence. They were read as the same kind because they share
three words (prices, without, GST) and those are half of all their words; no model was asked.

The lesson, as recorded in the same transaction as the send-back (`events`, kind
`lesson_recorded`, by `daemon`):

```json
{
    "rule": "Show every price with GST included, and say so at the top.",
    "learned": true,
    "team_id": "0d320614-866d-4e47-b3e3-3d72519bd080",
    "lesson_id": "51949cbb-b17b-46a6-a922-d75853f722fe",
    "scope_kind": "team",
    "promoted_from": [53, 120],
    "observation_kind": "the quote shows prices without gst"
}
```

`53` and `120` are the two observations' event ids. Its words are the lead's own sentence from
the first send-back. With no sentence from a lead the rule quotes the reason instead; that was
seen in the first run, below.

## 3. The second brief, a day later, with nothing from the owner in between

At T0+421 s the daily review asked Exec to move on the goal (its ask is unchanged from Sprint
117). The scripted Exec commissioned "Price the dinner menu" to the lead. Messages from the owner
in the whole run up to then: 0. Lessons recorded by anyone but the daemon: 0.

The lessons part of what the lead was sent for that Work, exactly (from the scripted model's
call log, 00:47:49; it sits in the wake's own text, 3,107 bytes ahead of the brief, not in the
fixed prompt):

```text
# What the company has learned [work went wrong this way more than once; follow each one unless the owner has ruled otherwise]
- Show every price with GST included, and say so at the top. (team Menu, lesson 51949cbb)
```

and the brief it was given:

```text
# Work c4292d44-3de4-43d7-881b-8e02819b9d14 revision 1 attempt 1
Attempt UUID: dc69713c-f489-458c-8cd9-d54e5b8158d6
S119-DINNER: every dish on the dinner menu has a price, saved at /company/outputs/dinner-prices.md
```

The file it wrote on its first attempt:

```text
# Dinner menu

All prices include GST.

Soup 13.20 (GST included)
Pasta 24.20 (GST included)
```

Nothing was sent back and no observation was written for it. For comparison, the first attempts
at the lunch menu and the quote, before the lesson existed, wrote `Soup 12.00` and `Pasta 22.00`
with no GST.

**What this does and does not show.** It shows the lesson was in the lead's context for a
related job commissioned a day later by the company itself, with no correction from the owner in
between, and that a rule keyed on that text produced the right result first time. The scenario
has two rules for the dinner brief: with "What the company has learned" in the context ahead of
the brief, price with GST; without it, price without. The first fired. A real model was not
tried, so whether one would read the line and act on it is not shown here.

## 4. The cap

After the owner's standing instructions there were 26 active lessons: 25 the owner ruled and one
the company learned. What the lead was sent for its next job (00:51:16): 20 lessons, 1,968 bytes
of lesson lines (the cap is 20 lessons and 2,000 bytes), nothing cut:

```text
# What the owner has ruled [owner policy; follow each one; newest first]
- House rule 25: from now on this always holds for everyone here. (whole company, lesson 72e301da)
- House rule 24: from now on this always holds for everyone here. (whole company, lesson 9aa3aee4)
  … (rules 23 down to 8, one line each) …
- House rule 7: from now on this always holds for everyone here. (whole company, lesson b482a60b)
# What the company has learned [work went wrong this way more than once; follow each one unless the owner has ruled otherwise]
- Show every price with GST included, and say so at the top. (team Menu, lesson 51949cbb)
- … 6 more not shown: `restless lessons list`
```

The learned lesson kept its place; the six left out are the owner's six oldest rules. They are
still active and still listed by `restless lessons list` (26 lessons as the lead reads them).
Nothing was retired: no lesson here is six weeks old. Pruning by age was not run on the stack;
it is in the database test.

## 5. The weekly message says what changed about how we work

The part of the weekly direction session's ask that is new, exactly (00:50:40, computed from
`events`, no model):

```text
What changed about how we work since the last one, from the records:
- Learned, for the Menu team, after the same thing went wrong twice: Show every price with GST included, and say so at the top.
- Started the Menu team
End your message to the owner with one short part headed "What changed about how we work": each of those lines in a few plain words with its reason, so they can say if one is wrong. Send that part even if you have no direction question.
```

What the owner was sent (the scripted Exec's words, 00:50:47) ended:

```text
What changed about how we work
- The menu team now shows every price with GST included, after work came back twice for leaving it out.
```

## 6. The team review rode a daily review

Before the second daily review, `restless org scorecard` read `review_due: true`: "Owner signal
since 2 Nov: 25 corrections" and seven attempts. The learned lesson is not counted there: only
what the owner said is the owner's signal.

| Daily review | Carried the team review | Why |
| --- | --- | --- |
| 00:47:40 (T0+421 s) | no | No review was due: the owner had given no signal |
| 00:54:41 (T0+841 s) | yes | Due by the owner's signal, and none carried in the last week. No owner message was unread; no turn was added |

The ask ended: "A review of how the company is organised is also due: the owner has given enough
new signal since the last one. Its numbers and what to do are under Company state in this turn;
do it now and tell the owner each change in one line of today's brief." Exec's turn held the
review block ("# Review how the company is organised … Owner signal since 2 Nov: 25
corrections."), and the scripted Exec recorded the review at 00:54:44. That it is not carried
again the next day, and never with no new signal, was not waited for on the stack; it is in the
database test.

## 7. The outcome record

`restless work outcome` from the host, no model (two of the seven):

```text
Price the dinner menu
  Expected: S119-DINNER: every dish on the dinner menu has a price, saved at /company/outputs/dinner-prices.md Assumed: The dinner menu has the same dishes as last season.
  What happened: Dinner menu is priced, every price with GST included. (1 attempt)
  Cost: $0.02
  Verdict: Finished, not reviewed

Write the catering quote
  Expected: S119-QUOTE: a priced quote for forty covers, saved at /company/outputs/catering-quote.md
  What happened: Catering quote is priced, every price with GST included. (2 attempts, sent back once)
  Cost: $0.02
  Verdict: Finished, not reviewed
```

The costs are what the daemon's spend ledger holds for each Work; here they are the scripted
model's token counts priced as the named model, so they are not real spend. "unknown" (a Work
with no ledger entry) was not seen on the stack; it is in the unit and database tests.

## The first run, which was thrown away

The first company (`s119_test`) was destroyed and the run started again, for two reasons found
in it:

- The scripted checker sent the redone lunch menu back every time, because a scripted model
  cannot read the file. In 40 seconds the Work went to revision 6. That run did show three things
  the clean run does not: the same Work sent back twice the same way is also a repeat and made
  the lesson; the third, fourth and fifth send-backs were counted on that lesson and made no
  second one (five observations, one lesson); and with no sentence from a lead the lesson read
  `Work has been sent back twice for the same reason: "Prices are shown without GST". Check for
  it before handing work in.` The checker now passes a redo on its second look, which is a
  script and proves nothing about the redo.
- This build's CLI had been copied into the wrong one of the company's two containers, so the
  lead's `lessons add --once` failed with "command not found" and no sentence was recorded.

## What was not shown

- A real model. No connection exists for this profile without entering a credential.
- Brand for Exec and leads. Nothing is released in this company, so no turn carried a Brand
  block (0 of 40 calls), which is the "with no release, nothing is added" half. The half with a
  release is in a database test, not on the stack.
- A failed Attempt becoming an observation, an owner's one-off correction (`lessons add --once`
  with no Work), expiry of an observation after four weeks, pruning after six weeks, a pruned
  lesson coming back, and the team review not riding twice in a week: database tests only.
- A lead's own Work being sent back (the ask then rides its next Attempt, not a message).
- The Work page. The outcome record was read from the CLI and is served by the owner API; no
  page shows it yet.

## Teardown

Removed at the end of the session: both test companies, this session's dev profile `dev172673`
with its containers, volumes and network, its state folder, the model-gateway profile folders,
the borrowed `node_modules` and `web/build` links, the scripted model and the daemon.
