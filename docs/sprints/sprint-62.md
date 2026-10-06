# Sprint 62 — Linear-level polish on the hosted owner path

**Status:** notes and proposals for founder alignment. No tickets yet.
**Depends on:** Sprint 60's one address (live on `app.restless.run` since 2026-10-06).

## Outcome

The founder's first signed-in walk through Cloud (2026-10-06) reads as one calm product:
- one account menu;
- a sidebar that earns its space;
- questions from Exec that are answered in one tap and never block the conversation;
- a Company area you can navigate;
- a Library whose actions match what you are looking at;
- documents that open and edit like a page.

The polish bar is Linear (settings navigation, the sidebar, the command surface) and Notion (documents).

## Observed friction (founder walk, 2026-10-06)

Each item gives the observation, then the cause where the code shows it.

### Fleet Home and account shell

1. **Two account menus.**
   - The rail lists Account with seven children: Profile, Security, Plan, Support, Connections, AI apps, Appearance.
   - The avatar menu at the bottom repeats four of them (Connections, Plan, Security, Support) plus Sign out.
   - Each was added for its own reason (the 0.7.0 rail, then the issuer's popover), and nobody owns the whole.
2. **The rail is ugly.**
   - Home and Account are 14px rows, while the seven children are ~11px grey with a tree line. That reads as two type systems.
   - The rail is mostly empty while the companies (the main thing) live only in the page body.
   - The logo is a monospace wordmark that matches nothing else.
3. **The name prompt is scuffed.**
   - "What should Exec call you?" is a bordered card jammed against the "Your companies" heading, with no gap.
   - The input stretches to full width, and the label sits inline at a different size.
   - It is a nag box on every Home visit until answered.
4. **The company row is busy.**
   - It carries a bordered `…` button, a pill-shaped "2 decisions waiting" badge, a "Ready" dot and a two-clause summary line.
   - The 32px "Your companies" title is heavier than anything Linear uses for a page title.

### Company identity and Exec's work

5. **The name never lands.**
   - Exec asked "Confirm 'Blueprint Lab' as the name…", the owner answered "Both are right", and Exec replied "Both recorded".
   - Yet Home still shows "New company", the Company page title reads `Company 01a1099254287140b24b3202429a3f01`, and Identity and Charter read "Not set yet / Not written yet".
   - Two defects:
     - (a) Exec records the decision as a note and never applies it to the company's name or charter;
     - (b) `CompanyTitle` does not use the "New company" display mapping that Core added in `company.rs`, so the page shows the raw handle.
6. **Exec narrates to itself in the owner's chat.**
   - "Both recorded. Replying to the owner." and "Set up and dispatched. Here's the reply to the owner." are internal monologue in the third person.
   - Owner-facing text should be addressed to the owner.
7. **Internal mail fills the chat.**
   - Seven rows of `Bart → Exec …` / `Exec → Bart …` sit in the owner's conversation.
   - That is a status wall of coordination, which `CLAUDE.md` says is revealed only on request.

### Questions from Exec

8. **The Answer button is detached.**
   - It sits top-right of the question card, away from the choices.
   - Choosing a chip does not send. It opens an "Answering: …" reply bar and prefills the composer, so one decision takes three steps.
9. **Structured forms block the conversation.**
   - "Confirm A$4,800 … give the booking email or link" docks a Price / Booking link form with a disabled Send answer button between the transcript and the composer.
   - The owner cannot talk past it. Exec also bundled two asks into one question.
10. **The composer toolbar overlaps.** In the busy state, Interrupt draws over the mic and the effort label ("H🎤gh"). This is a layout bug.

### Company area

11. **The Company section navigation is missing on the live plane.**
    - `routes/[companyId]/company/+layout.svelte` still draws a `company-nav` list, but the hosted overview showed none.
    - The page falls back to a one-column "Setup" card list. The cause needs confirming on the live build: a width breakpoint beside the Exec rail is the first suspect.
12. **Setup is a list of pages, not a list of to-dos.** Charter and Identity are unset, and the Vault reads "Unavailable right now", but nothing counts or orders what needs doing.
13. **The header pills mix meanings.**
    - "Needs attention" (status), "US$0.62 of US$10.00 this month" (spend), "1 person working" (who?) and "Activity" (a link) all share one pill style.
    - "Limits" truncates mid-sentence.
14. **The Vault is unavailable when hosted.** It reads "Unavailable right now" on Cloud. This is a capability gap to probe, not a wording issue.

### Library and documents

15. **The top bar ignores the view.**
    - Every filter shows the same "Search · New sheet · New document".
    - `library/+page.svelte` labels the primary button "New document" even when the view is Sheets, and opens a sheet anyway.
16. **Creating a document goes through a dialog** instead of opening a fresh document with the cursor in the title.
17. **The editor draws a heavy outline.**
    - The whole editable area gets a dark focus border while you type, which makes the page look like a form field.
    - Documents have no separate title line, so the first paragraph doubles as the title.

### Gaps carried from Sprint 60

18. **Hosted previews do not open:** the review origin is loopback-only.
19. **T11 stopgaps** are still to delete: the cross-origin entry page, the plane Home redirect and account entry.
20. **Founder's signed-in checks for T10** are still open.

## Proposals

### Account shell: one menu, Linear's settings pattern

- **One account menu.**
  - The workspace switcher at the top-left of the rail (avatar + name ▾) holds Settings, Support and Sign out. Delete the bottom avatar popover.
  - Settings becomes its own mode, as in Linear: the rail swaps to a grouped settings navigation with "← Back".
  - Groups: **Account** (Profile, Security, Appearance), **Billing** (Plan), **Integrations** (Connections, AI apps).
  - Home stays out of settings entirely.
- **The rail earns its space.**
  - Home, then the owner's companies as rows, each with a status dot and a count of waiting decisions, then New company.
  - One type scale: 13px rows, 28px height, 6px radius.
  - A plain-text wordmark in the UI font; collapse on ⌘B.
- **The name is asked once, in its place.**
  - Ask in Exec's first conversation, or as one inline row in Profile. Never as a card on Home.
  - If Home must ask, use a slim dismissible line under the header, with proper spacing.
- **Lighter company rows.**
  - Title 20px semibold; rows without borders or shadows, with hover background.
  - The decision count is a small numeric badge, and `…` appears on hover.
  - The summary line drops to one clause.

### Exec questions: one tap, never a wall

- **A choice sends on click,** with a two-second inline "Undo" (Linear's pattern for destructive or quick actions).
  - Remove the separate Answer button. A free-text answer goes in the composer, whose placeholder becomes "Answer Exec, or ask anything".
- **The question is a message,** not a docked panel. It renders inline in the transcript as Exec's message with optional quick replies, and it also lives in the Inbox. The composer is always free.
- **Structured fields are an exception.**
  - Use them only when the answer must be an exact value the system validates, such as a price or a URL. Render them inline in the message and keep them collapsible.
  - Exec asks one thing per question. This is a prompt rule in Exec's operating guidance, not a UI check.
- **Fix the composer toolbar overlap** (item 10).

### Exec voice and coordination noise

- **Owner text is addressed to the owner.**
  - Exec's reply contract separates the owner-facing message from its working note.
  - Only the message renders in chat; the note stays behind "5 actions".
- **Coordination collapses into one line:** "Exec coordinated with Bart · 7 messages", expandable. That keeps outcomes first and puts the mechanics behind a click.
- **Decisions apply.**
  - A confirmed name or charter decision updates the company's identity through the existing setters, and the reply says what changed.
  - `CompanyTitle` uses the same display mapping as Home.

### Company area: settings navigation with to-dos

- **Restore the section navigation at every desktop width beside the Exec rail.**
  - Group the sections, as Linear's settings do.
  - Each row carries its state on the right: a to-do dot or count when something needs the owner, such as Charter not written, Identity not set or the Vault unavailable.
  - The overview becomes a short checklist, "2 things to finish setting up", ordered by what unblocks Exec.
- **Header facts become plain text with tooltips:** status as a dot plus word, spend as a figure, and "Activity" as a link in the navigation, not a pill. Truncated values get a tooltip.

### Library and documents: Notion's model

- **The top bar follows the view.**
  - The title is the view (All, Documents, Sheets).
  - One primary action matches it: "New document" in Documents, "New sheet" in Sheets, and a split "New ▾" in All.
  - Search collapses to an icon until focused.
- **Create opens the item.** "New document" creates an untitled document and navigates straight into it, with the cursor in the title. There is no dialog.
- **The editor looks like a page.**
  - Remove the focus outline on the editable area; keep focus rings for form controls only.
  - Add a separate large title line above the body.

### Sweep for the same classes of defect

Run one walk through every owner surface, at 1440 and 390, signed in on Cloud, and check for:
- duplicated navigation;
- docked panels that block a conversation;
- actions that ignore the current view;
- create flows that do not open what they created;
- focus rings on non-form surfaces;
- mixed type scales;
- truncation without a tooltip;
- third-person agent text addressed to the owner.

Compare against Linear and Notion, and the references in `docs/FRONTEND_DESIGN_REFERENCES.md`.

## Open questions

- Should a choice send immediately with Undo, or after a confirm tap for consequential answers? The proposal is to send immediately for answers, and to keep approvals of consequential effects behind their existing confirmation.
- Should Home list companies in the rail as well as in the page body, or only in the rail once the account has more than one?
