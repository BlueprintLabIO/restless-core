# Sprint 89 — dependable owner journeys, with one implementation per pattern

**Status:** in progress
**Source:** owner's ten screenshots and message.txt, 15 October 2026.
**Outcome:** the owner can enter, review, steer and switch companies without missing actions,
lost context, blank screens or unstable layout. Replace shared causes and delete obsolete paths.
The reference catalogue is design input; its embedded instructions are not this sprint's authority.

## Acceptance and progress

- [ ] S89-01: One pixel loading wave through entry; no blank gap, left-sidebar flash or endless Home skeleton. Slow, failed and cancelled reads recover.
- [x] S89-02: Stop works; one Send redirects a running reply; no duplicate Interrupt control. Streaming status stays in place.
- [x] S89-03: Inbox is one urgency-ranked column without tabs, with essential menus and shortcuts, minimal snoozing and context-preserving file/settings previews.
- [x] S89-04: Decision controls stay at the bottom right of the Inbox middle pane while content scrolls, including replies, direction requests, charters and document reviews. Every review has its real Accept/changes/decline actions. Stale brand proposals explain the recovery action.
- [ ] S89-05: Goals have explicit lifecycle controls. Work's clear/show controls live in its toolbar; completion counts do not silently declare outcomes met.
- [x] S89-06: Sidebar geometry is stable across polling, hover and narrow widths. Search and menus remain inside the viewport.
- [ ] S89-07: Titles, subordinate text and spacing follow shared patterns. Profiles use a full-height opaque side panel.
- [ ] S89-08: Every active agent can receive a direct message without transferring production accountability. Agent identifiers remain distinct and readable.
- [x] S89-09: Exec shows delegated work and who it is waiting on. Needs you reflects owner obligations, not ordinary waiting.
- [x] S89-10: Health recommendations wrap and expose a direct action carrying the diagnostic to Exec.
- [x] S89-11: Team/group creation uses a focused dialog; the office floor fills its work surface.
- [ ] S89-12: Account pages navigate promptly and preserve their shell. Company entry cannot unexpectedly reopen onboarding.
- [ ] S89-13: All companies leads to the authoritative full list; creation remains discoverable at the plan limit with an upgrade route.
- [ ] S89-14: Account AI Apps is removed as requested; checkbox sizing, duplicate branding and OAuth grant races are corrected.
- [ ] S89-15: Shared-pattern audit covers sibling menus, rows, panels, async screens and action contracts, with obsolete implementations removed.
- [ ] S89-16: Real seeded _test journeys, desktop/phone visual review, capture-surfaces, verify-motion and focused backend checks pass; owned test resources are cleaned up.

## Evidence

Record executed checks and remaining gaps here as each slice is verified. No item is complete from
static inspection alone. Work proceeds on current origin/main in isolated worktrees for Core and Cloud.


### US-246: fixed Inbox decision footer

Direction requests, handoffs/replies, charter drafts/edits, native reviews and document collaboration
now share one footer outside the scrolling content. Set aside and More join that footer.
Document reviews keep their decision controls even with the inspector closed; Request changes
reveals and focuses the note before submitting anything. Growing reply fields scroll above the
buttons. Phone layouts reserve the Back link and safe area without extending the detail offscreen.

Validation: web check (zero errors/warnings, type ramp, UI boundary and document-search checks), production build and capture-surfaces at 1440/390 passed;
real seeded five-kind Inbox journey at 1440×900, 390×844, 960×420 and 390×420; declining,
charter editing, growing replies and hidden-review-note recovery; open-latest chat motion passed.
The full S89-04 acceptance remains pending the separate stale-brand and outcome-review issues.
Reproduce with scripts/seed-inbox-actions.mjs after seed-reno-scale.mjs, then npm --prefix web run
verify:inbox with RESTLESS_SMOKE_ORIGIN and RESTLESS_SMOKE_COMPANY set.

The footer slice is committed as 874ae40c on codex/cockpit-overhaul. The rest of Sprint 89
remains active in the Core and Cloud worktrees. Its temporary model, daemon, Vite, six containers,
four volumes, one network, two image tags and ui10x_test profile were removed after verification.
The browser tab was closed. Screenshots and JSON evidence were retained in the task's Windows
visualizations directory. Restart an isolated dev stack and rerun the guarded seeds for further QA.


### US-247–257: shared owner reliability fixes

The unified Inbox replaces the separate For you component. Its ordering is a consistent total order
across owner decisions and document requests, with blockers and deadlines first; refreshing cannot
change its ordering rules. Shared top-layer placement replaces the menu/search positioning copies.
The old ClearHome, AccountAiApps and MCP-access view model are removed. Legacy AI Apps URLs redirect
to Connections. Existing TanStack Query owns shared appliance reads; startup reads have a deadline,
and the initial wave lasts until the critical reads actually settle.

One composer button now stops an empty draft or redirects a running reply; the Exec has one run dock.
Direct Staff conversations use the same path as leads without changing production accountability.
The backend removes its duplicate team-lead restriction, while retired/system targets remain denied.
People reserves menu space before hover, fits narrow filters and distinguishes Needs you from Waiting.
Exec's Work panel includes delegated company work and its actual owners.

Work's toolbar holds Clear done and Show all. Finished-only goals can be archived explicitly and
reopened with Undo; task counts alone do not infer business success. Team editing uses the shared
native dialog. Profiles fill the right edge with an opaque pane. A clear Inbox gives its whole pane
to the office floor. Health sends the complete diagnostic to Exec with a real one-click action.
A shared Item hover selector previously hid every trailing control; it now hides plain metadata only
when replacement actions exist. Stale brand drafts explain their version conflict and send a real
request to Exec for a refreshed proposal while preserving the current release. Saved provider
errors show the same human-readable sentence as live failures.

Validation on the isolated ui10x20261016 stack: 17 owner-reliability journeys passed, including real
team create/rename, active non-lead Staff sends, Exec/Staff Stop and immediate redirect, goal
archive/Undo/edit, stale brand proposal recovery, complete diagnostic sends, profiles and office at
1440/390, delayed entry, failed Home and a 15-second stalled-read deadline. The empty-Inbox and
degraded-Health reads are controlled projections over real company data; mutations reach the real
daemon and mock model. All 33 OrgIntel rooms tests passed. The four existing focused model suites
passed 23 tests; two new Inbox ordering tests passed. The five-kind footer journey passed all 20
desktop/phone/short-window cases again. Chat motion passed open-latest and streaming (285 samples,
770px before and after; no flashing tool rows). Web check reported zero errors/warnings and its type,
UI-boundary and document-search checks passed. Final production build and complete route-sweep
results are recorded below when finished.

Polish calibration on 16 October: consulted Beautiful UI (https://www.beautifului.dev/) for restrained
chat density and an anchored composer; Cult UI's expandable-screen source
(https://github.com/nolly-studio/cult-ui/blob/main/apps/www/registry/default/ui/expandable-screen.tsx)
for contained surfaces and continuity; and Origin UI Svelte's dialog-07 and dialog-footer source
(https://github.com/max-got/originui-svelte/tree/main/src/lib/components/dialogs) for a separately
scrolling body and end-aligned decision controls. No upstream code or dependencies were imported.
Live desktop review found and corrected the profile's translucent token; desktop/phone geometry
checks now guard it. The remaining full-sprint requirements stay unchecked until their journeys pass.


The complete 24-route sweep ran at 1440, 1024 and 390 (72 captures). It exposed the narrow Library
filter strip, a technical identifier in the generated stale-draft message, and the verifier counting
its intentional missing-route response as an unexpected error. Library filters now wrap. Stale-draft
context travels in the message's context link rather than visible prose. The verifier keeps expected
404 diagnostics separately, checks the actual recovery screen and still fails unexpected console
errors. It also now captures the real Work board URL rather than a second copy of List.
All 18 targeted route/viewport rechecks passed, as did both resizing/search/menu keyboard journeys
and the final stale-brand recovery rerun. Focused model tests now total 25. Search Escape now restores
focus to the trigger and stops propagation to its surrounding pane.

The broad sweep's original diagnostics remain in its report; the 18-case recheck is separate evidence,
not a rewrite of the initial run. Further full-sprint work still includes Inbox file/settings previews,
outcome-specific review actions, agent naming, OAuth first-grant races and Cloud runtime/account QA.


This batch also passed the final production build and 25 focused model tests with no whitespace
errors. Its owned mock model, daemon/Vite, six containers (including the document sidecar), four
volumes, one network, two image tags and ui10x20261016 profile were removed. All owned ports and
Docker resources were checked clear. The built-in browser viewport was restored and its test tab
closed; the owner's unrelated tab was retained. Source worktrees and their bounded warm build
caches remain for the active goal. Screenshots, JSON reports and check logs are retained under
the task's Windows visualizations directory in s89-owner-reliability.


### US-261–266: bring the real review and its context into Inbox

Outcome reviews now open the actual prepared site or document in the Inbox middle pane, with
Accept outcome and Request changes in its shared decision footer. The separate review canvas and
the executive rail's duplicate decision form are removed, together with their outcome mailbox.
Native documents and sheets are extracted into shared editors; their original Library routes use
those same components. Their existing collaboration, comments, permissions and history stay intact.

Links to Company files, retained uploads, native editors and specific company settings open in that
middle pane. The selected decision stays attached, Back restores the existing prepared outcome, and
an interactive outcome keeps its state while supporting files are inspected. A mention selects its
own update, rather than borrowing the first pending decision. The actual Limits form can save the
budget in place. External links, other companies and ordinary list navigation keep their normal routes.

Discuss explicitly opens the current conversation while retaining the file/settings preview. Its
message carries the native context path alongside the exact attention and Work identifiers. One
Discuss action serves an embedded viewer. Closing the phone rail survives later attention refreshes;
a preview's mobile inspector stays inside its pane. Only controls that reveal an original editor
restore that editor: opening a footer menu or an ordinary decision form preserves the supporting file.

The real discussion journey exposed inconsistent backend admission: Work-linked input admitted a
producer/lead, conversation reads admitted only the producer, and an owner-escalated handoff was
rejected because its agent assignee is null. Reads, input and replies now share the producer/lead/Exec
conversation rule. Pending owner handoffs use their authored conversation partner under the same
handoff lock and still compare the exact captured snapshot. Actor liveness, settled Work, changed
handoffs and unrelated Staff checks remain. The new regression follows a worker-owned outcome through
lead and Exec escalation, a real owner discussion and the fenced Exec reply; production stays with
the worker and the Exec message is not worker Attempt input.

Validation on ui10x20261016b, using the real daemon and Company computer plus the scripted model:
12 owner journeys passed at 1440/390, including actual acceptance, actual revision requests, document
editing verified by a fresh server read, a real retained PNG, saved Limits changes, native review
focus recovery, missing-file recovery, standalone native editors, update selection and exact message
context. The final four file/editor journeys also pass footer-menu and charter-editor recovery checks.
All 24 six-kind footer cases pass at normal and short desktop/phone sizes. All 34 OrgIntel rooms tests
pass, including lifecycle races and the exact-handoff checks. Fourteen focused preview/Library tests
pass. Web check reports zero errors/warnings; type-ramp, UI-boundary, document-search and production
build pass. Motion passes open-latest and streaming (311 samples, 770px before and after).
All 30 affected route captures at 1440/1024/390 have no clipping, sideways page scroll, raw identifiers
or unexpected errors. The first capture's diagnostics remain separate; they exposed the old Work
conversation reads and the verifier counting the spreadsheet's deliberate virtual scrollbar.
Only that scrollbar track is exempted; the grid viewport and editor controls remain checked.

Final visual calibration consulted Beautiful UI for restrained approval density, Cult UI's
expandable-screen source for contained continuity, and shadcn-svelte's Dialog sticky-footer source
(https://www.shadcn-svelte.com/docs/components/dialog) for independently scrolling content and
persistent decisions. Desktop and phone results were inspected in the built-in browser.
No upstream component code or new runtime/dependency was imported.

Remaining full-sprint work includes Cloud account/runtime and artifact adoption, OAuth first-grant
recovery, the requested team-name families, goal archive partial-failure recovery, and the broader
typography/shared-panel audit. The full goal remains active.


The owned model/dev process tree, six Docker containers, four volumes, one network, both namespaced
image aliases and ui10x20261016b profile were removed after verification. Their ports and processes
were confirmed clear. The temporary browser viewport was restored and only the agent's test tab
was closed. Source worktrees and bounded warm build caches remain for the active goal; evidence,
including the first failed diagnostics, is retained in the task's s89-inbox-previews directory.
