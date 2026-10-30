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

## Priority A0: resource safety, compute efficiency and AI token efficiency

Owner priority, 29 October, following the production outage: deliver this before the remaining
sprint tracks. This promotes H1's approved Resources design into the first delivery batch and adds
explicit compute and AI efficiency acceptance. The other sprint requirements remain open.

Owner clarification, 30 October: memory or CPU pressure must throttle the owning account, never
take down the whole site. The account budget covers all its companies and workloads together;
company and computer limits sit within it. Protect sign-in, navigation and Inbox access, and prove
one pressured account cannot consume another account's capacity or the platform's reserve.

- [ ] **A0.1 Measure the real baseline.** Record idle and active CPU/RAM/storage per company and
  service, host headroom, swap, browser count and concurrent work. For AI record calls, input,
  cached input, output, retries and actual cost per useful completed outcome. Retain before/after
  results for the same workload, model route and quality checks; missing measurements stay unknown.
- [ ] **A0.2 Lightweight idle companies.** Start desktops/browsers and other heavy services only
  when their capability is requested. Release idle resources safely, preserving profiles, saved
  work and prepared human steps. An idle company must not require a running Chrome desktop or
  spend AI tokens merely to remain available. Verify on-demand start and recovery in practice.
- [ ] **A0.3 Enforced account capacity.** Keep per-company CPU/RAM/process/storage caps and enforce
  the owner's aggregate account budget across all companies, plus the host-wide budget, including
  simultaneous starts and release roll-forward. Per-computer controls never replace the account cap.
  Protect the app, database, tunnel and host services. At capacity, queue new heavy work with an
  observable reason and resume condition instead of exhausting RAM or thrashing swap. Existing
  runtime/Authority limits remain the source; do not add a second general scheduler or meter.
- [ ] **A0.4 Efficient AI execution.** Remove duplicate/no-change model wakes, polling-driven
  reasoning, repeated orientation and routine supervisory relay. Bound retries. Reuse verified
  results and compact durable context rather than repeatedly resending histories, tool output or
  media. Choose a sufficient admitted model for the task and preserve measured outcome quality.
  Enforce company AI budgets before chargeable work, reconcile real usage afterwards, and keep
  the app responsive when a cap is reached. A quiet company with no due work makes zero model calls.

  [Metered Claude cache retirement](sprint-109/cache-retirement.md) now uses the configured short
  cache window for large cold history, including persisted session reuse after restart. Local
  policy, gateway and actual OMP wire checks pass; Core `0e3c44c1` is deployed with healthy
  desired/applied revision `125/125`, verified on 30 October Sydney time. Native/subscription
  routes retain their existing window. This is a bounded A0.4 fix, not full A0 completion.

  [Normal-scheduler idle verification](evidence/sprint-109-resources/README.md#normal-scheduler-idle-verification)
  now proves zero model calls over 330 seconds in two connected, always-on synthetic companies,
  with successful owner replies before and after. The verifier joins the source-fingerprinted
  local release checks; the full resource gate and active-work scenarios remain open.
- [ ] **A0.5 Resources UI (H1, first batch).** Show actual CPU/RAM/storage against limits, host
  headroom and what is using resources, plus AI token/cache/cost usage and budgets. Include clear
  controls for supported caps, history and reasons work is waiting. Distinguish company usage,
  shared host capacity and reserved platform resources. Prices and currency come from the real
  catalogue, including relevant backup/tax costs; a design value never becomes a real offer.

  [Owner CPU/RAM controls and account-pressure evidence](sprint-109/compute-controls.md)
  now cover real cap edits, persistence, safe pending reductions and admission recovery.
  Four real jobs were contained to a 768 MiB / half-core test account pool with memory
  throttling and zero OOM kills; all 128 entry checks across that plane and a separate
  peer passed. This is local containment evidence, not the complete Cloud resource gate.
- [ ] **A0.6 Resource release gate.** Exercise multiple idle companies, on-demand browsers,
  simultaneous heavy work, exhausted compute/AI budgets, restart and upgrade on a constrained
  host. Verify stable owner entry, protected headroom, bounded concurrency, no idle AI spend and
  no duplicate effects. Preserve browser/role/zero-jank checks and compare resource and token
  consumption for equivalent useful outcomes before declaring this complete.
  Under real account CPU and memory pressure, probe navigation/identity, Inbox and Work APIs as
  well as health. Verify platform services and another owner's account remain responsive; an
  account's backlog must throttle or wait without causing a site-wide failure.

The outage exposed eager computer startup and independent per-company limits whose combined
allowance can exceed the host. Increasing VM size provides headroom; completion requires the
measured efficiency and shared-capacity controls above. Paid capacity changes require a separate,
explicit owner decision. The proposed 16 GB production resize has not been approved or applied.

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
  that can't help. "Ready for you" is a review, not a Question. Open or render the actual work or
  attention item in context; do not send the owner elsewhere to search for it.
- [ ] **A5 Lead chats and step cards stay usable [28].** A step that says "here in the card" shows
  the company computer, or says why it can't. Investigate the reported computer panel appearing over
  the settings content; reproduce the actual state and fix the layering or placement without hiding
  the controls underneath. Verify the connected, unavailable and narrow-screen states.

## Track B: one conversation, one Inbox

- [ ] **B1 One Exec chat [5, 10].** Items never open a new item-scoped Exec chat. "Ask Exec about
  this" links the item into the one chat. Discuss is removed from cards and the doc toolbar. Remove
  the duplicate middle-pane reply composer; a floating Reply action opens the one right-side Exec
  chat with the actual current item attached. Use floating actions in middle panes where they save
  space without obscuring content, and verify keyboard and phone access.
- [ ] **B2 Everything waiting on the owner is an Inbox item [9, 42].** Brand rules waiting for an
  OK first, then any page-held approval. Exec can read what's in the Inbox, so it never says "if a
  card is there".
- [ ] **B3 One item per thing [7, 34].** A change to the same doc or Work amends its item; FYI
  notices ("changed", "shared", "Can edit") leave the Inbox for a short daily digest in the Exec
  chat. The Inbox count means "things only you can do".
- [x] **B4 Filter, not a Skipped drawer [20, 35].** A filter icon in the Inbox sidebar header
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
- [ ] **C5 Toasts, not banners [55].** "Secret stored securely" goes after a few seconds. Work
  loading failures and chat reconnect/send warnings must not insert banners that shift the page.
  Use accessible, non-shifting notices with retry and failure details where needed; preserve unsent
  messages and keep persistent problems discoverable.
- [ ] **C6 Rows don't swap [33].** Inbox rows keep the time and show `…` beside it on hover.
- [ ] **C7 Zero-jank gate.** A headless pass over every page and card that fails on any layout
  shift, any unstyled frame or any edge off the grid. Runs in release qualification.

## Track D: plain replies and owner-ready writing

- [ ] **D1 Drop the reply structure [18, 40, 41].** Cut `prompts/converse-with-owner.md` to a few
  rules: answer the question plainly, be accurate, use "you", link what you mention, no internal
  ids. No required opener, no bold, no closing limit line. The intent marker stays as hidden
  metadata and never shapes the text. Keep `restless-details` only as the Work details fold.
- [x] **D2 Measure it.** Run the same Reno prompts before and after; compare accuracy, repetition
  and length. Keep the result in this file.

  [Matched live reply comparison](sprint-109/evidence/reply-comparison.md), 30 October: seven question pairs
  on exact `openai-codex/gpt-5.6-sol`, medium effort, with identical frozen facts. The short contract
  used 854 fewer input tokens per reply; visible words were 268 before and 256 after across the seven
  replies. All decision-bearing facts were retained in the after sample; the old archive answer
  overclaimed that only two Library entries remained. The new model-status answer was longer.
  A live diagnostic omitted hidden routing metadata, so the short contract now explicitly ends with
  that marker; all seven final after replies passed structural routing checks. Cases, replies,
  usage and failed diagnostics are retained. This is a reply-only cold-context comparison, not
  a production Opus, whole-task quality, cache-efficiency or company-spend result. A0 remains open.
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
  reaches it. Hovering anywhere in a block highlights its whole area and exposes a reachable drag
  handle. Checkboxes line up with the first line of text, gaps are even, the block toolbar never
  covers text.
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
  In a local owner role journey, resizing from desktop to 390px with Exec chat open left its
  composer over the Resources page's Edit limit button, preventing a normal click. The earlier
  phone check started with the rail closed and did not cover this state. Reproduce and correct the
  open-rail phone layout as part of F4/F9; don't claim the Resources editor fully qualified until
  it works. Evidence: `/tmp/restless-identity-core-evidence-eJRnEj/journey.log`. The disposable
  journey cleaned up; it ended as a failed role run, not a passing release gate.
- [ ] **F5 Clipping [29, 49].** The stray chip in the card footer and Library's cut-off "Archived".
- [ ] **F6 Compact entry and concise copy.** Make the sign-in button proportionate to the surrounding
  controls at desktop and phone sizes. Remove unnecessary eyebrows and explanatory footer copy,
  including the "A snapshot, picked by hand" example, while keeping essential privacy information
  available in its appropriate place.
- [ ] **F7 Queued sends are distinct.** Show scheduled messages as queued above the composer,
  visually distinct from sent messages, with their schedule and usable edit/cancel actions. Confirm
  scheduling, send, cancellation and failure states without layout jumps.
- [ ] **F8 Company browser title.** Company pages use the company name as the browser tab title,
  independent of whether Inbox, Work or People is selected; verify navigation and multiple companies.
- [ ] **F9 Sidebar minimum width.** Allow the Inbox and other resizable left sidebars to minimise
  further when space permits, without clipping their controls; restore the person's saved preference.

## Track G: Work and the Library

- [ ] **G1 Board by default.** `routes/[companyId]/work/+page.svelte:117`: board unless
  `?view=list`. Cards move across columns live as Work changes, without a reload.
- [ ] **G2 Leads delegate [4].** A lead splits independent Work across Staff in parallel. The board
  flags a card waiting in Next while its owner has three or more in motion.
- [ ] **G3 Churn shows [50].** The board flags Work past a few revisions (A$500 pass was at 15), and
  the lead says why.
- [ ] **G4 Board shape [51, 52].** All columns use consistent widths, including empty columns;
  horizontal scrolling is acceptable. Keep card titles short and their full text accessible. This
  newer owner instruction supersedes the earlier request for narrow empty columns.
- [ ] **G5 Scratch out of sight [48].** Agents' working notes leave the owner's default Library
  view. Check the "Folders 2000" count, which looks like a cap.

## Track H: new surfaces (design first, build after review)

The owner approved the Resources, Email and map preview layouts on 29 October. Build may proceed;
the illustrative values in the preview do not establish real measurements or prices. The owner
also identified `https://restless.run` as the Reno production target for final smoke verification.

- [ ] **H1 Resources page [56], promoted to A0.5.** Per company: storage, CPU and RAM in use against limits, history,
  what's using it, and one clear way to buy more with prices. Linear-quality design, reviewed by
  the owner before it's built.
- [ ] **H2 Email integration [15].** Resend as a first-class integration: guided setup, domain and
  DNS records, a test send, status, sending limits. Built on A2 and D3.
- [ ] **H3 Interactive maps [57].** Concept page maps get basic zoom, or a click through to Google
  Maps.

## Owner follow-up: active issue reconciliation

The owner supplied a consolidated issue list and three screenshots on 30 October. Its newer board
width instruction overrides the earlier narrow-empty-column idea. The list is in scope even where a
partial fix exists; an unchecked item needs its full acceptance and production smoke, not just a
similar-looking change. The screenshot of the computer overlay needs reproduction before its cause
is known. The Lexifina article in the pasted list is a marketing reference, not a request to copy or
publish its contents.

| Owner feedback | Sprint coverage | Verified status |
| --- | --- | --- |
| Oversized sign-in and unnecessary eyebrow/footer copy | F6 | Open |
| Mark done unavailable or slow; instant Undo | C1 | Partial: handoff and native-document Undo tested; Authority, generic set-aside and full prefetch remain |
| Double doc header/footer and raw-to-styled jump | E1, E2, C2 | Open |
| Review card must show the actual work and open the real item | A4, B2 | Open |
| List-item dragging, alignment, block hover and reachable handle | E3 | Open |
| Inbox filter instead of Skipped drawer | B4 | Checked against local and installed evidence; production Reno walkthrough remains |
| Repeated structured/bold agent replies | D1, D2 | Prompt comparison checked; full plain-reply acceptance remains open |
| Brand and other owner approvals belong in Inbox | B2 | Open |
| Discuss duplicates linked Exec chat; duplicate middle composer | B1 | Open |
| Cropped popover and inconsistent model connection status | B5, A1 | Open |
| Composer hint shifting and general alignment / chat spacing | C3, C7, F1, F3, F4 | Open |
| Margin comments rather than a separate comments pane | E4 | Open |
| Lead fails to delegate | G2 | Open |
| Resend capabilities, integration and secret names; vault versus account | D3, H2, A2, B6 | Open |
| Sidebar size, saved width and narrow minimum | F1, F2, F9 | Partial: narrower minimum deployed; account-saved preferences and complete role checks remain |
| Duplicate attention items | B3 | Open |
| Company storage, CPU and RAM controls and capacity purchase | A0.1–A0.6, H1 | Partial: CPU/RAM controls passed local checks; release and full account isolation gate remain |
| Computer panel unexpectedly covers settings | A5 | Open; reproduce and diagnose |
| Scheduled sends must look queued above composer | F7 | Open |
| Equal board column widths, horizontal scroll acceptable | G4 | Partial: equal-width change deployed; full title, role and Reno checks remain |
| Browser tab shows company name rather than selected page | F8 | Partial: company-only title deployed; full multi-company and Reno checks remain |
| Work and chat warnings shift content | C5, C7 | Partial: named Work and chat warning fixes deployed; all-surface release gate remains |

[Production delivery evidence for these partial fixes](sprint-109/ui-efficiency-release-30-oct.md).

## Later (agreed, not this sprint)

Grouped decisions, keyboard-first Inbox, watching work happen live, cost and time on every card,
deciding from a phone, a company timeline. See "10x and broader improvements" in the triage.

## Outside the code

- [ ] Add "all jank is banned" to the Restless company's house rules, where the owner's rules now
  live (needs the owner's OK).

## Order

A0 first: stabilise shared capacity, remove idle compute/token waste and deliver the Resources UI
with real caps and measurements. Then complete A, C1, D1 and B1-B3, followed by the remaining B, C,
E, F and G work and the approved new surfaces. Ship verified batches; resource stability and owner
entry are required before the next production rollout.

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

30 October: Core release qualification now includes the isolated real-chat movement regression at desktop and phone widths. It injects reconnect and send failures, checks zero transcript/composer movement, preserves the draft, and verifies retry, rediscovery and recovery. The Dagger release-environment check passed at both widths. This is one targeted part of C7, not the full six-surface, role and loading-state zero-jank gate.

30 October: Inbox and the shared Work, Library, Apps and Company sidebars can now be reduced to 160px. Browser tests on a seeded isolated company verified that each reached that width, its header controls remained within the sidebar, and the preference restored after reload in the same browser; the Inbox filter also opened. The fixture cleaned up. Account-wide, cross-device preference syncing and the role/production walkthrough remain open, so F2 and F9 are not complete.

30 October: reconnect and send failures in the Exec rail now use shared notices rather than inserting lines into the transcript or under the composer. A reserved header control reopens dismissed notices; reconnect retains Try again, and failed sends retain their draft and existing safe retry identity. Browser tests of the real rail with simulated delivery and connection outcomes at 1440px and 390px observed zero transcript and composer movement, draft preservation, retry, rediscovery and cleanup on recovery. Full end-to-end role and production checks remain open; this does not complete C5 or the zero-jank gate.

30 October: the Work stale-refresh warning uses the shared toast rather than inserting a banner above the board. A reserved header control reopens a dismissed warning, and retry and automatic recovery clear it. Against an isolated seeded company, browser checks at 1440px and 390px observed zero board movement during an injected API failure, plus dismissal, rediscovery, retry and automatic-recovery cleanup. The fixture reported no cleanup problems. Chat reconnect/send warnings, broader layout-shift checks, role coverage and an authenticated production walkthrough remain open; C5 and C7 are not complete.

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

### Correction deployed and observed

- PR #500 merged the connected-computer correction into main at
  `e82f6e7c4147f335344a74f056177fffc740141c`. Promotion passed 23 connection scenarios, with
  only the separately invoked skills smoke skipped, and activated release `29212ce0824274123713`.
  Fresh status: exact source commit, live service/binary/socket, owner health 200, no drain,
  no repair and no rollback.
- The installed Aris browser smoke then passed Work board default and Inbox All at 1440/390,
  with no page errors or fixture mutations. Evidence: `/tmp/s109-installed-smoke/report.json`
  and its screenshots. The built-in browser additionally waited for the actual computer to
  connect: its width was 428px, the Inbox sidebar remained visible at 340px, and Filter Inbox: All
  remained available. This replaces the earlier failed installed smoke, not its history.
- B4 is checked against the local four-filter/restore/error journey, eight layout captures,
  authenticated role journey, and corrected installed smoke. Account-saved filters remain F2.
- Both earlier test profiles' sprint109_test companies were removed, including company/attachment
  volumes, Documents sidecars, isolated databases/roles and fixture configuration. Shared original
  development infrastructure was preserved. The fresh sprint109 and sprint109final profiles and
  their dedicated Vault/database containers and volumes were also removed. Scripted-model and
  preview servers were stopped; the installed production appliance remains running.
- A4 current-state audit is in `review-delivery.md`: server brief freshness exists, but the
  review request does not yet carry the candidate version displayed to the owner. This and C1's
  durable pending decision remain implementation work, along with every other unchecked item.
- H layouts are prepared and exercised at 1024/390/320; owner review is still pending. The local
  appliance has no Reno company, so the original Reno production target remains requested. The
  full goal stays active; these deployments do not establish completion of the entire sprint.

### Signed Core and Cloud delivery, 29 October

The identity patch and steady document surface are merged and deployed on Core bec616403890.
The live owner plane reports that exact source, healthy, desired/applied revision 120/120; all six
company runtime outcomes are current. Cloud public health and canonical redirects passed.
See [release-29-oct.md](sprint-109/release-29-oct.md) for signed bundle identities, local role and
browser checks, deployment evidence, cleanup and the remaining private Reno browser verification.
The full sprint checklist remains open.

### Owner steering and the next C1 slice, 29 October

The owner supplied `restless.run` as the Reno production target and approved the Resources, Email
and map preview layouts. Track H design review is satisfied; real resource metrics and offers
still need to come from their actual providers. The owner asked work to continue while away.

The durable handoff Undo slice is described in [handoff-undo.md](sprint-109/handoff-undo.md).
It replaces the browser-owned timeout for Mark done, handoff Skip and outcome Accept. Authority
approvals, native-document decisions and generic set-aside remain before C1 can be checked.
No additional checklist item is claimed complete by this slice alone.
### Native document Undo, 29 October

- Native review Accept and saved collaboration Done now use durable source-owned five-second
  intents. Acceptance, its named version, the note to its requester and its receipt commit
  together; Work resumes through its existing document-review transaction. Undo publishes
  neither a note nor a mention. A person's form input survives Undo, acknowledgement and reload.
- Equivalent request keys from two tabs now retain the same receipt after Undo or completion,
  for both handoff and native decisions. Migrations 112/113 are additive; an expired pending
  native decision survived the real daemon stop, schema upgrade and restart with its original
  deadline, one acceptance and one requester mention.
- Fourteen real PostgreSQL decision scenarios and 22 related document scenarios pass. Thirteen
  desktop/phone UI/source scenarios pass, including a lost save response, separate failures with
  identical wording and exact notice clearance after cold loading. The final advance measurements
  were 160/153 ms. Frontend check/build, four surface captures and chat motion passed.
- The real owner/member/admin journey passed, including private saved review input and assigned
  member Undo. The first run exposed an intermittent room projection omission; the message was
  stored but absent in Alice's screen. That evidence is retained, and no Room fix is claimed.
- Removed the immediate frontend native-accept/collaboration helpers and reused the native comment
  transaction. Shared code is limited to verified human identity and notice rendering. The Inbox
  review no longer repeats its requester/version metadata in the form. Its loading pane reserves
  the same height, so the decision bar and notice remain still.
- See [source ownership, exact evidence and remaining scope](sprint-109/document-undo.md).
  C1 remains open for Authority approvals, generic set-aside and the full next-item prefetch audit.
  A4 admission/repair, mutable files and the other open sprint requirements remain required.
- This slice is verified locally. Its main integration, signed release and production cutover
  are recorded separately when they actually happen.

- Native verification cleanup completed: the official company destroy removed its cell,
  computer, kept files, spend records and volume. Its host stack/mock, four infrastructure
  containers, three infrastructure volumes, temporary PostgreSQL and private URL/profile were
  removed. The role runner removed its own resources. The read-only debt report retains
  pre-existing resources belonging to other runs; none was pruned.
  Evidence: `/tmp/s109-native-company-cleanup.log`, `/tmp/s109-native-cleanup.log`,
  `/tmp/s109-native-debt-after.log`.

## A0 first-slice evidence — 29 October

[Resource efficiency evidence](evidence/sprint-109-resources/README.md) records the
six-company matched comparison: computer-container memory fell from 2,368.6 MiB
to 608.81 MiB (74.3%) after all six browsers were used then left idle under
continuous health checks. Idle checks made zero model calls, with the local mock
log calibrated by a control request. Chrome now starts on demand; health reads
no longer pin it awake. Cold owner viewer/link opening and broker-only restart
passed against the real Runtime. The existing lean/cold AI history policy now
survives daemon restart through the existing scoped session locator.

This is a verified first slice, not A0 completion or a production savings claim.
Shared host capacity, reserve and queueing, the Resources controls/history UI,
and measured AI useful-outcome/token efficiency remain open. No paid resize was
approved or applied.


### Capacity admission follow-up, 30 October — not yet deployed

The owner chose to keep the current plan and reduce resource use. Shared-pool Work admission
and degraded-runtime entry are being implemented in
[capacity-admission.md](sprint-109/capacity-admission.md). New diagnostics defer under pressure,
individual Runtime headroom is checked, and cold Resources entry now passes at desktop/phone.
Two strict release/resume tests produced useful attached artifacts without child OOM kills or
new model requests while waiting. A pending-Work restart exposed extra startup cleanup processes;
the recovery barrier now waits for headroom. The rebuilt role/coediting journey, real-scale Resources
captures and motion comparison pass. Two conversations still stalled the deliberately small 768 MiB
stress Runtime; wider actor-start/concurrency control remains open. Release qualification and
deployment remain open. This is not A0 completion or a new production release.
