# T00 — GLM-5.3 live admission

**Disposition:** passed
**Date:** 24 August 2026
**Evidence class:** live provider/runtime/tool proof; not a coordination result

## What was proved

One fresh `high`-reasoning GLM-5.3 session ran through the Company Runtime, Restless credential broker
and model gateway. It used the OMP `write` and `read` tools to create and verify an exact 72-byte
artifact, returned the required completion marker and exposed provider-reported usage and cost in its
session evidence.

| Observation | Result |
| --- | --- |
| Requested runtime selector | `zai/glm-5.3` |
| Provider/model observed in assistant messages | `zai` / `glm-5.3` |
| Reasoning control | `high` requested; GLM-5.3 reasoning is always on |
| Tool artifact | exact, read back, SHA-256 `bd42111b02b0d94392f82c5f6a49558ba961f57f3eeaf8cc5c02414ea4c70013` |
| Completion marker | exact `CAPABILITY_PROBE_COMPLETE` |
| Elapsed | 10.481 seconds |
| New input | 468 tokens |
| Cache read | 4,544 tokens |
| Output | 281 tokens |
| Provider-reported cost | US$0.00307304 |

The committed machine-readable summary is [`glm53-admission-proof.json`](glm53-admission-proof.json).
The raw OMP session and artifact remain in the isolated ignored run
`v2/workdir/exp03-glm53-zai-admission`.

## Admission defect found before the pass

The public OpenRouter catalogue contained `z-ai/glm-5.3`, and the Company Runtime could list the
selector after refresh, but the Restless auth gateway returned `Unknown model`. An isolated gateway
catalogue probe showed why: the gateway allowlist is credential-scoped. The current broker exposed 58
models, including `zai/glm-5.3`, and zero OpenRouter models. Public availability was therefore not
routability.

The harness now:

- admits paid workers only behind an explicit flag while retaining free-only default checks;
- records the public catalogue identity and the exact runtime provider separately;
- supports the broker-authenticated first-party Z.ai route;
- parses exact OMP session evidence for model/API identity, tokens and cost.

The first successful tool call also revealed that the extension's transport rebind reset an explicit
`high` request to the model's `max` default. The admission parser now records the final observed
thinking level, the route default is pinned to `high`, and the final proof above observed exactly one
`high` level. This earlier call remains diagnostic rather than counted.

No failed admission made a provider model call or produced an artifact. These were infrastructure
diagnostics, not model retries and not organisational evidence.

## Consequence

Wave 0 may use `zai/glm-5.3`. A counted run still requires supervisor-only protocol conformance and a
frozen native task/evaluator. The gateway catalogue—not a public model page—is the authority on which
provider routes the company can actually call.
