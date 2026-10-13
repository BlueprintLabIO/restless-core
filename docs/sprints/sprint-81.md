# Sprint 81 — Make what the owner meant

**Status:** landed
**Programme:** quality of the company's work; owner feedback
**Depends on:** Sprint 79b. (Sprint 80 is the direction-and-permission spec.)

## Outcome

The owner reported that Reno's sites were low quality, text-heavy and visually buggy, made on
assumptions rather than what they wanted. Brand & voice and the charter were never asked for first.
Their feedback did not change how the company works next time. A Library link opened on "This file
is no longer in the Library" with nothing to do there.

This sprint fixes the causes:
- the company aligns before making;
- review looks at the rendered work and measures it;
- the owner's corrections become Brand & voice rules they approve;
- the dead link leads on to the newer version.

## Causes

| What the owner saw | Cause |
|---|---|
| Sites built on assumptions | nothing asked for the audience, the one action, the owner's references or the never-say list before production; the landing skill went from "references" to three directions without the owner |
| Text-heavy pages | no copy budget anywhere, and the review skill told critics to reject "short pages", which pushed toward length; the capture tool recorded no text density at all |
| Visual bugs reaching the owner | the critic had no density or brief facts to hold the candidate to, and the owner saw only finished work |
| Brand & voice and the charter not set first | the welcome interview asked about the charter only; nothing offered Brand & voice afterwards. The page was called "Identity", which said nothing about voice or look |
| Feedback did not stick | the Brand & voice evidence and proposal machinery (S31, S34) existed, but no doctrine told Exec to turn corrections into it |
| "No longer in the Library" dead end | a link to a version a redo had replaced found no listed file, so the page said only that |

## Changes

- **Exec doctrine:** a new "Align, look, learn" block (`prompts/align-and-learn.md`).
  - **Foundations before customer-facing work:** Exec asks which brands feel like the company's,
    offering three real reference companies, plus how it should sound and must never sound. It
    records the answers and proposes a Brand & voice release.
  - **A kickoff brief before customer-facing Work:** the audience, the one action, examples, and
    must and never, stated in the Work's outcome. Leads show the owner a first look before the
    full build.
  - **Learning from every correction:** corrections, rejections, undos, rewrites and stated
    preferences become evidence through `restless identity voice-evidence`, `evidence --pillar
    visual` or `voice-learn`. A lasting rule becomes a proposal the owner promotes; Exec says so in
    one line.
- **Welcome interview:** two questions before naming the company: brands like us, and how we sound
  and must never sound. Their read-back line seeds Brand & voice.
- **Skills:**
  - `landing-page-design`:
    - starts from the owner's brief, asking through Exec when it is incomplete, never guessing taste;
    - default copy budgets: hero at most 10 + 25 words, sections at most 60 words with a visual,
      paragraphs at most 40 words, at most 120 words per desktop screen;
    - a "First look" owner judgement on one or two directions before the full build, recorded as
      taste evidence.
  - `web-native-review`: the critic holds `density` to the brief's budget or the defaults and
    quotes the numbers. It rejects candidates that contradict the brief, and walls of text as well
    as thin pages.
- **`restless-web-review`:** the manifest records `density`:
  - words, screens and words per screen;
  - visuals;
  - the longest paragraph and paragraphs over 60 words;
  - per section: words, visuals, height and heading.
- **Cockpit:**
  - "Identity" is now **Brand & voice** (the Company sidebar, setup rows and page).
  - The owner's clear Inbox shows "Set your brand and voice · Set up with Exec" when none is set
    and nothing blocks the start.
  - The Library names a replaced file and links to its newer version from the same Work, for
    members too.

## Verification

- `restless-web-review` on a probe page: the 182-word "About us" section is reported with 0 visuals,
  `paragraphsOver60Words: 2` and `longestParagraphWords: 90`.
- `library-files.test.ts`: a link to a replaced site finds the newer one from its Work; a replaced
  file with nothing newer says so.
- The `restless-engine` context tests pass with the new block. The commands and values the doctrine
  names (`voice-evidence --kind expression_principle`, `evidence --pillar visual --kind guidance`)
  parse.
- On the dev stack:
  - the clear Inbox shows the nudge below the "No decisions" pill;
  - with no model connected, the start blocker takes its place instead;
  - the Library's replaced-file page renders as a full empty state.
- svelte-check 0 errors and 0 warnings; web unit tests 139/139.

## Not verified here

- An Exec turn following the new doctrine end to end needs a connected model. The next Reno-like
  run on Cloud is the check: the brief questions, the first look, and a correction becoming a
  proposal.

## Results

- No migration; schema stays 99.
