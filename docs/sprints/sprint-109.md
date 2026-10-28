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

## Delivery contracts (implementation review)

- C1: record pending actions durably before removing the card. Undo cancels the pending
  action; downstream work starts only after that window. Refresh, navigation and restart must
  not lose the action. A failed save restores the item and explains the failure.
- A4/B3: accepting a review binds to the exact displayed version. A newer revision requires
  reviewing again. Repair existing empty review cards as well as preventing new ones.
- C7: qualify cold and warm loads, slow responses, desktop and phone widths, and explicit
  loading, empty, failure and populated states of the six primary surfaces and card kinds.
  Save frame/geometry evidence for failures; intentional user-driven movement is distinguished
  from unexpected loading shifts. Grid alignment means named shared edges, not every text edge.
- D2: capture the original Reno replies before changing D1, with fixed prompts, context and
  model route. Record accuracy against verified company facts, repetition and length.
- G1: ship board by default with the first batch. Explicit list links remain list links.
- A2/D3/H2: retain secret usage references, model/computer/key health and the agreed integration
  catalogue (Email, DNS/Cloudflare, domains, payments) in the delivery audit; Resend is the first
  implemented integration.
- H: prepare reviewable resources, email and map designs before implementation; record owner
  review against those concrete designs before building the new surfaces.

## Implementation evidence

The full checklist above remains the completion target, including deployment and production smoke.
Do not check an item from compilation or source inspection alone. Each completed ticket records
its observed scenario, affected layer, removed machinery, and local/production evidence here.

First batch: A, C1, D1/D2, B1-B3 and G1's board default. Subsequent batches complete every remaining
track; batch ordering does not defer any sprint requirement.
### First implementation pass, 28 October

Worktree: `/home/yail/code/work/slot-3`, branch `feat/sprint-109` from origin/main b475dbde.

- G1 default: implemented and observed Board selected at `/sprint109_test/work`; selecting List
  writes `?view=list` and remains selected after reload. Live transition/role matrix pending.
- C3: deleted hint counter, focus bookkeeping and hint CSS; shortcut text is on Send. Focused and
  typed an unsent draft in the real browser; `.hc-hint` count remained zero.
- C5: real synthetic secret saved to isolated company Vault; shared toast observed, then expired.
  Replace/delete/error scenarios and role matrix pending.
- A3: replaced category-derived fallback titles with the handoff's requested action in pending and
  resolved projections; removed `human_step_title`. Rust check and real DNS handoff smoke pending.
- `npm run check`: zero Svelte errors/warnings, type/UI/document-search checks passed.
- Seeded with `scripts/seed-reno-scale.mjs sprint109_test`. `capture-surfaces.mjs` on Work at
  1440 and 390 passed; evidence `/tmp/sprint109-surfaces/{report.json,work-*-light.png}`.
- `verify-motion.mjs open-latest`: passed, opening and return scroll gaps both 0px.
- Local daemon/CLI build passed (3m36s). Local Doctor: runtime/browser/OrgIntel/API available;
  Documents unavailable (identity not provisioned). Resolve before document checks.
- WSL intermittently refused new process connections with WSAETIMEDOUT; subsequent minimal probes
  succeeded. No WSL restart or stable-appliance change was performed.
- No deployment or full-sprint completion claimed. Checklist remains open until its complete
  acceptance evidence, including admin/member and production smoke, exists.
- A3 named Rust regression passed: `preparing_handoff_has_no_external_human_step_and_stable_title`
  (1 passed). This checks preparation-title behavior; DNS action-title projection still needs a
  real handoff smoke and is not complete from this test alone.
- C6: stacked rows now reserve the menu width beside the time; deleted the hover-time hiding and
  overlapping menu placement. Frontend check and browser hover geometry pending.
- The development host stack and the two sprint109_test company containers were stopped after the
  smoke. Seeded test data and its stopped volumes are retained for the next acceptance pass.
  The shared dev Vault/database belong to the existing slot profile and were not removed.
### Second implementation pass, 28 October

- A2: one canonical Resend sending binding; legacy config names migrate at read and are persisted
  on save. Vault accepts dotted names and has Rename beside Replace value. Copy-confirm-rebind-
  delete preserves values and consumers; fixed model/finance references cannot be moved. Failed
  cleanup leaves the old unbound value visible rather than losing the live key.
- A2 real Authority/Infisical round trip passed (`web/scripts/verify-vault-renames.mjs`): canonical
  create, exact value preserved across rename, exact replacement, updated sending reference,
  idempotent same-name rename, collision/stale revision/foreign-path refusal and synthetic cleanup.
  The real browser create and Rename form also succeeded; the row and its consumers updated.
- A1: rail and chip share the selected actor's route calculation. The intelligence API includes
  the observed Exec admission issue used by Doctor; it no longer disables another lead's available
  route. Stop is withheld when a route is unavailable. A reserved line says before sending that
  the message will wait. Missing/starting/Exec-vs-lead route regressions pass. Live native/cooldown
  cases and the complete role matrix remain open.
- B1: Inbox Ask Exec creates a linked draft in the general chat. Legacy item links were opened in
  the real browser: the general Exec history and composer remained. Removed item-scoped query/send
  coordinates, inline Discuss toggles, scoped navigation helper, and document/sheet/file Discuss
  toolbars. Ordinary author-directed answers retain their recipient.
- B5: Skip moved into More. C6 real browser geometry: time stayed visible at x=245.75,
  y=235.09375 before/after opening its row menu. No label swap or movement was observed.
- A3 real DNS handoff projection passed: a synthetic identity-category handoff was refreshed through
  the daemon with action "Add the concepts DNS record at GoDaddy"; Attention returned that exact
  title instead of the category title.
- A5: every rendered inline computer attempts to show the prepared desktop; absent attachments show
  a fixed-size explanation rather than an empty area. The DNS fixture exposed the exact missing-
  attachment refusal. The owner API wording was then made readable; final real-attachment and role
  cases remain open.
- D1/D2: simplified the reply prompt; retained all three real-inference replay outputs. Final
  comparison: 175 to 161 visible words (8% fewer), snapshot facts preserved in 5/5 cases, routine
  no-email statements 3 to 1, Needs you callouts 2 to 0, supplied destinations correct in 2/2.
  The first shortened draft invented two unusable links; the retained evidence shows the correction.
  See `evidence/reply-comparison.md`. This is a five-case writing replay, not production execution.
- Rust checks passed: binding migration (1), typed-send alias guard (1), Vault scope/name boundaries
  (2), and context/audience/authority tests (19). Seven route/preview frontend regressions passed.
  Frontend diagnostics returned zero errors/warnings before the final small markup changes.
- Fresh isolated profile `/home/yail/.restless-dev/sprint109`, namespace sprint109, offset 15433:
  Doctor passed all boundaries, including Documents. The earlier document failure was avoided by
  provisioning an isolated profile rather than reusing the slot's historical company state.
- Work, People chat and Vault captures passed at 1440/390 (`/tmp/sprint109-pass2-surfaces`). Motion
  passed opening/return at latest (0px gaps) and streaming (770px width before/after, 286 samples).
- Wider Inbox footer checks, admin/member browser checks, remaining tracks, debt removal, main
  integration, deployment and production smoke remain required. The full checklist is unchanged.

### Second pass verification completed

- `verify-inbox-actions.mjs`: all 24 combinations passed (six card kinds at four desktop/phone
  and short-height sizes). More contains Skip; declining and charter editing remain usable.
  Evidence: `/tmp/sprint109-inbox-actions`.
- Multiplayer browser journey passed with owner, admin and member, including concurrent native
  documents, invitations, role changes/revocation and owner-only Vault create/rename/delete.
  Evidence: `/tmp/sprint109-roles-02/journey.log`. The runner confirmed removal of its containers,
  volume, databases and roles. This proves the named scenarios, not every sprint requirement.
- Final frontend diagnostics returned zero errors/warnings; frontend production build and local
  daemon/CLI build passed. Production deployment and the remaining sprint checklist are still open.

### Main integration and first deployment

- PR #498 merged the first two implementation passes into main at
  `ffffb56b855dd4ee8b53d0f3b0bac7c5e990b9df`. Main requires a pull request; direct push was
  rejected and no repository rule was bypassed.
- `restless-dev promote` built that exact main commit, passed 23 connection scenarios and skipped
  the separately invoked skills smoke. Connection evidence:
  `/home/yail/.cache/restless/promote/connections-smoke-ffffb56b855d/evidence.json`.
- Appliance activation returned release `9ce1344ccfd4b2f634b0`, ready, no rollback. A fresh
  `restless appliance status` confirmed the matching source commit, live binary/service/socket,
  owner health 200, no drain or repair. `/api/companies` returned 200.
- Read-only browser smoke on the installed Aris company: `/aris/work` opened with Board selected
  and its Work board rendered. No simulated outcome or fixture mutation was made in a live company.
- This proves deployment to the local stable appliance. Its company list has no Reno company;
  the triaged Reno production URL has been requested for the final scenario smoke. Do not treat
  this local release check as proof that the original Reno problems are resolved.

### Filter and board follow-up (local verification)

- B4: Open/Skipped/Done/All now live in the Inbox header filter popover. Skipped uses an inline
  restorable list; the old bottom drawer is removed. Done uses the existing paginated decision
  history and links to the exact Activity record. The clear Inbox links to All so history remains
  reachable. The filter is in the URL; account preference synchronisation remains F2 work.
- `verify-inbox-filters.mjs` passed at 1440/390: all four views, reload, a real temporary dismissal,
  and restoring the original request. All fixture mutations were restored. Eight captures across
  those views and widths passed (`/tmp/s109-filter-surfaces`).
- A forced unavailable history read showed the failure notice without claiming "No completed
  decisions". The regression covers this unknown-versus-empty boundary.
- G4: empty board columns reserve 106px rather than the space of populated columns (192px in the
  observed narrow desktop pane). Titles stay within two lines, with their full text on hover and
  in Work detail. Work captures passed at 1440/390 (`/tmp/s109-board-shape`). This changes the card
  presentation, not existing authored Work titles.
- Final frontend checks and production build passed. A fresh authenticated multiplayer journey
  also passed with the new filter views: owner All loads both lists, a member does not load owner
  lists, and the existing admin screens, Vault refusals and live membership-revocation checks
  passed. Evidence: `/tmp/sprint109-roles-03/journey.log`; runner cleanup was confirmed.
- New surface layouts are prepared for owner review, with Resources, Resend setup/test/ready, and
  map click-through. All three passed interaction/overflow checks at 1024/390/320. Values are
  explicitly illustrative; real resource prices have not been invented. H remains unapproved and
  unbuilt. The full sprint remains active, including C1 (`instant-decisions.md`) and all unchecked
  acceptance requirements above.

### Installed smoke found a missed computer state

- PR #499 landed the filter/board batch at `02e6684c0730c32851b6d725b4afeed11bf22c35`.
  Promotion passed the same 23 connection scenarios (separate skills smoke skipped), activated
  release `cfa40c1e87961969e3e6`, and fresh status confirmed ready with owner health 200.
- The installed Aris smoke then failed: a real attached computer triggered an older `:has()`
  layout rule that collapsed the Inbox sidebar to 2px and hid the filter. This was a desktop
  failure as well as a potential phone failure; the first progress note misidentified the width.
  Earlier fixtures had an unavailable computer placeholder, so they missed the connected state.
- Deleted the automatic collapse. The existing explicit computer Expand control remains the way
  to use more space. Added a real attached-computer fixture and a regression that waits until the
  desktop connects, measures the sidebar, uses its filter, then checks phone All/Done navigation.
- That real-computer regression and the full filter/restore/unavailable-history journey passed
  on a fresh isolated `sprint109final` profile. Frontend diagnostics passed. This correction still
  needs its own main integration, promotion and installed smoke before B4 is complete.
