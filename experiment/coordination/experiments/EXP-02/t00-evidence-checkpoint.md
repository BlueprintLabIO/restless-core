# EXP-02 T0 — evidence checkpoint and drain

**Status:** checkpoint and drain complete on 23 August 2026; A4 fixture freeze remains Wave 0 work.

## EXP-01 state at handoff

- No W01 host runner or cognitive agent process remained live.
- Two labelled R-BREADTH B1 containers remained alive as `sleep infinity` Runtime cells after their
  supervising host process disappeared:
  - `restless-coord-v2-exp01-w01-rbreadth-r1-b1-terra-work-b81d87cb5`;
  - `restless-coord-v2-exp01-w01-rbreadth-r1-b1-terra-studio-lead`.
- The lab database had already closed Attempt `attempt-4cd433d30f` as `unknown` with runtime outcome
  `controller_cancelled`; Work `work-b81d87cb54` was blocked.
- The Attempt had no artifact row and no terminal callback.
- Its exact persistent workspace nevertheless contained untracked
  `research/evidence/player-interviews.md`, 7,895 bytes, SHA-256
  `c41ddc4c3f391db0a5fa2a9f844b55375a4669c1cad41eb6bb062827618a1b66`.
- The last timeline event was the producer starting a Git commit command after its declared content
  check passed. The command never returned to the harness; the workspace remained dirty.

This is diagnostic evidence, not a counted A4 arm: the A4 contract and evaluator were not frozen
before the failure. It directly confirms the activation condition—productive evidence can survive
while the organisation has only a generic unknown state and no artifact handle.

## Handoff decisions

- EXP-01 is superseded without rewriting any completed result.
- The unfinished W01 R-BREADTH/G-UX replications transfer to EXP-02 A2 only if A4/A1 leave a measured
  causal-context bottleneck.
- The preserved dirty file remains in the ignored experiment workspace as linked evidence; it is not
  promoted, committed into the knowledge base or counted as accepted research.
- Stop the exact two idle containers after this checkpoint. Do not delete their host workspace or
  state database.

## Exit evidence

- [x] exact containers stopped;
- [x] host workspace and state database still present;
- [x] no coordination-lab container remains running;
- [x] A4/P5 contract, fixture hash and negative control frozen before execution.

Conformance run `exp02-a4-conformance-r1` passed 10/10. Candidate SHA-256 is
`9748029525d1df6aacdb6105b75fb94ac3cbc8843b7a9294709d2005b75de0c0`; frozen source SHA-256 is
`5b216639de84031729e798af81091a4e752f889f69b41ed7f116d30c3f868b21`.
