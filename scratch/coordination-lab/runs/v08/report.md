# v08 — bounded durable telemetry without weakening live streaming

## Failure from v07

The loose-team worker produced 116,145,725 bytes of JSONL while doing no useful work. Pi emitted a
growing partial assistant message for every token and the durable log copied that full snapshot each
time. This was quadratic trace amplification, not useful observability.

## Change under test

Separate the owner-facing ACP event transport from durable telemetry:

- ACP continues to send each text and thought delta immediately;
- the durable log records bounded lifecycle summaries instead of full message snapshots;
- adjacent durable text/thought deltas are coalesced up to 4 KiB;
- buffers flush before tool/lifecycle events and before the terminal prompt event, preserving order.

No coordination, model, prompt, or tool policy changed.

## Evidence

- Model: `cohere/north-mini-code:free`; live prompt/completion prices `0`
- Equivalent pre-change probe: `pi-harness/.generated/north.events.jsonl`
  - 1,134,021 bytes
- Lifecycle-normalised but uncoalesced probe: `pi-harness/.generated/telemetry.events.jsonl`
  - 556,712 bytes
- Final bounded probe: `pi-harness/.generated/telemetry2.events.jsonl`
  - 37,111 bytes, 41 ordered records, three coalesced delta records
- Reduction versus equivalent pre-change trace: **30.56x / 96.73%**
- Reduction versus normalisation alone: **15.00x**
- The final record is a chronological `prompt_end` with `outcome=completed`, exact model identity,
  aggregated usage, and its live free-model proof.
- `npm run check` and `npm run build` passed before the final probe.
- Runtime source SHA-256: `3081b6874b00eb696d6d0f893d89f3d2218fb42f6a526a34b8b9441cc503c6ff`
- Final evidence SHA-256: `82ae39e445e142484e0bd4aa157061e28e549fc9d336e73e7864ae9b2af14a21`

## Score

Harness telemetry score: **100/100**.

This is a harness-only probe, not an outcome score. It proves the tested path retains live ACP
streaming, bounded durable deltas, chronological tool/lifecycle boundaries, terminal truth, model
identity, and usage without repeated transcript snapshots.

## Decision

Retain. Do not tune telemetry further unless a real run reveals a new loss of information. The next
constraint is organisational: can explicit Work and terminal callbacks make a worker challenge a
stale assignment and produce a useful artifact?
