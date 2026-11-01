# Native Document decisions keep their Undo window

Sprint 109 C1, continuing the handoff slice. Native review Accept and saved collaboration Done
now record an exact pending decision before the Inbox advances. The company database owns the
five-second deadline; the browser only presents it. A pending receipt hides that exact Inbox item.
The existing document transactions perform the eventual changes.

A review records the displayed document and review versions, assigned reviewer, current named
version and any Work review dependency. It rechecks the original source and verified human
membership when applying. The Work producer must already be blocked before accepting its review
can be queued. A changed candidate, assignment or membership refuses the pending effect.

The accepted named version, acceptance event, review note and applied receipt commit together.
A Work-bound note resumes its Work through the existing review transaction. An ordinary review
posts its note using the native comment transaction and mentions the original requester only
when acceptance commits. Undo publishes neither note nor mention and resumes no Work.

The person's last uncommitted name and note remain readable through a document-scoped, private
source read after Undo, failure or reload, including after the notice is acknowledged. Another
account or person cannot recover that input. There is no browser credential storage or queued
HTTP replay.

Two tabs with different request keys can converge on one pending decision. Migration 113 retains
each equivalent request key against its canonical receipt in source-specific tables, including
existing handoff and document receipts. Replaying either tab after Undo or completion returns
that original result and deadline. It cannot start a new action. Migration 112 creates the native
pending decisions; neither migration rewrites review evidence.

The shared code is limited to verified human identity checks and notice rendering. Handoff and
Native Document decisions keep separate source admission, receipts and transaction ownership.
The old frontend direct-accept and immediate-collaboration helpers were removed. The comment
transaction is reused instead of maintaining a second acceptance-note implementation.

Durable failures have distinct notice identities even when their wording matches. When three
are visible, further durable failures wait for a visible slot; dismissal acknowledges exactly
one source receipt. A live Undo is never evicted to make room.

## Verification

- Real PostgreSQL: eight native and six handoff scenarios pass. They cover original deadlines,
  reopen, concurrent workers, Undo/replays, equivalent two-tab request keys after cancellation
  and completion, candidate changes, assignment/membership changes, Work readiness/resumption,
  transaction rollback, private draft recovery and the requester's committed mention.
  Evidence: `/tmp/s109-native-tests-alias.log`.
- Actual daemon restart: the stack was stopped while the decision was pending, left stopped
  beyond its deadline, upgraded through migration 113 and restarted. The original timestamp
  survived at database precision; one acceptance, one note and one requester mention committed.
  Evidence: `/tmp/s109-native-restart-paused.log`, `/tmp/s109-native-restart-final.log`.
- The authenticated owner/member/admin journey passes, including the assigned member's queue,
  Undo and private draft. The owner cannot decide or cancel that member's review or read its
  saved note. Evidence: `/tmp/s109-native-roles-2/journey.log`. The first run exposed an
  intermittent Room projection omission before reaching these checks; its source/read/screen
  evidence remains at `/tmp/s109-native-roles/room-missing.json`. No Room fix is claimed here.

- Thirteen real browser/source scenarios pass at 1440 and 390 pixels, including save refusal,
  loss of the reply after a successful save, recovered Undo and stable retry, private form
  restoration, reconnect/hover expiry, changed-source refusal, collaboration completion and
  four separately acknowledged failures with identical wording. Cold reload also checks that
  the notice stays eight pixels above the final decision bar. Evidence:
  `/tmp/s109-native-browser-verified/report.json`; measured advance was 160/153 ms.
- Final frontend checks: zero errors/warnings; production build passes. Inbox/Library document
  captures at both widths and open-latest/stream motion pass (277 samples, 770 to 770 pixels).
  Evidence: `/tmp/s109-native-web-check-final-5.log`, `/tmp/s109-native-web-build-final-5.log`,
  `/tmp/s109-native-captures-verified/report.json`, `/tmp/s109-native-motion.log`.
- The stacked Inbox review reserves a smaller fixed preview/form split and omits the requester,
  summary and version metadata already shown above it. Loading/error content fills the same
  pane, keeping its decision bar and notices in place. The Library inspector retains its full
  review metadata. The built-in browser was inspected at desktop and phone widths.

## Remaining scope

C1 stays open for Authority approvals, generic set-aside and the audit of next-item prefetch.
A4 still requires admission/repair of missing or stale prepared reviews and mutable-file checks.
No full-sprint completion or signed production cutover follows from the checks above.

Visual calibration: Beautiful UI's compact actions, Cult UI's restrained state continuity and
Origin UI Svelte's inline Undo notifications. References consulted: https://www.beautifului.dev/,
https://www.cult-ui.com/docs and https://originui-svelte.pages.dev/notifications. No upstream code,
new visual runtime or component package was imported.
