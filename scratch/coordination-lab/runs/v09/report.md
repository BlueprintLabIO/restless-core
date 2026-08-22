# v09 — OrgIntel mode-C cold-start failure

## Change under test

Replace the loose prose handoff with explicit OrgIntel state and the seven coordination commands,
carried as ACP-supplied MCP tools into the same first-party Pi harness. Exec was read-only and was
given an observed seed capability summary plus the full commercial directive.

## Evidence

- Seed: `514b7b3d0a65e093af608b08ca142344412181f4`, clean and unchanged
- Exec model: `nvidia/nemotron-3-super-120b-a12b:free`; live prompt/completion prices `0`
- First turn: 7/7 turns, 7 tool calls, 53,321 input / 1,892 output tokens, $0
- Exec successfully called `inspect_coordination` once
- The remaining six calls all attempted repository orientation with the native `read` tool and failed:
  `/`, `.`, `outputs/cosmon-game`, `../`, `514b7b3`, and `journal 0003`
- `read` intentionally accepts files, not directories; the harness exposed no scoped listing or search
  affordance
- The first turn ended truthfully as `max_turn_requests` / `max_turns`
- The event-driven controller started one fresh Exec wake; the experiment stopped it as churn and
  recorded `controller_cancelled`
- Work: 0; Attempts: 0; commits/artifacts: 0; repository still clean at the exact seed
- Durable first-turn trace: 131,203 bytes / 127 records, rather than the prior quadratic trace

## Score

Outcome score: **16/100** (no-artifact cap 39).

| Dimension | Points | Evidence |
| --- | ---: | --- |
| Accepted outcome /30 | 0 | no Work, commit, or artifact |
| Coordination /20 | 1 | canonical state was inspected, but no responsibility was commissioned |
| Recovery/truth /15 | 7 | max-turn and controller cancellation were explicit; fresh wake was possible |
| Review/evidence /15 | 0 | no candidate existed |
| Efficiency/attention /10 | 0 | six of seven calls were failed orientation attempts |
| Harness/control /10 | 8 | exact read-only launch, free-model proof, chronological events, limits, usage, and stop state |

## Dominant failure

This is not evidence against Work callbacks because the run never reached Work. The minimal harness
removed ordinary coding-harness perception—directory listing and scoped text search—while retaining
only file reads. A capable model then spent its entire bounded turn guessing paths. Read-only must not
mean blind.

## 10x hypothesis and decision

Add two small, deterministic, path-scoped native tools—`list` and `search`—for every actor. They are
general computer perception, not project planning or organisational semantics. Do not grant Exec a
writable shell and do not add more turns. Retry the exact mode-C mechanism after proving these tools
cannot escape the workspace.

Preserve v09 as the cold-start baseline.
