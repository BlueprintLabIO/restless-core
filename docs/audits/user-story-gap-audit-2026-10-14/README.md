# User-story gap audit — 14 October 2026

The audit has 210 owner user stories, each with its click path through the cockpit. Every path was traced through the code and checked for missing functions and UI/UX gaps. Each gap is filed as an issue under [`issues/`](./issues/) and mirrored as a GitHub issue titled `UXG-NNN: …`. The audit fixes nothing.

**How it was run.** Round 1 covered 100 stories across seven surfaces: Brand & voice, Inbox, Work, Library, People, Apps and Company/onboarding. Each surface was traced in the SPA (`web/src`), and the owner API (`crates/restless-owner`) was checked for endpoints with no UI and UI with no backend. Round 2 added 110 stories and 135 issues from angles round 1 did not cover: the Exec chat, the document and sheet editors in depth, the company computer and Intelligence pages, cross-cutting shell behaviour, a systematic check of endpoints and CLI against the UI, and end-to-end journeys across surfaces. Round 2 was deduplicated against round 1. Every gap cites `file:line` evidence on `main`, and the highest-impact claims were spot-checked by hand. Nothing was run against a live company, so findings are code-traced, not dogfood-observed: confirm each one before building.

**Totals.** 210 stories: 11 works, 139 partial, 60 missing. 325 issues: 54 high, 168 medium, 103 low.

**Recurring themes (round 1).**
- *Create but never edit.* This is the most common gap. Exec-drafted brand statements (UXG-001), schedule prompts (UXG-166), know-how (UXG-133), Work titles and goals (UXG-046, UXG-050), connections (UXG-132), Vault secrets (UXG-171) and email mandates (UXG-120) can be created but not edited or removed.
- *Data-model shapes instead of owner shapes.* Brand & voice is shown as scattered statement rows rather than one document (UXG-002). Apps Browse uses tiles that hide the facts needed to choose (UXG-152). Raw run states and paths show on Work (UXG-063).
- *One-click irreversible acts.* Stopping Work (UXG-042), removing an app (UXG-129), single-key approvals (UXG-018), single-click "Always allow" (UXG-010) and archiving a company (UXG-179) all lack a confirmation or an undo.
- *Owner-only affordances that lock out cofounders.* @mention (UXG-114), reactions (UXG-106), notifications (UXG-028) and "Waiting on you" (UXG-054) work only for the owner.
- *Spec promises without code.* Examples: the actor profile (UXG-102), budget periods and warnings (UXG-169), a company-wide freeze (UXG-185) and outcome-native review on Work (UXG-040).

**Round 2 highlights.**
- *The handover desktop can't be controlled.* The Inbox embeds the live company computer without `interactive`, so an owner handed a sign-in or 2FA step can watch it but not type (UXG-243). After Return control the view breaks (UXG-244), and a frozen computer can't be restarted (UXG-242).
- *Failures go silent.* A failed Exec reply has no Retry (UXG-191). There is no Stop for a running turn (UXG-192). When the budget stops the company nobody is told (UXG-306), and failed or uncertain approved actions never return to the Inbox (UXG-308).
- *Terminal-only capabilities.* Backup and restore (UXG-293), approving a publication (UXG-291), turning on sharing (UXG-296) and the payment rail (UXG-312) can only be done from the CLI.
- *Core business loops are missing.* Exec can't reply to a customer who wrote in (UXG-310), there is no weekly review (UXG-320), and nothing gathers everything about one client in one place (UXG-321).

Severity: **high** breaks a core owner loop or trust · **medium** clear UX or function gap · **low** polish or consistency.

## Stories



### Brand & voice

| ID | Story | Path | Verdict | Issues |
|---|---|---|---|---|
| US-001 | As an owner, I want to set up my brand and voice the first time so every site, deck and message starts from it. | Inbox (clear) → "Set your brand and voice" nudge → Exec chat; or Company → Brand & voice → "Ask Exec to draft" / "Write it yourself" | Works | — |
| US-002 | As an owner, I want to correct one sentence in the brand statements Exec drafted and I promoted, so the team stops using the wrong claim. | Company → Brand & voice → Direction → Edit → only my own four pillar textareas appear; Exec's released statements are not editable | Missing | [UXG-001](issues/UXG-001-let-the-owner-edit-or-remove-any-released-brand-statement-no.md), [UXG-005](issues/UXG-005-stop-hiding-exec-statements-that-still-ship-when-the-owner-w.md) |
| US-003 | As an owner, I want to delete a brand statement that is wrong or outdated. | Company → Brand & voice → no delete on any statement; saving always carries every non-owner statement forward | Missing | [UXG-001](issues/UXG-001-let-the-owner-edit-or-remove-any-released-brand-statement-no.md) |
| US-004 | As an owner, I want to tweak Exec's identity proposal before accepting it, instead of all-or-nothing. | Company → Brand & voice → Waiting for your decision → expand proposal → only Promote / Reject (reason required) | Missing | [UXG-003](issues/UXG-003-let-the-owner-amend-an-exec-identity-proposal-before-promoti.md) |
| US-005 | As an owner, I want to read my brand as one coherent document I can skim and send to a designer. | Company → Brand & voice → Direction, Current version, Released statements (four folds), Reviews, Version history each show fragments of the same statements | Missing | [UXG-002](issues/UXG-002-present-brand-voice-as-one-readable-brand-document-instead-o.md), [UXG-007](issues/UXG-007-let-the-owner-export-or-share-the-brand-guide.md) |
| US-006 | As an owner, I want to roll back to last month's brand version after a bad change. | Company → Brand & voice → Version history → fold shows statements; no restore | Missing | [UXG-004](issues/UXG-004-allow-restoring-an-earlier-brand-version.md) |
| US-007 | As an owner, I want to set my logo, colours and fonts as part of the brand so visual work is on-brand. | Brand & voice → Visual language is one free-text box; logo lives under Company → Charter | Partial | [UXG-006](issues/UXG-006-give-visual-identity-structured-fields-logo-colours-type-in.md) |

### Inbox

| ID | Story | Path | Verdict | Issues |
|---|---|---|---|---|
| US-008 | As an owner, I want to open the app and see at once whether anything needs me. | /{company} → Inbox list + detail, or the "No decisions" pill with the office canvas when clear | Partial | [UXG-016](issues/UXG-016-show-whether-the-company-can-keep-going-while-an-item-waits.md), [UXG-017](issues/UXG-017-order-the-inbox-by-consequence-and-deadline-not-age-alone.md), [UXG-029](issues/UXG-029-reconsider-the-pixel-office-as-the-clear-inbox-home.md) |
| US-009 | As an owner, I want to approve a spend or tool request safely so nothing goes out by accident. | Inbox → item → "Hold to approve" / "Always allow…" / "Ask for changes" / Decline | Partial | [UXG-010](issues/UXG-010-make-always-allow-at-least-as-deliberate-as-a-one-off-approv.md) |
| US-010 | As an owner, I want to answer an agent's question (with form fields when needed) and fix my answer if I got it wrong. | Inbox → item → fields + reply → Send (⌘↵); the item then vanishes | Partial | [UXG-019](issues/UXG-019-link-the-inbox-to-what-i-decided-and-allow-amending-a-recent.md) |
| US-011 | As an owner, I want to put a decision off until Monday so it stops nagging me today. | Row ⋯ → "Snooze for an hour" (or H); nothing longer | Missing | [UXG-011](issues/UXG-011-let-the-owner-snooze-until-a-chosen-time-or-date.md), [UXG-012](issues/UXG-012-add-snooze-and-copy-link-to-the-inbox-item-detail.md) |
| US-012 | As an owner, I want to dismiss an item that isn't needed and undo it if I was wrong. | Detail "Dismiss", or row ⋯ "Not needed" → toast Undo → "Set aside" fold → Restore | Partial | [UXG-008](issues/UXG-008-add-undo-to-dismiss-in-the-inbox-item-detail.md), [UXG-009](issues/UXG-009-use-one-name-for-dismiss-not-needed-set-aside.md), [UXG-022](issues/UXG-022-give-set-aside-dates-the-asker-and-a-visible-failure-state.md) |
| US-013 | As an owner, I want to see what I decided last month and what it led to. | Nothing in Inbox links there; must know Company → Activity → Decisions | Partial | [UXG-019](issues/UXG-019-link-the-inbox-to-what-i-decided-and-allow-amending-a-recent.md) |
| US-014 | As an owner, I want to hand a decision to my admin or a team lead so I'm not the bottleneck. | No control in the Inbox | Missing | [UXG-020](issues/UXG-020-let-the-owner-delegate-an-inbox-item-to-a-member-or-admin.md) |
| US-015 | As an owner, I want to see what Exec changed for others today and undo one. | "Today" line (plain text) → scroll to For you → applied row → Undo (cannot open the change) | Partial | [UXG-013](issues/UXG-013-make-the-today-line-link-to-its-lists.md), [UXG-014](issues/UXG-014-let-an-applied-change-row-open-the-change-it-made.md) |
| US-016 | As an owner, I want to answer a team member's permission request. | Inbox → request → Approve (Y) / "Approve, and always allow" (A) / Decline (N) | Partial | [UXG-015](issues/UXG-015-remove-eyebrow-kickers-in-direction-requests-and-the-compute.md), [UXG-018](issues/UXG-018-stop-single-keystroke-approvals-of-direction-requests.md), [UXG-027](issues/UXG-027-give-a-failed-direction-request-a-way-forward.md) |
| US-017 | As a member, I want to track or withdraw a request I made through Exec. | Exec rail → "Your requests" strip (max 3, not clickable, no withdraw) | Partial | [UXG-021](issues/UXG-021-let-a-requester-see-all-their-requests-and-withdraw-a-pendin.md) |
| US-018 | As a team lead (member), I want to pass a judgement I shouldn't make up to the owner. | No UI (endpoint exists) | Missing | [UXG-023](issues/UXG-023-wire-the-existing-pass-to-owner-endpoint-for-members.md) |
| US-019 | As an owner, I want to finish a sign-in or 2FA step on the company computer and have the work resume. | Item → open outcome → computer view → Take control → Leave computer → back to item → Mark done | Partial | [UXG-025](issues/UXG-025-let-the-owner-finish-a-human-step-from-the-computer-view.md) |
| US-020 | As an owner, I want to edit a document an agent asked me to work on, knowing what they asked. | Inbox → document item → editor opens directly, no ask/asker shown → Done | Partial | [UXG-026](issues/UXG-026-show-what-was-asked-and-by-whom-on-document-items.md) |
| US-021 | As an owner, I want to be told when something needs me while I'm away from the tab or the computer. | Profile menu → Desktop notifications (tab must be open; no email/push setting) | Partial | [UXG-028](issues/UXG-028-notify-members-not-just-owners-of-new-inbox-items.md), [UXG-030](issues/UXG-030-add-notifications-that-work-with-the-tab-closed-and-an-email.md) |
| US-022 | As an owner, I want to work through the Inbox from the keyboard without misfires. | J/K, R, E, H, C, ⌘.; `?` opens shortcuts | Partial | [UXG-018](issues/UXG-018-stop-single-keystroke-approvals-of-direction-requests.md), [UXG-031](issues/UXG-031-make-the-shortcuts-sheet-list-every-key-that-works.md), [UXG-032](issues/UXG-032-stop-e-meaning-mark-done-in-the-inbox-but-mark-read-in-for-y.md) |
| US-023 | As an owner on my phone, I want to triage the Inbox. | List → tap item → detail (list hidden) → "← Inbox" | Partial | [UXG-012](issues/UXG-012-add-snooze-and-copy-link-to-the-inbox-item-detail.md), [UXG-033](issues/UXG-033-make-the-inbox-badge-match-the-list.md) |

### Work

| ID | Story | Path | Verdict | Issues |
|---|---|---|---|---|
| US-024 | As an owner, I want to start a new piece of work (optionally under a goal) so the team picks it up. | Work → "Tell Exec what you want" → rail opens with empty composer → Exec decides; goal view has no "add work here" | Partial | [UXG-058](issues/UXG-058-start-add-work-from-a-goal-with-context-prefilled-and-confir.md) |
| US-025 | As an owner, I want to see what is blocked and why so I can unblock it. | Work → "Waiting" group → row meta has no reason → detail → reason only inside folded "Run details" | Partial | [UXG-044](issues/UXG-044-show-why-blocked-work-is-blocked-in-the-list-and-detail-head.md), [UXG-047](issues/UXG-047-show-attempt-history-feedback-and-inputs-on-work-detail.md), [UXG-057](issues/UXG-057-unify-status-words-waiting-waiting-on-you-blocked-needs-you.md) |
| US-026 | As an owner, I want to act on work that needs me from the Work page. | Work → "Waiting on you" → row → detail shows no decision, no link to Inbox | Partial | [UXG-039](issues/UXG-039-surface-the-pending-decision-on-the-work-detail-page.md), [UXG-053](issues/UXG-053-mark-needs-you-on-board-cards.md) |
| US-027 | As an owner, I want to review the finished outcome natively (live site, deck) from its Work page. | Work detail → Evidence → Outputs → "Open" only for Library kinds or public https | Partial | [UXG-040](issues/UXG-040-bring-the-prepared-reviewtarget-to-the-work-detail-page.md) |
| US-028 | As an owner, I want to reject a result or ask for changes on finished work. | Done Work detail → only "Discuss" / "Ask {lead}" chat links | Missing | [UXG-041](issues/UXG-041-let-the-owner-accept-reject-or-request-changes-from-work.md) |
| US-029 | As an owner, I want to pause work and resume it later, and undo a mistaken stop. | Work detail → "Stop…" (permanent abandon) or "Resume…" (blocked only) | Missing | [UXG-042](issues/UXG-042-separate-pause-from-stop-for-good-and-warn-before-abandoning.md), [UXG-043](issues/UXG-043-give-stopped-work-a-place-to-be-found-and-restored.md) |
| US-030 | As an owner, I want to find old or stopped work. | Work → Search (title/outcome) / Done fold / Archived; stopped work excluded everywhere | Partial | [UXG-043](issues/UXG-043-give-stopped-work-a-place-to-be-found-and-restored.md), [UXG-055](issues/UXG-055-fix-the-no-work-yet-empty-state-on-a-failed-search.md), [UXG-056](issues/UXG-056-widen-work-search-and-keep-it-in-the-url.md) |
| US-031 | As an owner, I want to change a work item's priority mid-flight. | No control; priority never rendered | Missing | [UXG-045](issues/UXG-045-show-and-let-the-owner-change-work-priority-add-sort.md) |
| US-032 | As an owner, I want to rename a work item, fix its scope, or move it to another goal. | Work detail → title, brief, goal read-only | Missing | [UXG-046](issues/UXG-046-allow-renaming-a-work-item-and-editing-its-scope-or-goal.md) |
| US-033 | As an owner, I want to reassign the lead or team for work or a goal. | Work detail rail → "Lead" is plain text | Missing | [UXG-048](issues/UXG-048-wire-lead-team-reassignment-for-work-and-goals.md) |
| US-034 | As an owner, I want to see what a piece of work or a goal has cost. | Nowhere on Work list, board or detail | Missing | [UXG-049](issues/UXG-049-show-cost-per-work-and-per-goal.md) |
| US-035 | As an owner, I want to rename, set a finish line or due date for, close, or reopen a goal. | Only `/goal` and `/goal clear` composer commands; `/goal clear` closes the most recent goal | Missing | [UXG-050](issues/UXG-050-add-goal-editing-close-this-goal-and-reopen-in-the-work-ui.md), [UXG-052](issues/UXG-052-provide-a-goal-detail-page.md) |
| US-036 | As an owner, I want to see the history of attempts and the feedback given. | Work detail → Run details → "Latest run" only | Partial | [UXG-047](issues/UXG-047-show-attempt-history-feedback-and-inputs-on-work-detail.md) |
| US-037 | As an owner, I want to filter or sort work by owner, status or priority and scan the board on my phone. | Sidebar: goal + three fixed views; no sort; mobile board stacks Next → In motion → Waiting → Done | Partial | [UXG-051](issues/UXG-051-add-owner-and-status-filters-and-decide-on-the-work-graph-vi.md), [UXG-059](issues/UXG-059-reorder-or-replace-the-board-on-mobile.md), [UXG-060](issues/UXG-060-cap-or-fold-the-board-s-done-column.md) |
| US-038 | As a team owner (member), I want "Waiting on you" to show my pending judgements. | Work → "Waiting on you" → always 0 for non-owners | Missing | [UXG-054](issues/UXG-054-make-waiting-on-you-work-for-team-owners-members.md) |

### Library

| ID | Story | Path | Verdict | Issues |
|---|---|---|---|---|
| US-039 | As an owner, I want to rename any document or file so I can find it later. | Row ⋯ → Rename (docs and sheets only; opens editor with ?rename=1) | Partial | [UXG-071](issues/UXG-071-allow-renaming-work-outputs-and-uploaded-files.md), [UXG-089](issues/UXG-089-return-the-editor-s-back-link-to-where-the-owner-came-from.md) |
| US-040 | As an owner, I want to put related files into a folder (e.g. "Clients/Acme"). | No path | Missing | [UXG-069](issues/UXG-069-add-folders-or-collections-to-the-library.md) |
| US-041 | As an owner, I want to delete something by mistake and get it back. | Row ⋯ → Archive → toast Undo / Archived → Restore; Archived → ⋯ → Delete permanently… | Partial | [UXG-072](issues/UXG-072-allow-permanent-deletion-of-archived-work-outputs.md), [UXG-073](issues/UXG-073-let-archived-files-be-opened-or-previewed.md), [UXG-086](issues/UXG-086-allow-deleting-a-document-from-its-own-page.md) |
| US-042 | As an owner, I want to find "that pricing number" by search. | Sidebar "Search files": doc text only; sheets/files by title | Partial | [UXG-074](issues/UXG-074-search-inside-sheets-and-comments-from-the-library-search-bo.md), [UXG-080](issues/UXG-080-let-a-docx-xlsx-upload-become-an-editable-doc-or-sheet.md) |
| US-043 | As an owner, I want to sort and filter the Library so the newest or right kind comes first. | Name/Updated column heads; sidebar Kinds | Partial | [UXG-075](issues/UXG-075-sort-and-count-across-all-documents-not-just-loaded-pages.md), [UXG-076](issues/UXG-076-add-owner-shared-with-me-and-date-filters-plus-bulk-select.md), [UXG-087](issues/UXG-087-fix-library-table-head-and-sort-accessibility.md) |
| US-044 | As an owner, I want to see a document's history and restore an older version. | Doc → Comments and history → History → Preview → Restore as new version | Partial | [UXG-077](issues/UXG-077-show-the-actual-old-document-when-previewing-history.md), [UXG-082](issues/UXG-082-let-an-old-document-version-be-downloaded.md) |
| US-045 | As an owner, I want to restore an older version of a sheet in place. | Sheet ⋯ → History → version ⋯ → Recover as a copy | Partial | [UXG-078](issues/UXG-078-restore-sheet-versions-in-place-like-documents.md), [UXG-079](issues/UXG-079-show-who-made-each-sheet-version.md) |
| US-046 | As an owner, I want to share a document with my outside accountant and later revoke it. | Share → email → `inviteToCompany` (company member) → remove person (Undo) | Partial | [UXG-070](issues/UXG-070-stop-email-sharing-from-making-outsiders-full-company-member.md), [UXG-083](issues/UXG-083-give-owners-a-way-forward-when-email-invites-are-not-set-up.md) |
| US-047 | As an owner, I want to upload a PDF or photo so it is kept and shared with the team. | New → Upload files… / drag-drop → "From you" (5 MB, uploader-only, no share/rename) | Partial | [UXG-071](issues/UXG-071-allow-renaming-work-outputs-and-uploaded-files.md), [UXG-084](issues/UXG-084-make-uploaded-files-shareable-and-visible-to-the-team.md) |
| US-048 | As an owner, I want to export a document as PDF or Word to send to a client. | Doc ⋯ → Download as Markdown / Print or save as PDF | Partial | [UXG-081](issues/UXG-081-export-documents-as-word-docx.md) |
| US-049 | As an owner, I want to import an Excel or Word file as something I can edit. | New offers Document/Sheet/Upload only; import only inside an existing sheet or hidden ?import=1 | Partial | [UXG-080](issues/UXG-080-let-a-docx-xlsx-upload-become-an-editable-doc-or-sheet.md), [UXG-088](issues/UXG-088-refresh-the-sheet-list-and-keep-library-error-states-honest.md) |
| US-050 | As an owner, I want to ask Exec to tighten up a document so I don't have to edit it. | Doc → Discuss → Exec gets comment access only | Partial | [UXG-085](issues/UXG-085-let-discuss-actually-allow-exec-to-make-the-requested-docume.md), [UXG-090](issues/UXG-090-offer-discuss-with-exec-on-sheets-and-link-the-file-on-file.md) |
| US-051 | As an owner, I want to ask Exec to edit or analyse a sheet or uploaded file. | Sheet: no Discuss; file viewer: Discuss with empty draft | Missing | [UXG-090](issues/UXG-090-offer-discuss-with-exec-on-sheets-and-link-the-file-on-file.md) |
| US-052 | As an owner, I want to see who edited what. | Rows "edited by X"; doc History editors; sheets last editor only | Partial | [UXG-079](issues/UXG-079-show-who-made-each-sheet-version.md) |
| US-053 | As an owner, I want to request a review of a document from a specific person. | Doc ⋯ → Request review… → "What should the reviewer judge?" (no reviewer picker) | Partial | [UXG-091](issues/UXG-091-hide-or-explain-request-review-for-people-who-cannot-request.md), [UXG-092](issues/UXG-092-let-the-requester-choose-a-reviewer-and-use-plain-language-i.md) |

### People

| ID | Story | Path | Verdict | Issues |
|---|---|---|---|---|
| US-054 | As an owner, I want to invite my cofounder from People. | Company → Members → Invite; People shows "Invite a colleague" only while the company is nearly empty | Partial | [UXG-098](issues/UXG-098-add-invite-and-member-management-to-people.md) |
| US-055 | As an owner, I want to remove someone who left. | Company → Members → row ⋯ → Remove…; clicking a colleague in People only opens a DM | Partial | [UXG-098](issues/UXG-098-add-invite-and-member-management-to-people.md) |
| US-056 | As an owner, I want to pause someone's access or cancel a lost invite safely. | Members → ⋯ → Pause access / Cancel invitation (fire immediately, no undo) | Partial | [UXG-099](issues/UXG-099-confirm-or-offer-undo-for-cancelling-invitations-and-pausing.md) |
| US-057 | As an owner, I want to hand ownership of the company to my cofounder. | No path; role select is Member/Admin only, owner cannot leave | Missing | [UXG-100](issues/UXG-100-support-transferring-company-ownership.md) |
| US-058 | As an owner, I want to hire a new AI staff member for a job. | No People affordance; empty state says staff appear as Exec builds the team | Missing | [UXG-101](issues/UXG-101-give-the-owner-a-way-to-hire-retire-pause-or-re-brief-ai-sta.md) |
| US-059 | As an owner, I want to let go of, pause or re-brief a staff member who isn't working out. | People → staff row → DM only | Missing | [UXG-101](issues/UXG-101-give-the-owner-a-way-to-hire-retire-pause-or-re-brief-ai-sta.md), [UXG-102](issues/UXG-102-build-the-actor-profile-the-spec-requires.md) |
| US-060 | As an owner, I want to change which model a staff member thinks with. | People → DM → brain icon → pick model (silent) | Partial | [UXG-103](issues/UXG-103-confirm-model-changes-and-say-when-they-take-effect.md) |
| US-061 | As an owner, I want to see what a person is doing right now and open that work. | People directory second line ("Blocked: …"); whole row links to DM | Partial | [UXG-102](issues/UXG-102-build-the-actor-profile-the-spec-requires.md), [UXG-104](issues/UXG-104-make-a-person-s-current-work-clickable-from-people.md), [UXG-105](issues/UXG-105-unify-what-is-this-person-doing-between-people-and-the-offic.md) |
| US-062 | As an owner, I want to start a group room with a few people. | People → + → tick people (flat list) → Start conversation | Partial | [UXG-107](issues/UXG-107-add-search-and-grouping-to-the-new-conversation-people-picke.md) |
| US-063 | As an owner, I want to rename or archive a room I created, and read archived rooms. | Room → Users icon ("Participants") → Name / Archive; Chats → Archived → Restore only | Partial | [UXG-108](issues/UXG-108-make-room-rename-and-archive-discoverable.md), [UXG-109](issues/UXG-109-let-a-room-owner-leave-or-hand-over-a-group.md), [UXG-110](issues/UXG-110-let-archived-groups-be-read-without-restoring-and-allow-hidi.md) |
| US-064 | As an owner, I want to edit or delete a message I sent. | Hover → pencil/trash → Save (no Esc/⌘↵; mentions not re-parsed) | Partial | [UXG-111](issues/UXG-111-add-keyboard-shortcuts-to-message-editing-and-re-parse-menti.md) |
| US-065 | As an owner, I want to search old messages. | People search (unlabelled message hits) or in-room search | Partial | [UXG-112](issues/UXG-112-label-people-message-search-and-give-it-loading-and-empty-st.md), [UXG-113](issues/UXG-113-show-group-rooms-beyond-the-first-page-in-chats.md) |
| US-066 | As my cofounder (admin/member), I want to @mention someone so they're asked to reply. | Type @ → picker only for owners; members get nothing | Partial | [UXG-114](issues/UXG-114-give-admins-and-members-the-mention-picker.md), [UXG-115](issues/UXG-115-show-readable-names-in-mentions-while-composing.md) |
| US-067 | As an owner, I want to mute a noisy room. | No control anywhere | Missing | [UXG-116](issues/UXG-116-add-per-room-mute-and-notification-preferences.md) |
| US-068 | As an owner, I want to connect Telegram so I get approvals on my phone. | Company → Resources → Telegram → type Vault reference → Pair → send code | Partial | [UXG-117](issues/UXG-117-let-a-non-technical-owner-connect-telegram-without-typing-a.md), [UXG-118](issues/UXG-118-confirm-before-unpairing-telegram-and-let-the-owner-filter-w.md) |
| US-069 | As an owner, I want to revoke a standing permission I gave Exec and see what it covers. | Company → Members (bottom) → Standing permissions → Revoke | Partial | [UXG-119](issues/UXG-119-show-loading-error-states-and-scope-for-standing-permissions.md) |

### Apps

| ID | Story | Path | Verdict | Issues |
|---|---|---|---|---|
| US-070 | As an owner, I want to connect Gmail so Exec can read and answer customer email. | Apps → type "Gmail" → no catalogue match → only "Ask Exec" or paste an MCP URL | Missing | [UXG-127](issues/UXG-127-add-google-gmail-calendar-drive-microsoft-and-accounting-ser.md) |
| US-071 | As an owner, I want to connect Stripe and know what it may do before it is allowed. | Apps → Browse → Finance → Stripe → Add → sign in → permissions already granted | Partial | [UXG-137](issues/UXG-137-show-permissions-before-auto-allowing-an-added-app.md), [UXG-146](issues/UXG-146-show-couldn-t-load-instead-of-hiding-skills-or-saying-not-fo.md) |
| US-072 | As an owner, I want to connect a token-based service (GitHub) so Staff can manage issues. | Apps → Browse → GitHub → Add (no token asked; Vault field only via card body) | Partial | [UXG-128](issues/UXG-128-collect-the-token-when-a-token-auth-app-is-added-from-a-tile.md), [UXG-138](issues/UXG-138-replace-the-free-text-vault-secret-name-with-a-picker-or-inl.md) |
| US-073 | As an owner, I want to replace a token or change an app's address without losing its permissions. | App page → nothing to edit → Remove → add again → re-grant | Missing | [UXG-132](issues/UXG-132-allow-editing-a-connection-s-address-token-or-sign-in-withou.md) |
| US-074 | As an owner, I want to disconnect an app and first see what will lose access. | App page → Remove (runs at once) | Partial | [UXG-129](issues/UXG-129-confirm-before-removing-an-app-and-say-what-loses-access.md), [UXG-130](issues/UXG-130-remove-or-ask-about-a-plugin-bundle-s-know-how-when-the-bund.md) |
| US-075 | As an owner, I want to see what an app was used for, by whom, and for which Work. | App page → "Recent" (8 effects / 5 reads, purpose on hover) | Partial | [UXG-135](issues/UXG-135-make-app-activity-answer-what-was-it-used-for.md) |
| US-076 | As an owner, I want to switch an app off for a while without signing out. | App page → Freeze (blocks writes only) | Partial | [UXG-136](issues/UXG-136-offer-a-real-off-state-that-keeps-the-sign-in.md) |
| US-077 | As an owner, I want to approve an app Exec asked for, right in the chat, knowing why. | Exec rail → "Exec wants X" card (reason only on hover) → Add | Partial | [UXG-139](issues/UXG-139-show-why-exec-wants-an-app-in-the-chat-card-without-hovering.md), [UXG-137](issues/UXG-137-show-permissions-before-auto-allowing-an-added-app.md) |
| US-078 | As an owner, I want to decline an app request and tell Exec why or suggest another. | "Not needed" / "×" just dismisses | Partial | [UXG-140](issues/UXG-140-let-declining-an-app-request-carry-a-reason-or-alternative.md) |
| US-079 | As an owner, I want to read an imported skill (incl. scripts) before accepting it. | Apps → link → paste plugin URL → Waiting on you → skill → Accept/Decline on a one-line description | Partial | [UXG-131](issues/UXG-131-let-the-owner-read-a-skill-s-skill-md-and-scripts-before-acc.md), [UXG-141](issues/UXG-141-report-skipped-items-and-link-to-review-after-a-plugin-impor.md) |
| US-080 | As an owner, I want to write my own know-how ("how we quote jobs") so Staff follows it. | Apps → no create action | Missing | [UXG-133](issues/UXG-133-provide-a-way-to-create-and-edit-company-know-how.md) |
| US-081 | As an owner, I want to edit, disable or restore a skill after adding it. | Skill page → "Everyone" toggle and Remove only; removed skills vanish | Missing | [UXG-133](issues/UXG-133-provide-a-way-to-create-and-edit-company-know-how.md), [UXG-134](issues/UXG-134-show-removed-skills-with-restore-and-show-the-skill-s-versio.md), [UXG-142](issues/UXG-142-make-a-disabled-skill-a-visible-state-and-expose-per-agent-t.md) |
| US-082 | As an owner, I want to choose which AI model my company uses without learning jargon. | Company → Intelligence → Models (Connection / Harness / Model) | Partial | [UXG-143](issues/UXG-143-remove-technical-terms-from-the-model-choice.md), [UXG-144](issues/UXG-144-pick-one-place-to-add-model-keys-and-sign-ins.md) |
| US-083 | As an owner, I want to connect Claude Code/Codex to my company and later revoke access safely. | Account → AI apps → Create token → Copy → Revoke (no confirm) | Partial | [UXG-149](issues/UXG-149-confirm-token-revoke-and-handle-copy-failures-in-ai-apps.md) |
| US-084 | As an owner on my phone, I want to browse skills or a category, or add an app from a link. | Apps narrow → chips Ask / Waiting / In use / Browse only | Partial | [UXG-151](issues/UXG-151-make-all-apps-views-and-add-from-link-reachable-on-mobile.md) |

### Company

| ID | Story | Path | Verdict | Issues |
|---|---|---|---|---|
| US-085 | As an owner, I want to start a new company and give it a name I chose. | Home → Start a company → instantly created with a random name → /{id}/welcome → Exec's questions → read-back → Let's do it | Partial | [UXG-156](issues/UXG-156-let-the-owner-name-the-company-when-creating-it.md), [UXG-157](issues/UXG-157-report-a-failed-rename-during-onboarding-instead-of-dropping.md), [UXG-158](issues/UXG-158-use-a-multi-line-answer-box-on-the-welcome-page.md), [UXG-159](issues/UXG-159-make-the-empty-portfolio-text-match-the-real-button.md) |
| US-086 | As an owner who left the welcome chat halfway, I want to pick it up again. | Welcome → ← Home → later open company → no way back to welcome | Partial | [UXG-160](issues/UXG-160-provide-a-way-back-into-an-unfinished-welcome-conversation.md) |
| US-087 | As an owner, I want to edit my charter later without knowing Markdown. | Company → Charter → Edit (E) → raw Markdown textarea → Save | Partial | [UXG-162](issues/UXG-162-offer-a-non-technical-charter-editor-with-a-preview.md) |
| US-088 | As an owner, I want to see earlier charter versions and bring one back. | Company → Charter → click faint "Saved 3d ago" → history → no compare/restore | Partial | [UXG-163](issues/UXG-163-make-charter-history-findable-comparable-and-restorable.md) |
| US-089 | As an owner, I want to fill in my legal company details so contracts use the right legal name. | Company → Charter → Legal profile (read-only) | Missing | [UXG-164](issues/UXG-164-let-the-owner-add-or-edit-the-legal-profile.md) |
| US-090 | As an owner, I want a weekly Monday-9am check-in where Exec reviews last week's sales. | Company → Schedules → New schedule → fixed intervals or "Every weekday at…" only | Partial | [UXG-165](issues/UXG-165-support-specific-weekdays-monthly-schedules-and-a-company-ti.md) |
| US-091 | As an owner, I want to change what a schedule asks Exec to do, or how often. | Schedules → ⋯ → Change cadence / time only; prompt not editable | Partial | [UXG-166](issues/UXG-166-allow-editing-what-a-schedule-asks-exec-to-do.md), [UXG-167](issues/UXG-167-use-the-same-friendly-cadence-control-when-editing-a-schedul.md), [UXG-168](issues/UXG-168-add-run-now-and-rename-test-trigger.md) |
| US-092 | As an owner, I want to change the budget, choose monthly vs running, and be warned before hitting it. | Company → Limits → Spend → Edit limit → Save (running total only, no warning) | Partial | [UXG-169](issues/UXG-169-add-budget-periods-categories-and-early-warnings.md), [UXG-170](issues/UXG-170-warn-before-lowering-the-spend-limit-below-current-spend.md) |
| US-093 | As an owner, I want to store an API key, replace it when rotated, and delete it when unused. | Company → Vault → Add secret / "Replace an existing secret instead"; no delete | Partial | [UXG-171](issues/UXG-171-let-the-owner-delete-a-vault-secret-and-replace-any-secret-f.md), [UXG-172](issues/UXG-172-warn-when-a-new-secret-name-overwrites-an-existing-one-and-h.md) |
| US-094 | As an owner, I want to see why the company is unhealthy and fix it in one click. | Overview "Needs attention" → Health → failing row → action (only some checks) | Partial | [UXG-173](issues/UXG-173-give-every-failing-health-check-a-next-step.md), [UXG-174](issues/UXG-174-use-consistent-tones-and-plain-language-on-health.md) |
| US-095 | As an owner, I want to read what happened yesterday, filtered by type, date or person. | Company → Activity → All / Decisions / External (flat list) | Partial | [UXG-175](issues/UXG-175-let-activity-filter-by-date-person-and-text-and-group-by-day.md), [UXG-176](issues/UXG-176-page-external-actions-and-include-owner-setting-changes-in-a.md) |
| US-096 | As an owner, I want to rename my company and change its logo. | Company → Charter (!) → Company → Name / Logo | Partial | [UXG-177](issues/UXG-177-put-company-name-and-logo-where-owners-look-for-them.md), [UXG-178](issues/UXG-178-show-why-the-name-or-logo-didn-t-save-not-just-not-saved.md) |
| US-097 | As an owner, I want to archive a paused company and permanently delete a mistaken one. | Home → card ⋯ → Archive company (no confirm); Show archived → Restore; no delete | Partial | [UXG-179](issues/UXG-179-explain-what-archiving-a-company-does-before-doing-it.md), [UXG-180](issues/UXG-180-add-permanent-deletion-of-a-company.md), [UXG-181](issues/UXG-181-let-the-owner-open-an-archived-company-read-only-and-archive.md) |
| US-098 | As an owner, I want to export all my company's data as a backup. | No path | Missing | [UXG-182](issues/UXG-182-add-company-data-export.md) |
| US-099 | As an owner of several companies, I want to switch quickly and copy a setup from one to another. | Top bar → switcher → lands on home; any setting → ⋯ → Copy from another company… (one setting at a time) | Partial | [UXG-183](issues/UXG-183-keep-the-current-area-when-switching-companies-and-add-new-c.md), [UXG-184](issues/UXG-184-copy-a-full-setup-to-another-company-not-one-setting-at-a-ti.md) |
| US-100 | As an owner, I want one emergency switch to stop all outside actions and set payment allowances. | Company → Limits → Authority (read-only) | Missing | [UXG-185](issues/UXG-185-add-a-company-wide-freeze-of-outside-actions-and-payment-all.md) |

## Round 2 stories


### Exec chat

| ID | Story | Path | Verdict | Issues |
|---|---|---|---|---|
| US-101 | As an owner, I want to ask Exec something and watch its reply arrive so that I know it is working on my words | Topbar Exec (Ctrl/Cmd+J) → type in composer → Enter → turn dock shows "Exec is replying" and streams text → durable reply lands | Works | — |
| US-102 | As an owner, I want to retry a reply that failed so that a provider blip does not cost me my request | Send message → dock shows "Exec couldn't reply: …" → (no Retry) → retype or paste the message again | Missing | [UXG-191](issues/UXG-191-add-retry-to-a-failed-exec-reply.md) |
| US-103 | As an owner, I want to stop Exec mid-reply so that I can halt a wrong direction immediately | Exec working → composer → (must type text) → Interrupt / Ctrl+Enter | Partial | [UXG-192](issues/UXG-192-add-a-stop-button-that-halts-exec-s-reply-without-sending-a.md) |
| US-104 | As an owner, I want to see what Exec is doing right now even when I did not just message it so that silence never hides work | Rail open (or closed) while a schedule or Staff result wakes Exec → no dock, no button state → reply appears later as People badge | Partial | [UXG-193](issues/UXG-193-show-exec-s-live-work-even-when-the-owner-did-not-send-the-l.md), [UXG-194](issues/UXG-194-put-exec-s-unread-reply-and-working-state-on-the-exec-button.md) |
| US-105 | As an owner, I want to search and scroll my whole history with Exec so that I can find what we agreed weeks ago | Rail → search icon (Ctrl+F) → type → Enter / Show earlier messages | Partial | [UXG-195](issues/UXG-195-let-the-rail-reach-and-search-exec-history-older-than-the-la.md), [UXG-196](issues/UXG-196-keep-pinned-exec-messages-visible-regardless-of-age.md) |
| US-106 | As an owner, I want to attach files to a message and know the limits up front so that a send does not fail after I wait | Composer → paperclip → pick files → Send → server refusal | Partial | [UXG-198](issues/UXG-198-check-attachment-size-count-and-type-before-sending-from-the.md), [UXG-199](issues/UXG-199-keep-attached-files-quote-and-chosen-skills-when-switching-r.md) |
| US-107 | As an owner, I want my unsent message, files and retry state to survive navigation so that nothing I prepared is lost or duplicated | Type + attach in rail → switch topic or reload / click "Turn into Work" elsewhere → return | Partial | [UXG-199](issues/UXG-199-keep-attached-files-quote-and-chosen-skills-when-switching-r.md), [UXG-200](issues/UXG-200-persist-the-rail-s-retry-key-for-an-uncertain-send-across-re.md), [UXG-201](issues/UXG-201-stop-handed-in-drafts-from-silently-replacing-the-owner-s-un.md) |
| US-108 | As an owner, I want to find and safely use /goal and /loop so that I can set objectives and check-ins from the chat | Composer → type "/" at start → pick /goal or /loop → Enter → notice under composer | Partial | [UXG-202](issues/UXG-202-make-slash-commands-skills-references-and-people-discoverabl.md), [UXG-203](issues/UXG-203-confirm-before-loop-clear-stops-every-loop-and-let-goal-clea.md), [UXG-204](issues/UXG-204-stop-a-failed-goal-send-from-creating-a-duplicate-goal-on-re.md), [UXG-205](issues/UXG-205-record-command-results-in-the-conversation-instead-of-a-ling.md) |
| US-109 | As an owner, I want to leave Exec a message while its model or runtime is down so that it acts when it comes back | Model connection lapses → rail shows connect button instead of composer / composer disabled while reconnecting | Missing | [UXG-197](issues/UXG-197-let-the-owner-write-and-leave-a-message-while-exec-is-unreac.md) |
| US-110 | As a member or admin, I want the same rail context the owner gets so that I can work with Exec without guessing | Member opens Exec rail → no Under way, no handoffs, no # references, no commands | Partial | [UXG-206](issues/UXG-206-give-admins-operators-and-members-the-rail-context-the-owner.md) |
| US-111 | As an owner, I want to see a Staff member or group-room agent working on my message so that a DM is not a black box | People → Staff member → send → (no turn dock) | Partial | [UXG-207](issues/UXG-207-show-live-progress-when-talking-to-a-staff-member-or-an-agen.md) |
| US-112 | As an owner, I want to know whether Exec answered or quietly took my message so that I do not wait for a reply that will not come | Send → turn ends without a message → dock says "Replied" while receipt says "Seen by Exec" | Partial | [UXG-208](issues/UXG-208-say-seen-no-reply-instead-of-replied-when-exec-s-turn-ends-w.md) |
| US-113 | As an owner using a screen reader, I want new replies announced so that I can converse without hunting the transcript | Send → reply lands → only "Replied" is announced | Partial | [UXG-209](issues/UXG-209-announce-new-exec-replies-to-screen-readers.md) |
| US-114 | As an owner, I want to point Exec at exactly what I am looking at so that "explain this" means this paragraph or row | Main surface → context chip (page name) → send → later reread the sent message | Partial | [UXG-210](issues/UXG-210-let-a-selection-on-the-main-surface-travel-with-the-message.md), [UXG-211](issues/UXG-211-show-on-a-sent-message-which-screen-it-was-linked-to.md) |
| US-115 | As an owner, I want to start a fresh topic with Exec and find earlier topics so that conversations stay separable | Rail header → New conversation icon → (no cancel) → send / scroll back for prior focus | Partial | [UXG-212](issues/UXG-212-let-the-owner-cancel-a-pending-new-conversation-and-see-wher.md) |
| US-116 | As an owner, I want to copy a reply, a link to it, or the whole conversation so that I can share it outside | Hover message → copy icon / More → Copy link → (no conversation copy/export) | Partial | [UXG-213](issues/UXG-213-let-the-owner-copy-or-export-a-whole-conversation.md) |
| US-117 | As an owner, I want to see and change the model and effort Exec uses from the chat so that I can trade quality for cost quickly | Composer → brain chip (hover for name) → pick a model → (effort read-only) | Partial | [UXG-214](issues/UXG-214-show-the-current-model-on-the-composer-chip-and-allow-effort.md) |
| US-118 | As an owner, I want to resize, close and reopen the rail and have it remembered so that the main surface keeps my preferred width | Drag rail edge / arrow keys / Close icon / Ctrl+J → reload | Works | — |
| US-119 | As an owner, I want to answer Exec's question with one tap and an undo so that small decisions are quick | Exec asks → tap a likely answer → Undo within 2 s or let it send | Works | — |
| US-120 | As an owner, I want to peek at Exec's reasoning during a long turn so that I can redirect early | Dock → expand live notes → tools listed, thinking never shown | Partial | [UXG-215](issues/UXG-215-let-the-expanded-turn-dock-show-exec-s-thinking-collapsed-by.md) |

### Doc & sheet editors

| ID | Story | Path | Verdict | Issues |
|---|---|---|---|---|
| US-121 | As an owner, I want to underline, highlight, colour and align text so that a proposal or letter I write looks the way I would format it in Word or Google Docs. | Library → open document → select text → formatting bubble (Bold / Italic / Strike / Code / Link) | Partial | [UXG-216](issues/UXG-216-add-underline-highlight-text-colour-and-alignment-to-documen.md) |
| US-122 | As an owner, I want to paste a section from a Word file or a Google Doc and keep its pictures, underlines and table layout so that I do not have to rebuild it by hand. | Word or Google Docs → copy → Library → open document → paste | Partial | [UXG-217](issues/UXG-217-keep-images-formatting-and-merged-cells-when-pasting-from-wo.md) |
| US-123 | As an owner, I want to type @ in a document and pick a colleague or Staff member so that the page names who owns what, the way comments already can. | Library → open document → type "@" in the body | Missing | [UXG-218](issues/UXG-218-let-writers-mention-people-in-the-document-body.md) |
| US-124 | As an owner, I want to link to another document, a Work item or a file as a titled chip, and see where a link goes, so that pages connect to the rest of the company. | Library → open document → paste a cockpit link or press ⌘K → Link | Partial | [UXG-219](issues/UXG-219-insert-links-to-documents-work-and-files-as-titled-chips-and.md) |
| US-125 | As an owner, I want to find and replace a word across a long document, see how long it is, and jump between its headings so that I can edit a handbook or policy quickly. | Library → open document → ⌘F / look for outline or word count | Missing | [UXG-220](issues/UXG-220-add-find-and-replace-in-documents.md), [UXG-221](issues/UXG-221-show-an-outline-of-headings-and-make-in-page-links-work.md), [UXG-222](issues/UXG-222-show-word-and-character-count.md) |
| US-126 | As an owner, I want to drop a PDF or contract into a document, and resize, describe and caption pictures so that a brief carries its supporting material. | Library → open document → drag a PDF in / "/" → Image → select image | Partial | [UXG-223](issues/UXG-223-let-documents-carry-files-other-than-images.md), [UXG-224](issues/UXG-224-allow-resizing-describing-and-captioning-images.md) |
| US-127 | As an owner, I want to add rows above, columns to the left, mark a header row and widen columns so that a pricing table in a doc is easy to build. | Library → open document → "/" → Table → click a cell → table bar | Partial | [UXG-225](issues/UXG-225-extend-document-tables-rows-above-columns-left-header-row-wi.md) |
| US-128 | As an owner on a phone or using only the keyboard, I want to move, duplicate or delete a paragraph so that I can restructure a page without a mouse hover. | Phone → Library → open document → tap a block (no grip) / keyboard → Ctrl+/ | Partial | [UXG-226](issues/UXG-226-let-blocks-be-moved-duplicated-and-deleted-without-the-hover.md) |
| US-129 | As an owner, I want to comment on the exact words I selected so that my colleague knows which phrase I mean, not the whole list or table. | Library → open document → select a phrase inside a bullet → Comment | Partial | [UXG-227](issues/UXG-227-anchor-document-comments-to-the-selected-words-not-the-whole.md) |
| US-130 | As an owner on flaky Wi-Fi, I want to keep typing through a short connection drop and never see older text in place of my work so that I trust the editor. | Library → open document or sheet → connection drops → keep typing | Partial | [UXG-228](issues/UXG-228-keep-editors-editable-through-brief-connection-drops.md), [UXG-229](issues/UXG-229-don-t-strand-a-co-editor-s-unsaved-edits-when-someone-restor.md) |
| US-131 | As an owner pasting a long report, I want to be told plainly if a document is getting too large to save so that I don't keep writing into a page that is silently no longer saving. | Library → open document → paste a very long text → "Not saved" banner | Partial | [UXG-230](issues/UXG-230-say-plainly-when-a-document-is-too-large-to-save-and-warn-be.md) |
| US-132 | As an owner, I want a refused or oversized change in a sheet to lose only that change so that the edits I typed after it still save. | Library → open sheet → paste a large block → keep editing → "Your last change was not saved" | Partial | [UXG-231](issues/UXG-231-stop-a-refused-sheet-change-from-discarding-every-edit-queue.md) |
| US-133 | As an owner, I want to save a named version ("Sent to bank") of a document or sheet and start a new document from a template so that milestones and repeat formats are easy. | Library → New document / open document → History; Library → open sheet → More → Save version | Partial | [UXG-232](issues/UXG-232-let-writers-save-and-name-a-version-from-the-editor.md), [UXG-233](issues/UXG-233-offer-templates-and-use-the-document-kinds-when-starting-a-d.md) |
| US-134 | As an owner, I want to comment on a range of cells and have the comment stay with its row when someone inserts rows above so that discussions stay on the right figures. | Library → open sheet → select A2:D2 → right-click → Comment → insert row above | Partial | [UXG-235](issues/UXG-235-let-sheet-comments-cover-a-range-and-follow-their-cell-when.md) |
| US-135 | As an owner, I want to use formulas, number/currency/date formats, sort, filter, freeze rows, charts, tabs and keyboard navigation in a sheet so that it replaces my spreadsheet app. | Library → New sheet → engine menus (Edit / View / Insert / Format / Data) | Works | [UXG-236](issues/UXG-236-allow-pictures-in-sheets.md), [UXG-239](issues/UXG-239-use-the-company-s-locale-and-currency-for-sheet-formats.md) |
| US-136 | As an owner, I want to print or save a sheet as PDF, see it in my dark theme, and edit it on a phone so that it works wherever I am. | Library → open sheet → More (no Print) / dark theme / phone width | Partial | [UXG-237](issues/UXG-237-add-print-save-as-pdf-for-sheets.md), [UXG-238](issues/UXG-238-follow-the-dark-theme-in-the-sheet-grid.md), [UXG-240](issues/UXG-240-make-sheet-editing-on-a-phone-a-deliberate-tested-path.md) |
| US-137 | As an owner, I want visible Undo / Redo on desktop and printed documents with page breaks and page numbers so that I can recover and hand over a polished page. | Library → open document → … menu / Print or save as PDF | Partial | [UXG-234](issues/UXG-234-show-undo-and-redo-on-desktop-too.md), [UXG-241](issues/UXG-241-add-page-breaks-and-page-numbers-to-printed-documents.md) |

### Company computer & intelligence

| ID | Story | Path | Verdict | Issues |
|---|---|---|---|---|
| US-138 | As an owner, I want to restart a computer that passes its probes but is frozen (hung app, stuck dialog, black desktop) so that the company's work resumes without a support call | Company → Computer → (no lifecycle control) → Company → Health → (Restart appears only when a probe already fails) | Missing | [UXG-242](issues/UXG-242-let-the-owner-restart-a-running-computer-that-looks-healthy.md) |
| US-139 | As an owner, I want to type into the exact browser session the company prepared for me so that I can finish the sign-in or payment step it brought me | Inbox → "Your participation" item → work through on the computer → Take control → click and type in the desktop → Return control | Missing | [UXG-243](issues/UXG-243-make-the-inbox-handover-desktop-accept-the-owner-s-keyboard.md), [UXG-244](issues/UXG-244-keep-the-handover-desktop-connected-after-returning-control.md), [UXG-265](issues/UXG-265-replace-the-raw-browser-health-word-in-the-handover-header.md) |
| US-140 | As an owner, I want to step away for a minute to fetch a 2FA code without the agent being told I am done so that it does not resume over my half-finished step | Computer → Enter computer → click the desktop → wait more than 60 seconds | Partial | [UXG-245](issues/UXG-245-stop-a-60-second-pause-from-telling-the-agent-the-owner-has.md) |
| US-141 | As an owner, I want to see whether an agent is using the computer right now, and which one, before my first click takes control or changes its screen size | Computer → Enter computer → read the toolbar state → click the desktop | Partial | [UXG-246](issues/UXG-246-show-when-an-agent-is-using-the-computer-and-who-before-the.md), [UXG-247](issues/UXG-247-stop-a-viewer-from-resizing-the-shared-desktop-while-an-agen.md) |
| US-142 | As an owner on my phone, I want to type a code into the company browser so that a handover does not have to wait until I am at a desk | Phone → Inbox handover or Computer → Enter → tap a text field in the desktop | Missing | [UXG-248](issues/UXG-248-let-a-phone-user-type-into-the-live-desktop.md) |
| US-143 | As a keyboard user, I want a discoverable way out of the live desktop's keyboard capture so that I can reach Release control and Leave computer | Computer → Enter computer → Tab into the desktop → try to Tab out | Partial | [UXG-249](issues/UXG-249-tell-keyboard-users-how-to-leave-the-live-desktop.md) |
| US-144 | As an owner, I want to see and download a file that exists only on the company computer (an invoice the browser downloaded, an export) so that I can keep or forward it | Computer → (no file view) → Library (registered Work outputs and uploads only) | Missing | [UXG-250](issues/UXG-250-let-the-owner-see-and-download-files-that-exist-only-on-the.md) |
| US-145 | As an owner, I want to hand a file (an ID scan, a signed PDF) into the live desktop during a handover so that I can upload it in the prepared web form | Inbox handover → desktop → Clipboard panel (text only) → handover composer (sends to the lead, not the desktop) | Missing | [UXG-251](issues/UXG-251-let-the-owner-drop-a-file-into-the-live-desktop-during-a-han.md) |
| US-146 | As an owner, I want to know how long waking takes and why a sleeping computer will not wake so that I can fix the real cause | Computer → "Wake and enter" → "Waking…" → "could not wake. Try again in a minute." | Partial | [UXG-252](issues/UXG-252-say-when-the-monthly-hours-limit-is-why-the-computer-will-no.md), [UXG-253](issues/UXG-253-show-wake-progress-and-keep-trying-instead-of-failing-after.md), [UXG-255](issues/UXG-255-point-every-computer-is-stopped-message-at-the-place-that-ca.md) |
| US-147 | As an owner, I want to know when the computer is running out of disk, memory or CPU so that a failed run has an explanation and a fix | Company → Limits → Computer / Company → Health → Raw state | Missing | [UXG-254](issues/UXG-254-surface-disk-memory-and-cpu-pressure-on-the-company-computer.md) |
| US-148 | As a member, I want to open a link a colleague shared in chat so that I can read it | People → a chat → click an external link → bounced to Inbox with a refusal toast | Missing | [UXG-256](issues/UXG-256-let-people-choose-where-an-external-link-opens-and-stop-rout.md) |
| US-149 | As an owner, I want to know that a frequent schedule keeps the computer awake and stops when the monthly hours limit is reached so that I can set cadence and limits together | Company → Schedules → Create a schedule → Repeat "Every 30 minutes" | Partial | [UXG-257](issues/UXG-257-show-how-schedules-interact-with-computer-sleep-and-the-mont.md) |
| US-150 | As an owner, I want a model I choose to be proven working before agents depend on it so that a typo does not stall the company later | Company → Intelligence → Models → Change → Custom model… → Save | Partial | [UXG-258](issues/UXG-258-prove-a-chosen-model-answers-before-saving-it.md) |
| US-151 | As an owner, I want the Claude harness sign-in brought to me ready to complete, and a fix when installed agent software is out of date | Company → Intelligence → Company-only Claude → Sign in with Claude → Continue Claude sign-in in company desktop / Installed agent software | Partial | [UXG-259](issues/UXG-259-bring-the-claude-harness-sign-in-window-forward-with-control.md), [UXG-260](issues/UXG-260-offer-a-fix-when-installed-agent-software-does-not-match-the.md), [UXG-262](issues/UXG-262-read-the-brand-name-from-config-in-the-harness-billing-note.md) |
| US-152 | As an owner, I want to review a slide deck the company produced in place, like a Word or Excel outcome, so that I do not need PowerPoint installed | Inbox → outcome review item (.pptx) → "Browsers cannot show PowerPoint files. Download it" | Partial | [UXG-261](issues/UXG-261-preview-powerpoint-outcomes-in-the-cockpit-like-word-and-exc.md) |
| US-153 | As an owner, I want old links and each setting to land where I expect them so that I find harnesses and Telegram without hunting | old /company/harnesses link → Intelligence with harness section folded / Company → Limits → Telegram | Partial | [UXG-263](issues/UXG-263-move-telegram-out-of-the-limits-page.md), [UXG-264](issues/UXG-264-delete-the-never-rendered-authority-page-and-keep-the-harnes.md) |
| US-154 | As an owner, I want to send a password from my device into an app on the company computer and copy text back out | Computer → Enter computer → click to take control → Clipboard → Read device → Send → Ctrl+V in the app | Works | — |

### Shell & cross-cutting

| ID | Story | Path | Verdict | Issues |
|---|---|---|---|---|
| US-155 | As an owner, I want to move between Inbox, Work, Library, People, Apps and Company from one bar so that I always know where I am | Any company page → top bar tab (or G then I/W/L/P/A/C) | Works | — |
| US-156 | As an owner on a phone, I want to reach all six surfaces with my thumb so that I can run the company away from my desk | Phone → bottom dock → tap a surface | Partial | [UXG-282](issues/UXG-282-move-the-connection-banner-above-the-phone-dock-and-out-of-t.md) |
| US-157 | As an owner, I want ⌘K to find a document, sheet, comment or message by a phrase inside it so that I do not hunt through surfaces | ⌘K → type a phrase → pick a Library / Conversations / Comments row | Partial | [UXG-275](issues/UXG-275-say-when-k-search-could-not-reach-a-source-instead-of-no-mat.md) |
| US-158 | As an owner, I want to hand whatever I typed in ⌘K to Exec when nothing matches so that a question never dead-ends in "No match" | ⌘K → type a question → (no Ask Exec row) | Missing | [UXG-277](issues/UXG-277-offer-ask-exec-about-with-the-typed-text-when-k-has-no-answe.md) |
| US-159 | As an owner, I want ⌘K to find the apps I already connected, schedules and recent activity so that I can jump to them by name | ⌘K → type "Slack" or "weekly report" → result list | Partial | [UXG-276](issues/UXG-276-stop-offering-add-app-for-apps-already-connected-and-let-k-f.md) |
| US-160 | As a hosted owner or member, I want an expired session to bring me back to the same page with my unsent text intact so that a lapse costs me nothing | Company page → session lapses → automatic reload / re-entry → same page | Partial | [UXG-271](issues/UXG-271-keep-unsent-text-across-session-re-entry-and-update-reloads.md), [UXG-270](issues/UXG-270-give-the-couldn-t-open-this-company-dialog-a-way-out-when-re.md) |
| US-161 | As an owner or member on Cloud, I want to sign out from inside a company so that I can leave a shared computer safely | Company page → profile circle → (no Sign out) → Account → account menu → Sign out | Missing | [UXG-272](issues/UXG-272-add-sign-out-to-the-in-company-profile-menu.md), [UXG-289](issues/UXG-289-clear-per-company-browser-memory-when-a-person-signs-out.md) |
| US-162 | As an owner, I want a mistyped, removed or archived company address to send me somewhere useful so that I am never stuck behind a modal | Open /<unknown-id> → "We couldn’t open this company" → Try again only | Partial | [UXG-270](issues/UXG-270-give-the-couldn-t-open-this-company-dialog-a-way-out-when-re.md), [UXG-290](issues/UXG-290-write-the-error-page-in-owner-language-and-name-the-product.md), [UXG-288](issues/UXG-288-reserve-app-route-names-so-a-company-id-cannot-be-shadowed.md) |
| US-163 | As an owner, I want old bookmarks (Rooms, Connections, Skills, Authority, Decisions, work/documents) to keep working so that shared links do not break | Open an old URL → redirected to its current home | Works | [UXG-285](issues/UXG-285-remove-the-aris-fallback-from-every-company-route-and-retire.md) |
| US-164 | As an owner, I want to be told when the connection drops and when it is back so that I trust what is on screen | Daemon unreachable → bottom banner "Connection lost. Reconnecting…" → Retry now → "Reconnected." | Partial | [UXG-282](issues/UXG-282-move-the-connection-banner-above-the-phone-dock-and-out-of-t.md) |
| US-165 | As an owner keeping a tab open for days, I want to know when a new version is deployed so that I am not running stale code against a new daemon | Long-open tab → deploy happens → (no notice) → next lazy load fails → silent reload | Partial | [UXG-273](issues/UXG-273-tell-an-open-tab-when-a-new-version-has-been-deployed.md), [UXG-271](issues/UXG-271-keep-unsent-text-across-session-re-entry-and-update-reloads.md) |
| US-166 | As an owner, I want every undoable action’s toast to stay reachable long enough, by mouse or keyboard, so that I can take it back | Archive a company or dismiss an item → toast with Undo → click or Tab to Undo | Partial | [UXG-266](issues/UXG-266-keep-failure-toasts-until-dismissed-and-never-evict-a-live-u.md), [UXG-267](issues/UXG-267-make-undo-reachable-from-the-keyboard-within-its-window.md) |
| US-167 | As an owner with several companies, I want my background tab and desktop notifications to tell me which company needs me so that I respond in time | Tab in background → new Inbox item → desktop notification / tab title | Partial | [UXG-268](issues/UXG-268-notify-for-every-company-and-name-the-company-in-each-deskto.md), [UXG-269](issues/UXG-269-show-what-needs-the-owner-in-the-tab-title-or-favicon.md) |
| US-168 | As a keyboard and screen-reader user, I want skip links, the command menu and pickers to behave the same on every page so that I can work without a mouse | Tab from page top → Skip to content → ⌘K → arrow / Enter | Partial | [UXG-267](issues/UXG-267-make-undo-reachable-from-the-keyboard-within-its-window.md), [UXG-279](issues/UXG-279-ignore-enter-while-an-ime-is-composing-in-k-and-other-picker.md), [UXG-286](issues/UXG-286-give-the-home-and-account-frame-the-skip-link-k-and-correct.md) |
| US-169 | As an owner using dark mode, I want every surface to follow the theme so that no pane flashes white | Profile → Dark → open Library sheet / any page | Partial | [UXG-238](issues/UXG-238-follow-the-dark-theme-in-the-sheet-grid.md) |
| US-170 | As an owner outside the US, I want dates, times and money written one consistent way in my locale so that I can read them at a glance | Any page with timestamps or spend → compare formats | Partial | [UXG-280](issues/UXG-280-use-the-shared-date-time-and-money-formatters-everywhere.md) |
| US-171 | As an owner, I want the browser Back button to leave the Inbox after I walked it with J/K so that history means places, not keystrokes | Inbox → J ×10 → browser Back | Partial | [UXG-281](issues/UXG-281-do-not-add-a-browser-history-entry-for-every-j-k-step-in-the.md) |
| US-172 | As a member, I want my profile menu to offer only what I can use so that I do not land on owner-only pages | Member company page → profile circle → Account | Partial | [UXG-274](issues/UXG-274-stop-sending-members-to-the-owner-s-account-pages.md) |
| US-173 | As an owner, I want a slow page move to let me genuinely stay where I am so that a late load does not yank me away | Click a link → 12 s stall → "Stay on this page" | Partial | [UXG-278](issues/UXG-278-make-stay-on-this-page-actually-cancel-the-stalled-navigatio.md) |
| US-174 | As an owner sensitive to motion, I want transitions, the loading bar and celebrations to calm down under reduced motion so that the app is comfortable | OS reduced motion on → navigate, open ⌘K, finish work | Works | — |
| US-175 | As an owner, I want to print a Work page, Activity or Limits cleanly so that I can file or share a paper copy | Any non-document page → browser Print | Partial | [UXG-287](issues/UXG-287-add-print-styles-beyond-documents.md) |
| US-176 | As a customer, I want the shipped app to carry only my product’s name and no internal demo pages so that it looks finished and keeps founders’ data private | Open /gallery/home or the offline fallback page | Partial | [UXG-283](issues/UXG-283-take-the-product-name-from-brand-config-in-the-shell-offline.md), [UXG-284](issues/UXG-284-stop-shipping-the-gallery-and-office-demo-routes-with-a-foun.md) |

### Endpoint & CLI parity

| ID | Story | Path | Verdict | Issues |
|---|---|---|---|---|
| US-177 | As an owner, I want to stop Exec in the middle of a reply without having to type a new message, so that I can cancel a direction I regret before it acts | Rail → Exec is working → (no Stop control; only type a new message and press Ctrl/Cmd+Enter to interrupt). CLI only: restless chat → Ctrl-C | Partial | [UXG-192](issues/UXG-192-add-a-stop-button-that-halts-exec-s-reply-without-sending-a.md) |
| US-178 | As an owner, I want to authorise a build the company has prepared for publishing, seeing its audience, expiry and limits, so that the company can ship a demo or game server without me opening a terminal | Company → Resources → published service shows "awaiting owner authorization" with no action. CLI only: restless publish authorize --publication <id> | Missing | [UXG-291](issues/UXG-291-let-the-owner-authorise-a-prepared-publication-from-the-cock.md) |
| US-179 | As an owner, I want to invite testers to, revoke an invitation from, and stop a published build, so that I control who can reach what the company put online | No UI. CLI only: restless publish invite / revoke / stop / show / list | Missing | [UXG-292](issues/UXG-292-let-the-owner-invite-revoke-and-stop-a-live-publication-with.md) |
| US-180 | As an owner, I want to back up all my companies and restore them on a new machine from the product, and see when the last backup ran, so that a lost laptop does not lose my business | No UI. CLI only: restless appliance backup <file> / restless appliance restore <file> | Missing | [UXG-293](issues/UXG-293-offer-appliance-backup-and-restore-in-the-product-with-a-las.md) |
| US-181 | As an owner, I want to see which Restless release I am on, apply an update and roll back a bad one from the product, so that keeping current does not require a terminal | No UI. CLI only: restless appliance upgrade / rollback / status | Missing | [UXG-294](issues/UXG-294-show-the-running-release-and-allow-update-and-rollback-from.md) |
| US-182 | As an owner, when work admission is paused or schedule wakes need repair, I want a button that repairs it, so that I am not told to run a shell command from a banner | Home → appliance notice shows "Run `restless appliance resume`" text, no action. CLI only: restless appliance resume / start / wake-due | Partial | [UXG-295](issues/UXG-295-replace-terminal-commands-in-appliance-repair-banners-with-a.md) |
| US-183 | As an owner, I want to turn on sharing (individual sign-in, private Tailscale addresses) from the product, so that I can invite my team without following installer commands | Company → Members → Enable sharing → Download setup → copy three terminal commands. CLI only: restless appliance prepare-sharing / tailscale-addresses / enable-sharing | Partial | [UXG-296](issues/UXG-296-activate-sharing-and-tailscale-addresses-without-leaving-the.md) |
| US-184 | As an owner, I want to save a named version of a document at a meaningful moment, as I can with a sheet, so that I can return to "the version I sent the client" | Library → open document → (no Save version; sheets have one). CLI only: restless document checkpoint | Missing | [UXG-297](issues/UXG-297-let-the-owner-save-a-named-version-of-a-document-as-they-can.md) |
| US-185 | As an owner, I want to see which agent and which model spent my model budget, so that I can tell whether a role or a model choice is wasting money | Company → Limits → Spend shows only one total bar. CLI only: restless spend | Missing | [UXG-298](issues/UXG-298-break-model-spend-down-by-agent-and-model.md) |
| US-186 | As an owner, when spend reads "At least $X" because some calls reported no price, I want to know why and clear it after checking, so that the limit is trustworthy again | Company → Limits → Spend → "At least" prefix, no explanation or action. CLI only: restless clear-poison / restless spend-correct | Missing | [UXG-299](issues/UXG-299-explain-at-least-spend-and-let-the-owner-clear-a-spend-poiso.md) |
| US-187 | As an owner, I want to form a team around a lead, rename it, change its lead or disband it, so that I can shape the organisation directly when I disagree with Exec | People → team directory → only "who owns this team" select. CLI only: restless teams create / update / lead / disband | Missing | [UXG-300](issues/UXG-300-let-the-owner-create-rename-re-lead-and-disband-teams.md) |
| US-188 | As an owner, I want to see which scheduled runs fired or were skipped, recover a skipped one, and choose what happens when my laptop was asleep, so that a missed Monday report is not silently lost | Company → Schedules → only "last fired". CLI only: restless schedule history / recover / policy | Missing | [UXG-301](issues/UXG-301-show-schedule-run-history-and-let-the-owner-recover-a-skippe.md) |
| US-189 | As an owner, I want to choose fallback models used when my main model is unavailable, so that the company keeps working through an outage or quota | Company → Model → one model only. CLI only: restless company set model_failover | Missing | [UXG-302](issues/UXG-302-let-the-owner-choose-fallback-models.md) |
| US-190 | As an owner, I want to connect my business bank or payment account and see balances and recent payments, so that payment allowances mean something I can check | Company → Limits → Payment allowances (read-only, no account, no balances). CLI only: restless finance connect-airwallex / balances / show | Missing | [UXG-312](issues/UXG-312-let-the-owner-connect-the-payment-rail-and-approve-payees-fr.md) |
| US-191 | As an owner, I want to put the company computer to sleep or stop it now, so that I can halt running costs or activity immediately | Company → Doctor offers start / restart / reconcile only. CLI only: restless sleep / restless down | Missing | [UXG-242](issues/UXG-242-let-the-owner-restart-a-running-computer-that-looks-healthy.md) |
| US-192 | As an owner, I want to approve a contact in advance (for example my accountant) so that Exec does not need to ask before the first email | Company → Limits → Approved contacts → Revoke only. CLI only: restless approve <party> | Missing | [UXG-303](issues/UXG-303-allow-pre-approving-a-contact-from-approved-contacts.md) |
| US-193 | As an owner, I want to see the full history of changes Exec applied for other people and how often Exec's quick decisions were right, so that I can judge whether its delegation is earning trust | Inbox → Today shows today's digest only; Activity shows decisions and outside actions. No UI for direction/applied history or the decisions record | Missing | [UXG-304](issues/UXG-304-surface-or-delete-the-structured-decisions-record-route.md), [UXG-013](issues/UXG-013-make-the-today-line-link-to-its-lists.md) |
| US-194 | As an owner, I want every button in the cockpit to reach a real endpoint with the shape it expects, so that actions never fail silently | Systematic check of web/src/lib/model fetch paths against owner.rs routes | Works | [UXG-305](issues/UXG-305-prune-unused-client-contracts-left-after-the-route-parity-ch.md) |

### Cross-surface journeys

| ID | Story | Path | Verdict | Issues |
|---|---|---|---|---|
| US-195 | As a brand-new owner, I want to learn before I start talking that Exec needs a model, and come back to the same interview after connecting one, so that my first hour isn't spent on a dead end | Home → Start a company → Welcome ("So, tell me about this idea of yours") → type answer → "I can't reply until I have a model… Connect one" → Company → Intelligence → connect → (no way back to the interview) | Partial | [UXG-322](issues/UXG-322-check-for-a-model-before-the-welcome-interview-starts.md) |
| US-196 | As an owner, I want to review the first proposal Exec delivers, comment on it, ask for a revision and then find the accepted result, so that the first-outcome loop closes without me having to work out the plumbing | Rail brief → Inbox review item → Review in rail → comment in the document → Request changes in rail → Accept → "Open conversation" toast → Library → document (no link back to its Work) | Partial | [UXG-317](issues/UXG-317-link-a-native-document-to-the-work-it-was-produced-for.md), [UXG-318](issues/UXG-318-route-a-document-comment-asking-for-a-change-to-the-work-s-r.md), [UXG-325](issues/UXG-325-point-to-the-accepted-result-after-an-inbox-decision.md) |
| US-197 | As an owner, I want Exec to handle a customer who emailed us end to end and show me what was said, so that support runs without me writing every reply | Company → Access & limits → propose email mandate → Inbox approve mandate → customer writes in → Exec replies once → customer writes back → Exec cannot send a second reply → Inbox "First contact: party" (no Work or thread context) | Missing | [UXG-310](issues/UXG-310-let-exec-reply-to-a-customer-who-wrote-in-not-just-send-one.md), [UXG-311](issues/UXG-311-show-which-work-and-customer-an-approval-or-external-action.md) |
| US-198 | As an owner, I want a weekly review of what the company finished, decided, sent and spent, so that I can judge the week and set next week's direction in one sitting | Inbox "Today" line (today only) → Work board Done column (no dates) → Company → Activity (decisions and external actions, no Work, no period) → Company → Limits (lifetime total spend only) | Missing | [UXG-320](issues/UXG-320-provide-a-weekly-review-across-work-decisions-actions-and-sp.md) |
| US-199 | As an owner, I want to be told when the model budget runs out overnight and see work resume once I raise it, so that the company doesn't stall silently | Budget exhausted at 2am → (no Inbox item, no tab badge, no Telegram) → owner notices nothing is happening → Company → Limits → Edit limit → Save → (no wake sent) | Missing | [UXG-306](issues/UXG-306-tell-the-owner-when-the-model-budget-stops-the-company.md), [UXG-307](issues/UXG-307-resume-stalled-work-when-the-owner-raises-the-spend-limit.md) |
| US-200 | As an owner, I want to hear when an action I approved fails or ends with an unknown result, so that I can retry, reconcile or tell the customer | Inbox approval → Allow once → action runs and fails → (nothing in Inbox) → Company → Activity → row says "It ran and failed." | Partial | [UXG-308](issues/UXG-308-bring-a-failed-or-uncertain-approved-action-back-to-the-owne.md), [UXG-311](issues/UXG-311-show-which-work-and-customer-an-approval-or-external-action.md) |
| US-201 | As an owner, I want a scheduled job that failed or was skipped overnight to reach me, so that recurring work doesn't quietly stop | Schedule runs at 2am and fails, or is skipped because it was missed by over two hours → (no Inbox item, no Company badge) → Company → Schedules → "Last run needs attention" | Partial | [UXG-309](issues/UXG-309-surface-failed-and-skipped-scheduled-runs-outside-the-schedu.md) |
| US-202 | As an owner going on holiday, I want to hand decisions to a deputy, pause recurring work in one step and get only urgent pings on my phone, so that the company keeps going while I'm away | (no away control) → Company → Schedules → Pause each schedule one by one → Members (admins cannot approve) → Telegram pairing (links from a local cockpit open only on the paired computer) | Missing | [UXG-314](issues/UXG-314-add-an-away-mode-that-pauses-recurring-work-and-names-a-depu.md), [UXG-315](issues/UXG-315-make-telegram-notices-actionable-away-from-the-paired-comput.md) |
| US-203 | As an owner bringing in a cofounder, I want to make them an admin who can see what the company did and spent and take decisions in their area, so that we can split the work | Company → Members → Invite as Admin → cofounder signs in → Inbox (only what was handed to them) → Company → Activity / Limits / Apps (refused) | Partial | [UXG-313](issues/UXG-313-give-an-admin-cofounder-visibility-into-activity-spend-and-a.md) |
| US-204 | As an owner, I want to review the website Exec built, ask for changes, approve publishing and then see it live, so that the site actually ships | Inbox review → Review opens live preview in company browser → Request changes in rail → Accept → (no publish approval in Inbox) → (no live link anywhere) | Partial | [UXG-291](issues/UXG-291-let-the-owner-authorise-a-prepared-publication-from-the-cock.md), [UXG-324](issues/UXG-324-link-the-computer-is-stopped-review-message-to-the-recovery.md) |
| US-205 | As an owner, I want to set up a spending allowance with approved payees, then approve one purchase Exec prepared, so that the company can pay suppliers within my limits | (no Apps entry for the payment provider) → (no allowance or payee setup in Company) → Inbox payment item → "Review and approve in Airwallex" → Airwallex → back to Work | Partial | [UXG-312](issues/UXG-312-let-the-owner-connect-the-payment-rail-and-approve-payees-fr.md) |
| US-206 | As an owner, I want to connect the app Exec asked for mid-task and have that task resume on its own, so that I never have to report back | Rail app-request card or Inbox → Apps → app page → Connect → Allow → request resolved → Work resumes | Works | — |
| US-207 | As an owner, I want to sign in again to an app whose sign-in expired and get back to what I was doing while the stalled work picks up, so that an expired token costs me one click | Inbox "Sign in to X again" → Apps → app page → provider sign-in → callback lands on the Apps page (not the Inbox) → work that failed during the outage gets no wake | Partial | [UXG-323](issues/UXG-323-return-the-owner-to-where-they-started-after-an-app-sign-in.md) |
| US-208 | As an owner, I want to find everything about one client (their work, documents, messages, emails and payments) from one place, so that I can catch up before a call | ⌘K → type client name → Work titles, documents, comments, room messages → (no client record, nothing from Activity, no inbound mail) → People (internal people only) → Company → Activity (no party filter) | Partial | [UXG-321](issues/UXG-321-let-the-owner-find-everything-about-one-client-in-one-place.md), [UXG-311](issues/UXG-311-show-which-work-and-customer-an-approval-or-external-action.md), [UXG-317](issues/UXG-317-link-a-native-document-to-the-work-it-was-produced-for.md) |
| US-209 | As an owner, I want to hear when a corrected brand fact makes something already shipped wrong, and decide what happens to it, so that a fix reaches customers | Company → Brand & voice → correct a fact → drift finding appears only on that page → (no Inbox item, no Company badge) | Partial | [UXG-319](issues/UXG-319-bring-brand-drift-decisions-to-the-inbox.md) |
| US-210 | As an owner opening an old notification link, I want to see who decided the item and what happened, so that I don't act twice or wonder | Telegram or desktop notification → link ?item= → toast "That item has been resolved or is no longer in the Inbox." → next item opens → (no decision, decider or Activity link) | Partial | [UXG-316](issues/UXG-316-say-who-decided-an-item-when-an-old-link-opens-it.md) |

## Issues

Ticket status lives only in this checklist. Tick an item when its fix lands on `main`.


### Round 1


#### Brand & voice

- [ ] [UXG-001](issues/UXG-001-let-the-owner-edit-or-remove-any-released-brand-statement-no.md) Let the owner edit or remove any released brand statement, not only their own pillar text — **high** · missing-function
- [ ] [UXG-002](issues/UXG-002-present-brand-voice-as-one-readable-brand-document-instead-o.md) Present Brand & voice as one readable brand document instead of fragmented statement rows — **high** · ux
- [ ] [UXG-003](issues/UXG-003-let-the-owner-amend-an-exec-identity-proposal-before-promoti.md) Let the owner amend an Exec identity proposal before promoting, and make the reason optional — **medium** · missing-function
- [ ] [UXG-004](issues/UXG-004-allow-restoring-an-earlier-brand-version.md) Allow restoring an earlier brand version — **medium** · missing-function
- [ ] [UXG-005](issues/UXG-005-stop-hiding-exec-statements-that-still-ship-when-the-owner-w.md) Stop hiding Exec statements that still ship when the owner writes a pillar — **high** · ux
- [ ] [UXG-006](issues/UXG-006-give-visual-identity-structured-fields-logo-colours-type-in.md) Give visual identity structured fields (logo, colours, type) in one place — **medium** · missing-function
- [ ] [UXG-007](issues/UXG-007-let-the-owner-export-or-share-the-brand-guide.md) Let the owner export or share the brand guide — **low** · missing-function

#### Inbox

- [ ] [UXG-008](issues/UXG-008-add-undo-to-dismiss-in-the-inbox-item-detail.md) Add Undo to "Dismiss" in the Inbox item detail — **high** · consistency
- [ ] [UXG-009](issues/UXG-009-use-one-name-for-dismiss-not-needed-set-aside.md) Use one name for "Dismiss / Not needed / Set aside" — **medium** · consistency
- [ ] [UXG-010](issues/UXG-010-make-always-allow-at-least-as-deliberate-as-a-one-off-approv.md) Make "Always allow" at least as deliberate as a one-off approval — **high** · ux
- [ ] [UXG-011](issues/UXG-011-let-the-owner-snooze-until-a-chosen-time-or-date.md) Let the owner snooze until a chosen time or date — **medium** · missing-function
- [ ] [UXG-012](issues/UXG-012-add-snooze-and-copy-link-to-the-inbox-item-detail.md) Add Snooze and Copy link to the Inbox item detail — **medium** · mobile
- [ ] [UXG-013](issues/UXG-013-make-the-today-line-link-to-its-lists.md) Make the "Today" line link to its lists — **medium** · spec-drift
- [ ] [UXG-014](issues/UXG-014-let-an-applied-change-row-open-the-change-it-made.md) Let an applied-change row open the change it made — **medium** · spec-drift
- [ ] [UXG-015](issues/UXG-015-remove-eyebrow-kickers-in-direction-requests-and-the-compute.md) Remove eyebrow kickers in direction requests and the computer view — **low** · consistency
- [ ] [UXG-016](issues/UXG-016-show-whether-the-company-can-keep-going-while-an-item-waits.md) Show whether the company can keep going while an item waits — **medium** · spec-drift
- [ ] [UXG-017](issues/UXG-017-order-the-inbox-by-consequence-and-deadline-not-age-alone.md) Order the Inbox by consequence and deadline, not age alone — **medium** · spec-drift
- [ ] [UXG-018](issues/UXG-018-stop-single-keystroke-approvals-of-direction-requests.md) Stop single-keystroke approvals of direction requests — **high** · ux
- [ ] [UXG-019](issues/UXG-019-link-the-inbox-to-what-i-decided-and-allow-amending-a-recent.md) Link the Inbox to "what I decided" and allow amending a recent answer — **medium** · missing-function
- [ ] [UXG-020](issues/UXG-020-let-the-owner-delegate-an-inbox-item-to-a-member-or-admin.md) Let the owner delegate an Inbox item to a member or admin — **medium** · missing-function
- [ ] [UXG-021](issues/UXG-021-let-a-requester-see-all-their-requests-and-withdraw-a-pendin.md) Let a requester see all their requests and withdraw a pending one — **medium** · missing-function
- [ ] [UXG-022](issues/UXG-022-give-set-aside-dates-the-asker-and-a-visible-failure-state.md) Give "Set aside" dates, the asker, and a visible failure state — **low** · ux
- [ ] [UXG-023](issues/UXG-023-wire-the-existing-pass-to-owner-endpoint-for-members.md) Wire the existing "pass to owner" endpoint for members — **medium** · missing-function
- [ ] [UXG-024](issues/UXG-024-gate-hold-to-accept-and-request-changes-with-the-same-real-r.md) Gate "Hold to accept" and "Request changes" with the same real roles — **low** · consistency
- [ ] [UXG-025](issues/UXG-025-let-the-owner-finish-a-human-step-from-the-computer-view.md) Let the owner finish a human step from the computer view — **low** · ux
- [ ] [UXG-026](issues/UXG-026-show-what-was-asked-and-by-whom-on-document-items.md) Show what was asked, and by whom, on document items — **medium** · ux
- [ ] [UXG-027](issues/UXG-027-give-a-failed-direction-request-a-way-forward.md) Give a failed direction request a way forward — **medium** · spec-drift
- [ ] [UXG-028](issues/UXG-028-notify-members-not-just-owners-of-new-inbox-items.md) Notify members, not just owners, of new Inbox items — **medium** · missing-function
- [ ] [UXG-029](issues/UXG-029-reconsider-the-pixel-office-as-the-clear-inbox-home.md) Reconsider the pixel office as the clear-Inbox home — **medium** · consistency
- [ ] [UXG-030](issues/UXG-030-add-notifications-that-work-with-the-tab-closed-and-an-email.md) Add notifications that work with the tab closed, and an email preference — **medium** · missing-function
- [ ] [UXG-031](issues/UXG-031-make-the-shortcuts-sheet-list-every-key-that-works.md) Make the shortcuts sheet list every key that works — **low** · a11y
- [ ] [UXG-032](issues/UXG-032-stop-e-meaning-mark-done-in-the-inbox-but-mark-read-in-for-y.md) Stop E meaning "mark done" in the Inbox but "mark read" in For you — **medium** · ux
- [ ] [UXG-033](issues/UXG-033-make-the-inbox-badge-match-the-list.md) Make the Inbox badge match the list — **low** · ux
- [ ] [UXG-034](issues/UXG-034-let-a-sent-rail-message-be-corrected-after-the-agent-has-rea.md) Let a sent rail message be corrected after the agent has read it — **low** · missing-function
- [ ] [UXG-035](issues/UXG-035-expose-unread-snoozed-and-kind-to-screen-readers-in-the-inbo.md) Expose unread, snoozed and kind to screen readers in the Inbox — **medium** · a11y
- [ ] [UXG-036](issues/UXG-036-stop-offering-mark-done-on-app-requests-and-surface-fetch-fa.md) Stop offering "Mark done" on app requests and surface fetch failures — **low** · consistency
- [ ] [UXG-037](issues/UXG-037-let-an-owner-give-a-reason-when-declining-an-email-mandate.md) Let an owner give a reason when declining an email mandate — **low** · consistency
- [ ] [UXG-038](issues/UXG-038-show-non-owners-only-the-inbox-actions-they-may-take.md) Show non-owners only the Inbox actions they may take — **low** · consistency

#### Work

- [ ] [UXG-039](issues/UXG-039-surface-the-pending-decision-on-the-work-detail-page.md) Surface the pending decision on the Work detail page — **high** · ux
- [ ] [UXG-040](issues/UXG-040-bring-the-prepared-reviewtarget-to-the-work-detail-page.md) Bring the prepared ReviewTarget to the Work detail page — **high** · spec-drift
- [ ] [UXG-041](issues/UXG-041-let-the-owner-accept-reject-or-request-changes-from-work.md) Let the owner accept, reject or request changes from Work — **high** · missing-function
- [ ] [UXG-042](issues/UXG-042-separate-pause-from-stop-for-good-and-warn-before-abandoning.md) Separate "pause" from "stop for good" and warn before abandoning — **high** · ux
- [ ] [UXG-043](issues/UXG-043-give-stopped-work-a-place-to-be-found-and-restored.md) Give stopped work a place to be found and restored — **high** · missing-function
- [ ] [UXG-044](issues/UXG-044-show-why-blocked-work-is-blocked-in-the-list-and-detail-head.md) Show why blocked work is blocked in the list and detail header — **medium** · ux
- [ ] [UXG-045](issues/UXG-045-show-and-let-the-owner-change-work-priority-add-sort.md) Show and let the owner change Work priority; add sort — **medium** · missing-function
- [ ] [UXG-046](issues/UXG-046-allow-renaming-a-work-item-and-editing-its-scope-or-goal.md) Allow renaming a Work item and editing its scope or goal — **medium** · missing-function
- [ ] [UXG-047](issues/UXG-047-show-attempt-history-feedback-and-inputs-on-work-detail.md) Show attempt history, feedback and inputs on Work detail — **medium** · spec-drift
- [ ] [UXG-048](issues/UXG-048-wire-lead-team-reassignment-for-work-and-goals.md) Wire lead/team reassignment for Work and Goals — **medium** · missing-function
- [ ] [UXG-049](issues/UXG-049-show-cost-per-work-and-per-goal.md) Show cost per Work and per Goal — **medium** · spec-drift
- [ ] [UXG-050](issues/UXG-050-add-goal-editing-close-this-goal-and-reopen-in-the-work-ui.md) Add goal editing, close-this-goal and reopen in the Work UI — **high** · missing-function
- [ ] [UXG-051](issues/UXG-051-add-owner-and-status-filters-and-decide-on-the-work-graph-vi.md) Add owner and status filters, and decide on the Work graph view — **medium** · spec-drift
- [ ] [UXG-052](issues/UXG-052-provide-a-goal-detail-page.md) Provide a goal detail page — **medium** · spec-drift
- [ ] [UXG-053](issues/UXG-053-mark-needs-you-on-board-cards.md) Mark "needs you" on board cards — **medium** · ux
- [ ] [UXG-054](issues/UXG-054-make-waiting-on-you-work-for-team-owners-members.md) Make "Waiting on you" work for team owners (members) — **medium** · consistency
- [ ] [UXG-055](issues/UXG-055-fix-the-no-work-yet-empty-state-on-a-failed-search.md) Fix the "No work yet" empty state on a failed search — **low** · ux
- [ ] [UXG-056](issues/UXG-056-widen-work-search-and-keep-it-in-the-url.md) Widen Work search and keep it in the URL — **low** · missing-function
- [ ] [UXG-057](issues/UXG-057-unify-status-words-waiting-waiting-on-you-blocked-needs-you.md) Unify status words: Waiting / Waiting on you / Blocked / Needs you — **medium** · consistency
- [ ] [UXG-058](issues/UXG-058-start-add-work-from-a-goal-with-context-prefilled-and-confir.md) Start "add work" from a goal with context prefilled, and confirm creation — **low** · ux
- [ ] [UXG-059](issues/UXG-059-reorder-or-replace-the-board-on-mobile.md) Reorder or replace the board on mobile — **low** · mobile
- [ ] [UXG-060](issues/UXG-060-cap-or-fold-the-board-s-done-column.md) Cap or fold the board's Done column — **low** · ux
- [ ] [UXG-061](issues/UXG-061-fix-work-board-semantics-for-screen-readers.md) Fix Work board semantics for screen readers — **medium** · a11y
- [ ] [UXG-062](issues/UXG-062-replace-hover-only-title-explanations-on-work-with-touch-rea.md) Replace hover-only title explanations on Work with touch-reachable tips — **low** · a11y
- [ ] [UXG-063](issues/UXG-063-hide-run-internals-and-raw-enum-words-from-the-default-work.md) Hide run internals and raw enum words from the default Work view — **low** · consistency
- [ ] [UXG-064](issues/UXG-064-use-one-name-for-goal-less-work-other-work-vs-unassigned.md) Use one name for goal-less work ("Other work" vs "Unassigned") — **low** · consistency
- [ ] [UXG-065](issues/UXG-065-make-goal-rows-real-links-in-the-work-sidebar.md) Make goal rows real links in the Work sidebar — **low** · consistency
- [ ] [UXG-066](issues/UXG-066-keep-list-context-when-opening-discuss-from-work.md) Keep list context when opening "Discuss" from Work — **low** · ux
- [ ] [UXG-067](issues/UXG-067-do-not-require-a-written-reason-to-resume-work.md) Do not require a written reason to resume work — **low** · ux
- [ ] [UXG-068](issues/UXG-068-add-row-quick-actions-and-later-bulk-actions-export-to-the-w.md) Add row quick actions (and later bulk actions/export) to the Work list — **low** · missing-function

#### Library

- [ ] [UXG-069](issues/UXG-069-add-folders-or-collections-to-the-library.md) Add folders or collections to the Library — **high** · missing-function
- [ ] [UXG-070](issues/UXG-070-stop-email-sharing-from-making-outsiders-full-company-member.md) Stop email sharing from making outsiders full company members — **high** · missing-function
- [ ] [UXG-071](issues/UXG-071-allow-renaming-work-outputs-and-uploaded-files.md) Allow renaming Work outputs and uploaded files — **medium** · missing-function
- [ ] [UXG-072](issues/UXG-072-allow-permanent-deletion-of-archived-work-outputs.md) Allow permanent deletion of archived Work outputs — **medium** · missing-function
- [ ] [UXG-073](issues/UXG-073-let-archived-files-be-opened-or-previewed.md) Let archived files be opened or previewed — **medium** · ux
- [ ] [UXG-074](issues/UXG-074-search-inside-sheets-and-comments-from-the-library-search-bo.md) Search inside sheets and comments from the Library search box — **medium** · consistency
- [ ] [UXG-075](issues/UXG-075-sort-and-count-across-all-documents-not-just-loaded-pages.md) Sort and count across all documents, not just loaded pages — **medium** · ux
- [ ] [UXG-076](issues/UXG-076-add-owner-shared-with-me-and-date-filters-plus-bulk-select.md) Add owner, "shared with me" and date filters, plus bulk select — **medium** · missing-function
- [ ] [UXG-077](issues/UXG-077-show-the-actual-old-document-when-previewing-history.md) Show the actual old document when previewing history — **medium** · ux
- [ ] [UXG-078](issues/UXG-078-restore-sheet-versions-in-place-like-documents.md) Restore sheet versions in place, like documents — **medium** · consistency
- [ ] [UXG-079](issues/UXG-079-show-who-made-each-sheet-version.md) Show who made each sheet version — **low** · ux
- [ ] [UXG-080](issues/UXG-080-let-a-docx-xlsx-upload-become-an-editable-doc-or-sheet.md) Let a .docx/.xlsx upload become an editable doc or sheet — **high** · missing-function
- [ ] [UXG-081](issues/UXG-081-export-documents-as-word-docx.md) Export documents as Word (.docx) — **medium** · missing-function
- [ ] [UXG-082](issues/UXG-082-let-an-old-document-version-be-downloaded.md) Let an old document version be downloaded — **low** · consistency
- [ ] [UXG-083](issues/UXG-083-give-owners-a-way-forward-when-email-invites-are-not-set-up.md) Give owners a way forward when email invites are not set up — **medium** · ux
- [ ] [UXG-084](issues/UXG-084-make-uploaded-files-shareable-and-visible-to-the-team.md) Make uploaded files shareable and visible to the team — **medium** · missing-function
- [ ] [UXG-085](issues/UXG-085-let-discuss-actually-allow-exec-to-make-the-requested-docume.md) Let Discuss actually allow Exec to make the requested document edit — **high** · spec-drift
- [ ] [UXG-086](issues/UXG-086-allow-deleting-a-document-from-its-own-page.md) Allow deleting a document from its own page — **low** · ux
- [ ] [UXG-087](issues/UXG-087-fix-library-table-head-and-sort-accessibility.md) Fix Library table head and sort accessibility — **low** · a11y
- [ ] [UXG-088](issues/UXG-088-refresh-the-sheet-list-and-keep-library-error-states-honest.md) Refresh the sheet list and keep Library error states honest — **low** · ux
- [ ] [UXG-089](issues/UXG-089-return-the-editor-s-back-link-to-where-the-owner-came-from.md) Return the editor's back link to where the owner came from — **low** · ux
- [ ] [UXG-090](issues/UXG-090-offer-discuss-with-exec-on-sheets-and-link-the-file-on-file.md) Offer "Discuss with Exec" on sheets and link the file on file Discuss — **medium** · consistency
- [ ] [UXG-091](issues/UXG-091-hide-or-explain-request-review-for-people-who-cannot-request.md) Hide or explain "Request review…" for people who cannot request one — **low** · ux
- [ ] [UXG-092](issues/UXG-092-let-the-requester-choose-a-reviewer-and-use-plain-language-i.md) Let the requester choose a reviewer, and use plain language in Review — **medium** · ux
- [ ] [UXG-093](issues/UXG-093-remove-the-hard-coded-aris-company-and-brand-from-library-ro.md) Remove the hard-coded "aris" company and brand from Library routes — **low** · consistency
- [ ] [UXG-094](issues/UXG-094-show-agent-document-proposals-as-a-diff-not-two-full-texts.md) Show agent document proposals as a diff, not two full texts — **medium** · ux
- [ ] [UXG-095](issues/UXG-095-decide-whether-archived-documents-stay-editable.md) Decide whether archived documents stay editable — **low** · consistency
- [ ] [UXG-096](issues/UXG-096-handle-a-failed-sheet-history-load.md) Handle a failed sheet-history load — **low** · ux
- [ ] [UXG-097](issues/UXG-097-surface-or-delete-the-document-backlinks-api.md) Surface or delete the document backlinks API — **low** · spec-drift

#### People

- [ ] [UXG-098](issues/UXG-098-add-invite-and-member-management-to-people.md) Add invite and member management to People — **medium** · consistency
- [ ] [UXG-099](issues/UXG-099-confirm-or-offer-undo-for-cancelling-invitations-and-pausing.md) Confirm or offer Undo for cancelling invitations and pausing access — **medium** · ux
- [ ] [UXG-100](issues/UXG-100-support-transferring-company-ownership.md) Support transferring company ownership — **medium** · missing-function
- [ ] [UXG-101](issues/UXG-101-give-the-owner-a-way-to-hire-retire-pause-or-re-brief-ai-sta.md) Give the owner a way to hire, retire, pause or re-brief AI staff — **high** · missing-function
- [ ] [UXG-102](issues/UXG-102-build-the-actor-profile-the-spec-requires.md) Build the actor profile the spec requires — **high** · spec-drift
- [ ] [UXG-103](issues/UXG-103-confirm-model-changes-and-say-when-they-take-effect.md) Confirm model changes and say when they take effect — **low** · ux
- [ ] [UXG-104](issues/UXG-104-make-a-person-s-current-work-clickable-from-people.md) Make a person's current Work clickable from People — **medium** · ux
- [ ] [UXG-105](issues/UXG-105-unify-what-is-this-person-doing-between-people-and-the-offic.md) Unify "what is this person doing" between People and the office floor — **low** · consistency
- [ ] [UXG-106](issues/UXG-106-let-admins-and-members-react-and-pin-as-the-backend-allows.md) Let admins and members react and pin, as the backend allows — **medium** · consistency
- [ ] [UXG-107](issues/UXG-107-add-search-and-grouping-to-the-new-conversation-people-picke.md) Add search and grouping to the new-conversation people picker — **low** · ux
- [ ] [UXG-108](issues/UXG-108-make-room-rename-and-archive-discoverable.md) Make room rename and archive discoverable — **medium** · ux
- [ ] [UXG-109](issues/UXG-109-let-a-room-owner-leave-or-hand-over-a-group.md) Let a room owner leave or hand over a group — **low** · missing-function
- [ ] [UXG-110](issues/UXG-110-let-archived-groups-be-read-without-restoring-and-allow-hidi.md) Let archived groups be read without restoring, and allow hiding DMs — **low** · ux
- [ ] [UXG-111](issues/UXG-111-add-keyboard-shortcuts-to-message-editing-and-re-parse-menti.md) Add keyboard shortcuts to message editing and re-parse mentions on edit — **medium** · a11y
- [ ] [UXG-112](issues/UXG-112-label-people-message-search-and-give-it-loading-and-empty-st.md) Label People message search and give it loading and empty states — **medium** · ux
- [ ] [UXG-113](issues/UXG-113-show-group-rooms-beyond-the-first-page-in-chats.md) Show group rooms beyond the first page in Chats — **medium** · missing-function
- [ ] [UXG-114](issues/UXG-114-give-admins-and-members-the-mention-picker.md) Give admins and members the @mention picker — **high** · missing-function
- [ ] [UXG-115](issues/UXG-115-show-readable-names-in-mentions-while-composing.md) Show readable names in mentions while composing — **low** · ux
- [ ] [UXG-116](issues/UXG-116-add-per-room-mute-and-notification-preferences.md) Add per-room mute and notification preferences — **medium** · missing-function
- [ ] [UXG-117](issues/UXG-117-let-a-non-technical-owner-connect-telegram-without-typing-a.md) Let a non-technical owner connect Telegram without typing a Vault reference — **high** · ux
- [ ] [UXG-118](issues/UXG-118-confirm-before-unpairing-telegram-and-let-the-owner-filter-w.md) Confirm before unpairing Telegram and let the owner filter what it sends — **low** · ux
- [ ] [UXG-119](issues/UXG-119-show-loading-error-states-and-scope-for-standing-permissions.md) Show loading/error states and scope for standing permissions — **medium** · ux
- [ ] [UXG-120](issues/UXG-120-allow-editing-an-email-mandate-and-explain-why-add-mandate-i.md) Allow editing an email mandate and explain why Add mandate is disabled — **medium** · missing-function
- [ ] [UXG-121](issues/UXG-121-confirm-team-owner-changes-and-keep-the-select-truthful-on-f.md) Confirm team-owner changes and keep the select truthful on failure — **medium** · ux
- [ ] [UXG-122](issues/UXG-122-show-which-human-colleagues-are-online-in-people.md) Show which human colleagues are online in People — **medium** · missing-function
- [ ] [UXG-123](issues/UXG-123-make-the-people-chats-switch-an-accessible-tablist.md) Make the People/Chats switch an accessible tablist — **low** · a11y
- [ ] [UXG-124](issues/UXG-124-distinguish-a-message-from-a-directive-in-chat.md) Distinguish a message from a directive in chat — **medium** · spec-drift
- [ ] [UXG-125](issues/UXG-125-expose-or-delete-unused-people-room-endpoints.md) Expose or delete unused people/room endpoints — **low** · consistency
- [ ] [UXG-126](issues/UXG-126-show-what-an-agent-dm-is-about-before-the-first-message.md) Show what an agent DM is about before the first message — **low** · ux

#### Apps

- [ ] [UXG-127](issues/UXG-127-add-google-gmail-calendar-drive-microsoft-and-accounting-ser.md) Add Google (Gmail/Calendar/Drive), Microsoft and accounting services to the Apps catalogue — **high** · missing-function
- [ ] [UXG-128](issues/UXG-128-collect-the-token-when-a-token-auth-app-is-added-from-a-tile.md) Collect the token when a token-auth app is added from a tile or the ask box — **high** · missing-function
- [ ] [UXG-129](issues/UXG-129-confirm-before-removing-an-app-and-say-what-loses-access.md) Confirm before removing an app and say what loses access — **high** · ux
- [ ] [UXG-130](issues/UXG-130-remove-or-ask-about-a-plugin-bundle-s-know-how-when-the-bund.md) Remove (or ask about) a plugin bundle's know-how when the bundle is removed — **medium** · missing-function
- [ ] [UXG-131](issues/UXG-131-let-the-owner-read-a-skill-s-skill-md-and-scripts-before-acc.md) Let the owner read a skill's SKILL.md and scripts before accepting it — **high** · missing-function
- [ ] [UXG-132](issues/UXG-132-allow-editing-a-connection-s-address-token-or-sign-in-withou.md) Allow editing a connection's address, token or sign-in without removing it — **medium** · missing-function
- [ ] [UXG-133](issues/UXG-133-provide-a-way-to-create-and-edit-company-know-how.md) Provide a way to create and edit company know-how — **high** · missing-function
- [ ] [UXG-134](issues/UXG-134-show-removed-skills-with-restore-and-show-the-skill-s-versio.md) Show removed skills with Restore, and show the skill's version — **medium** · missing-function
- [ ] [UXG-135](issues/UXG-135-make-app-activity-answer-what-was-it-used-for.md) Make app activity answer "what was it used for" — **medium** · ux
- [ ] [UXG-136](issues/UXG-136-offer-a-real-off-state-that-keeps-the-sign-in.md) Offer a real "off" state that keeps the sign-in — **medium** · missing-function
- [ ] [UXG-137](issues/UXG-137-show-permissions-before-auto-allowing-an-added-app.md) Show permissions before auto-allowing an added app — **medium** · ux
- [ ] [UXG-138](issues/UXG-138-replace-the-free-text-vault-secret-name-with-a-picker-or-inl.md) Replace the free-text "Vault secret name" with a picker or inline entry — **medium** · ux
- [ ] [UXG-139](issues/UXG-139-show-why-exec-wants-an-app-in-the-chat-card-without-hovering.md) Show why Exec wants an app in the chat card without hovering — **medium** · a11y
- [ ] [UXG-140](issues/UXG-140-let-declining-an-app-request-carry-a-reason-or-alternative.md) Let declining an app request carry a reason or alternative — **low** · missing-function
- [ ] [UXG-141](issues/UXG-141-report-skipped-items-and-link-to-review-after-a-plugin-impor.md) Report skipped items and link to review after a plugin import — **low** · ux
- [ ] [UXG-142](issues/UXG-142-make-a-disabled-skill-a-visible-state-and-expose-per-agent-t.md) Make a disabled skill a visible state and expose per-agent/team assignment — **medium** · missing-function
- [ ] [UXG-143](issues/UXG-143-remove-technical-terms-from-the-model-choice.md) Remove technical terms from the model choice — **medium** · ux
- [ ] [UXG-144](issues/UXG-144-pick-one-place-to-add-model-keys-and-sign-ins.md) Pick one place to add model keys and sign-ins — **medium** · consistency
- [ ] [UXG-145](issues/UXG-145-use-one-verb-for-adding-an-app.md) Use one verb for adding an app — **low** · consistency
- [ ] [UXG-146](issues/UXG-146-show-couldn-t-load-instead-of-hiding-skills-or-saying-not-fo.md) Show "couldn't load" instead of hiding skills or saying "not found" — **medium** · ux
- [ ] [UXG-147](issues/UXG-147-stop-tab-from-sending-a-message-to-exec-in-the-apps-ask-box.md) Stop Tab from sending a message to Exec in the Apps ask box — **medium** · a11y
- [ ] [UXG-148](issues/UXG-148-name-model-connections-differently-from-app-connections.md) Name model "Connections" differently from app connections — **medium** · consistency
- [ ] [UXG-149](issues/UXG-149-confirm-token-revoke-and-handle-copy-failures-in-ai-apps.md) Confirm token revoke and handle copy failures in AI apps — **low** · ux
- [ ] [UXG-150](issues/UXG-150-make-appearance-follow-the-tooltip-rule-and-radio-keyboard-b.md) Make Appearance follow the tooltip rule and radio keyboard behaviour — **low** · a11y
- [ ] [UXG-151](issues/UXG-151-make-all-apps-views-and-add-from-link-reachable-on-mobile.md) Make all Apps views and "add from link" reachable on mobile — **medium** · mobile
- [ ] [UXG-152](issues/UXG-152-use-one-list-form-for-apps-instead-of-tiles-cards-in-browse.md) Use one list form for Apps instead of tiles/cards in Browse — **medium** · ux
- [ ] [UXG-153](issues/UXG-153-let-custom-harnesses-be-edited-and-removed-confirm-harness-d.md) Let custom harnesses be edited and removed; confirm harness disconnect — **low** · missing-function
- [ ] [UXG-154](issues/UXG-154-settle-naming-of-know-how-vs-skills-and-describe-local-tools.md) Settle naming of know-how vs skills and describe local tools accurately — **low** · consistency
- [ ] [UXG-155](issues/UXG-155-let-the-owner-change-a-company-s-model-from-account-access.md) Let the owner change a company's model from Account access — **low** · ux

#### Company

- [ ] [UXG-156](issues/UXG-156-let-the-owner-name-the-company-when-creating-it.md) Let the owner name the company when creating it — **medium** · ux
- [ ] [UXG-157](issues/UXG-157-report-a-failed-rename-during-onboarding-instead-of-dropping.md) Report a failed rename during onboarding instead of dropping it — **high** · ux
- [ ] [UXG-158](issues/UXG-158-use-a-multi-line-answer-box-on-the-welcome-page.md) Use a multi-line answer box on the welcome page — **low** · ux
- [ ] [UXG-159](issues/UXG-159-make-the-empty-portfolio-text-match-the-real-button.md) Make the empty-portfolio text match the real button — **low** · consistency
- [ ] [UXG-160](issues/UXG-160-provide-a-way-back-into-an-unfinished-welcome-conversation.md) Provide a way back into an unfinished welcome conversation — **medium** · missing-function
- [ ] [UXG-161](issues/UXG-161-cover-all-six-surfaces-in-the-tour-and-make-it-keyboard-frie.md) Cover all six surfaces in the tour and make it keyboard-friendly and replayable — **low** · a11y
- [ ] [UXG-162](issues/UXG-162-offer-a-non-technical-charter-editor-with-a-preview.md) Offer a non-technical charter editor with a preview — **medium** · ux
- [ ] [UXG-163](issues/UXG-163-make-charter-history-findable-comparable-and-restorable.md) Make charter history findable, comparable and restorable — **medium** · missing-function
- [ ] [UXG-164](issues/UXG-164-let-the-owner-add-or-edit-the-legal-profile.md) Let the owner add or edit the legal profile — **medium** · missing-function
- [ ] [UXG-165](issues/UXG-165-support-specific-weekdays-monthly-schedules-and-a-company-ti.md) Support specific weekdays, monthly schedules and a company time zone — **medium** · missing-function
- [ ] [UXG-166](issues/UXG-166-allow-editing-what-a-schedule-asks-exec-to-do.md) Allow editing what a schedule asks Exec to do — **high** · missing-function
- [ ] [UXG-167](issues/UXG-167-use-the-same-friendly-cadence-control-when-editing-a-schedul.md) Use the same friendly cadence control when editing a schedule — **medium** · ux
- [ ] [UXG-168](issues/UXG-168-add-run-now-and-rename-test-trigger.md) Add "Run now" and rename "Test trigger" — **low** · ux
- [ ] [UXG-169](issues/UXG-169-add-budget-periods-categories-and-early-warnings.md) Add budget periods, categories and early warnings — **high** · spec-drift
- [ ] [UXG-170](issues/UXG-170-warn-before-lowering-the-spend-limit-below-current-spend.md) Warn before lowering the spend limit below current spend — **medium** · ux
- [ ] [UXG-171](issues/UXG-171-let-the-owner-delete-a-vault-secret-and-replace-any-secret-f.md) Let the owner delete a Vault secret and replace any secret from its row — **high** · missing-function
- [ ] [UXG-172](issues/UXG-172-warn-when-a-new-secret-name-overwrites-an-existing-one-and-h.md) Warn when a new secret name overwrites an existing one, and hide raw references — **medium** · ux
- [ ] [UXG-173](issues/UXG-173-give-every-failing-health-check-a-next-step.md) Give every failing health check a next step — **medium** · ux
- [ ] [UXG-174](issues/UXG-174-use-consistent-tones-and-plain-language-on-health.md) Use consistent tones and plain language on Health — **low** · ux
- [ ] [UXG-175](issues/UXG-175-let-activity-filter-by-date-person-and-text-and-group-by-day.md) Let Activity filter by date, person and text, and group by day — **medium** · missing-function
- [ ] [UXG-176](issues/UXG-176-page-external-actions-and-include-owner-setting-changes-in-a.md) Page external actions and include owner setting changes in Activity — **medium** · spec-drift
- [ ] [UXG-177](issues/UXG-177-put-company-name-and-logo-where-owners-look-for-them.md) Put company name and logo where owners look for them — **medium** · ux
- [ ] [UXG-178](issues/UXG-178-show-why-the-name-or-logo-didn-t-save-not-just-not-saved.md) Show why the name or logo didn't save, not just "Not saved" — **medium** · a11y
- [ ] [UXG-179](issues/UXG-179-explain-what-archiving-a-company-does-before-doing-it.md) Explain what archiving a company does before doing it — **medium** · ux
- [ ] [UXG-180](issues/UXG-180-add-permanent-deletion-of-a-company.md) Add permanent deletion of a company — **medium** · missing-function
- [ ] [UXG-181](issues/UXG-181-let-the-owner-open-an-archived-company-read-only-and-archive.md) Let the owner open an archived company read-only and archive from inside Company — **low** · ux
- [ ] [UXG-182](issues/UXG-182-add-company-data-export.md) Add company data export — **medium** · missing-function
- [ ] [UXG-183](issues/UXG-183-keep-the-current-area-when-switching-companies-and-add-new-c.md) Keep the current area when switching companies, and add New company to the switcher — **low** · ux
- [ ] [UXG-184](issues/UXG-184-copy-a-full-setup-to-another-company-not-one-setting-at-a-ti.md) Copy a full setup to another company, not one setting at a time — **low** · missing-function
- [ ] [UXG-185](issues/UXG-185-add-a-company-wide-freeze-of-outside-actions-and-payment-all.md) Add a company-wide freeze of outside actions and payment-allowance controls — **high** · spec-drift
- [ ] [UXG-186](issues/UXG-186-make-the-company-sidebar-s-computer-and-limits-values-truthf.md) Make the Company sidebar's Computer and Limits values truthful — **medium** · consistency
- [ ] [UXG-187](issues/UXG-187-keep-settings-search-on-phones-and-narrow-panes.md) Keep settings search on phones and narrow panes — **low** · mobile
- [ ] [UXG-188](issues/UXG-188-read-the-brand-name-from-config-in-the-model-catalog-tooltip.md) Read the brand name from config in the model-catalog tooltip — **low** · consistency
- [ ] [UXG-189](issues/UXG-189-wire-up-or-delete-the-unused-accounthome-view.md) Wire up or delete the unused AccountHome view — **low** · consistency
- [ ] [UXG-190](issues/UXG-190-stop-hiding-needed-company-explanations-in-hover-only-titles.md) Stop hiding needed Company explanations in hover-only titles — **low** · a11y

### Round 2


#### Exec chat

- [ ] [UXG-191](issues/UXG-191-add-retry-to-a-failed-exec-reply.md) Add Retry to a failed Exec reply — **high** · missing-function
- [ ] [UXG-192](issues/UXG-192-add-a-stop-button-that-halts-exec-s-reply-without-sending-a.md) Add a Stop button that halts Exec's reply without sending a new message — **high** · missing-function
- [ ] [UXG-193](issues/UXG-193-show-exec-s-live-work-even-when-the-owner-did-not-send-the-l.md) Show Exec's live work even when the owner did not send the last message — **high** · missing-function
- [ ] [UXG-194](issues/UXG-194-put-exec-s-unread-reply-and-working-state-on-the-exec-button.md) Put Exec's unread reply and working state on the Exec button, not the People badge — **medium** · ux
- [ ] [UXG-195](issues/UXG-195-let-the-rail-reach-and-search-exec-history-older-than-the-la.md) Let the rail reach and search Exec history older than the last 100 messages — **high** · missing-function
- [ ] [UXG-196](issues/UXG-196-keep-pinned-exec-messages-visible-regardless-of-age.md) Keep pinned Exec messages visible regardless of age — **medium** · ux
- [ ] [UXG-197](issues/UXG-197-let-the-owner-write-and-leave-a-message-while-exec-is-unreac.md) Let the owner write and leave a message while Exec is unreachable — **medium** · spec-drift
- [ ] [UXG-198](issues/UXG-198-check-attachment-size-count-and-type-before-sending-from-the.md) Check attachment size, count and type before sending from the rail, and show upload progress — **medium** · consistency
- [ ] [UXG-199](issues/UXG-199-keep-attached-files-quote-and-chosen-skills-when-switching-r.md) Keep attached files, quote and chosen skills when switching rail conversations — **medium** · ux
- [ ] [UXG-200](issues/UXG-200-persist-the-rail-s-retry-key-for-an-uncertain-send-across-re.md) Persist the rail's retry key for an uncertain send across reloads and topic switches — **medium** · security
- [ ] [UXG-201](issues/UXG-201-stop-handed-in-drafts-from-silently-replacing-the-owner-s-un.md) Stop handed-in drafts from silently replacing the owner's unsent message — **medium** · ux
- [ ] [UXG-202](issues/UXG-202-make-slash-commands-skills-references-and-people-discoverabl.md) Make slash commands, skills, # references and @ people discoverable in the composer — **medium** · ux
- [ ] [UXG-203](issues/UXG-203-confirm-before-loop-clear-stops-every-loop-and-let-goal-clea.md) Confirm before /loop clear stops every loop, and let /goal clear choose the goal — **high** · ux
- [ ] [UXG-204](issues/UXG-204-stop-a-failed-goal-send-from-creating-a-duplicate-goal-on-re.md) Stop a failed /goal send from creating a duplicate goal on retry — **medium** · security
- [ ] [UXG-205](issues/UXG-205-record-command-results-in-the-conversation-instead-of-a-ling.md) Record command results in the conversation instead of a lingering status line — **low** · ux
- [ ] [UXG-206](issues/UXG-206-give-admins-operators-and-members-the-rail-context-the-owner.md) Give admins, operators and members the rail context the owner gets — **medium** · consistency
- [ ] [UXG-207](issues/UXG-207-show-live-progress-when-talking-to-a-staff-member-or-an-agen.md) Show live progress when talking to a Staff member or an agent in a group room — **medium** · consistency
- [ ] [UXG-208](issues/UXG-208-say-seen-no-reply-instead-of-replied-when-exec-s-turn-ends-w.md) Say "Seen, no reply" instead of "Replied" when Exec's turn ends without a message — **medium** · ux
- [ ] [UXG-209](issues/UXG-209-announce-new-exec-replies-to-screen-readers.md) Announce new Exec replies to screen readers — **medium** · a11y
- [ ] [UXG-210](issues/UXG-210-let-a-selection-on-the-main-surface-travel-with-the-message.md) Let a selection on the main surface travel with the message to Exec — **medium** · missing-function
- [ ] [UXG-211](issues/UXG-211-show-on-a-sent-message-which-screen-it-was-linked-to.md) Show on a sent message which screen it was linked to — **low** · ux
- [ ] [UXG-212](issues/UXG-212-let-the-owner-cancel-a-pending-new-conversation-and-see-wher.md) Let the owner cancel a pending new conversation and see where earlier ones began — **low** · ux
- [ ] [UXG-213](issues/UXG-213-let-the-owner-copy-or-export-a-whole-conversation.md) Let the owner copy or export a whole conversation — **low** · missing-function
- [ ] [UXG-214](issues/UXG-214-show-the-current-model-on-the-composer-chip-and-allow-effort.md) Show the current model on the composer chip and allow effort to be chosen there — **low** · ux
- [ ] [UXG-215](issues/UXG-215-let-the-expanded-turn-dock-show-exec-s-thinking-collapsed-by.md) Let the expanded turn dock show Exec's thinking, collapsed by default — **low** · spec-drift

#### Doc & sheet editors

- [ ] [UXG-216](issues/UXG-216-add-underline-highlight-text-colour-and-alignment-to-documen.md) Add underline, highlight, text colour and alignment to document formatting — **high** · missing-function
- [ ] [UXG-217](issues/UXG-217-keep-images-formatting-and-merged-cells-when-pasting-from-wo.md) Keep images, formatting and merged cells when pasting from Word or Google Docs — **high** · missing-function
- [ ] [UXG-218](issues/UXG-218-let-writers-mention-people-in-the-document-body.md) Let writers @mention people in the document body — **high** · missing-function
- [ ] [UXG-219](issues/UXG-219-insert-links-to-documents-work-and-files-as-titled-chips-and.md) Insert links to documents, Work and files as titled chips, and show where links go — **medium** · missing-function
- [ ] [UXG-220](issues/UXG-220-add-find-and-replace-in-documents.md) Add find and replace in documents — **medium** · missing-function
- [ ] [UXG-221](issues/UXG-221-show-an-outline-of-headings-and-make-in-page-links-work.md) Show an outline of headings, and make in-page "#" links work — **medium** · missing-function
- [ ] [UXG-222](issues/UXG-222-show-word-and-character-count.md) Show word and character count — **low** · missing-function
- [ ] [UXG-223](issues/UXG-223-let-documents-carry-files-other-than-images.md) Let documents carry files other than images — **medium** · missing-function
- [ ] [UXG-224](issues/UXG-224-allow-resizing-describing-and-captioning-images.md) Allow resizing, describing and captioning images — **medium** · missing-function
- [ ] [UXG-225](issues/UXG-225-extend-document-tables-rows-above-columns-left-header-row-wi.md) Extend document tables: rows above, columns left, header row, widths — **medium** · missing-function
- [ ] [UXG-226](issues/UXG-226-let-blocks-be-moved-duplicated-and-deleted-without-the-hover.md) Let blocks be moved, duplicated and deleted without the hover grip — **medium** · a11y
- [ ] [UXG-227](issues/UXG-227-anchor-document-comments-to-the-selected-words-not-the-whole.md) Anchor document comments to the selected words, not the whole block — **medium** · ux
- [ ] [UXG-228](issues/UXG-228-keep-editors-editable-through-brief-connection-drops.md) Keep editors editable through brief connection drops — **high** · ux
- [ ] [UXG-229](issues/UXG-229-don-t-strand-a-co-editor-s-unsaved-edits-when-someone-restor.md) Don't strand a co-editor's unsaved edits when someone restores a version — **medium** · ux
- [ ] [UXG-230](issues/UXG-230-say-plainly-when-a-document-is-too-large-to-save-and-warn-be.md) Say plainly when a document is too large to save, and warn before it gets there — **medium** · ux
- [ ] [UXG-231](issues/UXG-231-stop-a-refused-sheet-change-from-discarding-every-edit-queue.md) Stop a refused sheet change from discarding every edit queued behind it — **high** · ux
- [ ] [UXG-232](issues/UXG-232-let-writers-save-and-name-a-version-from-the-editor.md) Let writers save and name a version from the editor — **medium** · missing-function
- [ ] [UXG-233](issues/UXG-233-offer-templates-and-use-the-document-kinds-when-starting-a-d.md) Offer templates and use the document kinds when starting a document — **medium** · missing-function
- [ ] [UXG-234](issues/UXG-234-show-undo-and-redo-on-desktop-too.md) Show Undo and Redo on desktop too — **low** · ux
- [ ] [UXG-235](issues/UXG-235-let-sheet-comments-cover-a-range-and-follow-their-cell-when.md) Let sheet comments cover a range and follow their cell when rows move — **medium** · missing-function
- [ ] [UXG-236](issues/UXG-236-allow-pictures-in-sheets.md) Allow pictures in sheets — **low** · missing-function
- [ ] [UXG-237](issues/UXG-237-add-print-save-as-pdf-for-sheets.md) Add Print / save as PDF for sheets — **low** · missing-function
- [ ] [UXG-238](issues/UXG-238-follow-the-dark-theme-in-the-sheet-grid.md) Follow the dark theme in the sheet grid — **low** · consistency
- [ ] [UXG-239](issues/UXG-239-use-the-company-s-locale-and-currency-for-sheet-formats.md) Use the company's locale and currency for sheet formats — **low** · ux
- [ ] [UXG-240](issues/UXG-240-make-sheet-editing-on-a-phone-a-deliberate-tested-path.md) Make sheet editing on a phone a deliberate, tested path — **low** · mobile
- [ ] [UXG-241](issues/UXG-241-add-page-breaks-and-page-numbers-to-printed-documents.md) Add page breaks and page numbers to printed documents — **low** · missing-function

#### Company computer & intelligence

- [ ] [UXG-242](issues/UXG-242-let-the-owner-restart-a-running-computer-that-looks-healthy.md) Let the owner restart a running computer that looks healthy but is stuck — **high** · missing-function
- [ ] [UXG-243](issues/UXG-243-make-the-inbox-handover-desktop-accept-the-owner-s-keyboard.md) Make the Inbox handover desktop accept the owner's keyboard and pointer — **high** · missing-function
- [ ] [UXG-244](issues/UXG-244-keep-the-handover-desktop-connected-after-returning-control.md) Keep the handover desktop connected after returning control — **high** · ux
- [ ] [UXG-245](issues/UXG-245-stop-a-60-second-pause-from-telling-the-agent-the-owner-has.md) Stop a 60-second pause from telling the agent the owner has finished — **high** · ux
- [ ] [UXG-246](issues/UXG-246-show-when-an-agent-is-using-the-computer-and-who-before-the.md) Show when an agent is using the computer, and who, before the owner's click takes over — **medium** · ux
- [ ] [UXG-247](issues/UXG-247-stop-a-viewer-from-resizing-the-shared-desktop-while-an-agen.md) Stop a viewer from resizing the shared desktop while an agent works in it — **medium** · ux
- [ ] [UXG-248](issues/UXG-248-let-a-phone-user-type-into-the-live-desktop.md) Let a phone user type into the live desktop — **high** · mobile
- [ ] [UXG-249](issues/UXG-249-tell-keyboard-users-how-to-leave-the-live-desktop.md) Tell keyboard users how to leave the live desktop — **medium** · a11y
- [ ] [UXG-250](issues/UXG-250-let-the-owner-see-and-download-files-that-exist-only-on-the.md) Let the owner see and download files that exist only on the company computer — **high** · missing-function
- [ ] [UXG-251](issues/UXG-251-let-the-owner-drop-a-file-into-the-live-desktop-during-a-han.md) Let the owner drop a file into the live desktop during a handover — **medium** · missing-function
- [ ] [UXG-252](issues/UXG-252-say-when-the-monthly-hours-limit-is-why-the-computer-will-no.md) Say when the monthly hours limit is why the computer will not wake — **medium** · ux
- [ ] [UXG-253](issues/UXG-253-show-wake-progress-and-keep-trying-instead-of-failing-after.md) Show wake progress and keep trying instead of failing after 20 seconds — **medium** · ux
- [ ] [UXG-254](issues/UXG-254-surface-disk-memory-and-cpu-pressure-on-the-company-computer.md) Surface disk, memory and CPU pressure on the company computer — **medium** · missing-function
- [ ] [UXG-255](issues/UXG-255-point-every-computer-is-stopped-message-at-the-place-that-ca.md) Point every "computer is stopped" message at the place that can start it — **low** · consistency
- [ ] [UXG-256](issues/UXG-256-let-people-choose-where-an-external-link-opens-and-stop-rout.md) Let people choose where an external link opens, and stop routing members to a page they cannot open — **medium** · ux
- [ ] [UXG-257](issues/UXG-257-show-how-schedules-interact-with-computer-sleep-and-the-mont.md) Show how schedules interact with computer sleep and the monthly hours limit — **medium** · ux
- [ ] [UXG-258](issues/UXG-258-prove-a-chosen-model-answers-before-saving-it.md) Prove a chosen model answers before saving it — **medium** · spec-drift
- [ ] [UXG-259](issues/UXG-259-bring-the-claude-harness-sign-in-window-forward-with-control.md) Bring the Claude harness sign-in window forward, with control, instead of opening the bare desktop — **medium** · ux
- [ ] [UXG-260](issues/UXG-260-offer-a-fix-when-installed-agent-software-does-not-match-the.md) Offer a fix when installed agent software does not match the required build — **low** · ux
- [ ] [UXG-261](issues/UXG-261-preview-powerpoint-outcomes-in-the-cockpit-like-word-and-exc.md) Preview PowerPoint outcomes in the cockpit like Word and Excel — **medium** · ux
- [ ] [UXG-262](issues/UXG-262-read-the-brand-name-from-config-in-the-harness-billing-note.md) Read the brand name from config in the harness billing note — **low** · consistency
- [ ] [UXG-263](issues/UXG-263-move-telegram-out-of-the-limits-page.md) Move Telegram out of the Limits page — **low** · consistency
- [ ] [UXG-264](issues/UXG-264-delete-the-never-rendered-authority-page-and-keep-the-harnes.md) Delete the never-rendered authority page and keep the harness anchor when redirecting — **low** · consistency
- [ ] [UXG-265](issues/UXG-265-replace-the-raw-browser-health-word-in-the-handover-header.md) Replace the raw browser health word in the handover header — **low** · ux

#### Shell & cross-cutting

- [ ] [UXG-266](issues/UXG-266-keep-failure-toasts-until-dismissed-and-never-evict-a-live-u.md) Keep failure toasts until dismissed and never evict a live Undo — **medium** · ux
- [ ] [UXG-267](issues/UXG-267-make-undo-reachable-from-the-keyboard-within-its-window.md) Make Undo reachable from the keyboard within its window — **medium** · a11y
- [ ] [UXG-268](issues/UXG-268-notify-for-every-company-and-name-the-company-in-each-deskto.md) Notify for every company and name the company in each desktop notification — **medium** · missing-function
- [ ] [UXG-269](issues/UXG-269-show-what-needs-the-owner-in-the-tab-title-or-favicon.md) Show what needs the owner in the tab title or favicon — **low** · ux
- [ ] [UXG-270](issues/UXG-270-give-the-couldn-t-open-this-company-dialog-a-way-out-when-re.md) Give the "couldn’t open this company" dialog a way out when retrying cannot help — **high** · ux
- [ ] [UXG-271](issues/UXG-271-keep-unsent-text-across-session-re-entry-and-update-reloads.md) Keep unsent text across session re-entry and update reloads, and say when re-entry gave up — **medium** · ux
- [ ] [UXG-272](issues/UXG-272-add-sign-out-to-the-in-company-profile-menu.md) Add Sign out to the in-company profile menu — **medium** · missing-function
- [ ] [UXG-273](issues/UXG-273-tell-an-open-tab-when-a-new-version-has-been-deployed.md) Tell an open tab when a new version has been deployed — **medium** · ux
- [ ] [UXG-274](issues/UXG-274-stop-sending-members-to-the-owner-s-account-pages.md) Stop sending members to the owner’s Account pages — **medium** · consistency
- [ ] [UXG-275](issues/UXG-275-say-when-k-search-could-not-reach-a-source-instead-of-no-mat.md) Say when ⌘K search could not reach a source instead of "No match" — **medium** · ux
- [ ] [UXG-276](issues/UXG-276-stop-offering-add-app-for-apps-already-connected-and-let-k-f.md) Stop offering "Add <app>" for apps already connected and let ⌘K find what the company has — **low** · ux
- [ ] [UXG-277](issues/UXG-277-offer-ask-exec-about-with-the-typed-text-when-k-has-no-answe.md) Offer "Ask Exec about …" with the typed text when ⌘K has no answer — **medium** · missing-function
- [ ] [UXG-278](issues/UXG-278-make-stay-on-this-page-actually-cancel-the-stalled-navigatio.md) Make "Stay on this page" actually cancel the stalled navigation — **medium** · ux
- [ ] [UXG-279](issues/UXG-279-ignore-enter-while-an-ime-is-composing-in-k-and-other-picker.md) Ignore Enter while an IME is composing in ⌘K and other pickers — **medium** · a11y
- [ ] [UXG-280](issues/UXG-280-use-the-shared-date-time-and-money-formatters-everywhere.md) Use the shared date, time and money formatters everywhere — **low** · consistency
- [ ] [UXG-281](issues/UXG-281-do-not-add-a-browser-history-entry-for-every-j-k-step-in-the.md) Do not add a browser history entry for every J/K step in the Inbox — **low** · ux
- [ ] [UXG-282](issues/UXG-282-move-the-connection-banner-above-the-phone-dock-and-out-of-t.md) Move the connection banner above the phone dock and out of the toasts’ way — **medium** · mobile
- [ ] [UXG-283](issues/UXG-283-take-the-product-name-from-brand-config-in-the-shell-offline.md) Take the product name from brand config in the shell, offline page and manifest — **low** · consistency
- [ ] [UXG-284](issues/UXG-284-stop-shipping-the-gallery-and-office-demo-routes-with-a-foun.md) Stop shipping the gallery and office demo routes, with a founder’s email, to every install — **medium** · security
- [ ] [UXG-285](issues/UXG-285-remove-the-aris-fallback-from-every-company-route-and-retire.md) Remove the 'aris' fallback from every company route and retire the Aris knowledge route — **low** · consistency
- [ ] [UXG-286](issues/UXG-286-give-the-home-and-account-frame-the-skip-link-k-and-correct.md) Give the Home and Account frame the skip link, ⌘K and correct menu semantics the company shell has — **low** · a11y
- [ ] [UXG-287](issues/UXG-287-add-print-styles-beyond-documents.md) Add print styles beyond documents — **low** · ux
- [ ] [UXG-288](issues/UXG-288-reserve-app-route-names-so-a-company-id-cannot-be-shadowed.md) Reserve app route names so a company id cannot be shadowed — **low** · consistency
- [ ] [UXG-289](issues/UXG-289-clear-per-company-browser-memory-when-a-person-signs-out.md) Clear per-company browser memory when a person signs out — **low** · security
- [ ] [UXG-290](issues/UXG-290-write-the-error-page-in-owner-language-and-name-the-product.md) Write the error page in owner language and name the product in its title — **low** · ux

#### Endpoint & CLI parity

- [ ] [UXG-291](issues/UXG-291-let-the-owner-authorise-a-prepared-publication-from-the-cock.md) Let the owner authorise a prepared publication from the cockpit — **high** · missing-function
- [ ] [UXG-292](issues/UXG-292-let-the-owner-invite-revoke-and-stop-a-live-publication-with.md) Let the owner invite, revoke and stop a live publication without the terminal — **medium** · missing-function
- [ ] [UXG-293](issues/UXG-293-offer-appliance-backup-and-restore-in-the-product-with-a-las.md) Offer appliance backup and restore in the product, with a last-backup indicator — **high** · missing-function
- [ ] [UXG-294](issues/UXG-294-show-the-running-release-and-allow-update-and-rollback-from.md) Show the running release and allow update and rollback from the cockpit — **medium** · missing-function
- [ ] [UXG-295](issues/UXG-295-replace-terminal-commands-in-appliance-repair-banners-with-a.md) Replace terminal commands in appliance repair banners with actions — **medium** · ux
- [ ] [UXG-296](issues/UXG-296-activate-sharing-and-tailscale-addresses-without-leaving-the.md) Activate sharing and Tailscale addresses without leaving the cockpit — **high** · missing-function
- [ ] [UXG-297](issues/UXG-297-let-the-owner-save-a-named-version-of-a-document-as-they-can.md) Let the owner save a named version of a document as they can a sheet — **medium** · consistency
- [ ] [UXG-298](issues/UXG-298-break-model-spend-down-by-agent-and-model.md) Break model spend down by agent and model — **medium** · missing-function
- [ ] [UXG-299](issues/UXG-299-explain-at-least-spend-and-let-the-owner-clear-a-spend-poiso.md) Explain "At least" spend and let the owner clear a spend poison — **medium** · ux
- [ ] [UXG-300](issues/UXG-300-let-the-owner-create-rename-re-lead-and-disband-teams.md) Let the owner create, rename, re-lead and disband teams — **medium** · missing-function
- [ ] [UXG-301](issues/UXG-301-show-schedule-run-history-and-let-the-owner-recover-a-skippe.md) Show schedule run history and let the owner recover a skipped run — **medium** · missing-function
- [ ] [UXG-302](issues/UXG-302-let-the-owner-choose-fallback-models.md) Let the owner choose fallback models — **low** · missing-function
- [ ] [UXG-303](issues/UXG-303-allow-pre-approving-a-contact-from-approved-contacts.md) Allow pre-approving a contact from Approved contacts — **low** · missing-function
- [ ] [UXG-304](issues/UXG-304-surface-or-delete-the-structured-decisions-record-route.md) Surface or delete the structured decisions record route — **low** · spec-drift
- [ ] [UXG-305](issues/UXG-305-prune-unused-client-contracts-left-after-the-route-parity-ch.md) Prune unused client contracts left after the route parity check — **low** · consistency

#### Cross-surface journeys

- [ ] [UXG-306](issues/UXG-306-tell-the-owner-when-the-model-budget-stops-the-company.md) Tell the owner when the model budget stops the company — **high** · missing-function
- [ ] [UXG-307](issues/UXG-307-resume-stalled-work-when-the-owner-raises-the-spend-limit.md) Resume stalled work when the owner raises the spend limit — **medium** · missing-function
- [ ] [UXG-308](issues/UXG-308-bring-a-failed-or-uncertain-approved-action-back-to-the-owne.md) Bring a failed or uncertain approved action back to the owner — **high** · missing-function
- [ ] [UXG-309](issues/UXG-309-surface-failed-and-skipped-scheduled-runs-outside-the-schedu.md) Surface failed and skipped scheduled runs outside the Schedules page — **medium** · ux
- [ ] [UXG-310](issues/UXG-310-let-exec-reply-to-a-customer-who-wrote-in-not-just-send-one.md) Let Exec reply to a customer who wrote in, not just send one first-contact email — **high** · missing-function
- [ ] [UXG-311](issues/UXG-311-show-which-work-and-customer-an-approval-or-external-action.md) Show which Work and customer an approval or external action belongs to — **medium** · ux
- [ ] [UXG-312](issues/UXG-312-let-the-owner-connect-the-payment-rail-and-approve-payees-fr.md) Let the owner connect the payment rail and approve payees from the web — **high** · missing-function
- [ ] [UXG-313](issues/UXG-313-give-an-admin-cofounder-visibility-into-activity-spend-and-a.md) Give an admin cofounder visibility into Activity, spend and Apps — **medium** · missing-function
- [ ] [UXG-314](issues/UXG-314-add-an-away-mode-that-pauses-recurring-work-and-names-a-depu.md) Add an away mode that pauses recurring work and names a deputy — **medium** · missing-function
- [ ] [UXG-315](issues/UXG-315-make-telegram-notices-actionable-away-from-the-paired-comput.md) Make Telegram notices actionable away from the paired computer — **medium** · mobile
- [ ] [UXG-316](issues/UXG-316-say-who-decided-an-item-when-an-old-link-opens-it.md) Say who decided an item when an old link opens it — **low** · ux
- [ ] [UXG-317](issues/UXG-317-link-a-native-document-to-the-work-it-was-produced-for.md) Link a native document to the Work it was produced for — **medium** · ux
- [ ] [UXG-318](issues/UXG-318-route-a-document-comment-asking-for-a-change-to-the-work-s-r.md) Route a document comment asking for a change to the Work's revision — **medium** · spec-drift
- [ ] [UXG-319](issues/UXG-319-bring-brand-drift-decisions-to-the-inbox.md) Bring brand drift decisions to the Inbox — **medium** · ux
- [ ] [UXG-320](issues/UXG-320-provide-a-weekly-review-across-work-decisions-actions-and-sp.md) Provide a weekly review across work, decisions, actions and spend — **medium** · missing-function
- [ ] [UXG-321](issues/UXG-321-let-the-owner-find-everything-about-one-client-in-one-place.md) Let the owner find everything about one client in one place — **medium** · missing-function
- [ ] [UXG-322](issues/UXG-322-check-for-a-model-before-the-welcome-interview-starts.md) Check for a model before the welcome interview starts — **medium** · ux
- [ ] [UXG-323](issues/UXG-323-return-the-owner-to-where-they-started-after-an-app-sign-in.md) Return the owner to where they started after an app sign-in, and wake the work that failed meanwhile — **low** · ux
- [ ] [UXG-324](issues/UXG-324-link-the-computer-is-stopped-review-message-to-the-recovery.md) Link the "computer is stopped" review message to the recovery action — **low** · ux
- [ ] [UXG-325](issues/UXG-325-point-to-the-accepted-result-after-an-inbox-decision.md) Point to the accepted result after an Inbox decision — **low** · ux
