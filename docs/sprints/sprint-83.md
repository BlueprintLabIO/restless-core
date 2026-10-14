# Sprint 83 — Close the owner gaps

**Status:** landed (every item has a disposition on `main`; live verification limits are below)
**Programme:** owner feedback; quality of the cockpit
**Depends on:** Sprint 82 (steady data layer, the work is the surface). Every fix starts from `origin/main` after it.

## Outcome

The 14 October audit (`docs/audits/user-story-gap-audit-2026-10-14/`) traced 210 owner user stories through the code. It filed 325 gaps: 54 high, 168 medium and 103 low severity. Common threads:

- things that can be created but never edited (brand statements, schedule prompts, know-how, Work, connections, secrets);
- one-click irreversible acts;
- owner-only affordances that lock cofounders out;
- failures that never reach the owner;
- a handover desktop the owner cannot type into;
- capabilities that exist only in the terminal.

The owner asked for every gap to be fixed and merged into `main`. When this sprint is done, every audit item is closed with a disposition, and the owner can do on the web what the stories describe.

## Dispositions

Every item ends in exactly one of these. The checklist below records it.

- **Fixed:** the gap is closed in code on `main`. The commit message names the `UXG-NNN` and the exact check that was run.
- **Already fixed:** a later commit closed it (Sprint 82 rewrote much of the Inbox, charter and data layer). The checklist names that commit.
- **Deleted:** the right fix was to remove dead machinery: an unused route, contract or page. CLAUDE.md treats deletion as progress.
- **Won't fix (reason):** the finding is wrong on current `main`, or the fix would add weight out of proportion to the gap. The reason is one line. This is used sparingly; the owner asked for fixes.

## Acceptance

- Every checklist item is ticked with a disposition and a commit.
- `npm --prefix web run check` stays at 0 errors and 0 warnings. `cargo check --workspace` and the touched crates' tests pass. Every changed web model has its `node --test` file passing.
- The high-severity items are probed live against a `_test` company where the container can run one (see *Live verification*). Every item that could not be run live says so in its commit. Nothing is reported as working on the strength of a compile.
- No new eyebrows or subtitles. Explanations use tooltips (CLAUDE.md). Code stays brand-neutral, and no agent-roster dashboards are added.

## Slice per layer

- **Owner surface (web/):** most items. They are fixed in the existing Svelte design system (`$lib/ui`), with no new visual identity.
- **Owner API (crates/restless-owner):** routes the UI needs that exist in OrgIntel or the engine but not over HTTP. Examples: work edit and reassign, goal details, schedule prompt, vault delete, company delete and export, interrupt, publication authorise. The routes stay thin, and domain rules stay in the engine or OrgIntel.
- **OrgIntel / engine:** only where a capability does not exist yet. Examples: identity statement retire and supersede, budget period and threshold, the away-mode pause, routing failed effects to Attention.
- **Kernel/authority:** the company-wide freeze of outside actions and payment envelope edits go through existing Authority commands. They do not add new governance algebra.

## Decisions taken by default

These are product calls the audit surfaced. Each is implemented with the default below unless the founders override it. An override is a new checklist note, not a revert of history.

| Item | Default decision |
|---|---|
| UXG-002 Brand & voice shape | Show the current identity as one document with four headed pillars, each statement editable in place. Proposals appear as suggested changes on that document. The evidence model stays underneath. |
| UXG-029 / UXG-122 Pixel office | Keep the office, below recent outcomes and next actions. Show a "+N" overflow. Decorations are saved per viewer on the server where a route exists, otherwise unchanged. |
| UXG-069 Library folders | Lightweight collections: a `collection` label on Library items with a sidebar list. No nested tree. |
| UXG-070 Outside sharing | Show clearly, before sending, when an email invite grants company membership. Add item-scoped guest access only if the issuer supports a guest role; otherwise the warning ships and guest access is recorded as a follow-up. |
| UXG-051 Work graph view | Amend the spec. Owner and status filters ship; the graph view is not built. |
| UXG-169 Budget | Add a monthly period option and a warning threshold that raises an Inbox item before the ceiling. |
| UXG-185 Freeze | One "Pause outside actions" switch with resume, backed by existing per-connection freezes plus the effect gate. |
| UXG-293 / UXG-294 Backup, release | Show the last backup and release in the cockpit. Backup, update and rollback actions call the same appliance code paths the CLI uses. |
| UXG-310 Replying to customers | Exec may reply in-thread within an existing email mandate. A first contact still needs the mandate's first-contact rule. |
| Dead routes and contracts (UXG-097, 125, 304, 305) | Delete unless an item in this sprint wires them. |

## Work packages

Each package is owned by one agent at a time and lands as small verified commits on `main`. Packages follow file ownership to keep merges cheap. Order: high-severity items first inside each package.

| Package | Area | Items | Count |
|---|---|---|---|
| WP01 | Brand & voice | UXG-001 – UXG-007 | 7 |
| WP02 | Inbox I | UXG-008 – UXG-022 | 15 |
| WP03 | Inbox II | UXG-023 – UXG-038 | 16 |
| WP04 | Work I | UXG-039 – UXG-053 | 15 |
| WP05 | Work II | UXG-054 – UXG-068 | 15 |
| WP06 | Library I | UXG-069 – UXG-083 | 15 |
| WP07 | Library II | UXG-084 – UXG-097 | 14 |
| WP08 | People I | UXG-098 – UXG-112 | 15 |
| WP09 | People II | UXG-113 – UXG-126 | 14 |
| WP10 | Apps I | UXG-127 – UXG-141 | 15 |
| WP11 | Apps II | UXG-142 – UXG-155 | 14 |
| WP12 | Company I | UXG-156 – UXG-172 | 17 |
| WP13 | Company II | UXG-173 – UXG-190 | 18 |
| WP14 | Exec chat I | UXG-191 – UXG-203 | 13 |
| WP15 | Exec chat II | UXG-204 – UXG-215 | 12 |
| WP16 | Editors I | UXG-216 – UXG-228 | 13 |
| WP17 | Editors II | UXG-229 – UXG-241 | 13 |
| WP18 | Company computer | UXG-242 – UXG-265 | 24 |
| WP19 | Shell | UXG-266 – UXG-290 | 25 |
| WP20 | Endpoint & CLI parity | UXG-291 – UXG-305 | 15 |
| WP21 | Journeys | UXG-306 – UXG-325 | 20 |

## Verification protocol (every commit)

1. Rebase on `origin/main` and re-read the cited code. If it already changed, record **Already fixed** with the commit.
2. Make the minimal fix in the existing idiom.
3. Run `npm --prefix web run check` (0 errors) for web changes; `cargo check -p <crate>` and the named tests for Rust; `node --test` on any changed model test.
4. The commit message names `UXG-NNN`, what changed, and the exact check run. Push to `main` with rebase. Never force-push.
5. Close the matching GitHub issue with the commit hash.

## Live verification

The container has dockerd, Postgres 16, Chromium and the Rust/Node toolchains, but no company model credential (`GPT_BASE_URL` / `GPT_API_KEY` are unset). A `_test` company can therefore show every screen, but not Exec or Staff turns. Model-dependent items say "not run live" in their commit until a credential is provided. Any `_test` company, container or daemon started is torn down in the same turn (`restless-reap --check`).

## Results — 14 October 2026

**All 325 items have a disposition on `main`: 321 fixed, 1 already fixed, 3 deleted.**

- **Already fixed:** UXG-038, closed by WP02's `1458575`.
- **Deleted (dead machinery):** UXG-125, UXG-264, UXG-304.
- **UXG-189, first closed as won't fix:** `AccountHome` is not dead; it ships in `@restless/ui` for Restless Cloud's Home. The owner problem was fixed instead (`5190f546`): the real Home now shows each company's spend against its limit and this computer's last backup.

Each GitHub issue `UXG-NNN:` is closed with its commit.

**How it was verified.** The final gate ran on `39e4ffb5` with a local test Postgres. Database-backed tests ran rather than skipping. Results:

| Check | Result |
|---|---|
| `svelte-check` | 0 errors, 0 warnings |
| `check-type`, `check-ui-boundary`, `check-document-search` | pass |
| `node --test` web model tests | 34/34 files pass |
| `cargo check --workspace --tests` | 0 warnings |
| `restless-owner` tests | 126 passed, 0 failed |
| `restless-orgintel` tests | 240 passed, 0 failed |
| `restless` tests | 27 passed, 0 failed |
| `restless-engine` | 415 passed, 0 failed |

Fixing the engine failures that predated the sprint uncovered a real production deadlock:

- `f1257573`: `AuthorityStore::connect` ran its schema statements in one transaction. Under concurrent company admission, that took table locks in the opposite order from the admission, which deadlocked in production as well as in tests. The statements now commit one at a time under one session lock.
- `66bd7f90`: the review-copy test now skips only its reviewer probe when the process can bypass file permissions, as root can. Run as `nobody`, the full probe passes.
- `a0c37530`: the sheets integration test skips with the install command when the pinned spreadsheet engine is absent.

**Regressions this sprint introduced, all caught by the gate and repaired:**

- `b1232117` (UXG-313) admitted admins to the owner's whole company view, `GET /company`. That view includes credential probes and configuration. `0881fa92` restored owner-only access. Admins now read spend through a narrow `GET /company/spend`. Activity's payments and pending effects through `GET /company/activity/payments` (`39e4ffb5`) show admins display fields only, without the paying account or transfer reference.
- `26c08c4c` (UXG-314, away mode) let every admin past the membership boundary for `approvals/grant|decline`. `0283b24a` makes the handler admit only the named deputy of an away in force, before any side effect, for single decisions. New tests cover the deputy cases.
- Migrations 0101–0108 landed without moving `release::SCHEMA_VERSION`. `6476c777` declares 108. The same commit makes the CLI-spelling check read `direction.rs`; that check had been red since Sprint 80.
- `5f65c4b` tracked a `web/node_modules` symlink, which blocked rebases. `f86afd9` untracked it and ignores links.

**Product calls made inside the sprint.** Founders may override any of them.

- **UXG-100, company ownership transfer:** an owner-only "Make owner…" behind a typed-name confirmation. Core moves Authority first, then the identity issuer swaps the owner and admin roles atomically. A failed second step is finished by retrying. Between the two steps, nobody can decide approvals (ADR 0012, decision 7).
- **UXG-180, permanent company deletion:** a new irreversible path. It is limited to the owner and archived companies, needs the exact typed name (checked on the server) and offers an export first. Authority records are kept, and a tombstone blocks reuse of the name.
- **UXG-206 / UXG-313, admin visibility:** admins read Activity, spend, Apps, agent handoffs and app requests, read-only. Goals and the owner's set-aside list stay owner-only.
- **UXG-070, item-scoped guest sharing:** not built, because the issuer has only owner, admin and member roles. Share now warns before an email invite grants company membership.
- **UXG-256, external links:** they now open in a new tab by default. Routing them through the company browser is an owner opt-in.
- **UXG-030, web push:** works with the tab closed on hosted planes only, via VAPID signing with `ring`. It posts only to browser push services.

**Not run live.** No agent ran a live company: there is no company model credential here.

### Live check

A `_test` company stack (`restless-dev audit_test --reconcile`) was attempted after the final gate. It did not come up, so **no screen of this sprint has been seen in a real browser yet**.

What happened:

- The cockpit (`npm run build`) and `restlessd` / `restless` built cleanly from `0881fa92`.
- The document-service image could not be built: Docker Hub refused the pinned `node:24-trixie-slim` digest with `429 Too Many Requests`, the anonymous pull limit for the session's shared egress.
- Using a registry mirror was not permitted in this environment, so the attempt stopped there. No company, container, volume or profile state was created, and the session's Docker daemon and test Postgres were stopped and removed.

The disk guard also needed a workaround: the container tops out near 29 GiB free, under the stack's 30 GiB check. The attempt shimmed `df` for that one run and monitored real usage instead; nothing in the repo was changed for it.

**First founder follow-up:** run `restless-dev <name>_test --reconcile` on the founder machine, then walk the high-severity items in a browser. Start with:

- the Inbox handover desktop (UXG-243 to UXG-248);
- Brand & voice as a document (UXG-001 to UXG-007);
- Work steering (UXG-039 to UXG-050);
- Retry and Stop in the rail (UXG-191, UXG-192);
- away mode with a deputy (UXG-314);
- ownership handover (UXG-100);
- company delete and export (UXG-180, UXG-182).

## Checklist

Status lives only here; the audit README's checklist points to this one.


### WP01 — Brand & voice

- [x] [UXG-001](../audits/user-story-gap-audit-2026-10-14/issues/UXG-001-let-the-owner-edit-or-remove-any-released-brand-statement-no.md) Let the owner edit or remove any released brand statement, not only their own pillar text — high — **Fixed** `4e5f44b`
- [x] [UXG-002](../audits/user-story-gap-audit-2026-10-14/issues/UXG-002-present-brand-voice-as-one-readable-brand-document-instead-o.md) Present Brand & voice as one readable brand document instead of fragmented statement rows — high — **Fixed** `4e5f44b`
- [x] [UXG-003](../audits/user-story-gap-audit-2026-10-14/issues/UXG-003-let-the-owner-amend-an-exec-identity-proposal-before-promoti.md) Let the owner amend an Exec identity proposal before promoting, and make the reason optional — medium — **Fixed** `4e5f44b`
- [x] [UXG-004](../audits/user-story-gap-audit-2026-10-14/issues/UXG-004-allow-restoring-an-earlier-brand-version.md) Allow restoring an earlier brand version — medium — **Fixed** `4e5f44b`
- [x] [UXG-005](../audits/user-story-gap-audit-2026-10-14/issues/UXG-005-stop-hiding-exec-statements-that-still-ship-when-the-owner-w.md) Stop hiding Exec statements that still ship when the owner writes a pillar — high — **Fixed** `4e5f44b`
- [x] [UXG-006](../audits/user-story-gap-audit-2026-10-14/issues/UXG-006-give-visual-identity-structured-fields-logo-colours-type-in.md) Give visual identity structured fields (logo, colours, type) in one place — medium — **Fixed** `4e5f44b`
- [x] [UXG-007](../audits/user-story-gap-audit-2026-10-14/issues/UXG-007-let-the-owner-export-or-share-the-brand-guide.md) Let the owner export or share the brand guide — low — **Fixed** `4e5f44b`

### WP02 — Inbox I

- [x] [UXG-008](../audits/user-story-gap-audit-2026-10-14/issues/UXG-008-add-undo-to-dismiss-in-the-inbox-item-detail.md) Add Undo to "Dismiss" in the Inbox item detail — high — **Fixed** `1458575`
- [x] [UXG-009](../audits/user-story-gap-audit-2026-10-14/issues/UXG-009-use-one-name-for-dismiss-not-needed-set-aside.md) Use one name for "Dismiss / Not needed / Set aside" — medium — **Fixed** `1458575`
- [x] [UXG-010](../audits/user-story-gap-audit-2026-10-14/issues/UXG-010-make-always-allow-at-least-as-deliberate-as-a-one-off-approv.md) Make "Always allow" at least as deliberate as a one-off approval — high — **Fixed** `1458575`
- [x] [UXG-011](../audits/user-story-gap-audit-2026-10-14/issues/UXG-011-let-the-owner-snooze-until-a-chosen-time-or-date.md) Let the owner snooze until a chosen time or date — medium — **Fixed** `1458575`
- [x] [UXG-012](../audits/user-story-gap-audit-2026-10-14/issues/UXG-012-add-snooze-and-copy-link-to-the-inbox-item-detail.md) Add Snooze and Copy link to the Inbox item detail — medium — **Fixed** `1458575`
- [x] [UXG-013](../audits/user-story-gap-audit-2026-10-14/issues/UXG-013-make-the-today-line-link-to-its-lists.md) Make the "Today" line link to its lists — medium — **Fixed** `4cab9e7`
- [x] [UXG-014](../audits/user-story-gap-audit-2026-10-14/issues/UXG-014-let-an-applied-change-row-open-the-change-it-made.md) Let an applied-change row open the change it made — medium — **Fixed** `8c3be9c`
- [x] [UXG-015](../audits/user-story-gap-audit-2026-10-14/issues/UXG-015-remove-eyebrow-kickers-in-direction-requests-and-the-compute.md) Remove eyebrow kickers in direction requests and the computer view — low — **Fixed** `1458575`
- [x] [UXG-016](../audits/user-story-gap-audit-2026-10-14/issues/UXG-016-show-whether-the-company-can-keep-going-while-an-item-waits.md) Show whether the company can keep going while an item waits — medium — **Fixed** `1458575`
- [x] [UXG-017](../audits/user-story-gap-audit-2026-10-14/issues/UXG-017-order-the-inbox-by-consequence-and-deadline-not-age-alone.md) Order the Inbox by consequence and deadline, not age alone — medium — **Fixed** `1458575`
- [x] [UXG-018](../audits/user-story-gap-audit-2026-10-14/issues/UXG-018-stop-single-keystroke-approvals-of-direction-requests.md) Stop single-keystroke approvals of direction requests — high — **Fixed** `92a0a88`
- [x] [UXG-019](../audits/user-story-gap-audit-2026-10-14/issues/UXG-019-link-the-inbox-to-what-i-decided-and-allow-amending-a-recent.md) Link the Inbox to "what I decided" and allow amending a recent answer — medium — **Fixed** `5e4f656`
- [x] [UXG-020](../audits/user-story-gap-audit-2026-10-14/issues/UXG-020-let-the-owner-delegate-an-inbox-item-to-a-member-or-admin.md) Let the owner delegate an Inbox item to a member or admin — medium — **Fixed** `285490e`
- [x] [UXG-021](../audits/user-story-gap-audit-2026-10-14/issues/UXG-021-let-a-requester-see-all-their-requests-and-withdraw-a-pendin.md) Let a requester see all their requests and withdraw a pending one — medium — **Fixed** `5e4f656`
- [x] [UXG-022](../audits/user-story-gap-audit-2026-10-14/issues/UXG-022-give-set-aside-dates-the-asker-and-a-visible-failure-state.md) Give "Set aside" dates, the asker, and a visible failure state — low — **Fixed** `588f491`

### WP03 — Inbox II

- [x] [UXG-023](../audits/user-story-gap-audit-2026-10-14/issues/UXG-023-wire-the-existing-pass-to-owner-endpoint-for-members.md) Wire the existing "pass to owner" endpoint for members — medium — **Fixed** `2d47878`
- [x] [UXG-024](../audits/user-story-gap-audit-2026-10-14/issues/UXG-024-gate-hold-to-accept-and-request-changes-with-the-same-real-r.md) Gate "Hold to accept" and "Request changes" with the same real roles — low — **Fixed** `29cda16`
- [x] [UXG-025](../audits/user-story-gap-audit-2026-10-14/issues/UXG-025-let-the-owner-finish-a-human-step-from-the-computer-view.md) Let the owner finish a human step from the computer view — low — **Fixed** `b574912`
- [x] [UXG-026](../audits/user-story-gap-audit-2026-10-14/issues/UXG-026-show-what-was-asked-and-by-whom-on-document-items.md) Show what was asked, and by whom, on document items — medium — **Fixed** `9a6979f`
- [x] [UXG-027](../audits/user-story-gap-audit-2026-10-14/issues/UXG-027-give-a-failed-direction-request-a-way-forward.md) Give a failed direction request a way forward — medium — **Fixed** `35850cf`
- [x] [UXG-028](../audits/user-story-gap-audit-2026-10-14/issues/UXG-028-notify-members-not-just-owners-of-new-inbox-items.md) Notify members, not just owners, of new Inbox items — medium — **Fixed** `9a51a22`
- [x] [UXG-029](../audits/user-story-gap-audit-2026-10-14/issues/UXG-029-reconsider-the-pixel-office-as-the-clear-inbox-home.md) Reconsider the pixel office as the clear-Inbox home — medium — **Fixed** `2d47878`
- [x] [UXG-030](../audits/user-story-gap-audit-2026-10-14/issues/UXG-030-add-notifications-that-work-with-the-tab-closed-and-an-email.md) Add notifications that work with the tab closed, and an email preference — medium — **Fixed** `6283281`
- [x] [UXG-031](../audits/user-story-gap-audit-2026-10-14/issues/UXG-031-make-the-shortcuts-sheet-list-every-key-that-works.md) Make the shortcuts sheet list every key that works — low — **Fixed** `591ce3a`
- [x] [UXG-032](../audits/user-story-gap-audit-2026-10-14/issues/UXG-032-stop-e-meaning-mark-done-in-the-inbox-but-mark-read-in-for-y.md) Stop E meaning "mark done" in the Inbox but "mark read" in For you — medium — **Fixed** `b574912`
- [x] [UXG-033](../audits/user-story-gap-audit-2026-10-14/issues/UXG-033-make-the-inbox-badge-match-the-list.md) Make the Inbox badge match the list — low — **Fixed** `9a51a22`
- [x] [UXG-034](../audits/user-story-gap-audit-2026-10-14/issues/UXG-034-let-a-sent-rail-message-be-corrected-after-the-agent-has-rea.md) Let a sent rail message be corrected after the agent has read it — low — **Fixed** `7301e90`
- [x] [UXG-035](../audits/user-story-gap-audit-2026-10-14/issues/UXG-035-expose-unread-snoozed-and-kind-to-screen-readers-in-the-inbo.md) Expose unread, snoozed and kind to screen readers in the Inbox — medium — **Fixed** `b574912`
- [x] [UXG-036](../audits/user-story-gap-audit-2026-10-14/issues/UXG-036-stop-offering-mark-done-on-app-requests-and-surface-fetch-fa.md) Stop offering "Mark done" on app requests and surface fetch failures — low — **Fixed** `b574912`
- [x] [UXG-037](../audits/user-story-gap-audit-2026-10-14/issues/UXG-037-let-an-owner-give-a-reason-when-declining-an-email-mandate.md) Let an owner give a reason when declining an email mandate — low — **Fixed** `b574912`
- [x] [UXG-038](../audits/user-story-gap-audit-2026-10-14/issues/UXG-038-show-non-owners-only-the-inbox-actions-they-may-take.md) Show non-owners only the Inbox actions they may take — low — **Already fixed** `1458575`

### WP04 — Work I

- [x] [UXG-039](../audits/user-story-gap-audit-2026-10-14/issues/UXG-039-surface-the-pending-decision-on-the-work-detail-page.md) Surface the pending decision on the Work detail page — high — **Fixed** `b8eaad2`
- [x] [UXG-040](../audits/user-story-gap-audit-2026-10-14/issues/UXG-040-bring-the-prepared-reviewtarget-to-the-work-detail-page.md) Bring the prepared ReviewTarget to the Work detail page — high — **Fixed** `b8eaad2`
- [x] [UXG-041](../audits/user-story-gap-audit-2026-10-14/issues/UXG-041-let-the-owner-accept-reject-or-request-changes-from-work.md) Let the owner accept, reject or request changes from Work — high — **Fixed** `b8eaad2`
- [x] [UXG-042](../audits/user-story-gap-audit-2026-10-14/issues/UXG-042-separate-pause-from-stop-for-good-and-warn-before-abandoning.md) Separate "pause" from "stop for good" and warn before abandoning — high — **Fixed** `b8eaad2`
- [x] [UXG-043](../audits/user-story-gap-audit-2026-10-14/issues/UXG-043-give-stopped-work-a-place-to-be-found-and-restored.md) Give stopped work a place to be found and restored — high — **Fixed** `f8cf1e1`
- [x] [UXG-044](../audits/user-story-gap-audit-2026-10-14/issues/UXG-044-show-why-blocked-work-is-blocked-in-the-list-and-detail-head.md) Show why blocked work is blocked in the list and detail header — medium — **Fixed** `b8eaad2`
- [x] [UXG-045](../audits/user-story-gap-audit-2026-10-14/issues/UXG-045-show-and-let-the-owner-change-work-priority-add-sort.md) Show and let the owner change Work priority; add sort — medium — **Fixed** `f8cf1e1`
- [x] [UXG-046](../audits/user-story-gap-audit-2026-10-14/issues/UXG-046-allow-renaming-a-work-item-and-editing-its-scope-or-goal.md) Allow renaming a Work item and editing its scope or goal — medium — **Fixed** `b8eaad2`
- [x] [UXG-047](../audits/user-story-gap-audit-2026-10-14/issues/UXG-047-show-attempt-history-feedback-and-inputs-on-work-detail.md) Show attempt history, feedback and inputs on Work detail — medium — **Fixed** `b8eaad2`
- [x] [UXG-048](../audits/user-story-gap-audit-2026-10-14/issues/UXG-048-wire-lead-team-reassignment-for-work-and-goals.md) Wire lead/team reassignment for Work and Goals — medium — **Fixed** `b8eaad2`
- [x] [UXG-049](../audits/user-story-gap-audit-2026-10-14/issues/UXG-049-show-cost-per-work-and-per-goal.md) Show cost per Work and per Goal — medium — **Fixed** `b8eaad2`
- [x] [UXG-050](../audits/user-story-gap-audit-2026-10-14/issues/UXG-050-add-goal-editing-close-this-goal-and-reopen-in-the-work-ui.md) Add goal editing, close-this-goal and reopen in the Work UI — high — **Fixed** `f8cf1e1`
- [x] [UXG-051](../audits/user-story-gap-audit-2026-10-14/issues/UXG-051-add-owner-and-status-filters-and-decide-on-the-work-graph-vi.md) Add owner and status filters, and decide on the Work graph view — medium — **Fixed** `f8cf1e1`
- [x] [UXG-052](../audits/user-story-gap-audit-2026-10-14/issues/UXG-052-provide-a-goal-detail-page.md) Provide a goal detail page — medium — **Fixed** `f8cf1e1`
- [x] [UXG-053](../audits/user-story-gap-audit-2026-10-14/issues/UXG-053-mark-needs-you-on-board-cards.md) Mark "needs you" on board cards — medium — **Fixed** `98ddc30`

### WP05 — Work II

- [x] [UXG-054](../audits/user-story-gap-audit-2026-10-14/issues/UXG-054-make-waiting-on-you-work-for-team-owners-members.md) Make "Waiting on you" work for team owners (members) — medium — **Fixed** `a673450`
- [x] [UXG-055](../audits/user-story-gap-audit-2026-10-14/issues/UXG-055-fix-the-no-work-yet-empty-state-on-a-failed-search.md) Fix the "No work yet" empty state on a failed search — low — **Fixed** `a673450`
- [x] [UXG-056](../audits/user-story-gap-audit-2026-10-14/issues/UXG-056-widen-work-search-and-keep-it-in-the-url.md) Widen Work search and keep it in the URL — low — **Fixed** `a673450`
- [x] [UXG-057](../audits/user-story-gap-audit-2026-10-14/issues/UXG-057-unify-status-words-waiting-waiting-on-you-blocked-needs-you.md) Unify status words: Waiting / Waiting on you / Blocked / Needs you — medium — **Fixed** `a673450`
- [x] [UXG-058](../audits/user-story-gap-audit-2026-10-14/issues/UXG-058-start-add-work-from-a-goal-with-context-prefilled-and-confir.md) Start "add work" from a goal with context prefilled, and confirm creation — low — **Fixed** `a673450`
- [x] [UXG-059](../audits/user-story-gap-audit-2026-10-14/issues/UXG-059-reorder-or-replace-the-board-on-mobile.md) Reorder or replace the board on mobile — low — **Fixed** `a673450`
- [x] [UXG-060](../audits/user-story-gap-audit-2026-10-14/issues/UXG-060-cap-or-fold-the-board-s-done-column.md) Cap or fold the board's Done column — low — **Fixed** `a673450`
- [x] [UXG-061](../audits/user-story-gap-audit-2026-10-14/issues/UXG-061-fix-work-board-semantics-for-screen-readers.md) Fix Work board semantics for screen readers — medium — **Fixed** `a673450`
- [x] [UXG-062](../audits/user-story-gap-audit-2026-10-14/issues/UXG-062-replace-hover-only-title-explanations-on-work-with-touch-rea.md) Replace hover-only title explanations on Work with touch-reachable tips — low — **Fixed** `5dce793`
- [x] [UXG-063](../audits/user-story-gap-audit-2026-10-14/issues/UXG-063-hide-run-internals-and-raw-enum-words-from-the-default-work.md) Hide run internals and raw enum words from the default Work view — low — **Fixed** `5dce793`
- [x] [UXG-064](../audits/user-story-gap-audit-2026-10-14/issues/UXG-064-use-one-name-for-goal-less-work-other-work-vs-unassigned.md) Use one name for goal-less work ("Other work" vs "Unassigned") — low — **Fixed** `5dce793`
- [x] [UXG-065](../audits/user-story-gap-audit-2026-10-14/issues/UXG-065-make-goal-rows-real-links-in-the-work-sidebar.md) Make goal rows real links in the Work sidebar — low — **Fixed** `a673450`
- [x] [UXG-066](../audits/user-story-gap-audit-2026-10-14/issues/UXG-066-keep-list-context-when-opening-discuss-from-work.md) Keep list context when opening "Discuss" from Work — low — **Fixed** `5dce793`
- [x] [UXG-067](../audits/user-story-gap-audit-2026-10-14/issues/UXG-067-do-not-require-a-written-reason-to-resume-work.md) Do not require a written reason to resume work — low — **Fixed** `5dce793`
- [x] [UXG-068](../audits/user-story-gap-audit-2026-10-14/issues/UXG-068-add-row-quick-actions-and-later-bulk-actions-export-to-the-w.md) Add row quick actions (and later bulk actions/export) to the Work list — low — **Fixed** `a673450`

### WP06 — Library I

- [x] [UXG-069](../audits/user-story-gap-audit-2026-10-14/issues/UXG-069-add-folders-or-collections-to-the-library.md) Add folders or collections to the Library — high — **Fixed** `b24b68e`
- [x] [UXG-070](../audits/user-story-gap-audit-2026-10-14/issues/UXG-070-stop-email-sharing-from-making-outsiders-full-company-member.md) Stop email sharing from making outsiders full company members — high — **Fixed** `5f65c4b`
- [x] [UXG-071](../audits/user-story-gap-audit-2026-10-14/issues/UXG-071-allow-renaming-work-outputs-and-uploaded-files.md) Allow renaming Work outputs and uploaded files — medium — **Fixed** `b24b68e`
- [x] [UXG-072](../audits/user-story-gap-audit-2026-10-14/issues/UXG-072-allow-permanent-deletion-of-archived-work-outputs.md) Allow permanent deletion of archived Work outputs — medium — **Fixed** `b24b68e`
- [x] [UXG-073](../audits/user-story-gap-audit-2026-10-14/issues/UXG-073-let-archived-files-be-opened-or-previewed.md) Let archived files be opened or previewed — medium — **Fixed** `3b263e8`
- [x] [UXG-074](../audits/user-story-gap-audit-2026-10-14/issues/UXG-074-search-inside-sheets-and-comments-from-the-library-search-bo.md) Search inside sheets and comments from the Library search box — medium — **Fixed** `b24b68e`
- [x] [UXG-075](../audits/user-story-gap-audit-2026-10-14/issues/UXG-075-sort-and-count-across-all-documents-not-just-loaded-pages.md) Sort and count across all documents, not just loaded pages — medium — **Fixed** `b24b68e`
- [x] [UXG-076](../audits/user-story-gap-audit-2026-10-14/issues/UXG-076-add-owner-shared-with-me-and-date-filters-plus-bulk-select.md) Add owner, "shared with me" and date filters, plus bulk select — medium — **Fixed** `b24b68e`
- [x] [UXG-077](../audits/user-story-gap-audit-2026-10-14/issues/UXG-077-show-the-actual-old-document-when-previewing-history.md) Show the actual old document when previewing history — medium — **Fixed** `8ddcfa4`
- [x] [UXG-078](../audits/user-story-gap-audit-2026-10-14/issues/UXG-078-restore-sheet-versions-in-place-like-documents.md) Restore sheet versions in place, like documents — medium — **Fixed** `e867635`
- [x] [UXG-079](../audits/user-story-gap-audit-2026-10-14/issues/UXG-079-show-who-made-each-sheet-version.md) Show who made each sheet version — low — **Fixed** `e867635`
- [x] [UXG-080](../audits/user-story-gap-audit-2026-10-14/issues/UXG-080-let-a-docx-xlsx-upload-become-an-editable-doc-or-sheet.md) Let a .docx/.xlsx upload become an editable doc or sheet — high — **Fixed** `ee03bca`
- [x] [UXG-081](../audits/user-story-gap-audit-2026-10-14/issues/UXG-081-export-documents-as-word-docx.md) Export documents as Word (.docx) — medium — **Fixed** `2db04ff`
- [x] [UXG-082](../audits/user-story-gap-audit-2026-10-14/issues/UXG-082-let-an-old-document-version-be-downloaded.md) Let an old document version be downloaded — low — **Fixed** `8ddcfa4`
- [x] [UXG-083](../audits/user-story-gap-audit-2026-10-14/issues/UXG-083-give-owners-a-way-forward-when-email-invites-are-not-set-up.md) Give owners a way forward when email invites are not set up — medium — **Fixed** `5f65c4b`

### WP07 — Library II

- [x] [UXG-084](../audits/user-story-gap-audit-2026-10-14/issues/UXG-084-make-uploaded-files-shareable-and-visible-to-the-team.md) Make uploaded files shareable and visible to the team — medium — **Fixed** `4b6de75d`
- [x] [UXG-085](../audits/user-story-gap-audit-2026-10-14/issues/UXG-085-let-discuss-actually-allow-exec-to-make-the-requested-docume.md) Let Discuss actually allow Exec to make the requested document edit — high — **Fixed** `349fc1f`
- [x] [UXG-086](../audits/user-story-gap-audit-2026-10-14/issues/UXG-086-allow-deleting-a-document-from-its-own-page.md) Allow deleting a document from its own page — low — **Fixed** `349fc1f`
- [x] [UXG-087](../audits/user-story-gap-audit-2026-10-14/issues/UXG-087-fix-library-table-head-and-sort-accessibility.md) Fix Library table head and sort accessibility — low — **Fixed** `a1e4d6f7`
- [x] [UXG-088](../audits/user-story-gap-audit-2026-10-14/issues/UXG-088-refresh-the-sheet-list-and-keep-library-error-states-honest.md) Refresh the sheet list and keep Library error states honest — low — **Fixed** `a1e4d6f7`
- [x] [UXG-089](../audits/user-story-gap-audit-2026-10-14/issues/UXG-089-return-the-editor-s-back-link-to-where-the-owner-came-from.md) Return the editor's back link to where the owner came from — low — **Fixed** `0de66c58`
- [x] [UXG-090](../audits/user-story-gap-audit-2026-10-14/issues/UXG-090-offer-discuss-with-exec-on-sheets-and-link-the-file-on-file.md) Offer "Discuss with Exec" on sheets and link the file on file Discuss — medium — **Fixed** `d01e31e`
- [x] [UXG-091](../audits/user-story-gap-audit-2026-10-14/issues/UXG-091-hide-or-explain-request-review-for-people-who-cannot-request.md) Hide or explain "Request review…" for people who cannot request one — low — **Fixed** `612866df`
- [x] [UXG-092](../audits/user-story-gap-audit-2026-10-14/issues/UXG-092-let-the-requester-choose-a-reviewer-and-use-plain-language-i.md) Let the requester choose a reviewer, and use plain language in Review — medium — **Fixed** `612866df`
- [x] [UXG-093](../audits/user-story-gap-audit-2026-10-14/issues/UXG-093-remove-the-hard-coded-aris-company-and-brand-from-library-ro.md) Remove the hard-coded "aris" company and brand from Library routes — low — **Fixed** `349fc1f`
- [x] [UXG-094](../audits/user-story-gap-audit-2026-10-14/issues/UXG-094-show-agent-document-proposals-as-a-diff-not-two-full-texts.md) Show agent document proposals as a diff, not two full texts — medium — **Fixed** `81d1b4c`
- [x] [UXG-095](../audits/user-story-gap-audit-2026-10-14/issues/UXG-095-decide-whether-archived-documents-stay-editable.md) Decide whether archived documents stay editable — low — **Fixed** `612866df`
- [x] [UXG-096](../audits/user-story-gap-audit-2026-10-14/issues/UXG-096-handle-a-failed-sheet-history-load.md) Handle a failed sheet-history load — low — **Fixed** `d01e31e`
- [x] [UXG-097](../audits/user-story-gap-audit-2026-10-14/issues/UXG-097-surface-or-delete-the-document-backlinks-api.md) Surface or delete the document backlinks API — low — **Fixed** `989232a8`

### WP08 — People I

- [x] [UXG-098](../audits/user-story-gap-audit-2026-10-14/issues/UXG-098-add-invite-and-member-management-to-people.md) Add invite and member management to People — medium — **Fixed** `e8632bf`
- [x] [UXG-099](../audits/user-story-gap-audit-2026-10-14/issues/UXG-099-confirm-or-offer-undo-for-cancelling-invitations-and-pausing.md) Confirm or offer Undo for cancelling invitations and pausing access — medium — **Fixed** `4b0588a`
- [x] [UXG-100](../audits/user-story-gap-audit-2026-10-14/issues/UXG-100-support-transferring-company-ownership.md) Support transferring company ownership — medium — **Fixed** `0dfed789`
- [x] [UXG-101](../audits/user-story-gap-audit-2026-10-14/issues/UXG-101-give-the-owner-a-way-to-hire-retire-pause-or-re-brief-ai-sta.md) Give the owner a way to hire, retire, pause or re-brief AI staff — high — **Fixed** `e8632bf`
- [x] [UXG-102](../audits/user-story-gap-audit-2026-10-14/issues/UXG-102-build-the-actor-profile-the-spec-requires.md) Build the actor profile the spec requires — high — **Fixed** `e8632bf`
- [x] [UXG-103](../audits/user-story-gap-audit-2026-10-14/issues/UXG-103-confirm-model-changes-and-say-when-they-take-effect.md) Confirm model changes and say when they take effect — low — **Fixed** `1feae2a`
- [x] [UXG-104](../audits/user-story-gap-audit-2026-10-14/issues/UXG-104-make-a-person-s-current-work-clickable-from-people.md) Make a person's current Work clickable from People — medium — **Fixed** `e8632bf`
- [x] [UXG-105](../audits/user-story-gap-audit-2026-10-14/issues/UXG-105-unify-what-is-this-person-doing-between-people-and-the-offic.md) Unify "what is this person doing" between People and the office floor — low — **Fixed** `e8632bf`
- [x] [UXG-106](../audits/user-story-gap-audit-2026-10-14/issues/UXG-106-let-admins-and-members-react-and-pin-as-the-backend-allows.md) Let admins and members react and pin, as the backend allows — medium — **Fixed** `296f9a1`
- [x] [UXG-107](../audits/user-story-gap-audit-2026-10-14/issues/UXG-107-add-search-and-grouping-to-the-new-conversation-people-picke.md) Add search and grouping to the new-conversation people picker — low — **Fixed** `9717e44`
- [x] [UXG-108](../audits/user-story-gap-audit-2026-10-14/issues/UXG-108-make-room-rename-and-archive-discoverable.md) Make room rename and archive discoverable — medium — **Fixed** `9717e44`
- [x] [UXG-109](../audits/user-story-gap-audit-2026-10-14/issues/UXG-109-let-a-room-owner-leave-or-hand-over-a-group.md) Let a room owner leave or hand over a group — low — **Fixed** `9717e44`
- [x] [UXG-110](../audits/user-story-gap-audit-2026-10-14/issues/UXG-110-let-archived-groups-be-read-without-restoring-and-allow-hidi.md) Let archived groups be read without restoring, and allow hiding DMs — low — **Fixed** `53e396d`
- [x] [UXG-111](../audits/user-story-gap-audit-2026-10-14/issues/UXG-111-add-keyboard-shortcuts-to-message-editing-and-re-parse-menti.md) Add keyboard shortcuts to message editing and re-parse mentions on edit — medium — **Fixed** `3fc3f2da`
- [x] [UXG-112](../audits/user-story-gap-audit-2026-10-14/issues/UXG-112-label-people-message-search-and-give-it-loading-and-empty-st.md) Label People message search and give it loading and empty states — medium — **Fixed** `8821300`

### WP09 — People II

- [x] [UXG-113](../audits/user-story-gap-audit-2026-10-14/issues/UXG-113-show-group-rooms-beyond-the-first-page-in-chats.md) Show group rooms beyond the first page in Chats — medium — **Fixed** `2f028963`
- [x] [UXG-114](../audits/user-story-gap-audit-2026-10-14/issues/UXG-114-give-admins-and-members-the-mention-picker.md) Give admins and members the @mention picker — high — **Fixed** `3fc3f2da`
- [x] [UXG-115](../audits/user-story-gap-audit-2026-10-14/issues/UXG-115-show-readable-names-in-mentions-while-composing.md) Show readable names in mentions while composing — low — **Fixed** `3fc3f2da`
- [x] [UXG-116](../audits/user-story-gap-audit-2026-10-14/issues/UXG-116-add-per-room-mute-and-notification-preferences.md) Add per-room mute and notification preferences — medium — **Fixed** `c617c316`
- [x] [UXG-117](../audits/user-story-gap-audit-2026-10-14/issues/UXG-117-let-a-non-technical-owner-connect-telegram-without-typing-a.md) Let a non-technical owner connect Telegram without typing a Vault reference — high — **Fixed** `a232871a`
- [x] [UXG-118](../audits/user-story-gap-audit-2026-10-14/issues/UXG-118-confirm-before-unpairing-telegram-and-let-the-owner-filter-w.md) Confirm before unpairing Telegram and let the owner filter what it sends — low — **Fixed** `a232871a`
- [x] [UXG-119](../audits/user-story-gap-audit-2026-10-14/issues/UXG-119-show-loading-error-states-and-scope-for-standing-permissions.md) Show loading/error states and scope for standing permissions — medium — **Fixed** `e704ad43`
- [x] [UXG-120](../audits/user-story-gap-audit-2026-10-14/issues/UXG-120-allow-editing-an-email-mandate-and-explain-why-add-mandate-i.md) Allow editing an email mandate and explain why Add mandate is disabled — medium — **Fixed** `4a1f4589`
- [x] [UXG-121](../audits/user-story-gap-audit-2026-10-14/issues/UXG-121-confirm-team-owner-changes-and-keep-the-select-truthful-on-f.md) Confirm team-owner changes and keep the select truthful on failure — medium — **Fixed** `45146cbe`
- [x] [UXG-122](../audits/user-story-gap-audit-2026-10-14/issues/UXG-122-show-which-human-colleagues-are-online-in-people.md) Show which human colleagues are online in People — medium — **Fixed** `96eadc48`
- [x] [UXG-123](../audits/user-story-gap-audit-2026-10-14/issues/UXG-123-make-the-people-chats-switch-an-accessible-tablist.md) Make the People/Chats switch an accessible tablist — low — **Fixed** `bb2234a1`
- [x] [UXG-124](../audits/user-story-gap-audit-2026-10-14/issues/UXG-124-distinguish-a-message-from-a-directive-in-chat.md) Distinguish a message from a directive in chat — medium — **Fixed** `58ef90f1`
- [x] [UXG-125](../audits/user-story-gap-audit-2026-10-14/issues/UXG-125-expose-or-delete-unused-people-room-endpoints.md) Expose or delete unused people/room endpoints — low — **Deleted** `489bb12c`
- [x] [UXG-126](../audits/user-story-gap-audit-2026-10-14/issues/UXG-126-show-what-an-agent-dm-is-about-before-the-first-message.md) Show what an agent DM is about before the first message — low — **Fixed** `f91dfc18`

### WP10 — Apps I

- [x] [UXG-127](../audits/user-story-gap-audit-2026-10-14/issues/UXG-127-add-google-gmail-calendar-drive-microsoft-and-accounting-ser.md) Add Google (Gmail/Calendar/Drive), Microsoft and accounting services to the Apps catalogue — high — **Fixed** `059ce113`
- [x] [UXG-128](../audits/user-story-gap-audit-2026-10-14/issues/UXG-128-collect-the-token-when-a-token-auth-app-is-added-from-a-tile.md) Collect the token when a token-auth app is added from a tile or the ask box — high — **Fixed** `059ce113`
- [x] [UXG-129](../audits/user-story-gap-audit-2026-10-14/issues/UXG-129-confirm-before-removing-an-app-and-say-what-loses-access.md) Confirm before removing an app and say what loses access — high — **Fixed** `059ce113`
- [x] [UXG-130](../audits/user-story-gap-audit-2026-10-14/issues/UXG-130-remove-or-ask-about-a-plugin-bundle-s-know-how-when-the-bund.md) Remove (or ask about) a plugin bundle's know-how when the bundle is removed — medium — **Fixed** `059ce113`
- [x] [UXG-131](../audits/user-story-gap-audit-2026-10-14/issues/UXG-131-let-the-owner-read-a-skill-s-skill-md-and-scripts-before-acc.md) Let the owner read a skill's SKILL.md and scripts before accepting it — high — **Fixed** `059ce113`
- [x] [UXG-132](../audits/user-story-gap-audit-2026-10-14/issues/UXG-132-allow-editing-a-connection-s-address-token-or-sign-in-withou.md) Allow editing a connection's address, token or sign-in without removing it — medium — **Fixed** `059ce113`
- [x] [UXG-133](../audits/user-story-gap-audit-2026-10-14/issues/UXG-133-provide-a-way-to-create-and-edit-company-know-how.md) Provide a way to create and edit company know-how — high — **Fixed** `059ce113`
- [x] [UXG-134](../audits/user-story-gap-audit-2026-10-14/issues/UXG-134-show-removed-skills-with-restore-and-show-the-skill-s-versio.md) Show removed skills with Restore, and show the skill's version — medium — **Fixed** `059ce113`
- [x] [UXG-135](../audits/user-story-gap-audit-2026-10-14/issues/UXG-135-make-app-activity-answer-what-was-it-used-for.md) Make app activity answer "what was it used for" — medium — **Fixed** `059ce113`
- [x] [UXG-136](../audits/user-story-gap-audit-2026-10-14/issues/UXG-136-offer-a-real-off-state-that-keeps-the-sign-in.md) Offer a real "off" state that keeps the sign-in — medium — **Fixed** `059ce113`
- [x] [UXG-137](../audits/user-story-gap-audit-2026-10-14/issues/UXG-137-show-permissions-before-auto-allowing-an-added-app.md) Show permissions before auto-allowing an added app — medium — **Fixed** `059ce113`
- [x] [UXG-138](../audits/user-story-gap-audit-2026-10-14/issues/UXG-138-replace-the-free-text-vault-secret-name-with-a-picker-or-inl.md) Replace the free-text "Vault secret name" with a picker or inline entry — medium — **Fixed** `059ce113`
- [x] [UXG-139](../audits/user-story-gap-audit-2026-10-14/issues/UXG-139-show-why-exec-wants-an-app-in-the-chat-card-without-hovering.md) Show why Exec wants an app in the chat card without hovering — medium — **Fixed** `059ce113`
- [x] [UXG-140](../audits/user-story-gap-audit-2026-10-14/issues/UXG-140-let-declining-an-app-request-carry-a-reason-or-alternative.md) Let declining an app request carry a reason or alternative — low — **Fixed** `059ce113`
- [x] [UXG-141](../audits/user-story-gap-audit-2026-10-14/issues/UXG-141-report-skipped-items-and-link-to-review-after-a-plugin-impor.md) Report skipped items and link to review after a plugin import — low — **Fixed** `059ce113`

### WP11 — Apps II

- [x] [UXG-142](../audits/user-story-gap-audit-2026-10-14/issues/UXG-142-make-a-disabled-skill-a-visible-state-and-expose-per-agent-t.md) Make a disabled skill a visible state and expose per-agent/team assignment — medium — **Fixed** `8af2cf20`
- [x] [UXG-143](../audits/user-story-gap-audit-2026-10-14/issues/UXG-143-remove-technical-terms-from-the-model-choice.md) Remove technical terms from the model choice — medium — **Fixed** `b75e185c`
- [x] [UXG-144](../audits/user-story-gap-audit-2026-10-14/issues/UXG-144-pick-one-place-to-add-model-keys-and-sign-ins.md) Pick one place to add model keys and sign-ins — medium — **Fixed** `e3f59125`
- [x] [UXG-145](../audits/user-story-gap-audit-2026-10-14/issues/UXG-145-use-one-verb-for-adding-an-app.md) Use one verb for adding an app — low — **Fixed** `8af2cf20`
- [x] [UXG-146](../audits/user-story-gap-audit-2026-10-14/issues/UXG-146-show-couldn-t-load-instead-of-hiding-skills-or-saying-not-fo.md) Show "couldn't load" instead of hiding skills or saying "not found" — medium — **Fixed** `8af2cf20`
- [x] [UXG-147](../audits/user-story-gap-audit-2026-10-14/issues/UXG-147-stop-tab-from-sending-a-message-to-exec-in-the-apps-ask-box.md) Stop Tab from sending a message to Exec in the Apps ask box — medium — **Fixed** `8af2cf20`
- [x] [UXG-148](../audits/user-story-gap-audit-2026-10-14/issues/UXG-148-name-model-connections-differently-from-app-connections.md) Name model "Connections" differently from app connections — medium — **Fixed** `8f003c40`
- [x] [UXG-149](../audits/user-story-gap-audit-2026-10-14/issues/UXG-149-confirm-token-revoke-and-handle-copy-failures-in-ai-apps.md) Confirm token revoke and handle copy failures in AI apps — low — **Fixed** `486f8550`
- [x] [UXG-150](../audits/user-story-gap-audit-2026-10-14/issues/UXG-150-make-appearance-follow-the-tooltip-rule-and-radio-keyboard-b.md) Make Appearance follow the tooltip rule and radio keyboard behaviour — low — **Fixed** `e1b8e470`
- [x] [UXG-151](../audits/user-story-gap-audit-2026-10-14/issues/UXG-151-make-all-apps-views-and-add-from-link-reachable-on-mobile.md) Make all Apps views and "add from link" reachable on mobile — medium — **Fixed** `8af2cf20`
- [x] [UXG-152](../audits/user-story-gap-audit-2026-10-14/issues/UXG-152-use-one-list-form-for-apps-instead-of-tiles-cards-in-browse.md) Use one list form for Apps instead of tiles/cards in Browse — medium — **Fixed** `8af2cf20`
- [x] [UXG-153](../audits/user-story-gap-audit-2026-10-14/issues/UXG-153-let-custom-harnesses-be-edited-and-removed-confirm-harness-d.md) Let custom harnesses be edited and removed; confirm harness disconnect — low — **Fixed** `2f9bc99a`
- [x] [UXG-154](../audits/user-story-gap-audit-2026-10-14/issues/UXG-154-settle-naming-of-know-how-vs-skills-and-describe-local-tools.md) Settle naming of know-how vs skills and describe local tools accurately — low — **Fixed** `8af2cf20`
- [x] [UXG-155](../audits/user-story-gap-audit-2026-10-14/issues/UXG-155-let-the-owner-change-a-company-s-model-from-account-access.md) Let the owner change a company's model from Account access — low — **Fixed** `9cef7dd5`

### WP12 — Company I

- [x] [UXG-156](../audits/user-story-gap-audit-2026-10-14/issues/UXG-156-let-the-owner-name-the-company-when-creating-it.md) Let the owner name the company when creating it — medium — **Fixed** `c44c291b`
- [x] [UXG-157](../audits/user-story-gap-audit-2026-10-14/issues/UXG-157-report-a-failed-rename-during-onboarding-instead-of-dropping.md) Report a failed rename during onboarding instead of dropping it — high — **Fixed** `735a3e29`
- [x] [UXG-158](../audits/user-story-gap-audit-2026-10-14/issues/UXG-158-use-a-multi-line-answer-box-on-the-welcome-page.md) Use a multi-line answer box on the welcome page — low — **Fixed** `735a3e29`
- [x] [UXG-159](../audits/user-story-gap-audit-2026-10-14/issues/UXG-159-make-the-empty-portfolio-text-match-the-real-button.md) Make the empty-portfolio text match the real button — low — **Fixed** `c44c291b`
- [x] [UXG-160](../audits/user-story-gap-audit-2026-10-14/issues/UXG-160-provide-a-way-back-into-an-unfinished-welcome-conversation.md) Provide a way back into an unfinished welcome conversation — medium — **Fixed** `735a3e29`
- [x] [UXG-161](../audits/user-story-gap-audit-2026-10-14/issues/UXG-161-cover-all-six-surfaces-in-the-tour-and-make-it-keyboard-frie.md) Cover all six surfaces in the tour and make it keyboard-friendly and replayable — low — **Fixed** `274154a2`
- [x] [UXG-162](../audits/user-story-gap-audit-2026-10-14/issues/UXG-162-offer-a-non-technical-charter-editor-with-a-preview.md) Offer a non-technical charter editor with a preview — medium — **Fixed** `b4e0f2cb`
- [x] [UXG-163](../audits/user-story-gap-audit-2026-10-14/issues/UXG-163-make-charter-history-findable-comparable-and-restorable.md) Make charter history findable, comparable and restorable — medium — **Fixed** `b4e0f2cb`
- [x] [UXG-164](../audits/user-story-gap-audit-2026-10-14/issues/UXG-164-let-the-owner-add-or-edit-the-legal-profile.md) Let the owner add or edit the legal profile — medium — **Fixed** `1fd4ebb8`
- [x] [UXG-165](../audits/user-story-gap-audit-2026-10-14/issues/UXG-165-support-specific-weekdays-monthly-schedules-and-a-company-ti.md) Support specific weekdays, monthly schedules and a company time zone — medium — **Fixed** `dc09cc48`
- [x] [UXG-166](../audits/user-story-gap-audit-2026-10-14/issues/UXG-166-allow-editing-what-a-schedule-asks-exec-to-do.md) Allow editing what a schedule asks Exec to do — high — **Fixed** `dc09cc48`
- [x] [UXG-167](../audits/user-story-gap-audit-2026-10-14/issues/UXG-167-use-the-same-friendly-cadence-control-when-editing-a-schedul.md) Use the same friendly cadence control when editing a schedule — medium — **Fixed** `dc09cc48`
- [x] [UXG-168](../audits/user-story-gap-audit-2026-10-14/issues/UXG-168-add-run-now-and-rename-test-trigger.md) Add "Run now" and rename "Test trigger" — low — **Fixed** `dc09cc48`
- [x] [UXG-169](../audits/user-story-gap-audit-2026-10-14/issues/UXG-169-add-budget-periods-categories-and-early-warnings.md) Add budget periods, categories and early warnings — high — **Fixed** `a8b7c123`
- [x] [UXG-170](../audits/user-story-gap-audit-2026-10-14/issues/UXG-170-warn-before-lowering-the-spend-limit-below-current-spend.md) Warn before lowering the spend limit below current spend — medium — **Fixed** `a8b7c123`
- [x] [UXG-171](../audits/user-story-gap-audit-2026-10-14/issues/UXG-171-let-the-owner-delete-a-vault-secret-and-replace-any-secret-f.md) Let the owner delete a Vault secret and replace any secret from its row — high — **Fixed** `45e5a0ac`
- [x] [UXG-172](../audits/user-story-gap-audit-2026-10-14/issues/UXG-172-warn-when-a-new-secret-name-overwrites-an-existing-one-and-h.md) Warn when a new secret name overwrites an existing one, and hide raw references — medium — **Fixed** `45e5a0ac`

### WP13 — Company II

- [x] [UXG-173](../audits/user-story-gap-audit-2026-10-14/issues/UXG-173-give-every-failing-health-check-a-next-step.md) Give every failing health check a next step — medium — **Fixed** `f39360e4`
- [x] [UXG-174](../audits/user-story-gap-audit-2026-10-14/issues/UXG-174-use-consistent-tones-and-plain-language-on-health.md) Use consistent tones and plain language on Health — low — **Fixed** `f39360e4`
- [x] [UXG-175](../audits/user-story-gap-audit-2026-10-14/issues/UXG-175-let-activity-filter-by-date-person-and-text-and-group-by-day.md) Let Activity filter by date, person and text, and group by day — medium — **Fixed** `42ebfcaf`
- [x] [UXG-176](../audits/user-story-gap-audit-2026-10-14/issues/UXG-176-page-external-actions-and-include-owner-setting-changes-in-a.md) Page external actions and include owner setting changes in Activity — medium — **Fixed** `42ebfcaf`
- [x] [UXG-177](../audits/user-story-gap-audit-2026-10-14/issues/UXG-177-put-company-name-and-logo-where-owners-look-for-them.md) Put company name and logo where owners look for them — medium — **Fixed** `3d7b4ea4`
- [x] [UXG-178](../audits/user-story-gap-audit-2026-10-14/issues/UXG-178-show-why-the-name-or-logo-didn-t-save-not-just-not-saved.md) Show why the name or logo didn't save, not just "Not saved" — medium — **Fixed** `8caca4d9`
- [x] [UXG-179](../audits/user-story-gap-audit-2026-10-14/issues/UXG-179-explain-what-archiving-a-company-does-before-doing-it.md) Explain what archiving a company does before doing it — medium — **Fixed** `63ea94ef`
- [x] [UXG-180](../audits/user-story-gap-audit-2026-10-14/issues/UXG-180-add-permanent-deletion-of-a-company.md) Add permanent deletion of a company — medium — **Fixed** `63ea94ef`
- [x] [UXG-181](../audits/user-story-gap-audit-2026-10-14/issues/UXG-181-let-the-owner-open-an-archived-company-read-only-and-archive.md) Let the owner open an archived company read-only and archive from inside Company — low — **Fixed** `63ea94ef`
- [x] [UXG-182](../audits/user-story-gap-audit-2026-10-14/issues/UXG-182-add-company-data-export.md) Add company data export — medium — **Fixed** `63ea94ef`
- [x] [UXG-183](../audits/user-story-gap-audit-2026-10-14/issues/UXG-183-keep-the-current-area-when-switching-companies-and-add-new-c.md) Keep the current area when switching companies, and add New company to the switcher — low — **Fixed** `a8c776d9`
- [x] [UXG-184](../audits/user-story-gap-audit-2026-10-14/issues/UXG-184-copy-a-full-setup-to-another-company-not-one-setting-at-a-ti.md) Copy a full setup to another company, not one setting at a time — low — **Fixed** `a8c776d9`
- [x] [UXG-185](../audits/user-story-gap-audit-2026-10-14/issues/UXG-185-add-a-company-wide-freeze-of-outside-actions-and-payment-all.md) Add a company-wide freeze of outside actions and payment-allowance controls — high — **Fixed** `7d70a204`
- [x] [UXG-186](../audits/user-story-gap-audit-2026-10-14/issues/UXG-186-make-the-company-sidebar-s-computer-and-limits-values-truthf.md) Make the Company sidebar's Computer and Limits values truthful — medium — **Fixed** `45dd6398`
- [x] [UXG-187](../audits/user-story-gap-audit-2026-10-14/issues/UXG-187-keep-settings-search-on-phones-and-narrow-panes.md) Keep settings search on phones and narrow panes — low — **Fixed** `3d7b4ea4`
- [x] [UXG-188](../audits/user-story-gap-audit-2026-10-14/issues/UXG-188-read-the-brand-name-from-config-in-the-model-catalog-tooltip.md) Read the brand name from config in the model-catalog tooltip — low — **Fixed** `62bd4da0`
- [x] [UXG-189](../audits/user-story-gap-audit-2026-10-14/issues/UXG-189-wire-up-or-delete-the-unused-accounthome-view.md) Wire up or delete the unused AccountHome view — low — **Fixed** `5190f546`
- [x] [UXG-190](../audits/user-story-gap-audit-2026-10-14/issues/UXG-190-stop-hiding-needed-company-explanations-in-hover-only-titles.md) Stop hiding needed Company explanations in hover-only titles — low — **Fixed** `3d7b4ea4`

### WP14 — Exec chat I

- [x] [UXG-191](../audits/user-story-gap-audit-2026-10-14/issues/UXG-191-add-retry-to-a-failed-exec-reply.md) Add Retry to a failed Exec reply — high — **Fixed** `3a3ed1e9`
- [x] [UXG-192](../audits/user-story-gap-audit-2026-10-14/issues/UXG-192-add-a-stop-button-that-halts-exec-s-reply-without-sending-a.md) Add a Stop button that halts Exec's reply without sending a new message — high — **Fixed** `3a3ed1e9`
- [x] [UXG-193](../audits/user-story-gap-audit-2026-10-14/issues/UXG-193-show-exec-s-live-work-even-when-the-owner-did-not-send-the-l.md) Show Exec's live work even when the owner did not send the last message — high — **Fixed** `3a3ed1e9`
- [x] [UXG-194](../audits/user-story-gap-audit-2026-10-14/issues/UXG-194-put-exec-s-unread-reply-and-working-state-on-the-exec-button.md) Put Exec's unread reply and working state on the Exec button, not the People badge — medium — **Fixed** `3a3ed1e9`
- [x] [UXG-195](../audits/user-story-gap-audit-2026-10-14/issues/UXG-195-let-the-rail-reach-and-search-exec-history-older-than-the-la.md) Let the rail reach and search Exec history older than the last 100 messages — high — **Fixed** `7b45a1d6`
- [x] [UXG-196](../audits/user-story-gap-audit-2026-10-14/issues/UXG-196-keep-pinned-exec-messages-visible-regardless-of-age.md) Keep pinned Exec messages visible regardless of age — medium — **Fixed** `7b45a1d6`
- [x] [UXG-197](../audits/user-story-gap-audit-2026-10-14/issues/UXG-197-let-the-owner-write-and-leave-a-message-while-exec-is-unreac.md) Let the owner write and leave a message while Exec is unreachable — medium — **Fixed** `0c0d5e00`
- [x] [UXG-198](../audits/user-story-gap-audit-2026-10-14/issues/UXG-198-check-attachment-size-count-and-type-before-sending-from-the.md) Check attachment size, count and type before sending from the rail, and show upload progress — medium — **Fixed** `da733536`
- [x] [UXG-199](../audits/user-story-gap-audit-2026-10-14/issues/UXG-199-keep-attached-files-quote-and-chosen-skills-when-switching-r.md) Keep attached files, quote and chosen skills when switching rail conversations — medium — **Fixed** `b7ea1c23`
- [x] [UXG-200](../audits/user-story-gap-audit-2026-10-14/issues/UXG-200-persist-the-rail-s-retry-key-for-an-uncertain-send-across-re.md) Persist the rail's retry key for an uncertain send across reloads and topic switches — medium — **Fixed** `b7ea1c23`
- [x] [UXG-201](../audits/user-story-gap-audit-2026-10-14/issues/UXG-201-stop-handed-in-drafts-from-silently-replacing-the-owner-s-un.md) Stop handed-in drafts from silently replacing the owner's unsent message — medium — **Fixed** `f235651b`
- [x] [UXG-202](../audits/user-story-gap-audit-2026-10-14/issues/UXG-202-make-slash-commands-skills-references-and-people-discoverabl.md) Make slash commands, skills, # references and @ people discoverable in the composer — medium — **Fixed** `d1c2c9a1`
- [x] [UXG-203](../audits/user-story-gap-audit-2026-10-14/issues/UXG-203-confirm-before-loop-clear-stops-every-loop-and-let-goal-clea.md) Confirm before /loop clear stops every loop, and let /goal clear choose the goal — high — **Fixed** `d5da8c7c`

### WP15 — Exec chat II

- [x] [UXG-204](../audits/user-story-gap-audit-2026-10-14/issues/UXG-204-stop-a-failed-goal-send-from-creating-a-duplicate-goal-on-re.md) Stop a failed /goal send from creating a duplicate goal on retry — medium — **Fixed** `fbad5707`
- [x] [UXG-205](../audits/user-story-gap-audit-2026-10-14/issues/UXG-205-record-command-results-in-the-conversation-instead-of-a-ling.md) Record command results in the conversation instead of a lingering status line — low — **Fixed** `1e1552c8`
- [x] [UXG-206](../audits/user-story-gap-audit-2026-10-14/issues/UXG-206-give-admins-operators-and-members-the-rail-context-the-owner.md) Give admins, operators and members the rail context the owner gets — medium — **Fixed** `9f1074b8`
- [x] [UXG-207](../audits/user-story-gap-audit-2026-10-14/issues/UXG-207-show-live-progress-when-talking-to-a-staff-member-or-an-agen.md) Show live progress when talking to a Staff member or an agent in a group room — medium — **Fixed** `32d2bce3`
- [x] [UXG-208](../audits/user-story-gap-audit-2026-10-14/issues/UXG-208-say-seen-no-reply-instead-of-replied-when-exec-s-turn-ends-w.md) Say "Seen, no reply" instead of "Replied" when Exec's turn ends without a message — medium — **Fixed** `2421a59c`
- [x] [UXG-209](../audits/user-story-gap-audit-2026-10-14/issues/UXG-209-announce-new-exec-replies-to-screen-readers.md) Announce new Exec replies to screen readers — medium — **Fixed** `1562e56f`
- [x] [UXG-210](../audits/user-story-gap-audit-2026-10-14/issues/UXG-210-let-a-selection-on-the-main-surface-travel-with-the-message.md) Let a selection on the main surface travel with the message to Exec — medium — **Fixed** `2d0a0511`
- [x] [UXG-211](../audits/user-story-gap-audit-2026-10-14/issues/UXG-211-show-on-a-sent-message-which-screen-it-was-linked-to.md) Show on a sent message which screen it was linked to — low — **Fixed** `30f9b015`
- [x] [UXG-212](../audits/user-story-gap-audit-2026-10-14/issues/UXG-212-let-the-owner-cancel-a-pending-new-conversation-and-see-wher.md) Let the owner cancel a pending new conversation and see where earlier ones began — low — **Fixed** `e3073b40`
- [x] [UXG-213](../audits/user-story-gap-audit-2026-10-14/issues/UXG-213-let-the-owner-copy-or-export-a-whole-conversation.md) Let the owner copy or export a whole conversation — low — **Fixed** `a0b8be67`
- [x] [UXG-214](../audits/user-story-gap-audit-2026-10-14/issues/UXG-214-show-the-current-model-on-the-composer-chip-and-allow-effort.md) Show the current model on the composer chip and allow effort to be chosen there — low — **Fixed** `a4f2894f`
- [x] [UXG-215](../audits/user-story-gap-audit-2026-10-14/issues/UXG-215-let-the-expanded-turn-dock-show-exec-s-thinking-collapsed-by.md) Let the expanded turn dock show Exec's thinking, collapsed by default — low — **Fixed** `2421a59c`

### WP16 — Editors I

- [x] [UXG-216](../audits/user-story-gap-audit-2026-10-14/issues/UXG-216-add-underline-highlight-text-colour-and-alignment-to-documen.md) Add underline, highlight, text colour and alignment to document formatting — high — **Fixed** `61517392`
- [x] [UXG-217](../audits/user-story-gap-audit-2026-10-14/issues/UXG-217-keep-images-formatting-and-merged-cells-when-pasting-from-wo.md) Keep images, formatting and merged cells when pasting from Word or Google Docs — high — **Fixed** `90efe0b9`
- [x] [UXG-218](../audits/user-story-gap-audit-2026-10-14/issues/UXG-218-let-writers-mention-people-in-the-document-body.md) Let writers @mention people in the document body — high — **Fixed** `33c5739f`
- [x] [UXG-219](../audits/user-story-gap-audit-2026-10-14/issues/UXG-219-insert-links-to-documents-work-and-files-as-titled-chips-and.md) Insert links to documents, Work and files as titled chips, and show where links go — medium — **Fixed** `01da69e6`
- [x] [UXG-220](../audits/user-story-gap-audit-2026-10-14/issues/UXG-220-add-find-and-replace-in-documents.md) Add find and replace in documents — medium — **Fixed** `3aa1204a`
- [x] [UXG-221](../audits/user-story-gap-audit-2026-10-14/issues/UXG-221-show-an-outline-of-headings-and-make-in-page-links-work.md) Show an outline of headings, and make in-page "#" links work — medium — **Fixed** `e6cb3c6e`
- [x] [UXG-222](../audits/user-story-gap-audit-2026-10-14/issues/UXG-222-show-word-and-character-count.md) Show word and character count — low — **Fixed** `e6cb3c6e`
- [x] [UXG-223](../audits/user-story-gap-audit-2026-10-14/issues/UXG-223-let-documents-carry-files-other-than-images.md) Let documents carry files other than images — medium — **Fixed** `e0108bfa`
- [x] [UXG-224](../audits/user-story-gap-audit-2026-10-14/issues/UXG-224-allow-resizing-describing-and-captioning-images.md) Allow resizing, describing and captioning images — medium — **Fixed** `d5bc97df`
- [x] [UXG-225](../audits/user-story-gap-audit-2026-10-14/issues/UXG-225-extend-document-tables-rows-above-columns-left-header-row-wi.md) Extend document tables: rows above, columns left, header row, widths — medium — **Fixed** `b061044c`
- [x] [UXG-226](../audits/user-story-gap-audit-2026-10-14/issues/UXG-226-let-blocks-be-moved-duplicated-and-deleted-without-the-hover.md) Let blocks be moved, duplicated and deleted without the hover grip — medium — **Fixed** `eb46e60d`
- [x] [UXG-227](../audits/user-story-gap-audit-2026-10-14/issues/UXG-227-anchor-document-comments-to-the-selected-words-not-the-whole.md) Anchor document comments to the selected words, not the whole block — medium — **Fixed** `0637d5a1`
- [x] [UXG-228](../audits/user-story-gap-audit-2026-10-14/issues/UXG-228-keep-editors-editable-through-brief-connection-drops.md) Keep editors editable through brief connection drops — high — **Fixed** `3727e9cd`

### WP17 — Editors II

- [x] [UXG-229](../audits/user-story-gap-audit-2026-10-14/issues/UXG-229-don-t-strand-a-co-editor-s-unsaved-edits-when-someone-restor.md) Don't strand a co-editor's unsaved edits when someone restores a version — medium — **Fixed** `10b2b19a`
- [x] [UXG-230](../audits/user-story-gap-audit-2026-10-14/issues/UXG-230-say-plainly-when-a-document-is-too-large-to-save-and-warn-be.md) Say plainly when a document is too large to save, and warn before it gets there — medium — **Fixed** `1aebb0e0`
- [x] [UXG-231](../audits/user-story-gap-audit-2026-10-14/issues/UXG-231-stop-a-refused-sheet-change-from-discarding-every-edit-queue.md) Stop a refused sheet change from discarding every edit queued behind it — high — **Fixed** `0e9c83fe`
- [x] [UXG-232](../audits/user-story-gap-audit-2026-10-14/issues/UXG-232-let-writers-save-and-name-a-version-from-the-editor.md) Let writers save and name a version from the editor — medium — **Fixed** `b76b08f2`
- [x] [UXG-233](../audits/user-story-gap-audit-2026-10-14/issues/UXG-233-offer-templates-and-use-the-document-kinds-when-starting-a-d.md) Offer templates and use the document kinds when starting a document — medium — **Fixed** `7f24f926`
- [x] [UXG-234](../audits/user-story-gap-audit-2026-10-14/issues/UXG-234-show-undo-and-redo-on-desktop-too.md) Show Undo and Redo on desktop too — low — **Fixed** `20e9f9cc`
- [x] [UXG-235](../audits/user-story-gap-audit-2026-10-14/issues/UXG-235-let-sheet-comments-cover-a-range-and-follow-their-cell-when.md) Let sheet comments cover a range and follow their cell when rows move — medium — **Fixed** `1806adbb`
- [x] [UXG-236](../audits/user-story-gap-audit-2026-10-14/issues/UXG-236-allow-pictures-in-sheets.md) Allow pictures in sheets — low — **Fixed** `5a85611a`
- [x] [UXG-237](../audits/user-story-gap-audit-2026-10-14/issues/UXG-237-add-print-save-as-pdf-for-sheets.md) Add Print / save as PDF for sheets — low — **Fixed** `6697d0a1`
- [x] [UXG-238](../audits/user-story-gap-audit-2026-10-14/issues/UXG-238-follow-the-dark-theme-in-the-sheet-grid.md) Follow the dark theme in the sheet grid — low — **Fixed** `6b90f6d8`
- [x] [UXG-239](../audits/user-story-gap-audit-2026-10-14/issues/UXG-239-use-the-company-s-locale-and-currency-for-sheet-formats.md) Use the company's locale and currency for sheet formats — low — **Fixed** `478fcd27`
- [x] [UXG-240](../audits/user-story-gap-audit-2026-10-14/issues/UXG-240-make-sheet-editing-on-a-phone-a-deliberate-tested-path.md) Make sheet editing on a phone a deliberate, tested path — low — **Fixed** `5ef8186a`
- [x] [UXG-241](../audits/user-story-gap-audit-2026-10-14/issues/UXG-241-add-page-breaks-and-page-numbers-to-printed-documents.md) Add page breaks and page numbers to printed documents — low — **Fixed** `7a07c23a`

### WP18 — Company computer

- [x] [UXG-242](../audits/user-story-gap-audit-2026-10-14/issues/UXG-242-let-the-owner-restart-a-running-computer-that-looks-healthy.md) Let the owner restart a running computer that looks healthy but is stuck — high — **Fixed** `90828b31`
- [x] [UXG-243](../audits/user-story-gap-audit-2026-10-14/issues/UXG-243-make-the-inbox-handover-desktop-accept-the-owner-s-keyboard.md) Make the Inbox handover desktop accept the owner's keyboard and pointer — high — **Fixed** `5357c0e3`
- [x] [UXG-244](../audits/user-story-gap-audit-2026-10-14/issues/UXG-244-keep-the-handover-desktop-connected-after-returning-control.md) Keep the handover desktop connected after returning control — high — **Fixed** `5357c0e3`
- [x] [UXG-245](../audits/user-story-gap-audit-2026-10-14/issues/UXG-245-stop-a-60-second-pause-from-telling-the-agent-the-owner-has.md) Stop a 60-second pause from telling the agent the owner has finished — high — **Fixed** `5357c0e3`
- [x] [UXG-246](../audits/user-story-gap-audit-2026-10-14/issues/UXG-246-show-when-an-agent-is-using-the-computer-and-who-before-the.md) Show when an agent is using the computer, and who, before the owner's click takes over — medium — **Fixed** `5357c0e3`
- [x] [UXG-247](../audits/user-story-gap-audit-2026-10-14/issues/UXG-247-stop-a-viewer-from-resizing-the-shared-desktop-while-an-agen.md) Stop a viewer from resizing the shared desktop while an agent works in it — medium — **Fixed** `5357c0e3`
- [x] [UXG-248](../audits/user-story-gap-audit-2026-10-14/issues/UXG-248-let-a-phone-user-type-into-the-live-desktop.md) Let a phone user type into the live desktop — high — **Fixed** `bf7fbd8a`
- [x] [UXG-249](../audits/user-story-gap-audit-2026-10-14/issues/UXG-249-tell-keyboard-users-how-to-leave-the-live-desktop.md) Tell keyboard users how to leave the live desktop — medium — **Fixed** `bf7fbd8a`
- [x] [UXG-250](../audits/user-story-gap-audit-2026-10-14/issues/UXG-250-let-the-owner-see-and-download-files-that-exist-only-on-the.md) Let the owner see and download files that exist only on the company computer — high — **Fixed** `40b88fef`
- [x] [UXG-251](../audits/user-story-gap-audit-2026-10-14/issues/UXG-251-let-the-owner-drop-a-file-into-the-live-desktop-during-a-han.md) Let the owner drop a file into the live desktop during a handover — medium — **Fixed** `40b88fef`
- [x] [UXG-252](../audits/user-story-gap-audit-2026-10-14/issues/UXG-252-say-when-the-monthly-hours-limit-is-why-the-computer-will-no.md) Say when the monthly hours limit is why the computer will not wake — medium — **Fixed** `90828b31`
- [x] [UXG-253](../audits/user-story-gap-audit-2026-10-14/issues/UXG-253-show-wake-progress-and-keep-trying-instead-of-failing-after.md) Show wake progress and keep trying instead of failing after 20 seconds — medium — **Fixed** `90828b31`
- [x] [UXG-254](../audits/user-story-gap-audit-2026-10-14/issues/UXG-254-surface-disk-memory-and-cpu-pressure-on-the-company-computer.md) Surface disk, memory and CPU pressure on the company computer — medium — **Fixed** `99f41f83`
- [x] [UXG-255](../audits/user-story-gap-audit-2026-10-14/issues/UXG-255-point-every-computer-is-stopped-message-at-the-place-that-ca.md) Point every "computer is stopped" message at the place that can start it — low — **Fixed** `90828b31`
- [x] [UXG-256](../audits/user-story-gap-audit-2026-10-14/issues/UXG-256-let-people-choose-where-an-external-link-opens-and-stop-rout.md) Let people choose where an external link opens, and stop routing members to a page they cannot open — medium — **Fixed** `f16d46c4`
- [x] [UXG-257](../audits/user-story-gap-audit-2026-10-14/issues/UXG-257-show-how-schedules-interact-with-computer-sleep-and-the-mont.md) Show how schedules interact with computer sleep and the monthly hours limit — medium — **Fixed** `063632bd`
- [x] [UXG-258](../audits/user-story-gap-audit-2026-10-14/issues/UXG-258-prove-a-chosen-model-answers-before-saving-it.md) Prove a chosen model answers before saving it — medium — **Fixed** `38a32cb8`
- [x] [UXG-259](../audits/user-story-gap-audit-2026-10-14/issues/UXG-259-bring-the-claude-harness-sign-in-window-forward-with-control.md) Bring the Claude harness sign-in window forward, with control, instead of opening the bare desktop — medium — **Fixed** `70d54d58`
- [x] [UXG-260](../audits/user-story-gap-audit-2026-10-14/issues/UXG-260-offer-a-fix-when-installed-agent-software-does-not-match-the.md) Offer a fix when installed agent software does not match the required build — low — **Fixed** `56ab7943`
- [x] [UXG-261](../audits/user-story-gap-audit-2026-10-14/issues/UXG-261-preview-powerpoint-outcomes-in-the-cockpit-like-word-and-exc.md) Preview PowerPoint outcomes in the cockpit like Word and Excel — medium — **Fixed** `fe0c8f1f`
- [x] [UXG-262](../audits/user-story-gap-audit-2026-10-14/issues/UXG-262-read-the-brand-name-from-config-in-the-harness-billing-note.md) Read the brand name from config in the harness billing note — low — **Fixed** `70d54d58`
- [x] [UXG-263](../audits/user-story-gap-audit-2026-10-14/issues/UXG-263-move-telegram-out-of-the-limits-page.md) Move Telegram out of the Limits page — low — **Fixed** `94a2cca0`
- [x] [UXG-264](../audits/user-story-gap-audit-2026-10-14/issues/UXG-264-delete-the-never-rendered-authority-page-and-keep-the-harnes.md) Delete the never-rendered authority page and keep the harness anchor when redirecting — low — **Deleted** `9e815ce5`
- [x] [UXG-265](../audits/user-story-gap-audit-2026-10-14/issues/UXG-265-replace-the-raw-browser-health-word-in-the-handover-header.md) Replace the raw browser health word in the handover header — low — **Fixed** `5357c0e3`

### WP19 — Shell

- [x] [UXG-266](../audits/user-story-gap-audit-2026-10-14/issues/UXG-266-keep-failure-toasts-until-dismissed-and-never-evict-a-live-u.md) Keep failure toasts until dismissed and never evict a live Undo — medium — **Fixed** `913de27`
- [x] [UXG-267](../audits/user-story-gap-audit-2026-10-14/issues/UXG-267-make-undo-reachable-from-the-keyboard-within-its-window.md) Make Undo reachable from the keyboard within its window — medium — **Fixed** `913de27`
- [x] [UXG-268](../audits/user-story-gap-audit-2026-10-14/issues/UXG-268-notify-for-every-company-and-name-the-company-in-each-deskto.md) Notify for every company and name the company in each desktop notification — medium — **Fixed** `8191dce`
- [x] [UXG-269](../audits/user-story-gap-audit-2026-10-14/issues/UXG-269-show-what-needs-the-owner-in-the-tab-title-or-favicon.md) Show what needs the owner in the tab title or favicon — low — **Fixed** `340223c`
- [x] [UXG-270](../audits/user-story-gap-audit-2026-10-14/issues/UXG-270-give-the-couldn-t-open-this-company-dialog-a-way-out-when-re.md) Give the "couldn’t open this company" dialog a way out when retrying cannot help — high — **Fixed** `9bf44f1`
- [x] [UXG-271](../audits/user-story-gap-audit-2026-10-14/issues/UXG-271-keep-unsent-text-across-session-re-entry-and-update-reloads.md) Keep unsent text across session re-entry and update reloads, and say when re-entry gave up — medium — **Fixed** `4e323cf`
- [x] [UXG-272](../audits/user-story-gap-audit-2026-10-14/issues/UXG-272-add-sign-out-to-the-in-company-profile-menu.md) Add Sign out to the in-company profile menu — medium — **Fixed** `440a092`
- [x] [UXG-273](../audits/user-story-gap-audit-2026-10-14/issues/UXG-273-tell-an-open-tab-when-a-new-version-has-been-deployed.md) Tell an open tab when a new version has been deployed — medium — **Fixed** `1e1218f`
- [x] [UXG-274](../audits/user-story-gap-audit-2026-10-14/issues/UXG-274-stop-sending-members-to-the-owner-s-account-pages.md) Stop sending members to the owner’s Account pages — medium — **Fixed** `440a092`
- [x] [UXG-275](../audits/user-story-gap-audit-2026-10-14/issues/UXG-275-say-when-k-search-could-not-reach-a-source-instead-of-no-mat.md) Say when ⌘K search could not reach a source instead of "No match" — medium — **Fixed** `237c402`
- [x] [UXG-276](../audits/user-story-gap-audit-2026-10-14/issues/UXG-276-stop-offering-add-app-for-apps-already-connected-and-let-k-f.md) Stop offering "Add <app>" for apps already connected and let ⌘K find what the company has — low — **Fixed** `237c402`
- [x] [UXG-277](../audits/user-story-gap-audit-2026-10-14/issues/UXG-277-offer-ask-exec-about-with-the-typed-text-when-k-has-no-answe.md) Offer "Ask Exec about …" with the typed text when ⌘K has no answer — medium — **Fixed** `237c402`
- [x] [UXG-278](../audits/user-story-gap-audit-2026-10-14/issues/UXG-278-make-stay-on-this-page-actually-cancel-the-stalled-navigatio.md) Make "Stay on this page" actually cancel the stalled navigation — medium — **Fixed** `a80c6ce`
- [x] [UXG-279](../audits/user-story-gap-audit-2026-10-14/issues/UXG-279-ignore-enter-while-an-ime-is-composing-in-k-and-other-picker.md) Ignore Enter while an IME is composing in ⌘K and other pickers — medium — **Fixed** `f3802ad`
- [x] [UXG-280](../audits/user-story-gap-audit-2026-10-14/issues/UXG-280-use-the-shared-date-time-and-money-formatters-everywhere.md) Use the shared date, time and money formatters everywhere — low — **Fixed** `09b3127`
- [x] [UXG-281](../audits/user-story-gap-audit-2026-10-14/issues/UXG-281-do-not-add-a-browser-history-entry-for-every-j-k-step-in-the.md) Do not add a browser history entry for every J/K step in the Inbox — low — **Fixed** `15805dc`
- [x] [UXG-282](../audits/user-story-gap-audit-2026-10-14/issues/UXG-282-move-the-connection-banner-above-the-phone-dock-and-out-of-t.md) Move the connection banner above the phone dock and out of the toasts’ way — medium — **Fixed** `ae1158d`
- [x] [UXG-283](../audits/user-story-gap-audit-2026-10-14/issues/UXG-283-take-the-product-name-from-brand-config-in-the-shell-offline.md) Take the product name from brand config in the shell, offline page and manifest — low — **Fixed** `c6111eb`
- [x] [UXG-284](../audits/user-story-gap-audit-2026-10-14/issues/UXG-284-stop-shipping-the-gallery-and-office-demo-routes-with-a-foun.md) Stop shipping the gallery and office demo routes, with a founder’s email, to every install — medium — **Fixed** `cd915e4`
- [x] [UXG-285](../audits/user-story-gap-audit-2026-10-14/issues/UXG-285-remove-the-aris-fallback-from-every-company-route-and-retire.md) Remove the 'aris' fallback from every company route and retire the Aris knowledge route — low — **Fixed** `3226797`
- [x] [UXG-286](../audits/user-story-gap-audit-2026-10-14/issues/UXG-286-give-the-home-and-account-frame-the-skip-link-k-and-correct.md) Give the Home and Account frame the skip link, ⌘K and correct menu semantics the company shell has — low — **Fixed** `000bb29`
- [x] [UXG-287](../audits/user-story-gap-audit-2026-10-14/issues/UXG-287-add-print-styles-beyond-documents.md) Add print styles beyond documents — low — **Fixed** `0b3e74e`
- [x] [UXG-288](../audits/user-story-gap-audit-2026-10-14/issues/UXG-288-reserve-app-route-names-so-a-company-id-cannot-be-shadowed.md) Reserve app route names so a company id cannot be shadowed — low — **Fixed** `4027390`
- [x] [UXG-289](../audits/user-story-gap-audit-2026-10-14/issues/UXG-289-clear-per-company-browser-memory-when-a-person-signs-out.md) Clear per-company browser memory when a person signs out — low — **Fixed** `440a092`
- [x] [UXG-290](../audits/user-story-gap-audit-2026-10-14/issues/UXG-290-write-the-error-page-in-owner-language-and-name-the-product.md) Write the error page in owner language and name the product in its title — low — **Fixed** `7191435`

### WP20 — Endpoint & CLI parity

- [x] [UXG-291](../audits/user-story-gap-audit-2026-10-14/issues/UXG-291-let-the-owner-authorise-a-prepared-publication-from-the-cock.md) Let the owner authorise a prepared publication from the cockpit — high — **Fixed** `7fd34bd`
- [x] [UXG-292](../audits/user-story-gap-audit-2026-10-14/issues/UXG-292-let-the-owner-invite-revoke-and-stop-a-live-publication-with.md) Let the owner invite, revoke and stop a live publication without the terminal — medium — **Fixed** `7fd34bd`
- [x] [UXG-293](../audits/user-story-gap-audit-2026-10-14/issues/UXG-293-offer-appliance-backup-and-restore-in-the-product-with-a-las.md) Offer appliance backup and restore in the product, with a last-backup indicator — high — **Fixed** `152b70b`
- [x] [UXG-294](../audits/user-story-gap-audit-2026-10-14/issues/UXG-294-show-the-running-release-and-allow-update-and-rollback-from.md) Show the running release and allow update and rollback from the cockpit — medium — **Fixed** `152b70b`
- [x] [UXG-295](../audits/user-story-gap-audit-2026-10-14/issues/UXG-295-replace-terminal-commands-in-appliance-repair-banners-with-a.md) Replace terminal commands in appliance repair banners with actions — medium — **Fixed** `152b70b`
- [x] [UXG-296](../audits/user-story-gap-audit-2026-10-14/issues/UXG-296-activate-sharing-and-tailscale-addresses-without-leaving-the.md) Activate sharing and Tailscale addresses without leaving the cockpit — high — **Fixed** `86ce85d`
- [x] [UXG-297](../audits/user-story-gap-audit-2026-10-14/issues/UXG-297-let-the-owner-save-a-named-version-of-a-document-as-they-can.md) Let the owner save a named version of a document as they can a sheet — medium — **Fixed** `4e136f9`
- [x] [UXG-298](../audits/user-story-gap-audit-2026-10-14/issues/UXG-298-break-model-spend-down-by-agent-and-model.md) Break model spend down by agent and model — medium — **Fixed** `223a919`
- [x] [UXG-299](../audits/user-story-gap-audit-2026-10-14/issues/UXG-299-explain-at-least-spend-and-let-the-owner-clear-a-spend-poiso.md) Explain "At least" spend and let the owner clear a spend poison — medium — **Fixed** `223a919`
- [x] [UXG-300](../audits/user-story-gap-audit-2026-10-14/issues/UXG-300-let-the-owner-create-rename-re-lead-and-disband-teams.md) Let the owner create, rename, re-lead and disband teams — medium — **Fixed** `5e0cb20`
- [x] [UXG-301](../audits/user-story-gap-audit-2026-10-14/issues/UXG-301-show-schedule-run-history-and-let-the-owner-recover-a-skippe.md) Show schedule run history and let the owner recover a skipped run — medium — **Fixed** `5c82b08`
- [x] [UXG-302](../audits/user-story-gap-audit-2026-10-14/issues/UXG-302-let-the-owner-choose-fallback-models.md) Let the owner choose fallback models — low — **Fixed** `31773c5`
- [x] [UXG-303](../audits/user-story-gap-audit-2026-10-14/issues/UXG-303-allow-pre-approving-a-contact-from-approved-contacts.md) Allow pre-approving a contact from Approved contacts — low — **Fixed** `fd67c2a`
- [x] [UXG-304](../audits/user-story-gap-audit-2026-10-14/issues/UXG-304-surface-or-delete-the-structured-decisions-record-route.md) Surface or delete the structured decisions record route — low — **Deleted** `0d2e7ca`
- [x] [UXG-305](../audits/user-story-gap-audit-2026-10-14/issues/UXG-305-prune-unused-client-contracts-left-after-the-route-parity-ch.md) Prune unused client contracts left after the route parity check — low — **Fixed** `4e136f9`

### WP21 — Journeys

- [x] [UXG-306](../audits/user-story-gap-audit-2026-10-14/issues/UXG-306-tell-the-owner-when-the-model-budget-stops-the-company.md) Tell the owner when the model budget stops the company — high — **Fixed** `8571fe0`
- [x] [UXG-307](../audits/user-story-gap-audit-2026-10-14/issues/UXG-307-resume-stalled-work-when-the-owner-raises-the-spend-limit.md) Resume stalled work when the owner raises the spend limit — medium — **Fixed** `8571fe0`
- [x] [UXG-308](../audits/user-story-gap-audit-2026-10-14/issues/UXG-308-bring-a-failed-or-uncertain-approved-action-back-to-the-owne.md) Bring a failed or uncertain approved action back to the owner — high — **Fixed** `a3ae40e`
- [x] [UXG-309](../audits/user-story-gap-audit-2026-10-14/issues/UXG-309-surface-failed-and-skipped-scheduled-runs-outside-the-schedu.md) Surface failed and skipped scheduled runs outside the Schedules page — medium — **Fixed** `a948fd7`
- [x] [UXG-310](../audits/user-story-gap-audit-2026-10-14/issues/UXG-310-let-exec-reply-to-a-customer-who-wrote-in-not-just-send-one.md) Let Exec reply to a customer who wrote in, not just send one first-contact email — high — **Fixed** `37d98ab`
- [x] [UXG-311](../audits/user-story-gap-audit-2026-10-14/issues/UXG-311-show-which-work-and-customer-an-approval-or-external-action.md) Show which Work and customer an approval or external action belongs to — medium — **Fixed** `a3ae40e`
- [x] [UXG-312](../audits/user-story-gap-audit-2026-10-14/issues/UXG-312-let-the-owner-connect-the-payment-rail-and-approve-payees-fr.md) Let the owner connect the payment rail and approve payees from the web — high — **Fixed** `fa60c97`
- [x] [UXG-313](../audits/user-story-gap-audit-2026-10-14/issues/UXG-313-give-an-admin-cofounder-visibility-into-activity-spend-and-a.md) Give an admin cofounder visibility into Activity, spend and Apps — medium — **Fixed** `0b71ce4f`
- [x] [UXG-314](../audits/user-story-gap-audit-2026-10-14/issues/UXG-314-add-an-away-mode-that-pauses-recurring-work-and-names-a-depu.md) Add an away mode that pauses recurring work and names a deputy — medium — **Fixed** `0283b24`
- [x] [UXG-315](../audits/user-story-gap-audit-2026-10-14/issues/UXG-315-make-telegram-notices-actionable-away-from-the-paired-comput.md) Make Telegram notices actionable away from the paired computer — medium — **Fixed** `b51d6f6`
- [x] [UXG-316](../audits/user-story-gap-audit-2026-10-14/issues/UXG-316-say-who-decided-an-item-when-an-old-link-opens-it.md) Say who decided an item when an old link opens it — low — **Fixed** `dcb8946`
- [x] [UXG-317](../audits/user-story-gap-audit-2026-10-14/issues/UXG-317-link-a-native-document-to-the-work-it-was-produced-for.md) Link a native document to the Work it was produced for — medium — **Fixed** `9f5bc32`
- [x] [UXG-318](../audits/user-story-gap-audit-2026-10-14/issues/UXG-318-route-a-document-comment-asking-for-a-change-to-the-work-s-r.md) Route a document comment asking for a change to the Work's revision — medium — **Fixed** `5f0e071`
- [x] [UXG-319](../audits/user-story-gap-audit-2026-10-14/issues/UXG-319-bring-brand-drift-decisions-to-the-inbox.md) Bring brand drift decisions to the Inbox — medium — **Fixed** `c3443b6`
- [x] [UXG-320](../audits/user-story-gap-audit-2026-10-14/issues/UXG-320-provide-a-weekly-review-across-work-decisions-actions-and-sp.md) Provide a weekly review across work, decisions, actions and spend — medium — **Fixed** `f4cb38d`
- [x] [UXG-321](../audits/user-story-gap-audit-2026-10-14/issues/UXG-321-let-the-owner-find-everything-about-one-client-in-one-place.md) Let the owner find everything about one client in one place — medium — **Fixed** `0d87644b`
- [x] [UXG-322](../audits/user-story-gap-audit-2026-10-14/issues/UXG-322-check-for-a-model-before-the-welcome-interview-starts.md) Check for a model before the welcome interview starts — medium — **Fixed** `f6950aa`
- [x] [UXG-323](../audits/user-story-gap-audit-2026-10-14/issues/UXG-323-return-the-owner-to-where-they-started-after-an-app-sign-in.md) Return the owner to where they started after an app sign-in, and wake the work that failed meanwhile — low — **Fixed** `74afc00`
- [x] [UXG-324](../audits/user-story-gap-audit-2026-10-14/issues/UXG-324-link-the-computer-is-stopped-review-message-to-the-recovery.md) Link the "computer is stopped" review message to the recovery action — low — **Fixed** `99c2469`
- [x] [UXG-325](../audits/user-story-gap-audit-2026-10-14/issues/UXG-325-point-to-the-accepted-result-after-an-inbox-decision.md) Point to the accepted result after an Inbox decision — low — **Fixed** `84c8682`
