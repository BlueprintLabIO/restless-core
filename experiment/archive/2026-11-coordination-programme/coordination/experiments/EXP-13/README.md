# EXP-13 execution record

**Status:** Complete — `model-or-policy-limited`

**Started:** 29 August 2026

**Completed:** 29 August 2026

**Sprint:** [Durable visual operator frontier](../../../exp-sprints/exp-sprint-13-durable-visual-operator-frontier.md)

## Frozen model decision

- Exact local Z.ai route: `glm-5.3-flash` through the environment-backed `ZAI_BASE_URL` and
  `ZAI_API_KEY`.
- Admission result: provider rate-limit code `1310`; reset reported for 30 August 2026 at 11:11:53.
- Owner-directed fallback: exact `litellm/gpt-5.6-terra` through the locally provisioned
  OpenAI-compatible route.
- Terra admission: exact selector advertised text and image input; a live image request correctly
  read a current EXP-11 Swift Arrival CLIENT capture.

The earlier OpenRouter 403 probe was controller error and is not the model decision. The exact local
Z.ai probe above is authoritative.

## Isolation

- Company: `exp13_visual_operator_test`
- Restless port offset: `15000`
- State root: `/tmp/restless-exp13`
- One company-level USD 120 ceiling is the parent experiment envelope.
- No live external effects are authorised.
- Existing companies and the default daemon remain untouched.

## Frozen artifacts

- Swift Arrival: EXP-11 candidate `41f4fa53a2cd05ab17aea473f3d1be28979b2dcf`, seeded as a
  read-only ordinary directory.
- Website: Restless Cloud site commit `a812052e3252d2c546562ffe6447e07809a6f5ee`, built and copied as
  a read-only static candidate.
- Desktop task: labelled `_test` account-review CSV with deterministic expected output.

## Evidence boundary

Players receive only the public goal, controls, assigned tool shape and target artifact. They must not
read source, expected defects, another arm's outputs, adjudication notes or the deterministic desktop
oracle. A model report without direct image observations and a resulting-state trace is invalid.

The matched human-reference portion cannot be manufactured by an AI controller. Historical founder
observations are retained as context, but the provisional 80 percent human profile remains unsupported
unless a real human completes the frozen journeys independently.

## Outcome

The repaired thin operator materially improved the Swift Arrival run: 17 decisions versus B1's 34,
with no target loss, wrong-window event, focus repair or rejected interaction. It did not generalise to
the browser. W1 B2 could not derive a unique target through its permitted public interface, repeating
the target-discovery seam already repaired twice for G1. The frozen stop rule fired before D1, B3 or a
product repair loop.

The experimental implementation was purged. The next prerequisite is a launch-to-target identity
contract that returns a unique opaque handle without model guesswork or target-specific hidden
knowledge. See [results](RESULTS.md), [frictions](FRICTIONS.md) and [metrics](metrics.json).

## Retention

Only compact decision records, frozen inputs and checksums remain in the repository. Raw PNGs, browser
profiles, action traces, native process state and scratch operator code were deleted after terminal
synthesis. No product target changed during the experiment.
