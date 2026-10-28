# Displayed review version — 29 October 2026

The owner can keep reading a review while a newer explanation or recorded candidate arrives.
The current card and interactive state remain in place; “Review new version” opens the replacement
before acceptance. The owner API refuses a missing or stale displayed version without deciding.
Work's full brief now links to that prepared review rather than offering a second Accept control.

Observed on the isolated `sprint109v` development profile and `sprint109_test` company:

- Real PostgreSQL `restless-orgintel` escalation suite: all six scenarios passed, including
  refreshed explanations, changed Work revisions, replaced references, candidate publication
  waiting for an ongoing decision's Work lock, wrong reviewers and two concurrent acceptances.
- `verify-review-versions.mjs`: two clients at 1440 and 390 pixels retained their original heading
  and interactive counter after an actual brief refresh. Missing/stale versions returned 409;
  opening the new review reset the counter and permitted one acceptance; replay returned 400.
  The primary button's position and dimensions changed by no more than one pixel. Work's full
  brief exposed Review in Inbox and no blind Accept button.
- Authenticated multiplayer smoke passed owner, admin and member paths, including Vault refusals,
  Inbox history isolation, role changes and live membership revocation. Its processes were stopped
  by the runner. Evidence: `/tmp/s109-review-roles/journey.log`.
- Four Inbox/board captures at 1440/390 had no page errors, overflow, clipping or raw internal
  identifiers. Motion checks opened long chats at their latest message; streaming and final reply
  width both measured 770px over 294 samples.
- Frontend diagnostics and the production build passed. `git diff --check` passed.
- All twelve `verify-inbox-previews.mjs` journeys passed across desktop and phone: accept,
  request changes, real file/settings/document/sheet previews, native review recovery, the one
  Exec conversation with linked page context, and selecting a real mention independently.
  Evidence: `/tmp/s109-version-preview-cases/report.json`.

Visual calibration used Beautiful UI's restrained approval controls, Cult UI's continuity between
states and Origin UI Svelte's compact ordinary controls. No upstream source or runtime was imported.

The preview verifier now follows the agreed one-Exec-chat path and current budget form labels. Its
refresh check requests an ordinary cross-window notification and observes the resulting real
Attention response rather than depending on a background timer. The mention fixture now contains a
real mention node; it no longer fabricates a projection absent from the document content.

This is partial A4 progress. Same-path mutable bytes, reachability/rendering at delivery, repair of
old missing previews, and the full native-document review audit remain required. C1's durable Undo
is also open. These results do not establish completion of A4 or the full sprint.
