# v02 — cancellation reaches the real tool process

## Failure from v01

ACP cancellation reached `PiRuntime.cancel()`, but the first probe took the full 30 seconds. The shell
process died while its `sleep` child retained the output pipe, and a cancellation arriving between
tool-start notification and tool execution could predate listener registration.

## Structural fix

The run tool now creates a process group, kills the group, checks an already-aborted signal, and
rechecks on the child `spawn` event. Runtime result mapping records an explicit cancellation flag and
aggregates usage from all assistant messages rather than losing usage behind the synthetic aborted
message.

## Evidence

- Live model: `nvidia/nemotron-3.5-lightning:free`
- Live price: prompt `0`, completion `0`
- Model called exactly `sleep 30`
- Client sent `session/cancel` on the ACP `tool_call` update
- ACP terminal stop: `cancelled`
- Cancel-to-stop: **6 ms**
- End-to-end elapsed: **1.606 s**
- The 30-second process did not remain alive
- Usage preserved: 381 input, 75 output, zero reported cost
- Deterministic suite remained 5/5

## Score

Harness-only score: **95/100**. This is not comparable with the outcome scorecard.

| Harness criterion | Points |
| --- | ---: |
| Exact launch controls | 20/20 |
| Chronological live thought/text/tool streaming | 20/20 |
| Tool/write posture and credential stripping | 15/20 — shell isolation is a scratch boundary, not a production security boundary |
| Cancellation/stop propagation | 20/20 |
| Usage/model/error truthfulness | 20/20 |

## Decision

Retain the process-group cancellation. Do not build a custom sandbox around it: production relies on
the persistent Company Runtime/container boundary. V03 now tests whether the contract survives model
diversity rather than overfitting one provider model.
