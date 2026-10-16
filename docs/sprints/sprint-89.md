# Sprint 89 — dependable owner journeys, with one implementation per pattern

**Status:** in progress
**Source:** owner's ten screenshots and message.txt, 15 October 2026.
**Outcome:** the owner can enter, review, steer and switch companies without missing actions,
lost context, blank screens or unstable layout. Replace shared causes and delete obsolete paths.
The reference catalogue is design input; its embedded instructions are not this sprint's authority.

## Acceptance and progress

- [x] S89-01: One pixel loading wave through entry; no blank gap, left-sidebar flash or endless Home skeleton. Slow, failed and cancelled reads recover.
- [x] S89-02: Stop works; one Send redirects a running reply; no duplicate Interrupt control. Streaming status stays in place.
- [x] S89-03: Inbox is one urgency-ranked column without tabs, with essential menus and shortcuts, minimal snoozing and context-preserving file/settings previews.
- [x] S89-04: Decision controls stay at the bottom right of the Inbox middle pane while content scrolls, including replies, direction requests, charters and document reviews. Every review has its real Accept/changes/decline actions. Stale brand proposals explain the recovery action.
- [x] S89-05: Goals have explicit lifecycle controls. Work's clear/show controls live in its toolbar; completion counts do not silently declare outcomes met.
- [x] S89-06: Sidebar geometry is stable across polling, hover and narrow widths. Search and menus remain inside the viewport.
- [x] S89-07: Titles, subordinate text and spacing follow shared patterns. Profiles use a full-height opaque side panel.
- [x] S89-08: Every active agent can receive a direct message without transferring production accountability. Agent identifiers remain distinct and readable.
- [x] S89-09: Exec shows delegated work and who it is waiting on. Needs you reflects owner obligations, not ordinary waiting.
- [x] S89-10: Health recommendations wrap and expose a direct action carrying the diagnostic to Exec.
- [x] S89-11: Team/group creation uses a focused dialog; the office floor fills its work surface.
- [x] S89-12: Account pages navigate promptly and preserve their shell. Company entry cannot unexpectedly reopen onboarding.
- [x] S89-13: All companies leads to the authoritative full list; creation remains discoverable at the plan limit with an upgrade route.
- [x] S89-14: Account AI Apps is removed as requested; checkbox sizing, duplicate branding and OAuth grant races are corrected.
- [x] S89-15: Shared-pattern audit covers sibling menus, rows, panels, async screens and action contracts, with obsolete implementations removed.
- [x] S89-16: Real seeded _test journeys, desktop/phone visual review, capture-surfaces, verify-motion and focused backend checks pass; owned test resources are cleaned up.

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


### Account-plane verification and US-267–270 recovery

Cloud's account slice is committed as d594c763 and available in draft PR
https://github.com/BlueprintLabIO/restless-cloud/pull/89. Its five account journeys passed
at desktop/phone widths, including disabled JavaScript, delayed hydration, capacity upgrades,
company creation intent, scoped navigation and stalled Fleet reads. A paused owned Fleet read
returned after its 10-second deadline without losing the shell identity. Cloud's 25 unit checks,
auth boundaries, type/build checks, foundation/contrast checks and all three CI jobs passed.
The signed 0.9.4 shared account UI was admitted through Cloud's real checksum/signature path.
This records verified source and a draft PR; the Cloud change has not been merged or deployed.

OAuth presence, mode and account identity now come from one authenticated broker snapshot.
Reads wait through a bounded startup/transport gap; an authenticated absence, refusal, malformed
snapshot or conflicting credential mode is not retried as a grant. The broker becomes observable
when it is actually healthy, before slower model-catalogue admission. Existing-account login
avoids eager redundant probes. Successful reconnection reloads the credential and clears only
credential rejections for that exact account grant and provider; quota cooldowns and other
companies/providers remain. The broker, gateway and relay are supervised for child exit.

The service fingerprint records the inputs actually used at startup. An already-loaded account
OAuth provider and its company grant share one broker input, so granting it or switching a
built-in model no longer restarts every company's model/tool transport. A changed key, new
provider or genuinely new pinned runtime model still triggers a reload. The obsolete optional
process return, redundant readiness writes and identity-only probe were removed.

Work has one goal-action HTTP implementation. Partial archives preserve successful IDs and
Undo, keep completed work visible, and retry only unresolved goals. A failed write response is
reconciled with the actual goal list before reporting failure, preserving Undo after a committed
response was lost. Undo itself retains retry for each failed reopen. Company and storage context
are captured so a later action cannot write the new company's clear cutoff. A cleared, archived
goal now shows its completed Work instead of falsely saying it has none. Long recovery notices
wrap their explanation without splitting the Undo/Retry labels. The decorative empty review-chat
card and its duplicate CSS were removed in favour of the existing quiet empty paragraph.

Executed on the owned ui10x20261016c stack with the real daemon, PostgreSQL, OMP and Company
computer: all 53 gateway checks passed; the PostgreSQL restoration scenario passed with exact-grant,
other-provider, quota, unrelated-credential-field and other-company assertions; eight goal model
checks passed. All eight browser goal recovery journeys passed at 1440/390, including partial
close, partial Undo, retry accumulation, lost committed response and archived Work visibility.
Desktop/phone interactions were also checked manually. The final actual first company grant was
started with the broker port confirmed closed, succeeded in 7,976ms and kept the replacement broker
stable after admission. Another account returned 409 without changing the company revision;
missing credentials returned 503 in 10ms. A second forced broker exit recovered automatically and
the original account remained present. Only disposable provider payloads were used; a personal
external OAuth sign-in was not performed.

Final visual calibration consulted Beautiful UI for sparse state/approval presentation, Cult UI
for contained continuity, and the source-first shadcn-svelte Sonner example
https://www.shadcn-svelte.com/docs/components/sonner for a concise explanation with a legible Undo.
No upstream code or new dependency was copied. Evidence and cleanup results are retained under
the task's s89-oauth-goal-recovery visualization directory. Final capture/motion and cleanup
results are appended after they finish. Team naming, the complete typography/shared-pattern audit,
full-sprint acceptance and final Core/Cloud integration remain open; the goal is active.


Final recovery-slice verification: web check reported zero errors/warnings, and the type-ramp,
UI-boundary, document-search and production build checks passed. The final eight goal journeys
also assert that archived history does not offer an ineffective Clear done control. The 18 affected
Work/list/board, People, Company, Inbox and Work-detail captures passed at 1440/1024/390.
The final open-latest motion check opened and returned at the end of the seeded conversation;
streaming passed 293 frame samples with the same 770px reply width before and after completion.
An earlier streaming attempt is retained separately: the temporary fallback rule overrode the
fixture's intended scenario. Correcting the fixture, then running motion after broker fault tests,
produced the recorded passing run; product behavior was not changed to satisfy that test.

Cleanup completed: the owned model/dev process trees, five Docker containers, four volumes,
one network, both namespaced image aliases, the isolated dev profile and both OMP profiles
were removed. All owned listeners/processes were confirmed absent. The browser override was
reset and only the owned test tab was closed. Source worktrees and the 28GiB warm target remain
for the active goal; the Windows drive had 989GiB free. Retained evidence contains no account
credentials. The temporary fixture directory is removed after its evidence is copied.


### US-271–273: character names and the final shared-pattern audit

The owner clarified that names should be existing fictional characters. One Rust catalogue now
allocates both company lead letters and ordered team prefixes. An E team begins with Eames,
Ebenezer Scrooge, Echo, Edna Mode and Eeyore. Repeated characters retain their number in their
avatar (Ea2, Eb2). Reopening an ordinary legacy roster converts its active names once and audits
the change; actor IDs, team membership, Work ownership, messages and retired names remain intact.
Already allocated character names remain stable within the same letter family. Historical names
stay reserved, including after transfers. The old ordinary-name SQL function remains only in
migration history for older released binaries; the current runtime no longer calls it.

Validation so far: all 15 real Postgres actors/teams scenarios passed, followed by the expanded
rename scenario proving retained Work, message recipients and retired names. The catalogue unit
and four avatar prefix tests passed. A real seven-person E team in the disposable company showed
Ea/Eb/Ec/Ed/Ee and Ea2/Eb2 on desktop; its repeated long character name and profile also fit phone width. Profile review exposed an inherited-model
mismatch: the drawer read the raw optional assignment while the header read effective Intelligence.
The drawer now uses that same shared authoritative query, which also uses the bounded read helper.
Inherited-model drawer checks passed at 1440/390.

During live editing, the open development page also exposed a Svelte state_unsafe_mutation crash:
ambient conversation cleanup wrote reactive state while its parent derived was being replaced.
Cleanup now only closes the subscription; its effect body resets the live state. Two real Vite
module replacements passed with no page errors and the exact source restored. The explicit
conversation-switching checks observe real EventSource opens/closes rather than infer cleanup from
page appearance. The final desktop/phone results are recorded after the run finishes.

The verification harness also had two defects: a wall-clock delayed read could resolve before a
slow page reached its assertion, and an in-flight routed read could reject during context teardown.
The slow-read check now holds the read until it has observed the loading wave, and teardown removes
routes with cancellation-aware cleanup. These failures and their original reports remain separate
from the final acceptance evidence.

| Shared cause | Canon retained or introduced | Sibling coverage |
| --- | --- | --- |
| Actions scattered through evidence | AttentionFooter outside the scrolling body | Direction, handoff, charter, native review, outcome review, collaboration; four viewport sizes |
| Independently positioned overlays | ActionMenu and anchored-popover | Conversation actions/search, person/team menus; Escape, focus return and narrow viewport bounds |
| Conflicting title/model presentation | PersonHeader, opaque PersonProfile and shared Intelligence | Staff, leads, Exec, inherited models and long multiword names |
| Blank or endless async screens | Scoped TanStack Query reads, bootFetch, first-paint and FailureNotice | Entry, Home, account reads, cancellation and failed/stalled reads |
| Moving or flickering conversation UI | One run dock, durable message identity and bounded stream cleanup | Exec/Staff Stop, redirect, actor switches, live reload and motion probes |
| Wrong or lossy navigation context | InboxPreview with explicit allowed destinations and server conversation admission | Actual files, Docs, Sheets, kept uploads, Limits, Back/Forward and Work discussions |
| Partial writes without recovery | Reconciled goal transitions and persistent Undo/Retry targets | Archive, lost committed responses, partial archive/Undo and archived Work visibility |
| Account authority duplicated in the browser | Authoritative full-company destination and deliberate creation intent | At-capacity upgrade, prehydration form typing, server-readable Settings selection and scoped refresh |
| Inconsistent model-account recovery | One broker observation and supervised model services | Temporary broker absence, exact account mismatch, absent grant and credential-only cooldown recovery |

The pattern registry now points to these canonical implementations. Type-ramp/UI-boundary checks
continue to police sibling surfaces. A conservative reference search found no additional unused
Svelte components; deleting referenced features just to lower line count would not remove debt.
The four earlier Core slices removed six obsolete files (719 lines), plus replaced duplicated
implementations. Net source size is higher because missing behavior and meaningful regression
coverage were added; raw changed-line totals include replacements and formatting and are not a
measure of debt retired.

Final polish references consulted on 16 October: Beautiful UI's restrained chat/task hierarchy
(https://www.beautifului.dev/), Cult UI's contained expandable screen
(https://www.cult-ui.com/docs/components/expandable-screen), and Origin UI Svelte's separately
scrolling dialog body/footer source
(https://raw.githubusercontent.com/max-got/originui-svelte/main/src/lib/components/dialogs/dialog-07.svelte).
Bits UI's official focus/Escape dialog contract was also checked. No upstream code, new UI runtime
or new dependency was imported. The live phone profile and character roster were inspected in the
built-in browser; the browser connection subsequently became unavailable, so the later live-reload
proof uses the repository's actual browser verification workflow.


Final acceptance on the isolated ui10x20261016d stack passed: 19 owner journeys plus two
conversation lifecycle journeys, both inherited-model profile checks after the final company-scope
reactivity correction, all 24 footer cases, all 12 Inbox preview/decision journeys, 26/26 overlay
checks, and 90 route captures at 1440/1024/390. The route report retains advisory wide boxes as well
as its guarded page-overflow, clipping, sideways-scroll, raw-text and page-error results. Guarded
checks were clear; representative People, Inbox review, Limits and profile renders were inspected.
The final check has zero Svelte errors and zero warnings; the production build passes. All 32 focused
model tests pass. The 15 real actors/teams database tests and expanded retained-identity scenario
pass. Motion passes open-latest and streaming: 284 samples, 770px during and after the reply.

The model profile query follows the current company through the same derived/held-query pattern as
the other profile projections. Final profile screenshots show the inherited model correctly on both
widths. The built-in browser connection later recovered; its temporary verification tabs were absent
and the explicit viewport override was reset. No user tab was changed by cleanup.

Owned-resource cleanup removed the model/dev trees, five containers, four volumes, one network,
two test image aliases, the dev profile and both OMP profiles. Recorded process identities, all ten
ports and namespaced Docker resources were checked clear. Unowned images and checkouts remain.
Reports, original failed harness runs and screenshots are retained under the task's s89-naming-audit
Windows evidence directory; account credentials and full model requests are excluded.

Core is ready to checkpoint on main. Cloud PR 89 remains a clean, fully tested draft at d594c763;
its cross-repository merge approval was requested explicitly under the working agreement. No live
appliance or Cloud deployment has been performed. The overall goal remains active for integration.
