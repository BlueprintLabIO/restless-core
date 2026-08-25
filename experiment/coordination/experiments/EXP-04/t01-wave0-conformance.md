# EXP-04 Wave 0 — harness conformance

**Completed:** 25 August 2026
**Model for live gates:** first-party `zai/glm-5.3`
**External effects:** none; every artifact was fictional `_test` work

| Gate | Result | Evidence and disposition |
| --- | --- | --- |
| H1 session continuity | pass, with mechanism correction | Three wakes reused session `01a035c7-6960-7000-9924-391864b85ddd`; the second and third retained `AMBER-QUARTZ-7319` without transcript replay. Forced removal of that exact session backing produced a new session marked `reconstructed=true`. The ACP process is replaceable; the model session is hot. Provider cache fields remained `unknown`. |
| H2 cancellation | pass; in-flight usage unavailable | A live 120-second tool operation was interrupted in 2.425s. `PRE_CANCEL` survived, `POST_CANCEL` never appeared, the same Work/actor advanced exactly one revision, and the redirected Attempt produced `PRE_CANCEL\nRESUMED\n` with zero duplicate live owners. OMP supplied no usage event before cancellation, so cancellation usage remains honestly unknown. |
| H3 local closure | pass | Four disjoint Work/Attempt commits closed 48 accounts in randomized finish order with no assembler, duplicate, omission or overlap. Two mechanical composition orders produced tree `e058ed9b020f25998684d8e7f89468231c40f4cd` and byte-identical projections. |
| H4 telemetry/accounting | pass within provider visibility | Configured effort and phase are separate observable fields. Missing cost/cache/reasoning fields remain null. Session-cumulative tokens/cost are persisted for recovery and emitted as per-turn deltas, preventing resumed-session double counting. Raw reasoning is not collected. |
| H5 sustained provider capacity | Q2 pass; Q4 infrastructure-invalid | Two concurrent workers both produced exact tool artifacts and terminal reports in 50.718s. At Q4, all four sampled and wrote exact artifacts, but one session exposed only native tools and could not call `report`; three of four Attempts closed. Q4/Q8 stop without route roulette. |
| H6 evaluator separation | pass, correlated reviewer disclosed | Positive, negated and omitted exact cases separated correctly. The final blind packet exposed sources and artifacts but no topology, producer, trace or spend. A fresh same-provider GLM review scored both artifact families 9/10 usefulness and 10/10 grounding/tail safety; same-family correlation remains a limitation. |
| H7 causal material wakes | partial; policy-change cell not activated | Real H2 redirect works. Ordinary successful partition completions now coalesce until their local batch closes, while exceptional causes remain urgent. A frozen in-flight policy change was not run, so Q-SUPPORT is not activated and no broader live-guidance claim is made. |

## Retained repairs

- ACP `session/load` continuity scoped by actor and Work, with explicit cold reconstruction;
- historical notification replay suppression after session load;
- cumulative-session usage persisted for recovery and converted to per-turn budget deltas;
- null-preserving cancellation and provider telemetry;
- exact controller-to-Attempt cancellation;
- arbitrary frozen non-code fixture repositories;
- exact local-closure composition with disjoint-path/blob/ancestry checks;
- low-cost phase telemetry;
- ordinary local-batch completion coalescing.

No workflow engine, task router, global batch entity, cognitive assembler, raw reasoning store or new
production custody lifecycle was introduced.
