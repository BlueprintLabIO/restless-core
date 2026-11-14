# Sprint 114: lasting suggestions

**Status: planned, not started (9 Nov 2026 AEDT). Depends on nothing unreleased.**

The owner's point: a conversation with Exec should shape the company, not end as a short to and
fro. Today a correction lives in one chat. On Reno, the owner said the email openers should talk to
the business rather than about it. An earlier rewrite had already lost the same idea once, and
nothing stops it being lost again.

This sprint adds a suggestion to the end card, where the question goes today. Exec proposes the
exact words to add to a named company page, and the owner presses Accept or Reject.

## What the owner decided

- The suggestion sits in the end card, with simple Accept and Reject buttons.
- An accepted rule applies to **new work only**. Work already under way is not disturbed.

## What exists to build on

- The end card is drawn from the message's `ownerNeed`, `ownerReplies` and `ownerFields` intent
  fields, in `ExecutiveRail`.
- The charter already has revisions, read-back after saving, and drafts for the owner
  (`charter_apply`, `charter_propose`, shown in Company › Charter and the Inbox).
- Inbox decisions already bind to an exact digest and have an Undo held by Authority
  (`approval_decisions.rs`).
- The conversation contract already says Exec's agreement or guess never decides anything, and only
  the owner's own words do. The Accept press is those words.

## Scope of the first version

Exec only, in the owner's Exec chat, with three homes:

| Home | Holds | Today |
| --- | --- | --- |
| Charter | What the company is for and will not do | Revisioned, in Authority |
| Brand voice | How the company sounds | Company › Brand; storage to confirm in part A |
| How the owner works | How the owner likes things done | A Library document, found by title |

The current plan is not a home, because it is meant to change. Staff and leads do not suggest.

## A. Spike: homes and what "new work" can mean (one day at most)

1. Confirm where Brand voice is stored and how each of the three homes reaches an actor: in the
   system prompt, in context, or opened by the actor during a run.
2. Give How the owner works a stable identity, so a renamed document cannot lose it.
3. Measure each home's current size on the six real companies, and set each page's size limit from
   that.
4. Establish whether a Work can be held to the revisions it started with.

**If pinning needs versioned reads across the Library, stop and bring the options to the owner.**
The fallback is to deliver accepted rules only into context at Work start, and to accept that a
running actor which opens the page itself can see the new text.

## B. A suggestion is a durable record

1. The intent gains one optional field: home, add or replace, the exact text, the exact line it
   replaces, and a one-sentence reason. Every suggestion names its source owner message.
2. The engine turns it into a durable record holding the home's base revision and a digest of the
   text, in the same shape as a charter draft. The buttons act on the record, never on message text.
3. The engine refuses a suggestion when the source message is not the owner's, when the home is not
   one of the three, when the replaced line is not on the page, when the page would pass its limit,
   or when the exact text has already been rejected.
4. One open suggestion per company per home, and a newer one replaces it.

## C. When Exec offers one

Most replies should have no suggestion. The guide says to offer one only when the owner corrected
something or decided something that will apply next time, using their words and not a generalised
version. Grading words, one-off facts and things already on the page do not qualify. It prefers
merging into or replacing an existing line over adding one.

Exec checks past rejections with a command when preparing a suggestion, so they are not in every
prompt. The guide is offset by trims elsewhere, as in #728: token totals must not rise on the
overhead bench.

## D. The card

1. One slot at the end of the message. **A question goes first**, because it is holding up work. The
   suggestion waits and appears once the question is answered.
2. The card shows the exact words, the home by name, and the line being replaced. Two buttons:
   Accept and Reject. To change the wording, the owner replies in chat and gets a revised card.
3. Only the owner sees the buttons. Members see nothing in the first version.
4. An unanswered suggestion stays on its message. It does not reach the Inbox, notifications or
   Telegram, and it does not nag.
5. House rules apply: final styled render at once, no layout shift above the card, no label swaps,
   one tap target at phone width, and the card holds no chat of its own.

## E. Accept, Reject, Undo

1. **Accept** applies the exact stored text with no model turn, through the owner's own session, and
   then reads the page back before saying it saved. No agent can press it.
2. The card becomes one line: "Added to Brand voice", with View and Undo.
3. **Undo** restores the previous revision while the page has not changed since. If it has, Undo is
   replaced by a link to the page.
4. **Reject** closes the record and remembers the text. It says nothing further and wakes nobody.
5. **Stale page:** if the home changed after the suggestion was made, Accept refuses and says so.
   Nothing merges silently.
6. **Charter drafts:** `charter_apply` closes any waiting draft as replaced. Accepting a suggestion
   must not discard a draft the owner has not seen, so the card says a draft is waiting and Accept
   is refused until it is resolved.
7. Exec learns of the result on its next turn. Accepting does not wake it.

## F. New work only

An accepted change takes effect like this:

| Who | When it sees the change |
| --- | --- |
| A Work already started, including its retries | Never. It keeps the revisions it started with. |
| A Work started after Accept | From its first Attempt |
| Exec's conversation | From its next turn |

Each Work records the home revisions it started with, so it is possible to tell afterwards which
rules it worked under. A changed home in the launch contract retires a warm process (#721), and it
also breaks the prompt cache for that actor once. That is another reason to keep suggestions rare.

## Done when

- **The real case, on a real model:** replaying the Reno correction produces a card with the right
  words and home. After Accept, a new reviewer Work follows the rule, and a Work already running
  does not change.
- **Restraint:** on a replay of real Reno conversations, at most about one owner message in ten
  produces a suggestion, and none repeats a rejected one.
- **Live per role, desktop and phone:** owner accepts, rejects and undoes; member sees no buttons.
- **Failure cases, live:** double press, offline during Accept, stale page, waiting charter draft,
  Undo after a later change, and an agent trying to accept through its own session (refused).
- **Quality gates:** no new jank on the `--jank` gate. Layout shift on the card is about 0. Token
  totals on the overhead bench do not rise.

## How we will know it worked

The test is whether the same correction stops coming back. After two weeks on Reno, read the real
conversations and report suggestions offered, accepted and rejected, plus any correction the owner
had to repeat after accepting it. If repeats continue, the rules are not reaching the work, and that
gets fixed before any more homes or any more suggesters are added.

## Out of scope

- Suggestions from leads or Staff, members accepting, and group rooms.
- More homes, such as team charters or review checklists.
- Automatic learning with no owner press.
- Applying a rule retroactively to Work already running or finished.
