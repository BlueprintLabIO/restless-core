# Sprint 117: it comes to you with the right questions

**Status: planned, not started (14 Nov 2026 AEDT). Depends on Sprint 116 (the daily review).**

The owner's words: the company should not pick direction. It should "ask you the right questions to
get you to think about the next directions and to focus on the key things that really, really
matter".

Today nothing asks the owner about direction after setup. The landing page with nothing pending
says "No decisions". The rules say the opposite of what the owner wants: models decide "what
matters next", and a brief must "Recommend one course. Do not hand the judgement back". There is
also nothing to ask about, because assumptions and unknowns are never written down.

Founding lines this restores:

- `docs/specs/orgintel.md` §3.1, Frame: make explicit "assumptions and unknowns" and "the most
  decision-relevant uncertainty".
- `docs/specs/owner-cockpit.md` §15.5: with no decision waiting, "a calm summary of current
  objective, recent accepted progress, next expected review, and operating status".
- Helm's constitution: "The default message direction is toward the member, not from them".

The audit is clear that direction questions are a new requirement, not a restoration. The specs
optimise for less owner attention. This sprint aims for better attention: few questions, each
worth the owner's time.

## What the owner decided (14 Nov 2026)

- **On direction the company brings questions and its assumptions, and the owner decides.** On
  everything else it still recommends one course.
- Act then tell, and one Exec turn a day (see Sprint 116).
- Still binding from Sprint 109: no jank, rows never grow, chat stays in the rail, anything
  waiting for the owner is an Inbox item, and no new dashboard.

## What exists to build on

- The daily review (Sprint 116) and the check-in (`crates/restless-orgintel/src/checkin.rs`).
- Statement kinds with scope, expiry and source links: migration
  `0047_actor_context_checkpoints.sql` and `ActorContextEpistemicKind` in
  `crates/restless-orgintel/src/actor_context.rs`. The read path labels each statement and
  withholds stale ones (`crates/restless-engine/src/staff/context.rs`). `record_actor_checkpoint`
  is called only from tests, and there is no command to write one.
- Tap answers: `ownerNeed` and `ownerReplies`, drawn by
  `web/src/lib/components/ExecutiveRail.svelte`. Used at setup and for "Make this a goal?" only.
- The Today card at the top of the Inbox list (`DirectionToday.svelte`, inside `InboxList.svelte`).
  It counts applied changes and waiting requests, and hides when both are zero.
- The week page (`web/src/routes/[companyId]/company/week/+page.svelte`), a look back that Exec
  writes up on request.
- Notification intents (`crates/restless-owner/src/owner_notifications.rs`), which honour each
  person's digest time (`email_digest`, `last_release` in `notifications.rs`).

## A. Write down assumptions and the key uncertainty

**Problem.** B3 and E1. A question about direction has to come from something recorded.

**Build.**

1. When Work is commissioned, the lead writes its assumptions (three at most) and the one thing it
   is least sure of. A command or tool call writes them. It uses two of the existing kinds only:
   `assumption` and `unknown`.
2. The existing read path shows them to the lead and Staff on that Work, and to Exec.
3. No other kinds are written in this sprint. No confidence scores, no expiry sweeps.

**To confirm during ticketing.** A checkpoint row carries a whole working state (objective, status,
sources) bound to a session. If a checkpoint holding only assumption and unknown statements is
awkward, the simpler first carrier is a short section in the Work brief, indexed for the read
path. The spec prefers "readable files or structured document blocks indexed by OrgIntel".

**Done when.** A commissioned Work shows its assumptions and key uncertainty in the lead's next
context, and a scripted run with no assumptions written is told once to write them. It never
blocks the Work.

## B. The morning brief

**Problem.** B1 and B8. There is no "today". A plain Exec update stays in chat.

**Build.**

1. The daily review's one short message becomes the morning brief. No second model turn. It has
   three parts: what moved, what the company will do today unless the owner says no, and the
   decisions it needs.
2. The brief shows at the top of the company landing page, in the place the Today card holds now.
   One fixed-height row. Opening it shows the full brief in the item pane. It does not embed chat;
   a reply goes through the rail. The applied-changes count stays as a sub-row.
3. With no brief yet today the row shows yesterday's, dated. It never shows a spinner and never
   changes height.
4. The brief is sent through the existing notification intents, so it reaches the owner's email at
   their digest time.
5. For this to be a morning brief, the daily review runs in the owner's morning. Use the timezone
   the digest setting already holds. To confirm during ticketing with Sprint 116's interval.

**To confirm during ticketing.** The plan assumed Cloud's morning digest would pick this up. Cloud's
daily digest (`apps/fleet-web/src/lib/server/daily-digest.ts` in restless-cloud) carries counts
only, by design: "Fleet never reads a company". So the brief's text has to travel as a
notification intent with a summary, not inside that digest. Which intent kind carries it is open.

**Done when.** On a seeded company at desktop and phone width, the brief row is the first thing on
the landing page, the same height with and without a brief, and the email arrives once.

## C. The weekly direction session

**Problem.** B2, B5 and B7. Nothing asks about direction. The weekly review is a look back that the
owner has to open.

**Build.**

1. Once a week, the daily review turn also prepares at most three questions about direction. It
   rides that day's turn, so there is no extra wake.
2. Each question states what the company knows, what it is assuming (from part A), and what changes
   with each answer. Each has two or three tap answers.
3. The session is one Inbox item, "This week's direction". Each question is a fixed-height row;
   the three statements show as sub-rows when it is opened. Answers are taps. Whether one item or
   three reads better is to confirm in the browser.
4. An answer is the owner's own words. Exec acts on it in that turn, and records a changed goal
   through the existing goal commands.
5. Fewer than three is fine. If the company has no real direction question, it asks none.
6. The week page links to the session and stays a look back.

**Done when.** A scripted week produces a session with tap answers, an answer changes a goal, and
the Inbox item clears.

## D. The rule change, for direction only

**Problem.** B4.

**Build.**

1. `docs/COMPANY_OPERATING_RULES.md` §3: models decide what to build, how to approach it and what a
   result means. On what matters next for the company as a whole, they bring the question and
   their assumptions, and the owner decides.
2. `crates/restless-engine/skills/owner-briefs/SKILL.md`: keep "Recommend one course" for every
   brief except a direction question, which states the options and what follows from each.
3. Say plainly what counts as direction: which goals to hold or drop, who the company serves, what
   it will not do, and a material shift of spend between goals. Everything else is not direction.
4. Note the same exception against `docs/specs/orgintel.md` §2.2 ("choose goals and milestones").
   The check-in's "give your recommendation with each" stays, because its questions are about the
   next hour, not direction.

**Done when.** Prompt tests assert both the rule and its limit. A scripted routine decision still
arrives as one recommended course.

## E. A quiet owner and a thin mission

**Problem.** B6. Nothing says what the company does when the mission is thin and the owner is
silent.

**Build.** One rule in the `running-the-business` skill: do the cheapest useful thing toward the
mission, write down the assumption it rests on (part A), and ask once. Do not ask again until the
owner returns. Sprint 116's quiet rule already bounds the wakes.

**Done when.** A scripted company with a one-line mission and a silent owner produces one small
piece of Work, one recorded assumption and one question, then goes quiet.

## Tickets

| | Ticket | Layer | Closes | Reuses | Makes deletable |
| --- | --- | --- | --- | --- | --- |
| [ ] | 117-A Assumptions and key uncertainty at commissioning | OrgIntel, prompt | B3, E1 | Migration 0047 kinds and read path | Nothing; it gives a dormant table its first writer |
| [ ] | 117-B Morning brief row and email | Owner surface | B1, B8 | Daily review message, Today card, notification intents | The "No decisions" empty verdict as the only thing on the page |
| [ ] | 117-C Weekly direction session | Prompt, owner surface | B2, B5, B7 | `ownerReplies`, Inbox items, week page | The week page's "what you would change" clause, if the session covers it |
| [ ] | 117-D Rule change for direction | Prompt, docs | B4 | Operating rules §3, `owner-briefs` | The blanket ban on options for direction matters |
| [ ] | 117-E Quiet owner, thin mission | Prompt | B6 | `running-the-business`, Sprint 116 quiet rule | Nothing |
| [ ] | 117-F Acceptance week | Verification | B1 to B8 | One of the owner's companies | Nothing |

## Slice per layer

- **Kernel.** No change.
- **OrgIntel.** A writer for two existing statement kinds, or a brief section. No new table.
- **Runtime.** No change.
- **Owner surface.** One fixed-height row on the landing page and one Inbox item a week. No new
  page and no dashboard.

## Cost

No new wake. The brief is the daily review's message, and the weekly session rides one daily
review a week, which makes that turn longer. Writing assumptions adds a few lines to a
commissioning turn that was happening anyway. A company with no open goal still costs nothing.

## Done when

- **The acceptance week.** In one of the owner's own companies, after a week of real activity, the
  weekly session produces three questions. The owner and one outsider each mark every question
  "about direction" or "about status". Pass is all three judged about direction by both, and at
  least one that changes what the owner would do. Keep the questions and the marks.
- **Live per role.** Owner sees the brief and the session on desktop and phone. A member sees
  neither.
- **Quality gates.** `capture-surfaces` and `verify-motion` pass; the brief row never changes
  height; no extra model turn.

## Out of scope

- A company bar with objective, phase, health and spend (B9), and health reasons shown to the
  owner (B10).
- Success contracts and "what would prove this wrong" on the goal page (B11).
- Directives as records (B12), kickoff briefs (B13), phone alerts without a self-made bot (B14).
- Confidence, expiry and predictions on claims (E2, E3). No large set of statement types.
- Strategic planning as a mechanism (A7 strategic) and mandates (A8).
