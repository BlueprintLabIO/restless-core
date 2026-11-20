# Owner interaction: the company does the harness work

Status: design, 20 November 2026. Nothing here is built yet unless a section says so. Stories
US-399 to US-411 in [`docs/stories/README.md`](../stories/README.md) carry the checks.

## Why Restless feels like a worse Claude today

Used as a chat, Restless competes with a single coding agent on that agent's own ground, and it
loses: Exec hands work to a lead, the lead to Staff, and every hop adds time while the owner sees
less of the work than in a terminal. A good model cannot fix that, because the problem is not how
well the model answers. It is how much of the running of the work the owner still does by hand.

Working with one agent, the owner does five jobs besides saying what they want:

| Job the owner does | What it looks like |
| --- | --- |
| Scheduler | asks "progress?", chases slow runs, keeps track of what is running where |
| Verifier | re-tests every "fixed", finds the bug still there or back again |
| Memory | repeats corrections, restates the goal after the agent drifts |
| Planner | splits a big mandate into diagnose, plan, approve, build |
| Brake and router | stops overreach, picks which agent or model fits the job |

A company of agents costs more than one agent. It pays only when each layer takes one of these jobs
off the owner. That is the test for every feature below, and it does not depend on how good the
models are: even a perfect model leaves the owner scheduling, checking and remembering unless the
harness does it.

## Evidence: the owner's own records

The owner's 20 November report on ten months of their Claude Code and Codex use (21,091 prompts,
17,884 sessions, 3,398 messages coded for intent, kappa 0.87) gives the baseline. The findings are
associations in one person's records, not experiments.

| Finding | Job it shows | Design below |
| --- | --- | --- |
| A fix that did not hold was the main cause in 32 of 79 high-friction sessions | Verifier | D2 |
| An outcome with no method ("fix all root causes properly") is 1.8 times as likely to be redirected | Planner | D1 |
| After one redirect another is 3.9 times as likely | Memory | D3 |
| A screenshot attached cut the odds of a redirect to 0.32 | Verifier | D1, D2 |
| Status checks rose from 2% of prompts to about 8%; "progress?" typed 609 times | Scheduler | D4 |
| Half of agent time now has two or more agents running | Scheduler | D4 |
| Owner works weekdays 09:00 to 18:00 (0.5% of prompts at weekends); agents run around the clock | Scheduler | D4, D5 |
| Overreach: coding before asked, going on after "stop coding", over-engineering | Brake | D6 |
| Adviser-style models suit investigation; correction-heavy sessions sat with one model | Router | D6 |
| Short prompts (median 10 to 13 words) work; longer openers did not reduce follow-ups | all | P1 |

## Principles

- **P1. A short prompt in, a contract out.** The owner keeps writing ten words. The company expands
  them into a contract the owner can read in a glance. Asking the owner to write more is a failure.
- **P2. Done is a proven state, not a claim.** Work closes on evidence that matches its contract,
  checked by an agent other than the one that did it.
- **P3. The owner's attention is the budget.** The company pushes when something changes. Status is
  always on screen without a model turn. Everything else waits for one digest.
- **P4. Every owner signal is measured and changes the company.** A redirect, a status check, a fix
  sent back: each is an event, counted per company, that triggers a lesson or a self-review. This is
  the "enough new usage data" signal of the self-evolving company.
- **P5. The harness enforces modes; prompts do not.** "Look, don't touch" is a permission on the
  Work, not a sentence the model must remember.
- **P6. The owner can always take the wheel.** Stop is instant and total. A plan can be changed
  mid-flight by replying to it.
- **P7. Use what one chat cannot do.** Independent checkers, parallel attempts, a night shift,
  memory across months. Where the company adds nothing a single agent lacks, it should get out of
  the way.

## Designs

### D1. Every piece of Work has a contract

Today a Goal has `done_when` (migration 0083) and Work has a title and brief. Change: Exec turns each
owner request into a contract shown on the Work and in chat, in four lines:

- **Outcome:** what will be true.
- **Done when:** the check that proves it (a reproduction, a page that loads, a number).
- **Proof shown:** what the owner will see at the end (screenshot, log, link, before and after).
- **ETA and checkpoints:** when it should land and when the owner hears about it.

How the request is worded decides what happens next:

- A plain instruction ("rename the button to Save") runs at once. The contract is visible but needs
  no approval.
- An outcome with no method ("make onboarding not suck", "fix all the stuck jobs") first gets a
  **plan card**: Exec's diagnosis in a few lines, the plan, and the contract. One tap approves it;
  any reply changes it. Staff do not start building before the plan is approved. A "look and
  report" request needs no plan card, because it changes nothing.

Measure: redirects per outcome-only request, against the owner's 1.8 times baseline.

### D2. Proof of done, checked by someone else

Today a lead's own inspection can close Work, and the owner can send it back (`reopen_work`).
Change:

- Work cannot close without evidence that matches its contract's "done when". For a bug that means
  the failure reproduced before the change and the same check passing after it, in the real
  environment (the company computer, the live page, the real data), not only a unit test.
- A **checker** that did not do the work runs the check from a fresh context and tries to break the
  claim. The lead's inspection stays the default for small Work; the checker is required for anything
  the owner reported as broken and for anything sent back once.
- The owner's card shows the proof first, the summary second.
- **Probation:** a fixed bug's check re-runs later on its own (after the next release, or the next
  day). If it fails, the Work reopens by itself, the owner is told in one line, and it counts against
  the lead on the scorecard.
- After one failed fix, the lead may try two different approaches in parallel and let the checker
  pick by evidence (P7), within the three-heavy-agent limit.

Measure: Work reopened by the owner or by probation, and owner messages saying "still broken".

### D3. Exec knows what kind of message it got

Each owner message is classified as one of: instruction, outcome-only request, question or "why",
redirect or correction, status check, approval, stop. A cheap model or rules can do it; the class is
stored on the message.

| Class | What the company does |
| --- | --- |
| Outcome-only request | plan card first (D1) |
| Question or "why" | investigate mode: read, run, report; no edits (D6) |
| Redirect | restate the goal in one line and confirm it; record a lesson in the same turn (exists, PR 746) |
| Second redirect in a row on the same Work | pause that Work, show the contract again with what changed, and ask for one decision instead of more piecemeal fixes |
| Status check | answer from the board without a model turn where possible; count it as a missed push (D4) |
| Stop | hard stop (D6) |

Measure: redirect runs, against the owner's 3.9 times baseline.

### D4. Push, a board, and a digest instead of "progress?"

Today Exec checks in about hourly while the owner is engaged (PR 752, US-398). Change:

- **Board:** every running Work shows its contract line, phase, latest evidence, ETA and next
  checkpoint, computed from state. Seeing status never costs a model turn.
- **Push on change:** the owner hears when Work lands with proof, reaches a checkpoint, is blocked on
  them, or its ETA slips. Nothing else interrupts.
- Check-ins shrink to what changed against the contracts since the last one, plus the open questions.
- **Owner hours:** learned from when the owner writes (and editable). Outside them only "blocked on
  you, and it matters today" can interrupt.
- **Morning digest** at the start of the owner's day: what landed overnight with proof, numbered
  decisions waiting, what is queued for today.

Measure: share of owner messages that are status checks, against the owner's 8% baseline. The goal
is near zero.

### D5. A night shift the owner queues

Core never starts work by itself; the owner queues it. Near the end of the owner's hours Exec offers
a short list ("queue these for tonight?") with a finish line for each. They run within the spend
limit, with the same contracts and checks, and the results open the morning digest. Nothing that
needs the owner is started overnight.

### D6. Modes, a real stop, and models by role

- **Modes on Work:** investigate (read and run, no edits), plan, build, check. Enforced by the tools
  each agent is given in that mode, so "don't code" cannot be forgotten.
- **Stop:** "stop" from the owner ends every running turn on that Work within seconds, keeps its
  state, and says what was in flight. Resume is one reply.
- **Models by role:** the company picks the model for investigate, build and check from its own
  results (corrections and reopened Work per model per role) instead of one model for everything.

### D7. The company measures its owner's friction

The Team results page gains the owner-side numbers from the report, per company: status-check
share, redirects per outcome-only request, redirect runs, reopened Work, owner messages per landed
outcome. Self-review triggers when one of them moves past its baseline, so the company changes
itself on the owner's real signal, not on a clock. Lessons that repeat become a short operator
profile the owner can read and edit (hours, how they like to be asked, "never code before asked").

## How we will know it is better than one agent

The baseline is the owner's own Claude Code record above. Restless is at the next level when, on the
owner's own companies, it beats that record: fewer status checks, fewer redirects after big asks,
fewer fixes that come back, fewer owner messages per thing that landed.

Order of work:

1. Classify and count first (D3 classification, D7 numbers). It is cheap and gives a Restless
   baseline within a week, so every later change can be shown to move a number.
2. Proof of done with a checker and probation (D2): the largest single source of friction.
3. Contracts and the plan card (D1), then redirect handling (D3).
4. Board, push, owner hours and digest (D4); check-ins become deltas.
5. Modes and stop (D6), then the night shift (D5) and models by role.

Before each step: look for prior art and check the idea against this company's own data, per the
research-first rule. Open questions for the owner: whether every outcome-only request needs a plan
card or only larger ones; whether owner hours are learned or set; how much extra spend an
independent checker may cost.
