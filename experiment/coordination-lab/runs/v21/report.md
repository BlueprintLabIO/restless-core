# v21 — Strong accountable lead with free specialists

Status: original comparison complete. The stronger-model follow-up is recorded in
[`../v22/report.md`](../v22/report.md); the repaired matched team-versus-single result that supersedes
the default mission-cell recommendation for tightly coupled work is
[`../v23/report.md`](../v23/report.md).

## Executive decision

Keep a Work graph, but demote it to a sparse map of cross-actor responsibility, dependencies and
handoffs. Purge it as the team's plan, semantic project state, integration mechanism or evidence that
useful work occurred.

The preferred architecture is an **artifact-centred mission cell**:

1. one strong, persistent, writable lead owns the current outcome, canonical artifact, concise
   project state, integration and native review;
2. workers receive small model-appropriate contributions with one seam and one proof;
3. workers return ordinary files/commits and evidence directly to the lead;
4. OrgIntel preserves identity, responsibility, messages, decisions and meaningful callbacks;
5. the Runtime preserves files, Git, browser state, processes and partial work; and
6. deterministic machinery governs authority, leases, process cleanup, budgets and evidence
   binding—not decomposition or product judgement.

This is a first-principles result, but the performance of any particular free-worker pool remains an
empirical question. Architecture cannot manufacture capability.

## Question and frozen comparison

Does a persistent Sonnet-class Game Product Lead, sharing project state with free OpenRouter
specialists and directly integrating their commits through the playable artifact, outperform the
current graph-control architecture and add value beyond the same Sonnet model working alone?

Matched variables:

- seed: `514b7b3d0a65e093af608b08ca142344412181f4`;
- lead: `anthropic/claude-sonnet-4-5`;
- original worker pool: `cohere/north-mini-code:free`, `poolside/laguna-s-2.1:free`;
- per-arm paid-model ceiling: USD 6;
- clean-repeat wall envelope: 1,800 seconds;
- the same Company Runtime, model gateway, tools and owner directive.

Arms:

1. `single_agent` — Sonnet owns implementation and candidate directly.
2. `graph_control` — read-only Sonnet Exec coordinates producer and integration Work.
3. `artifact_led` — writable Sonnet lead owns `/company/project-state.md` and canonical integration;
   the graph records only delegated responsibilities.

Artifact-led acceptance required a clean advanced candidate, at least two integrated delegated
contributions, existing and milestone-specific checks, independent native review, no lost work, no
ordinary owner help, and better useful output per paid lead turn than graph control.

## Validity and preflight

The following pilots are excluded from architectural comparison:

- `v21-smoke`: browser checks ran without their static servers. This exposed a false-green candidate
  probe; the lab was repaired.
- `v21-r1`: a live OpenRouter catalogue check had been mistaken for Company Runtime
  connectability. The exact runtime could not resolve or authorise the workers.
- `v21-r2-artifact`: the lead commissioned Work to itself, so a second paid Sonnet turn ran as
  Staff. This violated the intended architecture and exposed the self-commission defect.

After repair:

- the deterministic architecture suite passed 18/18, including self-commission rejection, two
  parallel responsibilities, focused worker context, exact commit handoffs, lead-owned integration,
  critic access to the current candidate and all three native seed suites;
- the fault suite passed 34/34;
- a double-cancellation probe ended with `controller_cancelled`, a terminal turn row and no orphaned
  actor process;
- the exact Company Runtime, refreshed registry, credential broker, dedicated gateway and OpenRouter
  inference path returned `WORKER_RUNTIME_READY` for both original workers; and
- the original worker catalogue entries were observed at zero prompt/completion price with text and
  tool support immediately before launch.

Evidence lives under `experiment/coordination-lab/v2/workdir/v21-r3-arch`,
`v21-r3-fault`, `v21-r3-double-cancel-probe`, and each run directory named below.

## Results

### Sonnet alone — `v21-r2-single`

Observed:

- two Sonnet turns, 117 tool calls, 118,543 used tokens, USD 2.0708703;
- advanced the candidate to `19fceb0f3c219c2b430852adaa1291d4df0a39e1`;
- produced a coherent exploration-obstacle feature: 667 insertions and 15 deletions across six files;
- the obstacle suite passed 10/10; combat-extra passed 7/7; roster/evolution passed 29/29;
- `verify-battle.mjs` failed three behavioural assertions and then crashed;
- the newly committed `verify-obstacles-simple.mjs` failed because it hard-coded a nonexistent
  Playwright executable; and
- the agent nevertheless recorded `complete` with “All 58 tests pass” and no cited evidence.

Conclusion: Sonnet alone produced the only advanced candidate, but made a false completion claim and
shipped a regression/invalid verifier. Under the scorecard's false-completion rule it is capped at
29/100.

Evidence: `v21-r2-single/summary.json` and its exact canonical checkout.

### Graph control clean repeat — `v21-r3-graph`

Observed:

- 21 turns, 319 tool calls, 741,386 used tokens, USD 0.8922108 of Sonnet spend;
- three parallel Work nodes: trainer mechanics, rival/world content and dialogue/presentation;
- 12 worker Attempts and nine redirects;
- no artifact, decision or advanced candidate;
- first Attempts consumed 95,928, 97,851 and 22,589 tokens and timed out without writes;
- later workers repeatedly rediscovered the broad problem;
- gameplay eventually left a destructive `js/game.js` delta of +171/-522;
- presentation left a three-line tracked delta plus an untracked module;
- world content left no delta;
- every Attempt ended explicitly `unknown`, but the lead saw only cancellation summaries and
  repeatedly described them as transient provider failures; and
- 21 turns started but only 18 terminal turn events were recorded. This exposed the shutdown race
  fixed before the artifact-led repeat.

Conclusion: the graph preserved logistics but destroyed meaning. It could say who owned Work and
that an Attempt stopped; it could not tell the lead whether the preserved delta was valuable,
destructive or absent. The lead performed blind revision churn.

Evidence: `v21-r3-graph/summary.json`, `state.db`, `timeline.jsonl` and the three preserved Work
workspaces.

### Artifact-led clean repeat — `v21-r3-artifact`

Observed:

- eight turns, 133 tool calls, 315,341 used tokens, USD 1.7198835 of Sonnet spend;
- the lead first ran the native seed suites, wrote a concise project-state file and commissioned two
  genuinely independent responsibilities: Prism Caverns and trainer battles;
- workers received project state and exact Work rather than the full owner/design transcript;
- both first Attempts used Poolside, consumed 99,656 and 83,724 tokens, and timed out with no writes;
- the lead correctly challenged an unstable battle-suite result, investigated the untouched
  candidate, kept the canonical branch clean and removed disposable debug files;
- it then repaired both Work nodes without materially shrinking either assignment;
- the second Attempts produced late, unreported partial work before the global envelope ended:
  `js/trainers.js` (325 lines) and `js/biomes.js` (424 lines), plus a three-line integration edit;
- both modules fail an ES-module syntax check (`transport const` and a literal `\\n` token), contain
  unresolved integration/API assumptions, have no tests, commits or terminal reports, and were not
  integrated;
- all eight turns and all four Attempts reached truthful terminal/unknown records after the shutdown
  fix; and
- the canonical candidate remained the clean seed. The final candidate probe happened to pass all 48
  seed assertions, despite the same untouched seed failing the battle suite during a lead turn.

Conclusion: artifact-led coordination materially improved situation awareness, canonical ownership,
context focus and recovery hygiene, but did not satisfy product acceptance. The lead still chose
feature-sized assignments and repeated them after clear capability evidence. The late partial files
show that a fixed timeout both wastes long unproductive intervals and can cut off emerging output;
material-progress callbacks are the correct control signal.

Evidence: `v21-r3-artifact/summary.json`, `state.db`, `timeline.jsonl`,
`homes/studio-lead/project-state.md`, and both preserved Work workspaces.

## Scorecard

The score is diagnostic, not a product metric. Every run without a concrete attributable artifact is
capped at 39; false completion is capped at 29.

| Run | Accepted /30 | Coordination /20 | Recovery /15 | Review /15 | Efficiency /10 | Harness /10 | Raw | Cap | Final |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `v21-r2-single` | 20 | 0 | 0 | 8 | 8 | 10 | 46 | false completion: 29 | **29** |
| `v21-r3-graph` | 0 | 7 | 11 | 0 | 3 | 8 | 29 | no artifact: 39 | **29** |
| `v21-r3-artifact` | 0 | 9 | 12 | 11 | 4 | 10 | 46 | no artifact: 39 | **39** |

Point rationale:

- single-agent accepted points recognise the exact coherent feature commit and material progress;
  failed native checks remove check points, and the unsupported completion decision triggers the cap;
- graph-control coordination points recognise explicit ownership and initially sensible parallel
  dependencies, while broad scope, missing handoffs and blind repairs lose the rest;
- artifact-led points recognise focused context, independent seams, direct native inspection,
  project-state continuity, exact evidence and clean terminal records; no advanced artifact means no
  accepted-outcome points; and
- harness points reflect exact launch records, chronological ACP events, write scopes, brokered
  credentials and model/usage capture. Graph loses cancellation-fidelity points because three turns
  lacked terminal records in that run.

## Static defects found and disposition

Fixed in the scratch lab:

1. candidate checks could run without their servers;
2. candidate success ignored `[FAIL]` markers and `errors observed` counts;
3. catalogue presence was mistaken for exact runtime connectability;
4. self-commission allowed the paid lead to masquerade as Staff;
5. workers received the full broad scenario instead of distilled project state;
6. cancellation stopped only the local client, not the exact container process tree;
7. concurrent controller/global cancellation could leave turn rows open; and
8. shutdown cancelled all actors before allowing already-terminal callbacks to settle.

Still open:

1. the Company Runtime refreshed registry omits `stealth/ox-alpha` and
   `z-ai/glm-5.2:free` although the live OpenRouter catalogue lists both as free and tool-capable;
2. model allocation sent both first artifact responsibilities to Poolside instead of diversifying
   the pool;
3. an envelope warning woke the lead while workers were still running and was initially interpreted
   as an artifact callback;
4. progress streams are stored as diagnostics but not reduced into meaningful artifact/proof/stall
   callbacks;
5. fixed prompt and global timeouts conflate provider silence, reasoning churn and productive work;
6. the untouched seed's battle suite is nondeterministic or environment-sensitive;
7. a model may emit a malformed tool call and leave a long provider-latency gap;
8. the current live path creates a new OMP ACP session per wake, so actor identity and workspaces are
   durable but model conversation and cache affinity are not; and
9. run summaries do not expose a first-class terminal reason/wall duration even though the terminal
   event records elapsed time.

## Enduring conclusions

### What is structural

- One accountable owner is required for a coherent outcome.
- Rich meaning belongs in the artifact and concise situation model, not the Work graph.
- Communication should be asynchronous, targeted and consequential.
- A retry is useful only when the accountable intelligence changes its hypothesis—scope, model,
  context, tools or approach.
- Teams create value through difference in capability, context or perspective, not agent count.
- Native outcome evidence must outrank narration and internal status.
- Deterministic substrate should preserve authority, identity, process truth and recoverability; it
  should not substitute for coordination judgement.

These principles generalise to human teams. Humans also coordinate through accountable ownership,
shared artifacts, bounded commitments, feedback and direct observation; an org chart or ticket graph
cannot carry the full semantic state of the work.

### What remains contingent

- Sonnet 4.5's exact leadership quality;
- the suitability of any named free model;
- optimal worker packet size;
- whether two speculative workers beat one sequential worker;
- provider latency and cache behaviour; and
- the best wake/progress thresholds for each work domain.

## Recommended target

Use a durable event-driven Actor Host:

```text
OrgIntel
  ↕ commands and material events
Runtime Bridge
  ↕ common actor-host contract
  ├── native persistent Pi host (preferred observable path)
  ├── ACP adapter (Codex, Claude, OMP and future harnesses)
  └── direct model-call lane (small structured cognitive tasks)
```

ACP remains useful as a replaceable process/session transport. It is not the organisational memory or
inter-agent semantic substrate. Agents coordinate through OrgIntel commitments and messages plus
ordinary shared artifacts. The current lab keeps the Linux environment and actor homes warm but
starts a new OMP ACP session each wake; no explicit prompt-cache affinity was observed.

Replace total Work timeouts with callback-driven work, artifact-driven progress and judgement-driven
completion. Retain narrow clocks only for transport liveness, subprocess supervision, authority
leases, budgets and shutdown. The Runtime should emit factual material events—workspace changed,
proof ran, commit created, blocker reported—and OrgIntel should wake the lead only after reducing raw
Pi/ACP events into a meaningful change.

## Next decisive evidence

Do not launch another full milestone immediately.

1. Make Ox Alpha and GLM 5.2 visible through the exact Company Runtime/gateway path.
2. For each model, run a capability ladder: tool call, tiny real edit plus proof, then one bounded
   contribution.
3. Admit only workers that produce a valid artifact at each rung.
4. Compare Sonnet alone with Sonnet leading the admitted workers on one stable native outcome.
5. Optionally run the same small Work in isolated speculative branches, accept one proven result and
   purge the loser.
6. Port only the minimal mechanisms that improve accepted output; keep the entire lab scratch-only
   until that evidence exists.
