# Sprint 17 evidence report

**Report date:** 25 August 2026
**Exit status:** **Implementation complete; external provider validation explicitly deferred.**
Controlled faults, real ACP continuity, real-model Staff production, separate-lead review and the
paired native owner surface are green. Success contract 20 remains unrun until the founder supplies
and authorises one real Resend-signed inbound event.

## Outcome so far

The product now has one organisational meaning:

```text
owner outcome → available Exec → one accountable non-producing lead → Staff production
```

Exec appoints a lead and quiesces. A lead may frame, commission, observe, redirect, repair through
Staff and judge; productive lead-owned Work is rejected before launch. The worker owns the candidate
and attribution. A successful worker outcome first reaches its accountable lead, not the owner; only
the lead may prepare the current owner-altitude brief and escalate it to Exec.

Inbound email is now Authority-first and recoverable. The ingress authenticates the exact bounded raw
body and requires the provider delivery id. Authority stores distinct provider events, OrgIntel
projects each source reference once, exact RFC thread references route to the nearest responsibility,
and routine delivery-only events do not wake a model. Content, links and attachments remain bounded
untrusted evidence. A material reply during active Work reaches the accountable lead, whose exact
Work-linked feedback interrupts the worker and preserves state for a successor Attempt.

ACP processes remain disposable while the provider session stays hot only for one
company/actor/responsibility/workdir/model scope. Every launch receives fresh signed capabilities and
must prove its native coordination tools before the first productive prompt. Failed load reconstructs
explicitly; replay notifications do not become current work; cumulative cost becomes a per-wake delta;
and a process without a semantic callback becomes unknown rather than running or successful.

## Acceptance matrix

| # | Result | Evidence |
| ---: | --- | --- |
| 1 | Pass | Canon, prompts, scheduler and database scenarios agree on Exec → lead → Staff; productive lead Work is rejected. |
| 2 | Pass | No topology router was added; staffing remains lead judgement over sparse Work. |
| 3 | Pass | Real `zai/glm-5.3` continuity probe: fresh launch/capability per wake, same scoped provider session. |
| 4 | Pass | Forced missing provider session produced an explicit reconstructed wake with durable context. |
| 5 | Pass | ACP load replay is quarantined before readiness/live observation. |
| 6 | Pass | Usage is delta-accounted; absent cache/reasoning/cost fields remain unknown; no reasoning trace is stored. |
| 7 | Pass | Invalid coordination capability failed before the productive closure; repair resumed the unspent scoped session. |
| 8 | Pass | No-callback process exit becomes one unknown recovery capsule to the accountable lead. |
| 9 | Controlled pass; live pending | Exact raw-body signature, bounds and Authority-before-success are tested; the real provider callback is the open gate. |
| 10 | Pass | Svix delivery id is authoritative; distinct events sharing one email id remain distinct. |
| 11 | Pass | Authority-only crash, cursor loss and replay produce one projection and one owed wake. |
| 12 | Pass | OrgIntel retains the stable Authority source reference plus bounded provider/thread metadata. |
| 13 | Pass | Body is fenced as untrusted; attachments are bounded/quarantined; the real model ignored the injected admin/send instruction. |
| 14 | Pass | Exact reply/Work routing chooses worker or lead; unresolved departmental mail reaches lead; only unowned mail reaches Exec. |
| 15 | Pass | Only authentication, identity, exact correlation and obvious delivery suppression are deterministic. |
| 16 | Pass | Material active-Work feedback interrupts the exact worker; ordinary notifications coalesce without model theatre. |
| 17 | Pass | Isolated real-model run: Staff produced; a separate lead reviewed; no lead Work/artifact exists and the candidate hash was unchanged. |
| 18 | Pass under controlled source; live transport deferred | The exact bounded source, observed verification state, Staff candidate, uncertainty and lead-owned decision render together on desktop and mobile. See `owner-review-projection.md`; real Resend transport remains #20. |
| 19 | Pass for unsent scope | Drafting stayed Runtime work and the run recorded zero external effects. No reply was authorised. |
| 20 | **Deferred by founder** | No authenticated Resend receiving address was available and the founder deferred this provider validation; see `provider-entry-gate.md`. |
| 21 | Pass under controlled faults | Duplicate/distinct-event, crash window, load loss, missing MCP, no callback, injection, active Work and company-local provider degradation all preserve declared truth. Live-provider replay remains bundled with #20. |
| 22 | Pass | File polling and global coalescing were removed; explicit callback and bounded comparison scaffolding remain quarantined under `experiment/`. |

## Real-model product proof

The final proof used the configured `s17_signal_test` Company Runtime but a separate
`restless_s17_product_test` Postgres database, so the resident development daemon could not observe or
claim the test Work. A current-code coordinator owned the complete run.

- Model: `zai/glm-5.3` for both Staff and lead.
- Work: `44dae1d7-0520-4254-873e-782d0aff11c5`.
- Attempt: `7b6ed1c0-ffbc-41c6-9d65-dd56b35a10a3`, terminal state `produced`.
- Handoff: `fd8cf210-39b7-46f4-84e1-3967713b22dd`; lead brief current, then assigned to Exec and admitted for owner judgement.
- Candidate SHA-256 before and after lead review:
  `0251d914dcc4e250a2ac27bb35c601c8fa56920c2ee6fd699fb1003a2807aa22`.
- Worker usage snapshots: 2. Lead usage snapshots: 1. External effects: 0.
- Elapsed wall time: 250.88 seconds under activity-based supervision, with no fixed task timeout.

Artifacts:

- [`model-run-3-isolated-unsent-response.md`](model-run-3-isolated-unsent-response.md)
- [`model-run-3-result.json`](model-run-3-result.json)
- [`owner-review-projection.md`](owner-review-projection.md)
- [`model-run-1-unsent-response.md`](model-run-1-unsent-response.md) records the earlier useful
  candidate that exposed an over-specific evidence gate.

## Fault and harness evidence

- Live continuity probe passed on `zai/glm-5.3`: three fresh ACP launches retained one private marker
  in one scoped session, forced backing loss reconstructed, invalid MCP capability stopped before the
  prompt, and the repaired launch resumed.
- `crates/restless-orgintel/tests/inbound_projection.rs` proves external-source exact-once projection,
  atomic source-linked Work and exact reply routing before/during an Attempt.
- Daemon inbound scenarios prove Authority-commit crash recovery and company-local provider
  degradation with the failed company's cursor left owed.
- OrgIntel scenarios prove unknown Attempt recovery, causal interruption and successor attribution.
- [`s17-explicit-callback-external-event-results.json`](../../../../experiment/coordination-lab/v2/workdir/s17-explicit-callback-external-event-results.json)
  passed 4/4 checks: explicit callback delivery, no file polling and only the material arm creates a wake.
- [`wave0-deterministic-gates.json`](../../../../experiment/coordination/experiments/EXP-04/results/wave0-deterministic-gates.json)
  preserves local closure, zero assembler/duplicates, unknown usage fields, blind semantic review and
  batch-terminal lead wake behavior.

Two final-proof harness faults were found rather than papered over:

1. A prior interrupted charged stream left the disposable company correctly `metering_unknown`. The
   first evidence retry bypassed ordinary admission and received empty ACP turns. The live test now
   asserts exact spend availability before model launch; the inspected test-only poison was cleared
   from a known `$0.3244` accounted baseline.
2. The first successful export shared Postgres with the resident daemon, which noticed the lead
   handoff and launched an unwanted second wake. That candidate remained unchanged, but the trace was
   rejected as non-isolated. The final run moved to `restless_s17_product_test`; no resident process
   observed it.

The final visual pass found a third isolation lesson. Restoring preserved external-message state into
the scheduler-connected development database started a causal lead wake even though the Work looked
blocked. That process was terminated before a semantic callback, the copied state was removed, and
the accepted inspection used a scheduler-free current-code owner server. The paired review's mobile
pass also found and fixed a navigation defect that previously discarded the review when the
full-screen lead rail closed.

## Verification commands and observed results

```text
RESTLESS_TEST_DATABASE_URL=postgresql://localhost/restless_s17_product_test \
  scripts/verify-sprint-checkpoint
```

Passed: guarded live Postgres scenarios, strict Clippy, Rust formatting, 180 non-ignored Rust tests
across the workspace (including 146 daemon tests; 5 opt-in live/visual tests ignored), Svelte
diagnostics, type-ramp check and Prettier.

```text
RESTLESS_TEST_DATABASE_URL=postgresql:///restless_s17_product_test \
RESTLESS_S17_PRODUCT_TEST_RUNTIME_COMPANY=s17_signal_test \
RESTLESS_S17_PRODUCT_TEST_MODEL=zai/glm-5.3 \
RESTLESS_S17_PRODUCT_EVIDENCE_PATH=/Users/yao/Learning/restless/docs/sprints/sprint-17/evidence/model-run-3-isolated-unsent-response.md \
cargo test -p restlessd live_supervised_staff_prepares_native_review_without_lead_production -- --ignored --nocapture
```

Passed in 250.88 seconds with the exact result in `model-run-3-result.json`.

```text
RESTLESS_TEST_DATABASE_URL=postgresql:///restless_s17_product_test \
RESTLESS_S17_REVIEW_TEST_RUNTIME_COMPANY=s17_signal_test \
RESTLESS_S17_REVIEW_TEST_CANDIDATE=/Users/yao/Learning/restless/docs/sprints/sprint-17/evidence/model-run-3-isolated-unsent-response.md \
RESTLESS_S17_REVIEW_TEST_PRESERVE_STATE=1 \
cargo test -p restlessd \
  live_text_review_pairs_external_source_candidate_and_owner_decision \
  -- --ignored --nocapture
```

Passed with exact controlled source, bounded candidate, accountable lead and pending owner decision;
the preserved Work and handoff identifiers are in `owner-review-projection.md`.

```text
python3 experiment/coordination-lab/v2/runner.py external-event-architecture-test \
  s17-explicit-callback \
  --event-file experiment/coordination/experiments/EXP-03/T4/external-event.json
python3 experiment/coordination/experiments/EXP-04/exp04.py deterministic-gates \
  s17-wave0-regression
target/debug/restless doctor -c s17_signal_test
```

The callback probe passed 4/4, deterministic gates passed, and the final `doctor` re-probe reported
`live`: current image and source digest, coordination/browser/supervisor available, owner
gateway/cockpit APIs HTTP 200, OrgIntel available and 135 GiB free at the observed checkpoint.

## Deletion and quarantine record

- Deleted the direct-lead/player-coach alternative from current contracts and execution.
- Removed EXP-04's 50 ms file poller, global completion coalescing and empty reasoning-event flood.
- Reused Authority inbound custody, OrgIntel Work/messages, ACP, effects and ordinary process
  supervision; added no universal Signal/Event, workflow engine, provider catalogue or cross-database
  transaction.
- Test-only coordinator override, two ignored real-model probes and one ignored signed-provider probe
  remain `cfg(test)` quarantine.
- Failed disposable schemas, exact failed session locators and leaked test actors were removed. The
  `s17_signal_test` volume and `restless_s17_product_test` database remain ready for the provider gate.

## Deferred validation

[`provider-entry-gate.md`](provider-entry-gate.md) records the explicitly deferred real-provider arm.
If resumed, the founder must authenticate Resend, provide one test receiving address and authorise one
inbound test email. The host-only credential reference, direct signed-ingress probe, causal
source-message product probe, tunnel and isolated database are prepared. The CLI listener was rejected
as the verification bridge because it discards the temporary webhook's one-time signing secret; the
replacement keeps that secret only in test-process memory and removes the webhook after the accepted
event. No implementation or provider-tool installation remains before that founder input.
