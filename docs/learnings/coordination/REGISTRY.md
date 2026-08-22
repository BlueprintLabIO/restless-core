# Wildcard experiment registry

Status in this table is authoritative. Detailed plans live in `PROGRAM.md`; create an experiment
directory only when execution begins.

| ID | Wildcard | Family | Dependency | Status | Next gate |
|---|---|---|---|---|---|
| **W01** | Session mitosis | Shared history | Baseline B0/B1 | queued | Prove exact common-prefix fork and role/tool divergence |
| **W02** | Fork–work–reunite cycles | Shared history | W01 | queued | One fork, artifact-only reunion and second fork |
| **W03** | Exec-to-lead lineage fork | Shared history | W01 + redaction audit | queued | Neutral request capsule without executive authority leakage |
| **W04** | One brain, many hands | Cognitive topology | Baseline B0 | queued | Parallel bounded executors under one lead session |
| **W05** | Shared semantic blackboard | Shared state | Baseline B1 | queued | Patchable bounded situation model with provenance |
| **W06** | Causal context deltas | Communication compression | W05 or stable checkpoint | queued | Reconstruct exact relevant changes from base hash |
| **W07** | Proof-carrying handoffs | Artifact boundary | Baseline B1 | queued | Artifact + native proof + semantic diff, no narrative required |
| **W08** | Ambient coordination | Event routing | W07 | queued | Subscribe actor to material artifact/interface events |
| **W09** | Questions-only communication | Communication discipline | Baseline B1 | queued | Enforce blocker/question/interface/result-only channel |
| **W10** | Blind sibling diversification | Independence | W01 | queued | Same prefix plus private evidence shards and blind review |
| **W11** | Cross-finish experiment | Transferability | W01 | queued | Swap branches at midpoint and measure recovery/rework |
| **W12** | Micro-audition before ownership | Allocation | Fork primitive | queued | Two bounded probes, evidence-based lead selection |
| **W13** | Speculative shadow branch | Redundancy | Baseline B0 | queued | Keep shadow sealed unless primary stalls or fails |
| **W14** | Adaptive communication gates | Event routing | Metrics from W01–W09 | queued | Open rendezvous only on observable collision/uncertainty signal |
| **W15** | KV-state cloning | Runtime efficiency | Provider/runtime feasibility | queued | Clone inference state or test cached-prefix proxy |
| **W16** | Common room, private offices | Temporal topology | W01/W02 | queued | Scheduled artifact-bound rendezvous without continuous chat |

Allowed statuses: `queued`, `designing`, `running`, `provisional-win`, `provisional-loss`, `blocked`,
`replicating`, `accepted`, `rejected`, `superseded`.
