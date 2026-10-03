# Cockpit UX audit — 2 October 2026

This audit covers the Attention sidebar, the AttentionCard, the attention blocks in conversations, and
every Company page. The bar is Linear: dense but calm, one clear primary action, keyboard-first, quiet
chrome, and states that never contradict each other.

The audit was run against the live local cockpit (`localhost:7788`, company `sydney_resale`) at 1440×900
and 375×812, plus source on `dev` at `fcb5720`. That branch is 10 commits behind `origin/dev`, but those
commits only touch Sheets and Docs and none of the surfaces here. No code was changed.

Severity: **P0** breaks trust or the core loop · **P1** clear UX failure · **P2** polish.

## Status — 3 October 2026

Most findings below were addressed by `fda7685` ("Overhaul owner Attention and Company cockpit").
That commit delivered the grouped Attention inbox with snooze, undo, unread and row menus, one decision
ask with structured fields, the chat "N need you" strip in place of stacked cards, shared Company
headers and menus, the Access & limits tabs, the Doctor fold and aligned provider health.

A re-audit of `origin/dev` (`f1d25cf`) against recorded live data found these remaining gaps. They
were fixed on `feat/cockpit-overhaul`:

- **Retry storm.** A failed room read-cursor write re-armed its own effect and retried about 300 times a
  second. It now backs off exponentially.
- **Truth.** "Reconnect ChatGPT / Codex" was shown when the account sign-in worked and the company had
  simply not been given it. Companies now offers _Use your ChatGPT / Codex sign-in_, which deep-links to
  a preselected grant. Connections lists each company that can't start, with one **Give access** each.
- **Duplication.** The decision box's placeholder repeated the whole ask. One connection label was
  doubled ("ChatGPT / Codex · ChatGPT / Codex").
- **Layout.** "Copy from…" menus took whole rows of their own; they are now inline in heading rows with a
  floating panel. The Doctor failing row was crushed into a narrow column. The Members avatar and name
  sat apart. Spend put its edit control above the figure. The Computer page drew a second navigation
  bar inside the Company spine. The `?` tip fell below the decision title.
- **Hierarchy and copy.** An unused, expired company-only profile no longer gets a primary button. The
  nav group "Intelligence" contained an item also called "Intelligence"; the group is now
  _Capabilities_. The shortcut caption became a tooltip. Fixed "1 need(s)" grammar and the missing
  space in "bought it· Owner".
- **Account shell.** Companies and Settings now share Cloud's account shell: a Companies link and an
  account menu in place of a lone Settings button, "New company", in-flow notices, a status footer and
  archived companies that can be shown and restored. Connections is one row per provider. Older
  company-only sign-ins fold into a disclosure with a count.

---

## 0. Cross-cutting problems (fix these first, they cause most of the page-level issues)

| #   | Sev | Problem                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Evidence                    |
| --- | --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------- |
| X1  | P0  | **States contradict each other.** The blocker banner says "Connect the selected intelligence provider". Intelligence shows ChatGPT/Codex as _Access granted · In use_ and also _Account sign-in unavailable_. Resources shows the model as _configured unprobed_ with credential reference _absent_. Doctor says _Degraded_ but never names the provider. The Exec rail shows **+ Add intelligence provider** on most pages but a working composer on Intelligence. | live, all pages             |
| X2  | P0  | **The same information is shown over and over.** One owner need can show up as: an Attention sidebar row, the Folio, the card's `.request`, the card's `ifNoAction`, a chat card, a "Needs you" glance on the message, the ⌘K "Needs you" group, the top-bar badge and the blocker banner. Nothing removes the copies.                                                                                                                                              | §2, §3                      |
| X3  | P1  | **There's no shared page shell for Company pages.** Header styles vary page to page: title plus `?` InfoTip, then sometimes "Live", sometimes a Refresh button, sometimes Edit. Content widths vary, sections use different header styles, and empty states use three different treatments: a grey box (Members), a card with a CTA (Identity) and plain text (Vault, External activity).                                                                           | Company tab                 |
| X4  | P1  | **Secondary actions are shown as primary buttons.** "Copy _X_ from…" appears 6 times (Charter ×3, Intelligence ×1, Limits ×2), each as a full button inline with the content. Linear puts actions like these in a `⋯` menu or ⌘K.                                                                                                                                                                                                                                   | `CopyCompanySetting` usages |
| X5  | P1  | **Raw internals are on owner-facing screens**, which CLAUDE.md forbids ("no status walls… workflow IDs and logs only on request"). Examples: `SOURCE orgintel / handoff… · blocking`, `openai-codex/gpt-6-sol`, `model.inference.openai-codex`, `openbox`, `desktop-panel`, `release-health`, and dozens of rows reading "continue active Exec milestone… schedule · _mac must remain awake_ · orgintel".                                                           | Resources, Folio details    |
| X6  | P1  | **Timestamps are absolute and inconsistent.** Formats include "28 Sept, 17:24", "28 Sept 2026, 17:18", "Effective 28 Sept 2026, 14:41" and "28 Sept, 0:25" (no zero-padding). Every page has its own `when()` (6 copies). Linear uses relative time ("4d") with the absolute time in a tooltip.                                                                                                                                                                     | `when()` in 6 routes        |
| X7  | P1  | **Keyboard support exists but nobody can find it.** Attention supports J/K/↑/↓, but nothing shows that. There are no shortcuts for actions such as approving, opening, discussing, snoozing or copying a link, and no `?` cheat-sheet. Company pages have no shortcuts at all, for example jumping between pages or `E` to edit.                                                                                                                                    | `+page.svelte:278`          |
| X8  | P2  | **The icon language is off.** Every attention row uses the same `GLYPHS.rules` glyph whatever its category. The kind is shown three ways at once: tone colour, glyph and text label. The blocker banner's glyph is an unreadable 7px matrix dot.                                                                                                                                                                                                                    | `+page.svelte:782`          |
| X9  | P2  | **The Exec rail jumps.** On every Company page change, the rail lands at a different scroll position mid-conversation instead of staying put or showing the latest message.                                                                                                                                                                                                                                                                                         | live                        |

---

## 1. Attention sidebar (`routes/[companyId]/+page.svelte:755–806`, `cockpit.css:542–830`)

What it does today: a 280px column of rows, each 119px tall with 13/14px padding. Each row has a
kind label and date, a 2-line title and a 2-line generic action. The amber "Can't start yet" card sits
on top.

| #   | Sev | Problem                                                                                                                                                                                                                            | Target (Linear inbox)                                                                                                                                                                                                 |
| --- | --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A1  | P0  | **The list has no hierarchy.** Five items, three of them system/runtime blockers with no owner action ("Resume the active Exec milestone after a bounded retry delay" ×2), get the same weight as a real $1,000 purchase decision. | Group by what the owner must _do_: **Decide · Approve · Review · Fix setup**. Give each group a count header. System self-recovery items go into a single collapsed "Company is recovering" row, or go away entirely. |
| A2  | P0  | **Duplicate items.** Two rows have the same title and the same "Review the blocked opportunity" action (28 Sept 13:33 and 14:24).                                                                                                  | Fold duplicates into one row with a "×2" count. This also needs a source-side dedupe.                                                                                                                                 |
| A3  | P0  | **The blocker is shown twice.** The amber "Can't start yet" card and the "Exec needs a runtime repair" row describe the same root cause, and both point at Intelligence. The blocked rows below are also caused by it.             | One root-cause row, pinned at the top, with its dependents nested under it ("3 items waiting on this").                                                                                                               |
| A4  | P1  | **Titles repeat the kind.** "Blocked: Refresh…" sits under a red "Blocked" label. The second line is boilerplate: "Review the blocked opportunity." appears 3 times and adds nothing.                                              | Strip category prefixes from titles. Only show a second line when it adds specific information, such as an amount, a deadline or a counterparty. Otherwise use a single line.                                         |
| A5  | P1  | **Rows are too tall.** 119px rows mean only about 5 fit. The title (13px/600) and the meta (11px) are too close in size, and the meta row takes a full line.                                                                       | About 44–56px rows. Order: kind icon, title (1 line, ellipsis), then right-aligned relative time. Put the optional detail line in tertiary text. Hover shows full text in a tooltip.                                  |
| A6  | P1  | **No quick actions.** The only thing a row can do is select.                                                                                                                                                                       | Show hover/focus actions on each row: Open, Discuss with Exec, Snooze, Copy link, Done/Dismiss when allowed. Add a right-click context menu, and shortcuts (`E` done, `H` snooze, `C` discuss, `⌘.` copy link).       |
| A7  | P1  | **No list controls.** There's no count, filter, sort, or "showing N of M".                                                                                                                                                         | A thin header (`Attention · 5` plus filter chips or a `⋯` menu). It should be sticky.                                                                                                                                 |
| A8  | P1  | **The selected state is muddy.** It uses a 9% tone tint, a highlight blend and a 3px inset bar. For a "Decision" the tone is purple, so the selected row looks like just another tinted block.                                     | A neutral selected fill (`--surface-alt`) plus a 2px accent bar. Keep tone colour for the icon only.                                                                                                                  |
| A9  | P1  | **Mobile is broken.** At 375px, the amber card and one item take the whole visible list band (about 170px). The detail pane below starts with a 3-line 34px H1. You can't see the queue.                                           | On mobile, list and detail should be separate screens (list → push to detail, with a back button), as Linear mobile does.                                                                                             |
| A10 | P2  | **Stale links fail silently.** `?item=<missing>` shows the office with nothing selected and no message.                                                                                                                            | "That item was resolved" toast, then select the next item.                                                                                                                                                            |
| A11 | P2  | **No feedback on resolution.** When an item resolves, `listOut` fades it. The focus doesn't move to the next item and there's no undo.                                                                                             | Auto-advance to the next item, with an "Undo" toast for about 5s where the action can be reversed.                                                                                                                    |
| A12 | P2  | **Unread state isn't shown.** Rows give no sign of being new or updated since last viewed.                                                                                                                                         | Unread dot plus bold title; it clears when selected.                                                                                                                                                                  |
| A13 | P2  | The amber banner's 7px glyph and 26px tinted square are too faint to read. The wording "Connect the selected intelligence provider" doesn't say which provider.                                                                    | Name it ("ChatGPT/Codex sign-in expired") and add an inline **Reconnect** button.                                                                                                                                     |

---

## 2. AttentionCard (`lib/components/AttentionCard.svelte`, plus `OutcomeFolio`)

The detail pane for one decision stacks these blocks, captured from the DOM in order:

1. H1 title (repeats the sidebar row)
2. What happened (paragraph)
3. Why it matters (paragraph)
4. **Uncertain:** paragraph
5. **Recommended** paragraph: "Share the final amount paid, handover date…"
6. Card `.request`: "Please confirm the Oatlands PC final amount paid, handover date…" (**the same ask as #5**)
7. Discuss button
8. "Your decision" textarea plus a disabled "Record decision" button
9. `ifNoAction` paragraph
10. Details · 2 items (prepared by, InfoTip, evidence, a raw `SOURCE …` line)

| #   | Sev | Problem                                                                                                                                                                                                                                                                                                                                 | Target                                                                                                                                                                        |
| --- | --- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C1  | P0  | **The ask appears twice** (#5 and #6), and both versions are long. The owner reads about 180 words before reaching the input.                                                                                                                                                                                                           | One **Ask** block: a single bolded sentence. What happened and why it matters go into a collapsed "Context" section. Uncertainty becomes one inline line.                     |
| C2  | P0  | **There's no structured input for structured asks.** The card asks for amount paid, date, pickup cost and drives, then gives you one free-text "Tell the team how to proceed…" box.                                                                                                                                                     | Render fields the Exec has declared (amount, date, yes/no, choice, file). If there's no schema, keep the textarea but put the asked-for facts in its placeholder or as chips. |
| C3  | P1  | **Up to 9 sibling buttons are rendered unconditionally.** They come from an `{#each}` over actions plus 8 `{#if}` branches, all marked `btn primary`, and `.actions .btn.primary` forces each to 42px with `--intent-conversation` fill. Several primaries can show at once (e.g. "Open …" plus "Review outcome" plus "Open document"). | Exactly **one** primary per card, chosen by the model. Everything else goes into a single secondary row or a `⋯` menu.                                                        |
| C4  | P1  | **Every action uses the same button.** Navigation ("Open Intelligence provider"), authority (hold to approve) and conversation (Discuss) all look the same. Primary buttons are a dark slate fill, as heavy as a destructive confirm.                                                                                                   | Use a distinct look for each: link-style for navigation, hold-to-approve for authority, ghost for Discuss.                                                                    |
| C5  | P1  | **The email mandate warning is a 4-sentence wall of text.**                                                                                                                                                                                                                                                                             | One line plus an InfoTip, with the limits shown as a compact key/value list.                                                                                                  |
| C6  | P1  | **The `ifNoAction` text and "Review the outcome to accept it or request changes"** are always-on `.quiet` paragraphs.                                                                                                                                                                                                                   | Put them in a tooltip on the primary button, which CLAUDE.md requires ("tooltips… instead of subtitles").                                                                     |
| C7  | P1  | **Details exposes internals**: `SOURCE orgintel / handoff:… · blocking`, and evidence labels in letter-spaced mono.                                                                                                                                                                                                                     | Hide the raw source line behind a "Copy reference" action. Make evidence a list of linked chips.                                                                              |
| C8  | P1  | **System blockers get a card with nothing to do.** "Blocked: Resume the active Exec milestone…" offers only "Review opportunity", which leads back to the same content.                                                                                                                                                                 | Don't create an owner card unless there is an owner action.                                                                                                                   |
| C9  | P2  | Header status text ("Needs you", "Saving decision…") is 11px tertiary text pushed to the far right, where it's easy to miss during a write.                                                                                                                                                                                             | Show the state on the button itself (spinner, then a check), as Linear does.                                                                                                  |
| C10 | P2  | The textarea is 2 rows, can be resized, and has no ⌘↵ to submit or shortcut hint. Record decision is disabled with no hint as to why.                                                                                                                                                                                                   | Auto-grow the textarea, add a `⌘↵` hint, and give the disabled button a tooltip.                                                                                              |

---

## 3. Attention and summary blocks in conversations (`RoomConversation.svelte:1218`, `ConversationMessage.svelte:149`)

| #   | Sev | Problem                                                                                                                                                                                                                                                                                                                                                                                                           | Evidence                   | Target                                                                                                                                                                                                                                                         |
| --- | --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | P0  | **Every open attention item for the lead is added to the end of the transcript as a full card.** In the Exec chat that's 5 cards totalling about 1,013px (297 + 4×179) between the last message and the composer. Each has its own Details disclosure and a 42px primary button. This happens on every visit and never collapses.                                                                                 | screenshot, `chatCards: 5` | **Remove.** Replace with one slim strip above the composer: "5 need you · Oatlands decision →", linking to Attention. Show a card inline in the transcript only at the moment the Exec raises it, and turn it into a one-line "Resolved ✓" receipt afterwards. |
| R2  | P0  | **Two of those five cards are identical** ("Resume the active Exec milestone…").                                                                                                                                                                                                                                                                                                                                  | same                       | Falls out of A2/C8.                                                                                                                                                                                                                                            |
| R3  | P1  | **The "At a glance" `<dl>` (Outcome / Next / Needs you) restates the message body.** There are 16 instances in the Exec chat and 34 in the rail, each 68px, so about 1,100px of repetition in one thread. For example, the body says "I've updated the operating plan and passed your rescreen to the resale lead…" and the glance says "Outcome: Operating plan updated and rescreen routed to the resale lead". | DOM sample                 | **Remove from the transcript.** If it's worth keeping, show it only on the _latest_ Exec message, or as the collapsed preview of a long message.                                                                                                               |
| R4  | P1  | **The same Exec conversation is shown in two places.** On Attention and Company pages it's the right rail, with 100 messages loaded. In People › Exec it's the main pane. Both are rendered from the same thread, each with its own appended cards and glances.                                                                                                                                                   | live                       | Pick one place. Open People › Exec as the rail expanded, not a second render.                                                                                                                                                                                  |
| R5  | P2  | "Read full message" folds on the owner's own messages, and the trailing "Work details" disclosure on agent messages, add more chrome to every message.                                                                                                                                                                                                                                                            |                            | Show message actions on hover, as Linear comments do.                                                                                                                                                                                                          |

---

## 4. Company tab, page by page

### Navigation spine (`company/+layout.svelte`)

- **P1** 12 pages in 3 groups (Setup / Operations / Activity). The group headings are eyebrow labels, which CLAUDE.md discourages. "Access & limits" is the old Authority + Resources pages merged, which makes it the heaviest page. Proposed regrouping: **General** (Charter, Identity, Members), **Intelligence** (Provider, Skills, Vault), **Operations** (Schedules, Limits, Computer), **History** (Decisions, External activity), **Health** (Doctor, as a status dot rather than a page in the list).
- **P1** The nav shows no state at all. Intelligence and Doctor are failing, but their nav items show no dot or count. Linear shows status in the nav.
- **P1** **Computer** drops the spine (`company-focus-shell`) and shows "Company / Doctor" crumbs instead. The user leaves the settings frame with no consistent way back.
- **P2** Mobile has a native `<select>` page picker that takes a full 100px band above every page.

### Charter

- **P1** The charter is a ~400-word block of unstructured text, shown as one paragraph with no headings. It's the main thing on the page and impossible to scan. Parse it into fields (Mission, Priorities, Rules, Guardrails), or render it as Markdown with a collapsed view after about 6 lines.
- **P1** Three "Copy … from…" buttons sit under the content (X4). The company name is shown as an H3 with a button below it.
- **P1** "Quality bar" is a single `<select>` with "New work should be" as its label. It takes a full section for one control. Make it a segmented control in a settings row.
- **P2** "Effective 28 Sept 2026, 14:41 · Owner authorised" is pushed down to footer text; it should be a version chip next to Edit that opens the history.
- **P2** There's no inline edit. "Edit charter" switches modes. Linear lets you edit in place and saves automatically with a quiet "Saved" note.

### Identity

- **P2** The empty state works, but "Not written yet" plus an InfoTip plus a 2-line explanation sits in a card in an otherwise empty page. Offer **"Ask Exec to draft"**: the product's thesis is that intelligence does the work, yet the only option is "Write identity" by hand.

### Members

- **P1** "You · Owner" is shown as loose text with no avatar or row structure. The grey box "This company is local-only, so nobody else can be invited yet" is a dead end. Use a table row with an avatar, and a disabled "Invite" button with that text as its tooltip.

### Skills

- **P1** This is a flat list of 8 rows. Each row has a truncated description and the same "Restless" source label repeated 8 times. There's no search, no on/off control, no usage count ("used in 3 Work") and nothing happens on hover. Click does nothing visible.
- **P2** The "Company skills" header is an eyebrow above a list that has no other group.

### Intelligence (`CompanyProvider.svelte`, 1,448 lines)

- **P0** One row shows contradictory states (X1): _Access granted · In use · Account sign-in unavailable_. There's no **Reconnect** button in the row, although this is the company's single blocking problem.
- **P1** Page actions "Copy model choices from…" and "Manage account connections ↗" sit in the header as equals. "Add API key" sits inside a card. "Bring in an API connection from another company" is a plain text line that is actually a button.
- **P1** "12 more agents use the company default" is unclickable text. It should expand to show the agent overrides.
- **P1** "Company-only connections and advanced settings" was still a skeleton 2s after the page loaded. Show the content or explain why it isn't there.
- **P2** Model IDs are shown raw and inconsistently cased (`gpt-6-sol` vs `GPT-6 Sol`).

### Vault

- **P1** The page is mostly text: "Secure storage connected", "Secrets stored… Values remain hidden.", "No secrets stored for this company yet.", a disclosure, and a link. There's no "Add secret" button, no table, and Refresh is the only header action.
- **P2** "Manage intelligence connections →" duplicates the Intelligence page.

### Schedules

- **P1** The schedule title is the owner's raw request sentence ("Owner requested continued bargain sourcing for TV GPU…"). Next and last fire times are in separate paragraphs. "Test trigger" sits inline as a peer of the times. It should be in a `⋯` menu (confirmation behaviour not checked).
- **P1** Recent runs show `blocked · absolute outcome deadline expired` and `bounded wake delivery budget exhausted`, which are engine phrases. Both recent runs failed, but the schedule shows no warning. Use a row with a status dot, a human-readable reason and a "Fix" link.
- **P2** There's no pause/resume toggle or cadence editor, and no way to create a schedule from the page.

### Access & limits (`resources/+page.svelte` 741 lines + `CompanyLimits.svelte` 443 lines)

- **P0** This is the worst page. It holds spend, email authority, computer runtime limits, external parties, payment allowances, MCP connections (3 large cards), "Resources & access", and then **a raw service and status table**: chromium, openbox, desktop-panel, release-health, plus dozens of "continue active Exec milestone… · mac must remain awake · orgintel" rows. The page is many screens long and is exactly the "status wall" the product rules forbid.
- **P1** Split it into tabs or sub-pages: **Spend**, **Authority** (email, parties, payments), **Tools** (MCP), **Computer**. Move the service and status table into Doctor behind "Show raw state".
- **P1** The spend block repeats itself: $0.00 Spent / $10.00 Limit / $10.00 Left. Show one progress bar ("$0 of $10 · resets in 12d") with Edit next to it.
- **P1** Each MCP card has 4–5 key/value pairs, 3–4 buttons ("View Core receipts", "Re-probe and assign Work", "Disable connection", "Connection details") and a status sentence. "Last read: complete" and "on facebook-marketplace" run together in the page text, so check the spacing. Use a single row per connection: name, a status dot, last-call time and a `⋯` menu.

### Computer

- **P1** The page has a different layout from the rest of Company (no spine). Its controls are "Viewing only" and "Enter computer". "1 prepared handoff in Attention" is plain text, not a link.
- **P2** Screenshots timed out twice while this page rendered. Check whether the VNC/desktop iframe blocks paint on load.

### Doctor

- **P0** Doctor says _Degraded_ but lists 10 checks in a fixed order. The one failing check ("Computer version: Needs a rebuild") is fifth, with nothing marking it. **Doctor doesn't check the intelligence provider at all**, which is the actual reason the company can't start.
- **P1** "Checked by the company computer" is repeated on 8 of 10 rows. Sort failing checks to the top with a red dot and their own Fix action. Collapse the passing checks into "8 checks passing".
- **P1** "Rebuild company computer" sits at the bottom as a peer of the checks. It should be the Fix action on the failing row, with a confirmation step.

### Decision history

- **P2** The list is fine. It shows title and date only, and doesn't say what was decided. Add the outcome ("Approved · A$1,000"), the decider, and a link to its Work. "Review 5 pending in Attention" is useful; make it a pill in the header.

### External activity

- **P2** The empty state is a 2-sentence paragraph. Use a centred empty state with an icon and a single line, and put the explanation in a tooltip.

---

## 5. Other cockpit gaps noticed along the way

- **P1 Companies home**: the "Next" column for broken companies says "Connect the selected intelligence provider in Company → Intelligence", which is an instruction, not a button. The "Fix setup" link should go straight to a reconnect action. "Untitled project" sits in the list with no row actions for rename or archive.
- **P1 Top bar**: "Exec" shows a grey dot even though the Exec is down. The Attention tab has a 5 badge, but the Company tab has no indication that setup is broken.
- **P1 Exec rail**: when the provider is down, the composer is replaced by a full-width "+ Add intelligence provider" button. That's the right instinct, but the label is wrong because a provider _is_ added. It should say "Reconnect ChatGPT/Codex".
- **P2** The `?` InfoTip is used on almost every heading, sometimes twice per section, which makes it visual noise. Keep it only where the term is ambiguous.
- **P2** The design system is split between two large stylesheets (`cockpit.css` 2,598 lines, `company.css` 2,229 lines) plus per-component styles. Several rules are `.bridge-root`-scoped overrides of primitives. This is the root cause of X3. Consolidate page header, section, row, empty-state and settings-row primitives in `lib/ui`.

---

## 6. Suggested overhaul order

1. **Truth pass (X1, Doctor, Intelligence row).** Use a single provider health signal that every surface reads from: banner, nav dot, Doctor check, rail CTA and the Intelligence row.
2. **De-duplicate attention (A1–A4, C1, C8, R1–R3).** Root-cause grouping, source-side dedupe, no owner cards for system self-recovery, remove the chat card stack and the glance blocks.
3. **Attention list and card redesign (A5–A13, C2–C10)**, built to Linear inbox density and keyboard model.
4. **Company shell primitives (X3, X4, X6)**: page header, settings row, section, empty state, `⋯` menu, relative time. Then rebuild the pages in order of how bad they are: Access & limits → Doctor → Intelligence → Schedules → Charter → Skills → Vault → Members → Computer → History pages.
5. **Final polish pass**, as `FRONTEND_DESIGN_REFERENCES.md` requires, at 1440/390/320px.
