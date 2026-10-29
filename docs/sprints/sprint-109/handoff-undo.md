# Durable handoff Undo, 29 October

This is the handoff slice of C1. Mark done, handoff Skip and outcome Accept now save a bounded
human decision in the company's OrgIntel before advancing the Inbox. The original five-second
window belongs to PostgreSQL, so leaving the page does not discard the decision. The existing
handoff/review transactions record the effect and applied receipt together; retries do not create
another completion, review feedback or deadline. This is not a generic request replay service.

The captured principal contains the verified actor and membership epoch or plane owner account,
never a cookie, token or secret. Apply rechecks the exact displayed handoff, Work revision and
candidate reference, current membership/role and assignment. Held payments retain their existing
provider-observed boundary. Undo can only cancel that person's pending intent before its original
deadline. A failed commit leaves the request available with a durable, unacknowledged reason.

The controller uses body-free company change hints. It checks a pending receipt promptly while
ordinary queries retain their slower live-stream repair policy. The browser clock conservatively
subtracts request time from the source deadline; focus, hover, polling and replay cannot restart it.
A hidden phone detail footer no longer pushes the notice outside the viewport.

## Verification

- Six real PostgreSQL scenarios: original deadline/reopen, two workers and one effect, cancellation,
  idempotent IDs and competing decisions, owner-account receipt separation, stale candidate, assigned member/admin revocation and
  role changes, real company-owner actor aliases and demotion, and body-free committed change hints.
  `/tmp/s109-undo-test-final-scope.log`: six passed, none skipped.
- Existing escalation, outcome review and people-own-teams scenarios passed against real PostgreSQL:
  six, two and one respectively. `/tmp/s109-undo-regress-current.log`.
- Real daemon browser journeys at 1440 and 390 cover immediate advance, no early effect, Undo,
  tab close/reconnect with the original deadline, hover expiry, a failed initial network save,
  and a changed source before commit whose refusal survives reload until explicitly acknowledged.
  Initial test runs exposed a three-minute receipt polling gap and hidden-footer notice positioning;
  both were corrected. The final eight scenarios passed at `/tmp/s109-undo-current-browser-2/report.json`, including the canceled item returning to the live Inbox; measured save/advance was 141 ms desktop and 130 ms phone.
- Operational restart: queued a real handoff, stopped the official isolated dev stack before its
  deadline, observed both decision and handoff still pending after the deadline, rebuilt/restarted
  the same profile, and observed one applied receipt and one completion feedback record. PostgreSQL
  compared the deadline at its full stored precision. `/tmp/s109-undo-restart-paused.log` and
  `/tmp/s109-undo-restart-verified.log`.
- All 12 real Inbox preview journeys passed with deferred outcome acceptance. The current displayed
  review-version check also passed: missing/stale versions refused, displayed preview preserved in
  two clients, explicit new-version review accepted, and old-tab replay refused.
  `/tmp/s109-undo-previews.log`, `/tmp/s109-undo-review-versions/report.json`.
- Authenticated owner/member/admin multiplayer journey passed with schema 111 and the new daemon.
  Existing live revocation, private documents, sharing, direction, Vault refusals and owner entry
  passed. Its first preflight found the test hostname absent from this WSL host; the passing run
  supplied it in a private mount namespace, preserving the host's actual `/etc/hosts`.
  `/tmp/s109-undo-roles-final/journey.log`; runner confirmed its resources removed.

- Four desktop/phone captures passed clipping, sideways-scroll and raw-identifier probes.
  Chat motion passed open-at-latest and streaming: 284 samples, width 770 to 770.
  `/tmp/s109-undo-captures/report.json`, `/tmp/s109-undo-motion.log`.
- Frontend diagnostics/type checks and the production build passed. Final visual comparison used
  Beautiful UI and Origin UI Svelte notifications (short notice plus a separate action); Cult UI
  was unavailable on this pass. No second UI runtime or notification package was added.

## Remaining C1 scope

Authority approvals, native-document Accept/complete and generic set-aside still use their existing
paths. They need their own source-owned reversible admission boundaries before C1 can be checked.
Candidate byte identity/reachability and repair remain A4 work. This slice does not establish the
entire sprint or private Reno production-browser completion.
