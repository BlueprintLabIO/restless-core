# Sprint 109: no jank, one Inbox, the card is the work

Owner feedback from a day running the Reno company on production (27 Oct 2026), triaged on 28 Oct.
The full triage, with every screenshot finding numbered 1 to 57, is in the owner's
`Documents/restless-triage-2026-10-28.md`. Numbers in brackets below refer to it.

Owner decisions (28 Oct):

- **All jank is banned.** A screen appears in its final, styled form straight away. Nothing jumps,
  shifts, swaps style or blocks on a spinner. Jank is a bug, not polish.
- **Anything waiting for the owner is an Inbox item**, including a Brand rule waiting for an OK.
- **The structured Exec reply goes.** It makes the models less accurate and the chat cluttered.
- **Work opens on the board**, not the list.
- The improvements proposed in the triage are agreed.

Rule for every track: verify on a local stack with real-scale data in the built-in browser, as
owner, admin and member, before calling anything done; a release is cut from main only.

## Track A: broken or blocking (first)

- [ ] **A1 One source of model status [1, 36, 37].** The rail shows "Connect a model" and "Eames can
  reply after intelligence access is restored" while the composer shows Claude Opus 5.5 and a stop
  button with nothing running. Find which read is wrong (`ExecutiveRail.svelte` 1259-1276, 1727;
  composer streaming state) and make rail, composer, card and Doctor read one status. A message to
  a lead that can't reply says so before you send, never after. Every waiting item and card also
  says in one line why it's waiting and on whom.
- [ ] **A2 Secret names agree [2, 12].** The vault saved `resend`; sending looks for
  `resend.production`. One naming rule owned by Restless, a migration for existing keys, and
  edit and rename on vault rows.
- [ ] **A3 Card titles say the action [27].** Steps titled "Confirm your identity" were a GoDaddy
  record and a Cloudflare record. The title comes from what the step asks, not its kind.
- [ ] **A4 A review card carries its work or doesn't ship [8, 30, 31].** Review cards render the
  output inline (doc, page preview, result). A card with no linked preview is refused at creation
  and goes back to its author, instead of showing "No prepared preview was linked" and a Try again
  that can't help. "Ready for you" is a review, not a Question.
- [ ] **A5 Lead chats and step cards stay usable [28].** A step that says "here in the card" shows
  the company computer, or says why it can't.

## Track B: one conversation, one Inbox

- [ ] **B1 One Exec chat [5, 10].** Items never open a new item-scoped Exec chat. "Ask Exec about
  this" links the item into the one chat. Discuss is removed from cards and the doc toolbar.
- [ ] **B2 Everything waiting on the owner is an Inbox item [9, 42].** Brand rules waiting for an
  OK first, then any page-held approval. Exec can read what's in the Inbox, so it never says "if a
  card is there".
- [ ] **B3 One item per thing [7, 34].** A change to the same doc or Work amends its item; FYI
  notices ("changed", "shared", "Can edit") leave the Inbox for a short daily digest in the Exec
  chat. The Inbox count means "things only you can do".
- [ ] **B4 Filter, not a Skipped drawer [20, 35].** A filter icon in the Inbox sidebar header
  (Open, Skipped, Done, All) replaces the clipped Skipped panel.
- [ ] **B5 Fewer card actions [32].** Accept and Request changes visible; Skip, snooze, hand over
  and copy in the `…` menu. The menu opens in a top layer and is never cropped [14].
- [ ] **B6 Key requests have one home [11, 53, 54].** A per-company key lives in the company
  vault, and the card says so (no "Also in Settings › Accounts"). The card's words match its form.
  A request for a key already in the vault is not raised.

## Track C: instant and still

- [ ] **C1 Mark done is instant [6].** The card leaves at once with an Undo toast; the server call
  waits out the Undo window in the background (`InboxDetail.svelte` `markDoneSoon`). Same for
  approve, skip and accept. The next item is already loaded, so it shows at once.
- [ ] **C2 Docs open styled [13].** No plain frame before the editor: a skeleton in the final
  layout, or the editor's styles applied before it hydrates.
- [ ] **C3 Nothing shifts under the composer [19].** The key hints move to the send button's
  tooltip. Conditional text has reserved space or none.
- [ ] **C4 Jump to latest [38].** Only when scrolled up, and never over a message or button.
- [ ] **C5 Toasts, not banners [55].** "Secret stored securely" goes after a few seconds.
- [ ] **C6 Rows don't swap [33].** Inbox rows keep the time and show `…` beside it on hover.
- [ ] **C7 Zero-jank gate.** A headless pass over every page and card that fails on any layout
  shift, any unstyled frame or any edge off the grid. Runs in release qualification.

## Track D: plain replies and owner-ready writing

- [ ] **D1 Drop the reply structure [18, 40, 41].** Cut `prompts/converse-with-owner.md` to a few
  rules: answer the question plainly, be accurate, use "you", link what you mention, no internal
  ids. No required opener, no bold, no closing limit line. The intent marker stays as hidden
  metadata and never shapes the text. Keep `restless-details` only as the Work details fold.
- [ ] **D2 Measure it.** Run the same Reno prompts before and after; compare accuracy, repetition
  and length. Keep the result in this file.
- [ ] **D3 Agents know Restless [3, 15].** A capabilities read (what's built in, what's connected,
  what each key is called) that Exec checks before stating facts about Restless. Faults it finds in
  Restless are filed to the Restless company with evidence.
- [ ] **D4 Owner docs are owner-ready [44, 45].** No Work ids, message numbers, CLI commands or
  `[LINK TO …]` placeholders in a doc sent for review; the content comes first and revision notes
  go to history. Checked when the doc is shared, not after.

## Track E: docs like Notion

- [ ] **E1 One header row [16, 43].** Breadcrumb with a back arrow, toolbar on the same row,
  "Saved" inside it.
- [ ] **E2 Floating actions [17].** Archive and Back to Inbox float over the doc's bottom-right.
- [ ] **E3 Blocks [22, 46, 47].** Every list item has its own drag handle with a hover zone that
  reaches it. Checkboxes line up with the first line of text, gaps are even, the block toolbar
  never covers text.
- [ ] **E4 Comments in the margin [23].** Beside the text they belong to, not a separate page.
  A margin comment goes to the doc's author, who replies with tracked changes to accept in place.

## Track F: layout and chat

- [ ] **F1 Grid tokens [26].** Header height, row heights, gutters and paddings as tokens; the
  left sidebar's default right edge lines up with the left edge of the top tabs [21].
- [ ] **F2 Preferences follow the person [21].** Sidebar and pane widths, the Work view, filters
  and folds are saved to the account, the same in every company and on every device (today widths
  are localStorage per company and area).
- [ ] **F3 Chat spacing [24].** One spacing scale for paragraphs, lists, Work details and footers.
- [ ] **F4 Chat header [25, 39].** Fixed row heights; no tooltip over "About: …".
- [ ] **F5 Clipping [29, 49].** The stray chip in the card footer and Library's cut-off "Archived".

## Track G: Work and the Library

- [ ] **G1 Board by default.** `routes/[companyId]/work/+page.svelte:117`: board unless
  `?view=list`. Cards move across columns live as Work changes, without a reload.
- [ ] **G2 Leads delegate [4].** A lead splits independent Work across Staff in parallel. The board
  flags a card waiting in Next while its owner has three or more in motion.
- [ ] **G3 Churn shows [50].** The board flags Work past a few revisions (A$500 pass was at 15), and
  the lead says why.
- [ ] **G4 Board shape [51, 52].** Empty columns are narrow; card titles are short.
- [ ] **G5 Scratch out of sight [48].** Agents' working notes leave the owner's default Library
  view. Check the "Folders 2000" count, which looks like a cap.

## Track H: new surfaces (design first, build after review)

- [ ] **H1 Resources page [56].** Per company: storage, CPU and RAM in use against limits, history,
  what's using it, and one clear way to buy more with prices. Linear-quality design, reviewed by
  the owner before it's built.
- [ ] **H2 Email integration [15].** Resend as a first-class integration: guided setup, domain and
  DNS records, a test send, status, sending limits. Built on A2 and D3.
- [ ] **H3 Interactive maps [57].** Concept page maps get basic zoom, or a click through to Google
  Maps.

## Later (agreed, not this sprint)

Grouped decisions, keyboard-first Inbox, watching work happen live, cost and time on every card,
deciding from a phone, a company timeline. See "10x and broader improvements" in the triage.

## Outside the code

- [ ] Add "all jank is banned" to the Restless company's house rules, where the owner's rules now
  live (needs the owner's OK).

## Order

A first (it blocks real work), then C1, D1 and B1-B3 (the biggest felt changes), then the rest of
B, C, E and F, then G, then H once its designs are approved. Ship in batches, about twice a day.
