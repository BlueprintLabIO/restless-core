# v22 — ChatGPT OAuth, Sol lead, Terra Staff

Status: experiment complete; clean functional candidate accepted; orchestration mechanics require repair

Follow-up: the matched repaired rerun and Sol-alone comparison are in
[`../v23/report.md`](../v23/report.md). That experiment supersedes the default-team recommendation for
tightly coupled work: one strong accountable lead won decisively, while optional independent review
remained valuable.

## Decision

The artifact-centred mission-cell architecture works materially better when both the accountable lead
and Staff are capable. GPT-5.6 Sol led GPT-5.6 Terra to a functional integrated milestone during
19 minutes 37 seconds of orchestrated wall time. The envelope interrupted Sol's final verifier edits;
those edits were preserved, independently validated and committed during post-run finalisation. Both
delegated contributions were accepted and integrated. The final clean candidate is
`74f163ddd76782d5bdf7abac64f7d3a1f9546dbc`.

This is a **conditional architecture pass**, not a production-readiness pass:

- the team produced meaningful work rather than graph churn;
- the lead exercised strong product and integration judgement;
- persistent sessions achieved 95.55% cached input across completed turns; but
- report terminalisation, artifact binding, fixture ownership and deadline draining all exposed
  deterministic coordination bugs.

Keep the Work graph as a sparse responsibility and dependency map. Do not let it become the plan,
semantic project state, integration mechanism or evidence of progress. The decisive intelligence lived
in the persistent lead, concise project state, ordinary Git artifacts and native browser review.

## Setup and authentication

Observed on the host before launch:

- Codex CLI `0.149.0` was already logged in using ChatGPT OAuth;
- a fresh Terra readiness turn returned `TERRA_READY` in 4.84 seconds;
- a fresh Sol readiness turn returned `SOL_READY` in 4.19 seconds;
- no new OAuth ceremony was required;
- no ChatGPT credential was copied into a company container; and
- Sub2API was not installed or placed between Restless and OpenAI.

This run used a new scratch-only adapter, `v2/codex_turn.py`, around the official Codex client. It is
not the Pi SDK. It launches the model on the host, exposes only the OrgIntel MCP surface and the exact
actor workspace, captures Codex JSONL events, and persists session IDs:

- lead session: one durable session keyed by actor;
- Staff session: one durable session keyed by actor plus Work ID;
- actor instructions: the existing role system file;
- coordination: typed OrgIntel calls over MCP;
- artifact work: ordinary files and Git in the persistent Work workspace; and
- model telemetry: session, tool, text, usage, cached-input and terminal events.

The second Terra admission passed without YOLO using Codex auto-review plus explicit MCP preapproval.
The full mission used `COORD_CODEX_YOLO=1` because the owner explicitly authorised it. YOLO was confined
to the isolated lab and is not a proposed Restless authority policy.

## Admission ladder

### Terra attempt 1 — truthful launch-contract failure

Run: `v22-terra-codex-admission`

- 71.0 seconds, nine tool calls;
- 330,973 cumulative tokens, 301,824 cached input, 2,672 output;
- created the requested file;
- Codex workspace policy denied `.git/index.lock`;
- the noninteractive MCP report call was also denied; and
- the Work ended `blocked/unknown`, with no false success.

This was a harness failure, not a Terra capability result. Adding the exact Git write approval path and
preapproving the lab MCP surface repaired the launch contract.

### Terra attempt 2 — admitted

Run: `v22-terra-codex-admission-r2`

- 41.2 seconds, five tool calls;
- 143,142 cumulative tokens, 124,928 cached input, 1,305 output, 214 reasoning output;
- exact requested file;
- clean commit `4454dc827fb78b90c74c15bc17a591650dcccda9`; and
- successful terminal OrgIntel callback, Work `completed`, Attempt `produced`.

For context, Ox Alpha passed the tiny raw artifact probe in 30.3 seconds, but its full ACP callback
admission took 343.9 seconds and seven tools. GLM 5.2 Free produced the exact tiny artifact after
215.4 seconds but never emitted the required completion marker, so it failed strict admission. These
facts show why catalogue presence and even a created file are insufficient: the actor must complete the
whole artifact-plus-callback contract.

## Full mission

Run: `v22-sol-terra-artifact`

Frozen configuration:

- seed `514b7b3d0a65e093af608b08ca142344412181f4`;
- mode `artifact_led`;
- lead `gpt-5.6-sol`, medium reasoning;
- Staff `gpt-5.6-terra`, low reasoning;
- maximum one Staff process at a time;
- persistent lead and Work sessions;
- no per-turn work timeout;
- 1,200-second outer safety envelope; and
- subscription-backed calls recorded as USD 0 by the scratch gateway.

Sol chose one coherent milestone from the runnable seed: defeat or calm a corrupted gate guardian,
then enter a handcrafted Prism Caverns arrival chamber. It commissioned two bounded contributions:

1. World: one cavern module, minimal wiring, bidirectional gated transition and browser proof.
2. Gameplay: one guardian module, minimal wiring, existing Battle/Bond reuse, shared unlock flag and
   browser proof.

This was materially better scoped than the earlier Sonnet/Ox mission, which assigned a whole trainer
system, recurring rival, mini-boss and a broad 60–80 metre cave with 80–120 props.

### Outcome

- orchestrated wall time: 1,177.3 seconds;
- nine turns: eight clean completions and one outer-envelope termination;
- 100 recorded tool calls;
- 6,181,542 cumulative tokens across completed turns;
- 6,146,372 input tokens, of which 5,872,896 were cached (95.55%);
- 35,170 output tokens and 9,968 reasoning-output tokens;
- both Work items completed after one same-owner repair each;
- final candidate: `74f163ddd76782d5bdf7abac64f7d3a1f9546dbc`;
- delta from seed: 413 insertions and 21 deletions across nine files; and
- five clean candidate commits: cavern, coherence repair, guardian, portable battle proof, and
  operator-finalised self-contained verification based on Sol's interrupted edits.

The large input number is cumulative across every model continuation and therefore repeatedly counts
the growing conversation. The 95.55% cached share proves that sessions remained hot; it does not make
the six-million-token context loop free or desirable. Long lead review turns grew to 1.8 million
cumulative tokens. Semantic checkpoints and deliberate session rollover are still needed.

### Native evidence

The final evaluator ran every tracked verifier in its normal sorted order against the clean candidate:

| Proof | Assertions | Result |
|---|---:|---|
| Ordinary battle | 12 | pass, zero browser errors |
| Cavern gate, entry, return and render continuity | 9 | pass, zero browser errors |
| Two-sided combat and guard | 7 | pass, zero browser errors |
| Guardian, unlock and live cavern transition | 6 | pass, zero browser errors |
| Roster and evolution | 29 | pass, zero browser errors |
| **Total** | **63** | **all pass** |

Exact machine evidence is in
`v2/workdir/v22-sol-terra-artifact/context/candidate-evidence/7fabd4f80c4d583a.json`.

The screenshot is outcome evidence, but not evidence of polish. Independent visual inspection finds a
functional, readable and visually distinct chamber, but it is sparse: a flat dark floor, isolated
crystal clusters, little spatial composition and limited environmental storytelling. It does not yet
meet the requested “mysterious, beautiful, quiet, slightly dangerous” bar, and it is far from an MMO
content bar. The run succeeded as a coherent playable systems slice, not as a polished game milestone.

## What the stronger team did differently

Sol showed useful coordination judgement rather than merely issuing tasks:

- inspected the runnable seed before decomposing;
- chose one shared product milestone;
- reduced each Staff packet to one module, one seam and one executable proof;
- kept a concise persistent situation model;
- refused stale or failed handoffs despite plausible worker narration;
- integrated exact commits and resolved overlaps itself;
- drove the combined candidate in Chromium;
- found the companion more than 16 metres below the cavern floor;
- found the camera more than 50 metres behind the teleported player;
- separated gameplay regressions from Playwright, WebP and server-fixture failures; and
- repaired integration coherence before moving on.

Terra also produced meaningful work. Its first cavern and guardian Attempts created real modules,
focused proofs and clean commits within roughly 4.7 and 3.7 minutes. Its repair turns were narrower
and faster: 80.4 and 106.4 seconds. It truthfully described callback rejection rather than claiming
completion.

The contrast with `v21-sonnet-ox-artifact` is substantial:

| Run | Wall observation | Candidate | Work result |
|---|---:|---|---|
| Sonnet lead + Ox Alpha | about 83.5 minutes from first start to terminal envelope | unchanged seed | both Work blocked/unknown |
| Sol lead + Terra Staff | 19.6 minutes | clean advanced candidate | both Work completed/produced |

This is not a controlled model-only comparison. The provider path, session persistence, assignment
size and launch contract also changed. The enduring conclusion is that capable judgement plus the
artifact-centred architecture can work; the exact share attributable to Sol, Terra or the new harness
requires a matched experiment.

## Reproduced coordination defects

### 1. Terminal report and verification are incorrectly fused

Reproduced independently on both Staff contributions:

1. Staff called terminal `report(outcome_met)` with a clean candidate.
2. OrgIntel synchronously ran the declared gate inside that call.
3. One environment-sensitive gate failed.
4. OrgIntel marked the Attempt failed and revoked the lease while the actor process remained live.
5. Staff repaired the proof and amended the commit.
6. The corrected report was rejected because the Attempt was no longer running.

This forced two redundant Work revisions. The fix is a two-phase delivery protocol:

- `submit_candidate` is nonterminal and binds a candidate ref;
- independent verification produces factual results;
- the accountable reviewer accepts or requests revision; and
- only acceptance, voluntary block/abandon, cancellation or process death terminalises the Attempt.

A failed gate means “revision required,” not “the producer no longer exists.”

### 2. Artifact records can precede valid evidence

The failed reports wrote artifact rows before verification completed. The first cavern row named
`e4855fc…`, but Staff later amended the branch to `e0888b5…`; the stale ref was not available in the
canonical checkout. Sol had to inspect the producer workspace and reconstruct the true handoff.

Artifact acceptance must be transactional:

1. resolve and fetch the exact immutable ref into an integration inbox;
2. verify reachability, clean provenance and declared proof;
3. persist the accepted artifact and Attempt transition atomically; and
4. never mutate an accepted identity through amend.

### 3. Deadline handling kills productive work instead of draining it

The 20-minute envelope fired while Sol was running the final portability proof. The process stopped
with five useful verifier edits uncommitted. The edits were preserved and independently validated, but
the run required manual finalisation.

An outer budget envelope remains legitimate. It should enter a drain state:

- stop launching new Work;
- emit a checkpoint request to active actors;
- preserve session, workspace and current tool state;
- allow a bounded transport/process grace period; and
- hard-kill only after that grace period or an authority/budget invariant demands it.

The envelope is a safety boundary, not the semantic definition of work completion.

### 4. Fixture ownership was split

The post-run evaluator implicitly launched ports 8124 and 8231 while the new verifiers also launched
their own servers. A healthy guardian proof failed with `EADDRINUSE`. The first scratch repair removed
shared long-lived hidden servers and used an isolated ephemeral fallback only after exact
connection-refused evidence. V23 showed that this remains a compatibility shim: most legacy proofs do
not own fixtures, so failure-first retries still add latency. The enduring target is a declared or
self-owned proof fixture executed from an isolated candidate checkout.

### 5. Runtime identity needs a checked launch invariant

On one resumed Staff turn, Codex initially surfaced the dirty parent repository rather than the claimed
Work checkout. Terra recovered by changing to the explicit workspace, but this must not depend on model
attention. Before every actor turn, the Runtime Bridge should verify the exact working directory,
repository root, Work/Attempt identity and writable scope.

### 6. “Database healthy” is not “outcome healthy”

The run's SQLite `quick_check` remained `ok` while the first post-run candidate evidence was dirty and
one proof failed. Storage health, coordination health and outcome acceptance need separate named
signals.

## Architecture judgement

### Keep

- one accountable, persistent lead with canonical write ownership;
- durable actor identity and per-Work sessions;
- sparse Work records for responsibility, dependencies and leases;
- ordinary files, Git and native review targets;
- asynchronous material callbacks; and
- deterministic controls for authority, exact runtime identity, budgets and process truth.

### Purge

- broad feature-sized worker packets;
- Work graphs as semantic project plans;
- synchronous terminal callbacks that also verify and judge;
- implicit shared test fixtures;
- fixed per-turn work timeouts;
- completion inferred from process exit or a model's narration; and
- permanent retention of raw reasoning streams as organisational memory.

### ACP and opaque processes

ACP is not fundamentally the problem. The official Codex JSONL path exposed session IDs, tool starts
and completions, model text, usage, cached input and terminal state while preserving a hot model
session. ACP should remain a replaceable actor-host adapter, not the organisational substrate.

Inter-agent communication should continue through commitments, targeted messages, material callbacks
and shared artifacts. Do not make agents maintain a free-form group chat. The Actor Host should reduce
raw process events into facts such as workspace changed, commit created, proof passed, candidate
submitted, blocker reported and process lost.

## 10× structural target

1. **Durable Actor Host** — exact session, workspace, process and telemetry ownership behind one
   common adapter contract for Codex, Pi, ACP and direct model calls.
2. **Two-phase delivery** — submit candidate, verify, then accept or revise; callback failure is not
   process death.
3. **Integration inbox** — fetch immutable refs, verify reachability and bind evidence before OrgIntel
   records an accepted artifact.
4. **Material-event reducer** — wake the lead on commits, meaningful deltas, proof results and blockers,
   not token noise or elapsed-time guesses.
5. **Semantic checkpoint** — concise current outcome, decisions, accepted refs, risks and next proof;
   compact or roll sessions from this checkpoint when context growth stops adding value.
6. **Graceful budget drain** — stop dispatch, checkpoint, then terminate only after bounded transport
   grace.
7. **Capability ladder** — live readiness, exact tool/edit/callback admission, then one bounded real
   contribution before a model joins the pool.
8. **Native acceptance** — the lead reviews the runnable site, document, browser state or other native
   outcome; internal logs remain supporting evidence.

This generalises to non-coding work. Replace a Git commit with the domain's native artifact: a rendered
document, reconciled spreadsheet, prepared browser session, campaign draft, signed-off plan or sent
effect receipt. Ownership, bounded commitments, two-phase review, material callbacks and native outcome
inspection remain the same.

## Next decisive experiment

Do not build a broader game slice yet.

1. Repair the two-phase callback and artifact-binding defects in the scratch lab.
2. Add checked per-turn workspace identity and graceful deadline draining.
3. Rerun this exact mission from the same seed to measure whether both redundant revisions disappear.
4. Then run a matched pair: Sol alone versus Sol leading one Terra Staff member, same outcome, seed,
   native proofs and wall budget.
5. Only if the team arm wins on accepted output per lead turn should these mechanisms move toward the
   Restless implementation.

The next question is no longer “can agents coordinate?” They can. It is whether the team creates more
accepted value than the same strong lead alone once the control-plane bugs are removed.
