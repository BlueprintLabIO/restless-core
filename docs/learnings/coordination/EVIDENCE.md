# Coordination evidence index

This index maps current claims to inspectable evidence. It does not copy raw traces.

## Restless runs

| Evidence | What it supports or challenges |
|---|---|
| [`v01–v20 final report`](../../../scratch/coordination-lab/FINAL_REPORT.md) | Harness truthfulness, event streaming, model/provider separation, recovery-context and coordination failure history |
| [`durable lab learnings`](../../../scratch/coordination-lab/LEARNINGS.md) | Compact numbered observations from the complete scratch programme |
| [`v21 report`](../../../scratch/coordination-lab/runs/v21/report.md) | Sparse artifact-led Work versus graph-control comparison |
| [`v22 report`](../../../scratch/coordination-lab/runs/v22/report.md) | Two-phase delivery, runtime identity and drain defects preceding the matched run |
| [`v23 matched report`](../../../scratch/coordination-lab/runs/v23/report.md) | Strong lead alone versus lead plus one Staff member on a tightly coupled game slice; critic value and callback failure |

## External research priors

These guide experiment selection but do not count as Restless product evidence.

| Source | Prior imported |
|---|---|
| [Towards a Science of Scaling Agent Systems](https://arxiv.org/abs/2512.08296) | Task decomposability, coordination overhead, capability saturation and error amplification are candidate routing variables. |
| [Anthropic multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system) | Parallel agents are most promising for breadth-heavy independent research and can consume substantially more tokens. |
| [Multi-Agent Teams Hold Experts Back](https://arxiv.org/abs/2602.01011) | Peer consensus may dilute the strongest member; explicit decision rights should be tested against free-form discussion. |
| [Why Do Multi-Agent LLM Systems Fail?](https://arxiv.org/abs/2503.13657) | Specification, inter-agent alignment, verification and termination failures require separate attribution. |
| [Empirical Study of Multi-Agent Collaboration for Automated Research](https://arxiv.org/abs/2603.29632) | Parallel subagents and durable coauthoring teams should be treated as distinct architectures. |
| [AI Organizations Can Be More Effective but Less Aligned](https://alignment.anthropic.com/2026/ai-organizations/) | Global mandate and consequential constraints must remain visible at Exec/lead altitude and be evaluated for the whole organisation. |

## Evidence rules

- A raw run is immutable even when its interpretation changes.
- A report links exact scenario/version, starting ref, model configuration, artifact and acceptance evidence.
- A model's self-report is a claim, not completion evidence.
- `unknown` is recorded when a run, effect or artifact cannot be observed strongly enough.
- Simulated mechanism evidence from `_test` never becomes a live company's market or product fact.
- Architecture changes cite the discriminating runs or state clearly that they are owner decisions.
