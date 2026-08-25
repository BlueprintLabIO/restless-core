# EXP-03 T3 freeze — replicated sales queue

**Frozen:** 24 August 2026
**Status:** launch-admitted

## Inputs

- scenario SHA-256: `3a04e8d310d6ba888476aee313d16487ccf692e10f89a0f0fe6018e072fbd5bf`
- evaluator SHA-256: `91d97303a47d37db84a2dedea34ce26fb46896d8900d6117034305817d277b1a`
- joined-hash ordering seed: `5409ac436c5b0e9b770a8a5d590774dbb4daa40a95041223f587b49e1a4f70a5`
- predeclared order rule: even first byte → Q1 first; odd → Q2 first
- observed first byte: `0x54` (even)
- arm order: Q1, then Q2

The untouched seed fails the 18-check external evaluator. Both arms use byte-identical frozen inputs,
start from `514b7b3d0a65e093af608b08ca142344412181f4`, use `zai/glm-5.3` at high reasoning for every cognitive
actor, cap spend at US$8, and use the same 5,400-second envelope plus 180-second drain. All records are
fictional and no message is sent.

## Matched organisations

- **Q1:** non-producing supervisor → one `sales-operator` owning all eight prospect units and the
  complete review-ready batch.
- **Q2:** non-producing supervisor → two parallel same-role unit workers, `sales-operator-1` and
  `sales-operator-2`, on disjoint dossiers → dependency-linked `batch-assembler` owning the exact
  final batch. The assembler is counted as coordination/closure overhead, not a third unit producer.

Q2 advances to Q4 only if its marginal accepted throughput is positive after whole-batch quality,
lead attention, assembler cost, provider pressure and spend. Duplicate, missing or recreated units are
not parallel value. A clear Q2 loss closes the replicated-queue scaling branch without running Q4/Q8.
