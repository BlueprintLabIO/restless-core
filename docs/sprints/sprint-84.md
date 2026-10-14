# Sprint 84 — A dogfood loop that sees what the owner sees

**Status:** landed (see the commits naming each item on main)
**Programme:** quality of the cockpit; process
**Depends on:** Sprint 83 (the 210-story audit).

## Outcome

On 14 October the owner found about thirty problems in one day of real use, after several sweeps had
passed. The sweeps missed them for four reasons:

- They ran on empty `_test` companies, so nothing piled up, overflowed or went stale.
- They checked status codes and console errors, not whether a screen was good.
- They looked at still pages, so flicker and the snap at the end of streaming never showed.
- They were written as features ("approve a request"), not as the owner's goals ("my charter is set").

When this sprint is done, every UI change is checked on a company that looks like real use, against a
story list that grows from every report, and by automatic checks for the jank that kept creeping in.

## Changes

1. **Reno-scale seed** (`scripts/seed-reno-scale.mjs`). It fills a `_test` company on a dev stack with:
   - a lead and two staff;
   - a goal with 30 finished pieces of Work and 120 Library files;
   - Work in motion, waiting and blocked, with a long blocked reason;
   - long Exec threads with a long code block, a raw budget error and a long reply;
   - Exec-to-lead messages;
   - two direction requests.

   It refuses non-`_test` companies and adds nothing twice on a re-run.
2. **One story list that grows** (`docs/stories/README.md`).
   - It holds the 210 audit stories plus the owner's reports, each written as "I want X, and I see Y".
   - Every owner report becomes a story with the date it was reported and the check that now guards it.
   - Stories are tagged **core** (run on every UI change, about 50) or **full** (run before each release).
3. **Jank probes in `web/scripts/capture-surfaces.mjs`**, failing the run, at 1440, 1024 and 390 px,
   light and dark. The probes:
   - **Sideways scroll:** any scroll container whose content is wider than it is, outside code
     blocks and sheets.
   - **Overlap:** text running under a button or a hover toolbar.
   - **Clipped text:** text cut off with no ellipsis and no tooltip.
   - **Raw text:** UUIDs, `company_<hex>` handles, `[budget]`-style codes, model ids and shell
     commands (`restless …`, `$(`) shown to a person.
   - **Grey status text:** small right-aligned status text in navigation rows, which the owner asked
     to remove.
4. **Checks in motion** (`web/scripts/verify-motion.mjs`). On the seeded company it samples frames
   while a reply streams and while a chat opens. It fails if:
   - a message's width changes after streaming ends;
   - a tool-action row appears and then disappears within 500 ms;
   - a chat opens anywhere but at its latest message.
5. **The owner-path rule** in `CLAUDE.md`. A UI change is done only when it has been seen in the
   built-in browser on the seeded company, at desktop and phone width, with the core stories passing.

## Acceptance

- The seed runs twice on a fresh `_test` company. The second run adds nothing.
- `capture-surfaces` with the probes reports every issue in Sprint 85's list that is still present
  before Sprint 85, and none after it.
- `verify-motion` fails on today's flicker, snap and open-at-top, and passes once Sprint 85 fixes them.
- `docs/stories/README.md` has every story from the 14 October report, each linked to its fix.
