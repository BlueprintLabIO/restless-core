# Sprint 17 paired owner-review proof

**Observed:** 25 August 2026
**Result:** Pass under controlled external input. Real Resend transport validation remains the
separately deferred provider gate.

## Product outcome

The owner review now places the exact bounded source evidence, its observed verification state, the
Staff-produced candidate, material uncertainty and the accountable lead's exact decision together.
The existing lead rail remains the decision and discussion surface; the main canvas does not duplicate
its accept/request-changes controls.

- Company and database: `s17_signal_test` in isolated `restless_s17_product_test`.
- Work: `0031ce36-51e4-4e3e-a5c3-912d32fd38f2`.
- Owner handoff: `8cc738d5-c454-48fe-93b0-ed02e8fab5b1`.
- Source: `authority://controlled-review/1`, visibly labelled **Controlled test input** rather than
  provider-authenticated.
- ReviewTarget: `/company/outputs/s17-owner-review-proof.md`, bounded to UTF-8 Markdown/text under
  `/company` and materialised as the company user.
- Candidate SHA-256: `0251d914dcc4e250a2ac27bb35c601c8fa56920c2ee6fd699fb1003a2807aa22`,
  matching the previously accepted real-model Staff artifact.
- External effects: zero; the candidate remained explicitly `UNSENT`.

The ignored product probe
`live_text_review_pairs_external_source_candidate_and_owner_decision` called the actual Attention
projection and asserted the linked source, exact ReviewTarget, accountable lead and owner decision.
It did not fabricate a live-provider claim.

## Native inspection

An isolated current-code owner server, with no scheduler or model gateway, served the exact preserved
projection for visual inspection. At a 1280×720 desktop viewport the result was a stable three-part
surface: source, prepared outcome, and accountable-lead rail. At a 390×844 mobile viewport source and
candidate stacked in that order; the actor button reopened the lead and decision controls. There were
no browser warnings or errors.

The small-screen pass exposed and fixed one real navigation fault: dismissing the full-screen lead
rail also discarded the review URL, making the paired canvas unreachable. Closing the mobile overlay
now preserves the exact review context underneath.

The final pass compared the live result with Beautiful UI's compact source/approval language, Cult
UI's continuous inset workspace treatment, and Origin UI Svelte's restrained, accessible application
controls. The implementation keeps Restless's existing Svelte identity and uses those references only
as a polish bar:

- <https://www.beautifului.dev/>
- <https://github.com/nolly-studio/cult-ui/blob/main/apps/www/registry/default/ui/expandable-screen.tsx>
- <https://originui-svelte.pages.dev/>

## Rejected verification path

Copying isolated source-message state into the scheduler-connected development database was rejected.
Even blocked Work can have causal source/conversation wakes, and the resident scheduler started a lead
process. The process was terminated before a semantic callback, the copied organisational state was
removed, and the final inspection used the scheduler-free isolated server. This is now explicit
evidence that visual QA must not restore external-message state into a live scheduler merely because
the target Work appears paused.

## Scope

This closes the owner-review implementation gap without adding a renderer registry, custody lifecycle
or generic file proxy. Live HTTP ReviewTargets continue to use the existing ticketed Runtime path;
the only new materialisation is bounded Markdown/text. Provider authentication, a real receiving
address and a real signed callback remain deferred in
[`provider-entry-gate.md`](provider-entry-gate.md).
