# Coordination evidence index

This index maps current claims to inspectable evidence. It does not copy raw traces.

## Restless runs

| Evidence | What it supports or challenges |
|---|---|
| [`v01–v20 final report`](../coordination-lab/FINAL_REPORT.md) | Harness truthfulness, event streaming, model/provider separation, recovery-context and coordination failure history |
| [`durable lab learnings`](../coordination-lab/LEARNINGS.md) | Compact numbered observations from the complete programme |
| [`v21 report`](../coordination-lab/runs/v21/report.md) | Sparse artifact-led Work versus graph-control comparison |
| [`v22 report`](../coordination-lab/runs/v22/report.md) | Two-phase delivery, runtime identity and drain defects preceding the matched run |
| [`v23 matched report`](../coordination-lab/runs/v23/report.md) | Strong lead alone versus lead plus one Staff member on a tightly coupled game slice; critic value and callback failure |

## Active v24 baseline programme

| Evidence | What it supports or challenges |
|---|---|
| [`Exec delegation ADR`](../../docs/adr/0005-exec-dispatches-through-accountable-leads.md) + [`Exec prompt contract`](../../crates/restlessd/src/context.rs) | Accepted Exec → accountable-lead boundary is now present in architecture and the live prompt contract. |
| [`parallel-department behavioural test`](../../crates/restless-orgintel/tests/actors_and_teams.rs) | Against live test Postgres, a second lead Work/Attempt is dispatched while the first lead Attempt remains running; Exec owns neither. |
| `v24-baseline-architecture-r4` (34/34) | B0/B1/B2 actor rosters, writable candidate ownership, no shared-memory confound, GPT/ACP launcher separation, observable manipulation labels and release actor-host capability negotiation. Raw scratch result remains under the ignored v2 workdir until the v24 report checkpoint is committed. |
| `v24-requested-worker-proof` | On 23 August 2026 both `stealth/ox-alpha` and `z-ai/glm-5.2:free` were live-free, tool-capable, gateway-reachable and able to write an exact artifact. Admission elapsed 22.218s and 120.961s respectively. |
| [`C-SL-01 run index`](experiments/C-SL-01/runs.md) | First small/high-coupling cell. Counted B1 was structurally valid and callback-complete but failed the hidden evaluator 10/11; matched B0 passed 11/11 in about 44% less time and 40% fewer tokens. Earlier invalid/preflight runs remain attributed separately. |
| [`C-SH-01 run index`](experiments/C-SH-01/runs.md) | Small/high-separability cell and corrected replicate. Runtime capsules cut B1 wall time 50%, tokens 67% and worker tools 63%, yet B0 still won at owner-outcome parity: 29% faster, 23% fewer tokens and 62% fewer tools. Frozen 10/11 B1 proxy and separate passing rendered-bounds audit remain attributed rather than conflated. |
| [`C-LL-01 run index`](experiments/C-LL-01/runs.md) + [`blind review`](experiments/C-LL-01/blind-review.md) | Broad/high-coupling slice below strong-lead saturation. Both arms passed 23/23; blind review preferred B0. B0 reached decision 78.7% faster with 48.4% fewer input tokens and 68.6% fewer tools. B1 achieved real overlap, but its first artifact arrived after the solo decision point and required integration. A missing producer callback preserved its commit, surfaced `unknown`, and recovered through one evidence-based repair rather than a blind retry. |
| [`C-LH-01 run index`](experiments/C-LH-01/runs.md) | Large/high-separability cell has no counted arm. Preflight found an obsolete admission cutoff; r2 found a non-renewable 15-minute lease and preserved an unreported clean commit; r3 proved the renewable supervisor heartbeat, then exposed 14 more hidden GLM provider 429s and no worker diff. Across r2/r3, 65 provider rejections are attributed to `R1`/`R4`, not team performance. Restart gates prevent lucky-retry evidence. |
| `v24-baseline-architecture-r5` (38/38) + `v24-artifact-architecture-r5` (22/22) | Actors receive observed host/container browser capabilities; the lead's first-party adapter executed a native proof against the live mounted working tree and supplied its missing static fixture without host Playwright/CDP rediscovery. |

## Experiment Sprint 01

| Evidence | What it supports or challenges |
|---|---|
| [`T0 harness verification`](experiments/EXP-01/t00-harness-verification.md) | After the `experiment/` rename, 39/39 fault checks, 39/39 baseline-isolation checks and 22/22 artifact-architecture checks passed. A real Terra producer returned an exact gated commit and terminal callback in 50.75 seconds, woke the lead, and left Exec available for an independent request. Two focused-probe wrapper bugs were fixed before counted arms. |
| [`T1 task/evaluator freeze`](experiments/EXP-01/t01-task-evaluator-freeze.md) | E01 retained its high-saturation/high-separability game contract. The first E02 memo was rejected as below lead saturation; the revised four-dossier research pack was independently reclassified as genuinely separable and lead-saturating. Both external evaluators fail untouched seeds for the intended absent outcomes, and both fresh allocations randomised B1 first. |
| [`EXP-01 E01 result`](experiments/EXP-01/e01-ordinary-frontier.md) + [`blind review`](experiments/C-LH-01/blind-review-exp01.md) | Corrected counted C-LH pair closed. Both arms failed the same fresh-navigation owner gate (10/53). Blind review modestly preferred B1 9.1 vs B0 8.4 but not enough to outweigh the shared failure. B0 used 48.9% less wall time, 71.1% fewer tokens and 65.8% fewer tools. B1 exposed `S2`/`E5`: a useful CSS handoff plus one truthful callback repair did not repay coordination cost. |
| [`EXP-01 E02 result`](experiments/EXP-01/e02-ordinary-frontier.md) + [`blind review`](experiments/R-LH-01/blind-review-exp01.md) | Counted large parallel-breadth research pair closed. Both arms failed the frozen gate (B0 28/36; B1 27/36). Blind review narrowly preferred B1 9.1 vs B0 8.9, but not enough to outweigh the failures. The B1 worker received source IDs without the frozen cards, substituted unrelated repository evidence, and forced complete lead reimplementation (`C1`/`I3`/`E5`). B0 was 12.1% faster with 57.8% fewer tokens and 47.9% fewer tools. |
| [`W04 mechanism card`](experiments/EXP-01/w04-one-brain-many-hands.md) + [`bounded probe`](../coordination-lab/v2/probe_w04.py) | W04 is blocked as an unidentifiable manipulation: B0 already has ordinary process concurrency; the only cross-actor dispatch creates autonomous B1 Work; nested model execution is blocked; and no bounded operation executor exists. A shell wrapper would compare B0 with itself. |
| [`W01 mechanism card`](experiments/EXP-01/w01-session-mitosis.md) + [`conformance probe`](../coordination-lab/v2/probe_w01.py) | Corrected first-party conformance preserved an exact removed nonce from one parent into distinct Sol/Terra forks; each child produced one isolated clean role commit. The first probe was infrastructure-invalid because child fork sandbox defaulted read-only. The live G-WORLD screen is frozen; shared run-root authority remains audited rather than enforced. |
| [`Natural-lead result`](experiments/EXP-01/natural-lead-result.md) + [`blind review`](experiments/W01-G-WORLD-01/blind-review-natural-lead.md) | After one factual-interface repair, optional one-Staff N1 passed the frozen 18/18 journey and all fresh regression suites, then won blind quality 8.2 versus forced B1 6.4 and fresh B0 4.5. It was not faster. The first N1 had narrated nonexistent Staff; zero Work/Attempts exposed the manipulation failure. Sparse Work is therefore retained as factual cross-actor evidence while leadership remains judgement-led and free-form. |

## Experiment Sprint 02

| Evidence | What it supports or challenges |
|---|---|
| [`T0 evidence checkpoint`](experiments/EXP-02/t00-evidence-checkpoint.md) | EXP-01's orphaned R-BREADTH Terra process left a 7,895-byte dirty dossier with no artifact row or callback while its Attempt was correctly `unknown`. Exact idle containers were stopped without deleting the workspace/database. This activates artifact/process recovery without counting the accidental run. |
| [`A4/P5 result`](experiments/EXP-02/a4-result.md) | On an identical frozen stranded dossier, generic and capsule Sol leads both inspected the candidate, preserved unknown outcome, detected the planted unsupported claim and chose revision without rerun. Blind review preferred capsule provenance 9.8 vs 9.2, but it was 11.1% slower, used 10.1% more newly processed input and one extra tool. A richer capsule therefore failed its churn gate; compact attributable proof remains the product floor. |
| [`A1/P3 result`](experiments/EXP-02/a1-p3-result.md) | Exact source locator/digest let Terra return five frozen-corpus artifacts that Sol accepted instead of rejecting and rebuilding a source-absent placeholder. Both final packs failed 27/36, but fresh blind review preferred A1 9.3 vs 9.0 and wall time fell 5.6%; newly processed input rose 16.2% and tools 24.2%, so the finding is provisional and replicating. |
| [`A1/P1 replication`](experiments/EXP-02/a1-p1-result.md) | A1 produced one accepted Staff commit, passed the frozen 18/18 journey and cut wall time 16.8% versus the carried natural baseline. Fresh blind review nevertheless preferred the baseline 9.0 vs 7.6 because A1's native snapshot encoded weaker bridge, material-role and beyond-gate semantics. This clear cross-shape quality loss rejects and purges the capsule rather than promoting an efficiency-only win. |
| [`P0 routing result`](experiments/EXP-02/p0-result.md) | A natural Sol lead selected one real presentation producer on a task classified for zero Staff. The candidate passed 11/11, but the same outcome required 109% more wall time and 124.7% more newly processed input than carried B0. An independently bounded fragment is therefore not enough; its differentiated outcome value must repay integration. |
| [`P4 critic result`](experiments/EXP-02/p4-result.md) | Solo Sol and Terra-critic-plus-Sol both found the planted battle-proof and mobile-prompt failures and chose revision. Blind review preferred solo 6.0 vs 5.0; C1 doubled wall time and added 56.7% new input. The clean-control critic did not invent those failures and found a separate real missing `bridge.text` snapshot field, preserving counterevidence against the prior 18/18 label. |

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
