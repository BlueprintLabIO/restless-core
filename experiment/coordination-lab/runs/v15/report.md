# v15 — semantic telemetry for every streamed event family

## Failure from v11/v14

Text/thought deltas were bounded, but `toolcall_start`, `toolcall_delta`, and `toolcall_end` still
persisted Pi's full growing partial assistant message. Native tool logs could also copy complete write
contents and command output.

## Change under test

- Keep ACP owner-facing updates unchanged and token-live.
- Do not durably persist `toolcall_delta`; validated complete arguments already arrive at tool start.
- Reduce message starts/ends and tool-call boundaries to semantic summaries.
- Hash large prompt/tool strings and record byte counts/previews.
- Hash write/edit bodies; bound commands; summarize command output by bytes and digest.
- Re-run the same focused Work shape with the same North Mini model.

## Evidence

- Model: `cohere/north-mini-code:free`; live prompt/completion prices `0`
- ACP remained live: 1,143 `agent_thought_chunk` updates and 12 tool starts reached the client
- Durable v14 comparable trace: 331,076 bytes
- Durable v15 trace: 69,618 bytes / 191 records
- Reduction: **4.76x / 78.97%**
- Reduction versus v11's multi-actor amplified trace: 378.45x (not a workload-controlled comparison)
- Final trace SHA-256:
  `5269dd9d263bbcf143c38bf2cd9bc2010c190a995383719ff9b1ad7ce84aeb45`
- Chronology retained: launch → MCP/tool materialisation → prompt → message boundaries → tool
  start/end → terminal `prompt_end`
- The Work itself ended truthfully `unknown` after 12 turns. It wrote the marker file but did not
  commit/report; no artifact was inferred and the uncommitted file remains preserved.

## Score

Harness telemetry score: **95/100**.

All observed event families are now bounded and ACP streaming is unchanged. Five points remain open
because future Pi event variants need the same explicit semantic allowlist; unknown variants are not
yet rejected by schema. This is not an outcome score.

## New bottleneck

The failed Work is separate evidence. Its prompt said to use the current working directory, while the
embedded observed-state JSON still named Docker-only `/workspace`. North repeatedly probed that path,
used 12 turns, wrote the file last, and had no callback budget left. A context projection must not leak
the producer environment's locator into a different Runtime adapter.

## Decision

Retain telemetry. In v16, remove workspace/cell locators from Staff's organisational context and show
only exact Git state. Repair the same Work so its uncommitted marker is reused rather than recreated.
