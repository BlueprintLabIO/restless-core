# T05 — T4 customer-operations freeze and event conformance

**Status:** frozen before model results
**Date:** 24 August 2026

T4 is the first non-coding EXP-03 cell. It compares the same non-producing supervisor and one
end-to-end customer-operations worker under two event policies. The external change is triggered by an
observable first case artifact, never elapsed time.

## Immutable inputs

- Seed commit: `514b7b3d0a65e093af608b08ca142344412181f4`.
- Scenario: [`T4/scenario.md`](T4/scenario.md), SHA-256
  `957651a39cd7dc7b5e12a8dbc8f917528b07cea08b4d3b10fc98cf04923e39d6`.
- External event: [`T4/external-event.json`](T4/external-event.json), SHA-256
  `ed5217dca0715cead5104933a8c76c0502e37c5537010cb725ff32950efcf115`.
- External evaluator: [`T4/evaluate.mjs`](T4/evaluate.mjs), SHA-256
  `942030270ab7f8edd2e978be032efc1de66a089bd6010cc6b8329544948af0a1`.
- Trigger: first existence of `customer-ops/cases/C-101.md` in the worker workspace.
- Native outcome: review-ready unsent response/resolution pack plus deterministic file evaluator.
- Seed negative control: evaluator exits non-zero with 16/20 failures; the empty game seed cannot pass.

## Matched arms

| Arm | Exact event policy |
| --- | --- |
| **S1-E** | Event is recorded and immediately wakes the supervisor with exact Support Policy v2 and the C-103 CRM obstacle |
| **S1-T** | Identical event is recorded without an immediate wake; the supervisor first receives current external state on terminal/blocked worker evidence |

Both use `supervisor-lead` and one `customer-operations` worker on `zai/glm-5.3` at `high`, one Staff
slot, the same seed/runtime/tools/files, a US$8 ceiling and a 3,600-second operating envelope plus
120-second drain. The longer envelope is frozen before either arm because policy recovery may require a
supervisor wake, cancellation, repaired Attempt and final supervisor judgement. It is a budget, not a
semantic timeout.

## Frozen order

Order seed: `EXP-03:T4:GLM-5.3:terminal-vs-material:2026-08-24`
SHA-256: `7328ab0f206d410073e96e174989e1ee0b66741f5b506b685aab42cc966a3742`

Odd first byte selects **S1-E → S1-T**.

Run IDs:

- `exp03-t4-s1e-glm53-r1`
- `exp03-t4-s1t-glm53-r1`

## Conformance before cognition

Run `exp03-t4-event-conformance-r1` passed 4/4:

- both policies injected the exact event bytes after the exact artifact trigger;
- both recorded one `external_event_injected` fact with payload and hashes;
- terminal-only produced zero immediate lead wakes;
- material produced exactly one `external_event_material` lead wake.

The general supervisor architecture then passed 16/16 again in
`exp03-supervisor-architecture-r3`. No GLM call counts until both controls and all manifest hashes pass.

## Counted interpretation

- The external 20-check evaluator decides final policy correctness; actor confidence does not.
- Count stale v1 case content at event injection, at worker terminal and in the final candidate.
- Measure event-to-lead-wake, event-to-redirect, cancelled work, repair attempts, final acceptance,
  tokens, spend and elapsed time.
- A material-event win requires less stale work, faster safe recovery or a better final pack. Waking
  merely to restate the policy is overhead.
- Lead production, polling, mismatched event bytes or unobservable injection invalidates the arm.
- Neither arm sends a customer response or performs a refund, credit or security action.
