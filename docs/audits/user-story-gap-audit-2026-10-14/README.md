# User-story gap audit — 14 October 2026

The audit has 100 owner user stories, each with its click path through the cockpit. Every path was traced through the code and checked for missing functions and UI/UX gaps. Each gap is filed as an issue under [`issues/`](./issues/). The audit fixes nothing.

**How it was run.** The stories cover seven surfaces: Brand & voice, Inbox, Work, Library, People, Apps, and Company/onboarding. Each surface was traced in the SPA (`web/src`), and the owner API (`crates/restless-owner`) was checked for endpoints with no UI and UI with no backend. Every gap cites `file:line` evidence on `main` at `e3e7fdf`. The highest-impact claims were spot-checked by hand. Nothing was run against a live company, so findings are code-traced, not dogfood-observed: confirm each one before building.

**Totals.** 100 stories: 1 works, 70 partial, 29 missing. 190 issues: 30 high, 96 medium, 64 low.

**Recurring themes.**
- *Create but never edit.* This is the most common gap. Exec-drafted brand statements (UXG-001), schedule prompts (UXG-166), know-how (UXG-133), Work titles and goals (UXG-046, UXG-050), connections (UXG-132), Vault secrets (UXG-171) and email mandates (UXG-120) can be created but not edited or removed.
- *Data-model shapes instead of owner shapes.* Brand & voice is shown as scattered statement rows rather than one document (UXG-002). Apps Browse uses tiles that hide the facts needed to choose (UXG-152). Raw run states and paths show on Work (UXG-063).
- *One-click irreversible acts.* Stopping Work (UXG-042), removing an app (UXG-129), single-key approvals (UXG-018), single-click "Always allow" (UXG-010) and archiving a company (UXG-179) all lack a confirmation or an undo.
- *Owner-only affordances that lock out cofounders.* @mention (UXG-114), reactions (UXG-106), notifications (UXG-028) and "Waiting on you" (UXG-054) work only for the owner.
- *Spec promises without code.* Examples: the actor profile (UXG-102), budget periods and warnings (UXG-169), a company-wide freeze (UXG-185) and outcome-native review on Work (UXG-040).

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

## Issues

Ticket status lives only in this checklist. Tick an item when its fix lands on `main`.


### Brand & voice

- [ ] [UXG-001](issues/UXG-001-let-the-owner-edit-or-remove-any-released-brand-statement-no.md) Let the owner edit or remove any released brand statement, not only their own pillar text — **high** · missing-function
- [ ] [UXG-002](issues/UXG-002-present-brand-voice-as-one-readable-brand-document-instead-o.md) Present Brand & voice as one readable brand document instead of fragmented statement rows — **high** · ux
- [ ] [UXG-003](issues/UXG-003-let-the-owner-amend-an-exec-identity-proposal-before-promoti.md) Let the owner amend an Exec identity proposal before promoting, and make the reason optional — **medium** · missing-function
- [ ] [UXG-004](issues/UXG-004-allow-restoring-an-earlier-brand-version.md) Allow restoring an earlier brand version — **medium** · missing-function
- [ ] [UXG-005](issues/UXG-005-stop-hiding-exec-statements-that-still-ship-when-the-owner-w.md) Stop hiding Exec statements that still ship when the owner writes a pillar — **high** · ux
- [ ] [UXG-006](issues/UXG-006-give-visual-identity-structured-fields-logo-colours-type-in.md) Give visual identity structured fields (logo, colours, type) in one place — **medium** · missing-function
- [ ] [UXG-007](issues/UXG-007-let-the-owner-export-or-share-the-brand-guide.md) Let the owner export or share the brand guide — **low** · missing-function

### Inbox

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

### Work

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

### Library

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

### People

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

### Apps

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

### Company

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
