# v23 — matched Sol/Terra team versus Sol alone

Status: matched experiment complete; scratch control-plane repairs verified; strong singleton wins this
slice; production architecture recommendation updated

## Decision

> **Architectural correction recorded 23 August 2026:** this run conflated the company Exec with the
> project team lead. Its singleton result remains valid, but the singleton belongs below Exec. Exec
> always dispatches executable owner work to an accountable standing or temporary team lead and
> returns to availability. The lead then works alone for tightly coupled work or commissions Staff for
> separable work. See `docs/adr/0005-exec-dispatches-through-accountable-leads.md`.

For this tightly coupled Loop 4 game slice, **GPT-5.6 Sol working alone was decisively better than
GPT-5.6 Sol leading one GPT-5.6 Terra Staff member**.

Both arms started from exact seed `514b7b3d0a65e093af608b08ca142344412181f4`, received the same
scenario bytes (`sha256:99cd8b59f42dff6688e8f4a829534480e70a8502ca8cbfe85d1c428c628776da`),
used the same 1,200-second work envelope plus a 120-second drain allowance, and ran through the same
scratch coordinator. Both used ChatGPT OAuth through the official Codex CLI JSONL interface. YOLO was
enabled only inside these isolated scratch runs.

The single arm:

- finished in 736.80 seconds (12.28 minutes), 41.7% less wall time than the team;
- used one model turn, 35 tool calls and 2,639,249 reported tokens;
- recorded an explicit run-complete decision;
- produced clean commit `7e08c2ee899cae56dc2ee2ce42fd9c23d05c483a`;
- passed 58 evaluator browser assertions with zero browser errors;
- passed a separate independent weakened-bond unlock probe; and
- produced the visibly stronger gate and cavern review targets.

The team arm:

- finished at the drain boundary in 1,264.38 seconds (21.07 minutes);
- used nine model turns, 117 tool calls and 9,588,678 reported tokens;
- never recorded a run-complete decision;
- produced clean commit `8a7f680d0d21c09eeb12912a0a5cd8f1f1ad3cd5`;
- passed 61 evaluator browser assertions with zero browser errors; but
- failed its own independent critic's native visual judgement.

The team used 3.63× the reported tokens, 3.34× the tool calls and 1.72× the wall time. Cache ratios
were essentially identical: 95.80% team versus 95.70% single. Cache hotness was healthy and was not
the differentiator.

Do not make a Work graph or a standing team the default. Keep **Work** as an optional, sparse record of
one bounded cross-actor commitment. Default to one durable, capable, accountable lead working directly
in the canonical workspace. Let that lead commission Staff only when it can name a low-coupling native
artifact whose expected value exceeds the handoff and integration cost. Commission independent critics
more readily than parallel implementers.

This is one controlled pair, not statistical proof. It is nevertheless sufficient to reject the
current default architecture for this class of work: the team did not create more accepted value per
lead turn, per wall minute, or per token than the same strong lead alone.

## Matched setup

| Dimension | Team arm | Single arm |
|---|---:|---:|
| Run | `v23-matched-team` | `v23-matched-single` |
| Mode | `artifact_led` | `single_agent` |
| Seed | `514b7b3...` | `514b7b3...` |
| Scenario hash | `99cd8b59...` | `99cd8b59...` |
| Lead | `gpt-5.6-sol`, medium reasoning | `gpt-5.6-sol`, medium reasoning |
| Staff | one `gpt-5.6-terra`, low reasoning | none |
| Max Staff concurrency | 1 | 0 |
| Work envelope | 1,200 s | 1,200 s |
| Drain grace | 120 s | 120 s |
| Actor permissions | scratch YOLO | scratch YOLO |
| ACP/Pi | neither; official Codex CLI JSONL | neither; official Codex CLI JSONL |

The frozen mission is [`../v22/matched-mission.md`](../v22/matched-mission.md). It required one coherent
starter → guardian → existing Battle/Bond → shared unlock → authored cavern → return loop, native
browser evidence, review screenshots and no broad unrelated systems.

Order is a caveat. The single arm ran after the team arm on the same machine, although it received a
fresh model session, an exact seed checkout and no team artifacts in its prompt. Its trace shows no
reads of the team checkout. Machine-level browser installations and caches could still create a small
order effect. Both arms nevertheless independently spent time rediscovering browser tooling, so the
dominant capability-manifest defect reproduced rather than disappearing.

## Quantitative comparison

| Measure | Team | Single | Team / single |
|---|---:|---:|---:|
| Run elapsed | 1,264.38 s | 736.80 s | 1.72× |
| Model turns | 9 | 1 | 9.00× |
| Tool calls | 117 | 35 | 3.34× |
| Reported used tokens | 9,588,678 | 2,639,249 | 3.63× |
| Reported input tokens | 9,543,030 | 2,618,135 | 3.64× |
| Cached input tokens | 9,142,400 | 2,505,600 | 3.65× |
| Cache ratio | 95.80% | 95.70% | effectively equal |
| Serial model time | 1,569.1 s | 710.8 s | 2.21× |
| Browser assertions | 61 | 58 | different proof decomposition |
| Work records | 3 | 0 | — |
| Attempts | 4 | 0 | — |
| Run-complete decision | no | yes | — |

Reported token counts include cached input replay and are a host-throughput/context proxy, not a bill.
The OAuth client reported `$0.00`, so this experiment cannot compare monetary cost. The near-identical
cache ratios show that persistent sessions were already hot; adding more cache would not repair the
coordination or quality failures.

The team changed 8 files with 336 insertions and 11 deletions from the seed. The single changed 12 files
with 528 insertions and 32 deletions. The single implementation was broader, mostly because it also
introduced one candidate-owned browser-runtime locator and repaired inherited proof portability and
timing. Smaller diff size did not translate into the stronger experience in this run.

## Team arm

### Product work

Sol commissioned two bounded producer responsibilities around one shared `prismUnlocked` contract:

1. `gameplay-systems` implemented the visible corrupted guardian through the existing Battle and Bond
   seams in commit `2b870387537e5791c16cd0e50afde49c623c9c7b`.
2. `experience-presentation` implemented the gate and authored cavern module in commit
   `07483a2c65f563fdcb56b3988ab3cc7366a02533`.

Both completed on revision 1. There were no producer retry loops and no blocked producer Attempts. The
lead cherry-picked both contributions and used integrated screenshots to discover two defects that
component proofs had missed:

- cavern input consumed `F` before guardian Battle could start; and
- the cavern leaked Basin composition and read as a dark, shallow patch.

The lead repaired those integration seams and reached candidate `6f459a3...`. It later repaired two
proof defects, finishing at `8a7f680...`.

This is meaningful coordination, not empty graph churn. The problem is economic and qualitative: the
coordination did not outperform direct end-to-end work by the same lead.

### Independent critic

Sol commissioned `artifact-critic` after integration. Terra independently exercised the runnable
candidate, confirmed the functional loop and committed a strong severity-ranked
[`REVIEW.md`](../../v2/workdir/v23-matched-team/workspaces/work-198ce6804b/REVIEW.md).

Its judgement was **FAIL**:

- **High:** the documented blue return route was behind the arrival camera and was not discoverable in
  `screenshot-loop4-cavern.png`.
- **Medium:** the purported Prism Heart was a small pale diamond competing with similar crystals; empty
  floor and dark ceiling silhouettes dominated the composition.

The critic passed the gate screenshot and confirmed real guardian Battle, victory/bond unlock,
cavern transition and return functionality. This is strong evidence that independent native review is
valuable even when parallel implementation is not.

The critic twice ended after committing its report without calling the required terminal `report`
tool. OrgIntel marked both Attempts `unknown`/blocked. The lead reopened the same Work as revision 2,
which avoided duplicate Work but spent another model turn. At drain, the critic artifact existed and
the model had explained its findings, yet the canonical organization still treated the review as
outstanding. The lead therefore never judged or repaired the high-severity finding.

This is a host/protocol defect, not evidence that Terra lacked judgement. The same Terra model produced
the most incisive assessment in the team run; it failed a brittle final callback ceremony.

### Team outcome

Final commit `8a7f680d0d21c09eeb12912a0a5cd8f1f1ad3cd5` is executable. The repaired isolated
evaluator passed all five proofs and preserved a clean checkout:

- battle: 12;
- combat-extra: 7;
- guardian: 6;
- Prism Caverns: 7;
- roster/evolution: 29;
- total: 61, zero browser errors.

The exact post-repair evidence is
[`875c256d429e7077.json`](../../v2/workdir/v23-matched-team/context/candidate-evidence/875c256d429e7077.json).

Automated checks are green, but the independent native review remains a product-contract failure.
The candidate is functionally good and visually weaker than the single arm; it is not an accepted
completion of the whole observable contract.

## Single arm

Sol stayed in one durable turn and implemented the complete milestone end to end. It independently
encountered and repaired:

- the same missing browser-runtime capability information;
- milestone interaction/render integration errors;
- inherited Battle proof timing flakiness; and
- proof portability across the host and evaluator runtime.

It committed once at `7e08c2ee899cae56dc2ee2ce42fd9c23d05c483a` and recorded an explicit complete
decision.

The repaired isolated evaluator passed:

- battle: 12;
- combat-extra: 7;
- Prism milestone: 10;
- roster/evolution: 29;
- total: 58, zero browser errors;
- runtime locator: pass, with no gameplay assertion count.

The exact post-repair evidence is
[`9fc7fb477e32ea1b.json`](../../v2/workdir/v23-matched-single/context/candidate-evidence/9fc7fb477e32ea1b.json).

The milestone proof exercised the victory unlock path but not successful weakened bonding. A separate
independent browser probe, [`verify-single-bond.mjs`](verify-single-bond.mjs), opened the real weakened
Bond flow, completed it, observed `prismUnlocked`, guardian settlement, barrier removal, a return to
play mode and zero browser errors. Its recorded result is
[`single-bond-proof.json`](single-bond-proof.json).

### Independent visual judgement

The single gate capture has a clearer hierarchy: named corrupted guardian, large readable seal, strong
gate silhouette and explicit confrontation prompt. The single cavern capture has an unmistakable
central landmark, varied crystal clusters, large enclosing rock silhouettes, height/depth, a lit side
path and an immediate `G Return to Sunleaf Basin` prompt.

The return arch itself is behind the arrival camera, so the geometry is not visible in the initial
capture. Unlike the team candidate, the return action is immediately discoverable through the central
contextual prompt. This is a remaining presentation compromise, not the team's high-severity invisible
route failure.

Review targets:

- [`screenshot-prism-gate.png`](../../v2/workdir/v23-matched-single/canonical/screenshot-prism-gate.png)
- [`screenshot-prism-cavern.png`](../../v2/workdir/v23-matched-single/canonical/screenshot-prism-cavern.png)
- [`screenshot-loop4-gate.png`](../../v2/workdir/v23-matched-team/canonical/screenshot-loop4-gate.png)
- [`screenshot-loop4-cavern.png`](../../v2/workdir/v23-matched-team/canonical/screenshot-loop4-cavern.png)

## What the control-plane repairs achieved

### Two-phase candidate delivery

The scratch `report(outcome_met)` path now observes the exact clean commit, runs declared gates, checks
the workspace again and only then records an accepted artifact and terminalises the Attempt. A failed
gate returns `revision_required` while keeping the same Attempt live.

The deterministic fault suite passed 37/37, including:

- failed gate leaves the same Attempt running;
- failed gate records no accepted artifact;
- the same Attempt can repair and submit again;
- only the exact observed commit becomes the accepted artifact; and
- duplicate/stale callbacks remain harmless.

Neither live producer happened to trigger a declared-gate failure, so live-model evidence for that
specific branch remains absent. The old v22 terminalisation/recommission loop did not recur: both live
producer Work items completed on revision 1.

### Runtime identity

Before every Codex turn, the Actor Host verified the exact workspace, Git root and expected branch.
Resumed sessions inherited the exact subprocess working directory, and Staff session keys included
actor, Work and workspace input epoch. No v22-style parent-repository drift recurred.

The architecture suite explicitly accepted the right branch and rejected a wrong branch before model
launch.

### Event-driven coordination and graceful drain

The runner wakes actors through an in-process event queue on material coordinator events. Leads are
instructed not to poll active Attempts. A timer exists only for scheduled wakes and the outer run
envelope; it does not define semantic work completion.

At 1,200 seconds, the team run entered drain with Sol and the critic active. Dispatch stopped, both
turns were allowed to end, and the run terminalised 64.19 seconds into the 120-second grace period.
No productive process was killed. This is a successful replacement for the v22 hard envelope kill.

The remaining failure was semantic: an ended actor with a committed artifact but no terminal callback
was still marked unknown.

### Evaluator isolation

The first v23 evaluator fix removed shared long-lived hidden servers and retried a legacy client-only
proof only after an exact connection-refused error, using isolated ephemeral fixtures. This removed
`EADDRINUSE`, but the team still emitted 19 fixture retries and the single emitted 3 during their
original runs. Failure-first fixture discovery is slower than a declared proof contract.

The matched run exposed a second evaluator bug: `verify-prism-milestone.mjs` captures screenshots, so
the original post-run evaluator rewrote the two committed single screenshots in the canonical checkout.
Its saved old-schema evidence incorrectly retained `checkout_clean: true` because status was sampled
before the gates.

The mutated copies are preserved as:

- [`evaluator-mutated-single-gate.png`](evaluator-mutated-single-gate.png)
- [`evaluator-mutated-single-cavern.png`](evaluator-mutated-single-cavern.png)

The scratch evaluator now exports the exact candidate commit to an isolated temporary review directory,
runs every proof there, deletes that exact temporary directory, and records pre/post HEAD and status.
The architecture suite passes 20/20. Fresh re-evaluation of both matched candidates passed every proof
with `workspace_integrity_passed: true` and clean canonical checkouts before and after.

## Remaining structural defects

### 1. Missing terminal callbacks discard observable completed work

Current behavior maps “actor process ended without terminal `report`” to unknown Attempt, even if:

- the runtime observed a new clean commit;
- the expected artifact exists;
- the actor's final text identifies it; and
- native review can open it.

The Actor Host should emit `candidate_observed` and `report_missing`, attach the exact commit and changed
files as recoverable evidence, and wake the accountable lead with the artifact. It should re-prompt the
same session only if required information is actually missing. A missing callback must not make a
completed artifact invisible or force a second full model turn.

The lead still decides whether the artifact satisfies the outcome. This is evidence recovery, not
deterministic semantic acceptance.

### 2. Runtime capabilities are rediscovered by every actor

Team Sol, team Terra, the team critic and single Sol all spent time finding Playwright, Chromium and
fixture commands. The single eventually introduced `verify-runtime.mjs`, which checks the evaluator
path and one ChatGPT-host path. That was a useful local repair, but hard-coded product-specific paths
inside a game artifact are not the enduring system boundary.

The Runtime Bridge should live-probe once and inject a small observed capability manifest into every
turn, for example:

- browser proof command/adapter;
- browser executable and module availability;
- static fixture ownership and lifecycle;
- screenshot/output location;
- exact workspace, branch and writable scope; and
- tool versions where they materially affect execution.

Models choose what evidence to gather. The host supplies the already-observed means to gather it.

### 3. Proof fixture ownership is still implicit

Connection-refused retry is a compatibility shim, not a 10× design. A project-native proof should
declare or own its fixture through one stable executable entrypoint. The Runtime Bridge can then run a
proof in an isolated review checkout without parsing source for localhost ports or deliberately failing
once.

### 4. Work status can dominate outcome truth

The team had a clean executable candidate and a high-quality independent review artifact, yet its run
could not complete because the critic Work was blocked by protocol. The single had zero Work or Attempt
records and completed successfully.

OrgIntel must project Work as recoverable coordination, never as the definition of outcome truth. The
authoritative completion question is whether the accountable actor judged an observable native
candidate against the success contract with bound evidence.

### 5. Context replay remains large despite excellent cache

The team lead consumed five turns and 7.24 million reported tokens; the whole team consumed 9.59
million. Most input was cached, which helps provider cost, but does not eliminate wall latency, tool
rediscovery, attention dilution or integration burden.

Keep actors hot, but checkpoint semantic state: current outcome, accepted refs, review findings,
decisions, risks and next proof. Do not mistake a high cache ratio for organizational intelligence.

## 10× target architecture

### Default path: accountable singleton

1. One strong durable lead owns the outcome, canonical workspace and completion judgement.
2. The lead works directly while the slice is tightly coupled.
3. The Runtime Bridge supplies observed capabilities and emits material process/artifact events.
4. Native executable evidence is prepared continuously, not only at terminal report.
5. The lead records completion only against a clean evidence-bound candidate.

### Optional path: pull-based Staff

Create Work only when the lead can state all of the following:

- one accountable owner;
- a bounded outcome, not a role-shaped activity;
- a concrete native artifact or clean commit;
- low shared-state coupling with the lead's current work;
- an acceptance target the lead can independently run or inspect; and
- an expected gain large enough to repay context, integration and review cost.

The lead may keep working while Staff runs, but material callbacks—not polling—drive reintegration.
The host salvages observable artifacts if the callback ceremony fails.

### Review path: independent critic

Independent review is the part of the team design that clearly paid for itself. A critic receives the
runnable candidate, success contract and prepared review target, but not producer reasoning. Its report
is imported automatically from the observed artifact and presented to the lead for judgement.

Review should usually be a fresh session and may be a different model. It does not need a permanent
place in a standing org chart.

### Sparse Work substrate

Keep the Work table. Purge the Work graph as the default plan.

Work is useful for cross-actor responsibility, dependency, lease and recovery. It is not needed for
the singleton's ordinary local steps, thoughts, edits or tests. The graph should emerge only from real
delegation and disappear from the primary mental model when no delegation exists.

## ACP, opacity and first-party harnesses

This matched result did not use ACP or the Pi SDK. The scratch `codex_turn.py` adapter used official
`codex exec --json` and `codex exec resume`, preserving session IDs and exposing runtime identity,
tool starts/completions, model text, usage, cache data and terminal process state.

Therefore the reproduced failures are not evidence that ACP is fundamentally bad. They occurred above
the transport boundary: brittle terminal ceremony, missing capability context, implicit fixtures and
Work status eclipsing artifacts.

ACP can remain one replaceable Actor Host adapter if it emits or can be reduced to the same material
facts. If an ACP provider exposes too little process/tool/artifact state to support recovery, it should
fail admission or be used only for lower-authority bounded Work. Restless should not make opaque ACP
processes its source of organizational truth.

## Generalisation beyond coding and to human teams

The enduring principle is **minimum sufficient organization**.

Tightly coupled work benefits from one owner holding the whole causal model. Parallelism helps only
when work can be separated by a real artifact boundary. Every additional contributor creates context,
communication, integration and review costs. Independent criticism often separates more cleanly than
parallel production because it consumes a finished native artifact rather than co-owning mutable state.

This is true of human teams as well as model teams. It is not an argument for lone work everywhere.
It is the familiar condition for useful delegation: specialization or parallel latency must exceed the
coordination tax.

The architecture generalises to non-coding work by replacing commits and browser builds with the
domain's native artifacts:

- a rendered document or deck;
- a reconciled spreadsheet;
- a campaign draft plus preview;
- a prepared browser session;
- a research memo with inspectable sources; or
- an externally observable effect and receipt.

The same rules survive: one accountable lead, optional bounded commitments, observable artifact-first
recovery, event-driven callbacks, independent native review and owner judgement at the prepared last
mile. Non-coding dogfood is still required before claiming measured superiority in those domains.

## What to build next

Do not broaden the Work graph and do not run another large game mission yet.

1. Add artifact-first completion recovery for a process that ends without `report`.
2. Add one live-probed Runtime capability manifest and stable native-browser proof interface.
3. Replace failure-first port parsing with a declared/self-owned proof fixture contract.
4. Make the singleton the default execution path; make Work creation a lead judgement, not a required
   decomposition step.
5. Keep independent artifact review as an optional fresh-session stage.
6. Run three smaller repeated trials across coupling levels: tightly coupled, clearly separable and
   review-only delegation.
7. Run one non-coding native-artifact trial before generalising performance claims.

Only promote mechanisms that improve accepted native outcome per wall time and per lead turn. Work
counts, Attempt counts, cache ratios and passing internal schemas are diagnostics, not the product.
