# Sprint 85 — The owner's 14 October list, fixed to the bar

**Status:** landed except S85-21, which waits on the owner (which labels to use); each item's commit names it
**Programme:** owner feedback; quality of the cockpit
**Depends on:** Sprint 84 (seed, probes, motion checks). Every item is verified in the built-in browser on
the Reno-scale seed, at desktop and phone width.

## Outcome

The owner reported these from real use of Reno. Each one ends fixed, with the story that now guards it
(see `docs/stories/README.md`, S85-NN). The bar is Linear and Apple: remove before adding, no eyebrows,
and no right-aligned grey status text.

## Brand & voice

**S85-01. Brand & voice is four lists, not cards.**
- Truth, Voice, Visual language and Culture are separate sections. Each holds a list of short
  statements.
- Each row can be edited in place and removed. "Add" sits at the foot of each section.
- Exec's proposed statements show inside their section as drafts, with Accept and Dismiss.
- Saving publishes a new brand version, so history and restore (S83 UXG-004) keep working.

## Chat (Exec, leads and staff)

**S85-02. One chat for everyone.**
- Talking to a team member uses the same view as talking to a lead or Exec: the same header, the same
  composer, and no thread side panel unless one is opened.
- The pill at the top right that names an agent opens that agent's chat, never Exec's.

**S85-03. A chat opens at its latest message.**

**S85-04. Nothing flickers while an agent works.**
- Tool actions don't show and then fold away; they are folded from their first frame.
- A streamed reply keeps its width when streaming ends.

**S85-05. Remove the strips under the composer.**
- "Under way" and "Asked …: Done" go.
- What is under way lives in Work; requests live in the Inbox.

**S85-06. Errors read as sentences, in view.**
- A failed reply says what happened and what to do, with the action as a link. For example: "The
  company has reached its $100 spend ceiling. Raise it in Company › Limits."
- No raw `[budget] company_…` text, and nothing hidden under a fold.

**S85-07. "Needs you" lines are links.**
- A "Needs you:" line naming a page in the cockpit links to it, e.g. Company › Charter, Company ›
  Intelligence, or Apps.

**S85-08. Images in chat.**
- Images attached in the composer are a small row of thumbnails above the text. The row scrolls
  sideways when it overflows, and each thumbnail has a remove button.
- Sent images are thumbnails that open full size.
- Images sent earlier render.

**S85-09. Code blocks keep their copy button in view.** It sticks to the top of the visible part of the
block while the block scrolls.

**S85-10. Agent-to-agent messages show in that agent's chat.** When Exec messages a lead, the lead's
chat in People shows the message and the lead's reply. "From Exec" sits in the same thread as the
owner's own conversation with that lead.

**S85-11. "Add people" is a clean sheet.**
- A short title, a member list with avatars, and one primary button.
- Optional context sits behind "Add context", not as a large empty box.

## Library and Work

**S85-12. The Library shows what the company currently has.**
- Files whose Work has moved on (retired or superseded) leave the default views.
- The list refreshes when Work changes, without a reload.

**S85-13. Remove the Library "+" and the Work sparkle button.** New things come from Exec, or from the
command palette's "New".

**S85-14. Done Work stays out of the way.**
- The board's Done column shows the last 7 days, with "Show all" below.
- Done older than 7 days is archived automatically.
- A "Clear done" control archives the rest, with a small celebration.

**S85-15. A piece of Work opens beside its agent's chat.**
- Opening Work from a list shows the agent's chat with the Work's details in a side sheet: status,
  evidence, outputs, next and revision loop.
- The full Work page is removed. Its URL redirects to the chat with the sheet open.

**S85-16. No redundant tooltips.** A tooltip that repeats visible text (such as a file title) is
removed.

## Layout

**S85-17. Every left sidebar can be resized.** The Inbox, Work, Library, People and Company sidebars
can be dragged, and keep their width per person.

**S85-18. No right-aligned grey status text in navigation.** The Company navigation shows names only.
Status lives on each page.

**S85-19. "Owned by" fits the page.** Team ownership is a quiet line under the team name ("Owned by
you", or a person's name), with a menu to change it. It is no longer a form select.

**S85-20. No overflow.**
- People rows, Work rows and card titles ellipsize before hover toolbars and trailing controls.
- Toolbars never sit over text.

**S85-21. Model choice reads the same when creating and changing.** One component and one set of
labels: Connection, Model, Runs with.

## Inbox

**S85-22. Inbox and For you, not "Inbox · Inbox".**
- The tab that holds decisions is called "Decisions". "Inbox" is the page, and "For you" stays.
- The empty state no longer repeats a pill ("No decisions") over the same words.

## Acceptance

- Every item has a story in `docs/stories/README.md` and a check:
  - a `node --test` model test;
  - a `capture-surfaces` probe or a `verify-motion` check;
  - or, where neither fits, a named manual step in the built-in browser.
- Each item is seen working in the built-in browser on the Reno-scale seed. Its commit names the item
  and what was seen.
- `npm --prefix web run check` is at 0 errors and 0 warnings, and the web model tests pass.
