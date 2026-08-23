# Longitudinal scores

Scores are invalid without per-dimension evidence. Historical v0-v2 runs predate the current fixed
rubric and remain qualitative baselines until rescored from their preserved artifacts.

| Version | Mode | Models | Outcome /30 | Coordination /20 | Recovery /15 | Evidence /15 | Efficiency /10 | Harness /10 | Final /100 | Decision |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| historical-v0 | C-like | Claude Sonnet 4.5 | — | — | — | — | — | — | unscored | preserve baseline |
| historical-v1 | C-like | Claude Sonnet 4.5 | — | — | — | — | — | — | unscored | preserve baseline |
| historical-v2 | C-like | Claude Sonnet 4.5 | — | — | — | — | — | — | unscored | preserve baseline |

Harness-only probes (not outcome-score comparable):

| Version | Probe | Score /100 | Dominant gap |
| --- | --- | ---: | --- |
| v01 | ACP/Pi live stream | 80 | cancellation untested |
| v02 | cancellation | 95 | shell isolation remains Runtime-owned |
| v03 | model diversity | 90 | ACP v1 error stop semantics |
| v04 | MCP adapter | 95 | stdio only |
| v06 | truthful turn ceiling | 100 | none in tested path |
| v08 | bounded durable telemetry | 100 | none in tested path |
| v12 | missing callback remains unknown | 100 | positive callback path next |
| v13 | provider error during positive callback | 90 | positive callback still unproven |
| v14 | provider repair + positive callback | 95 | tiny Work used 10 tool calls |
| v15 | bounded all-family telemetry | 95 | future event variants need an allowlist |
| v16 | Runtime-neutral context + partial-work repair | 96 | Git listing and redundant write remain |

Outcome runs:

| Version | Mode | Models | Outcome /30 | Coordination /20 | Recovery /15 | Evidence /15 | Efficiency /10 | Harness /10 | Final /100 | Decision |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| v05 | A | Laguna S 2.1 | 0 | 0 | 0 | 0 | 2 | 5 | **7** | preserve baseline |
| v07 | B | Nemotron Super + Laguna S | 0 | 2 | 5 | 0 | 0 | 8 | **15** | preserve baseline |
| v09 | C | Nemotron Super | 0 | 1 | 7 | 0 | 0 | 8 | **16** | add scoped perception; retry |
| v10 | C | Nemotron Super | 0 | 4 | 3 | 0 | 1 | 8 | **16** | change Exec posture + wake context |
| v11 | C | Nemotron Super + North + Nemotron Lightning + Laguna S | 10 | 9 | 2 | 0 | 4 | 10 | **29** (cap) | purge unsafe finalisation |
| v17 | C | Nemotron Super + Gemma 31B/26B | 0 | 7 | 10 | 0 | 2 | 10 | **29** | decouple Actor from model |
| v18 | C | Nemotron Super; pool not reached | 0 | 7 | 10 | 0 | 3 | 10 | **30** | project Runtime failure onto Attempt |
| v19 | C | Nemotron Super + North + Nemotron Nano + Gemma 31B | 0 | 9 | 13 | 0 | 1 | 10 | **33** | add exact recovery context; finish, integrate, review |
| v20 | C | Nemotron Super + North | 0 | 4 | 10 | 0 | 0 | 7 | **21** | stop; guard Work identity and atomic result ingestion |
