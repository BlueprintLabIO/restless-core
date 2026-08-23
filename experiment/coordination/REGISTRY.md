# Wildcard experiment registry

Status in this table is authoritative. Detailed plans live in `PROGRAM.md`; create an experiment
directory only when execution begins.

## Ordinary-team calibration — runs before wildcards

| Cell | Domain | Size / structure | Arms | Status | Next gate |
|---|---|---|---|---|---|
| **C-SL** | Coding/product | Small, high coupling | B0/B1 | provisional-loss | B0 won 11/11 vs B1 10/11; retain as first boundary point |
| **C-SH** | Coding/product | Small, highly separable | B0/B1 | provisional-loss | B0 wins repeat at outcome parity: 29% faster, 23% fewer tokens, 62% fewer tools |
| **C-LL** | Coding/product | Broad, high coupling; below lead saturation | B0/B1 | provisional-loss | B0 won quality and cost; retain as below-saturation point, then test a genuinely lead-saturating case |
| **C-LH** | Coding/product | Large, highly separable | B0/B1 | blocked | `v28-clh01-b1-terra` was founder-stopped before integration/evaluation and is uncounted; await approval of Experiment Sprint 01 before allocating a fresh matched pair |
| **C-MM** | Coding/product | Medium, mixed | B0/B1/B2 | queued | Reserve only if the lead-saturating and research sentinels leave a boundary ambiguity |
| **R-SL** | Sourced research | Small, high coherence | B0/B1 | queued | Reserve; three below-saturation coding losses already establish the simple-region prior |
| **R-SH** | Sourced research | Small, independent evidence | B0/B1 | queued | Reserve; prefer the more discriminating large parallel-breadth sentinel |
| **R-LL** | Sourced research | Large, high coherence | B0/B1 | queued | Reserve unless domain versus coupling remains confounded after R-LH |
| **R-LH** | Sourced research | Large, independent evidence | B0/B1 | queued | Highest-value non-coding sentinel after a sustained Staff route is proved |
| **R-MM** | Sourced research | Medium, mixed | B0/B1/B2 | queued | Reserve only if a mixed integration point can change routing |

Historical v23 remains indexed in `EVIDENCE.md` and does not change a cell above to complete.

## Wildcards

| ID | Wildcard | Family | Dependency | Status | Next gate |
|---|---|---|---|---|---|
| **W01** | Session mitosis | Shared history | Baseline B0/B1 | queued | Activate only if a valid sentinel records briefing/reintegration `C1` or `C4`; then prove exact common-prefix fork and role/tool divergence |
| **W02** | Fork–work–reunite cycles | Shared history | W01 | queued | One fork, artifact-only reunion and second fork |
| **W03** | Exec-to-lead lineage fork | Shared history | W01 + redaction audit | queued | Neutral request capsule without executive authority leakage |
| **W04** | One brain, many hands | Cognitive topology | Baseline B0 | queued | Activate if autonomous Staff loses through cognitive duplication/integration `I3` or `E5` despite usable parallel work |
| **W05** | Shared semantic blackboard | Shared state | Baseline B1 | queued | Patchable bounded situation model with provenance |
| **W06** | Causal context deltas | Communication compression | W05 or stable checkpoint | queued | Reconstruct exact relevant changes from base hash |
| **W07** | Proof-carrying handoffs | Artifact boundary | Baseline B1 | queued | First candidate only if a valid run records callback/acceptance `V3`; artifact + native proof + semantic diff |
| **W08** | Ambient coordination | Event routing | W07 | queued | Activate after W07 only if late material delivery `M3`/`M5` remains |
| **W09** | Questions-only communication | Communication discipline | Baseline B1 | queued | Reserve unless a valid run measures narration/status `M1`; ordinary B1 traffic is already sparse |
| **W10** | Blind sibling diversification | Independence | W01 | queued | Same prefix plus private evidence shards and blind review |
| **W11** | Cross-finish experiment | Transferability | W01 | queued | Swap branches at midpoint and measure recovery/rework |
| **W12** | Micro-audition before ownership | Allocation | Fork primitive | queued | Two bounded probes, evidence-based lead selection |
| **W13** | Speculative shadow branch | Redundancy | Baseline B0 | queued | Keep shadow sealed unless primary stalls or fails |
| **W14** | Adaptive communication gates | Event routing | Metrics from W01–W09 | queued | Open rendezvous only on observable collision/uncertainty signal |
| **W15** | KV-state cloning | Runtime efficiency | Provider/runtime feasibility | queued | Clone inference state or test cached-prefix proxy |
| **W16** | Common room, private offices | Temporal topology | W01/W02 | queued | Scheduled artifact-bound rendezvous without continuous chat |

Allowed statuses: `queued`, `designing`, `running`, `provisional-win`, `provisional-loss`, `blocked`,
`replicating`, `accepted`, `rejected`, `superseded`.
