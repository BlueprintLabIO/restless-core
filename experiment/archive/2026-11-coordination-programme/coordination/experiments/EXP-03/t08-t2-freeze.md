# EXP-03 T2 freeze — marketing campaign system

**Frozen:** 24 August 2026
**Status:** launch-admitted

## Inputs

- scenario SHA-256: `f61e2d6c4e9522f9926070c398a21b8cb7125b478455bd241ca44bead79d2d88`
- evaluator SHA-256: `3d9dd62683a2d2c583cfbb43814412f8b25119c4813d13d9046ae3a86e08edb0`
- joined-hash ordering seed: `752017de54cf44dd60fea26a4db43be9422d2710bd070c6eca0f010cc6f548f6`
- predeclared order rule: odd first byte → S2 first; even → S1 first
- observed first byte: `0x75` (odd)
- arm order: S2, then S1

The untouched seed fails all 19 evaluator checks. Both prepared manifests copy the exact frozen bytes,
start from `514b7b3d0a65e093af608b08ca142344412181f4`, use `zai/glm-5.3` for every cognitive actor, expose one
Staff execution slot, cap spend at US$8, and use the same 5,400-second envelope plus 180-second drain.

## Matched organisations

- **S2:** non-producing supervisor → `marketing-strategist` → dependency-linked
  `marketing-producer`. Both distinct worker identities must own one produced Work; the final producer
  artifact must dependency-cover the strategist artifact.
- **S1:** non-producing supervisor → one end-to-end `marketing-operator`.

The task, product/customer evidence, exact output schema, channel choice and evaluator do not mention
or vary with organisation. Nothing is published.

## Interpretation gate

S2 earns a specialised-team win only if both contributions are genuinely used and the final whole
campaign improves quality or latency enough to repay its extra lead wake, briefing and handoff. A clean
strategy artifact that is recreated or unused is not team value. A clear S2 loss stops the larger
complementary-marketing branch.
