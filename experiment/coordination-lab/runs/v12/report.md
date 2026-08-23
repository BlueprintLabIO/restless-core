# v12 — missing callback remains unknown

## Failure from v11

A generic automatic finalisation turn converted a provider timeout plus clean unchanged checkout into
a false successful review.

## Change under test

- Delete the automatic finalisation continuation.
- When an actor process ends while its Attempt is still running, mark the Attempt `unknown`, preserve
  its workspace, block its Work, and wake Exec.
- Reject `outcome_met` when HEAD equals the Work input commit.
- Reject empty or shell-shaped single-string gate argv when Work is commissioned.
- Set all Staff to one model turn for the live probe so a missing callback is deterministic.

## Evidence

- Pi harness checks: 7/7
- Coordination/recovery fault checks: 32/32; SQLite quick check OK
- New deterministic checks prove:
  - a shell-shaped gate is rejected before Work creation;
  - unchanged-base completion is rejected;
  - an advanced commit still reports and replays idempotently.
- Exec model: `nvidia/nemotron-3-super-120b-a12b:free`; zero-price live proof
- Critic model: `nvidia/nemotron-3.5-lightning:free`; zero-price live proof
- Exec commissioned `work-84b8c67067` and quiesced
- Critic launch limit: one turn; it made one tool call and ended as
  `max_turn_requests` / `max_turns`
- OrgIntel result:
  - Attempt `attempt-ddf3485744`: `unknown`
  - Work: `blocked`
  - reason: `Actor process ended as max_turns without a terminal report`
  - artifacts: 0; decisions: 0
  - workspace HEAD: exact input `514b7b3d0a65e093af608b08ca142344412181f4`
  - workspace clean and preserved
- Staff turns started: exactly one. No automatic second Staff process exists.
- SQLite quick check after the live probe: OK
- Model usage: Exec 31,860 input/cache-read + 2,955 output; critic 3,664 input + 350
  output; $0

## Score

Recovery-invariant score: **100/100**.

This is a focused harness/OrgIntel recovery score, not an outcome leaderboard score. The tested path
preserves the distinction among completed, max-turn, and unknown; creates no false artifact; preserves
the workspace; emits an exact reason; and exposes an explicit Exec continuation point.

## Decision

Retain. `unknown` is not failure and not success. Repair, reassign, or abandon remains an explicit
organisational decision on a later wake.

Next prove the positive side: one small live Staff Work must advance the commit, satisfy a correctly
structured gate, report terminally in its original turn, and become a produced artifact without any
controller completion inference.
