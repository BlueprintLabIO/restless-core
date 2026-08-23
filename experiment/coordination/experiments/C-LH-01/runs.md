# C-LH-01 run index

Status: paused for the Experiment Sprint 01 founder start signal; no counted arm yet

## Frozen allocation

- Order seed: `C-LH-01:v27:ordinary-crossover:2026-08-23`.
- Order SHA-256: `04d1c44d0edce1beca8f0a68a0ec468e06b09dfcdf326e4b686f88bea45ec92d`.
- Odd first-32-bit parity selected: **B1 → B0**.
- Lead in both arms: `gpt-5.6-sol`, medium reasoning.
- B1 producer: `experience-presentation` on `z-ai/glm-5.2:free`, runtime-default reasoning.
- B1 topology: one accountable lead, exactly one producer, one Staff slot, ordinary artifact handoff.
- B0 topology: the same accountable lead with zero Staff below it.
- Outer envelope: 14,400 seconds per arm; actor completion is callback/process exit, not elapsed time.
- Drain: 120 seconds. Nominal spend ceiling: USD 6 per arm.

## Preflight evidence

- Scenario SHA-256: `bfda2c7d47285c794a9d4284f22164a42fbb5810870894b90c2a00e2de916c12`.
- Evaluator SHA-256: `dba9e9ff15047b4c20f43562efa9e6dc29d699fe7fb4ff0fcd913f9de1bda5d8`.
- Seed negative control: 7/53 pass, 46 intended absent-feature failures, zero browser errors.
- Two independent judges support the large/high-separability work shape; their B0 duration estimates
  disagree materially at 30–70 minutes versus 10–15 hours.
- A fresh evaluator audit found and removed four material sources of false pass/ambiguity before hash
  freeze. Partial tool implementations now retain a fixed result count rather than aborting the run.
- Baseline-isolation/runtime architecture: 39/39 on protocol 24.

## Counted arms

- `v27-clh01-b1-glm` is an attributed preflight-only failure, not an arm: the exact GLM readiness
  inference reached the harness's obsolete 90-second cutoff at 90.50 seconds before any lead turn,
  Work, Attempt or candidate change. The live catalogue still reported the model free and tool-capable.
  The admission envelope was moved beyond the previously observed 120.96-second valid GLM proof;
  actor completion remains callback/process-exit driven.
- `v27-clh01-b1-glm-r2` is an infrastructure-invalid diagnostic, not an arm. It created one Work and
  a clean producer commit, but the fixed 900-second Attempt lease expired while the deliberately
  timeout-free ACP process remained alive. The completed worker could therefore never submit its
  terminal callback. The lead truthfully rejected the absent coordination artifact and completed a
  clean lead-authored candidate. This run also exposed repeated OpenRouter 429 back-pressure and one
  late interface repair; both remain diagnostic observations, not matched-arm evidence.
- The repair renews the exact Attempt lease from the observable supervised Runtime process every five
  minutes. Completion is still callback/process-exit driven; the global run envelope remains the only
  outer bound. A wrong token cannot renew a lease. Fault/recovery verification passed 39 checks and
  baseline-isolation verification passed 39 checks after the change.
- `v27-clh01-b1-glm-r3` is a second infrastructure-invalid diagnostic, not an arm. The exact GLM
  route passed the stronger admission probe in 13.081 seconds, the lead commissioned one Work after
  19 seconds, and the live supervisor renewed its lease after five minutes. That is direct runtime
  proof that the lease repair works. The Staff ACP session nevertheless accumulated 14 observed
  OpenRouter 429 responses in 511 seconds, made no workspace change and produced no callback. It was
  stopped after the repeated provider error evidence, not because a semantic task timer expired. The
  lead's uncounted canonical commit `73a9cfd` was already doing the whole outcome independently when
  the diagnostic was stopped.
- Across r2 and r3, the Staff ACP sessions recorded 65 provider 429 responses. This is `R1` provider
  back-pressure plus `R4` hidden retry telemetry. It is neither evidence that B1 loses nor a reason to
  retry GLM until a lucky run appears.
- No counted B1 Run ID is allocated while the cell is paused. The frozen random order remains
  **B1 → B0**; B0 was not started because doing the second arm without a valid first arm would spend
  matched-comparison compute without closing the cell.

## Restart gates

C-LH may resume only after all of the following are true:

1. The exact candidate Staff route completes a multi-step tool-and-terminal-callback probe. A catalogue
   listing or one-token readiness response is insufficient evidence of sustained availability.
2. Provider rejection and retry state is observable to the supervisor and attributed separately from
   actor reasoning or organisational delay. A provider-saturated turn must reach an explicit circuit-
   breaker outcome without pretending elapsed time semantically completes the Work.
3. When a process ends without callback, the lead receives the observed workspace HEAD/status/diff or
   exact commit handle as recovery evidence. Physical preservation without an exposed handle is not an
   artifact handoff.
4. A replacement provider/model is frozen before a fresh B1 run. `stealth/ox-alpha` is the next
   candidate because it was explicitly requested and previously passed a live write probe; it does not
   become the counted model until gate 1 passes on the exact current route.
5. Only a structurally valid B1 is followed by the frozen B0. Provider-invalid diagnostics remain
   immutable and excluded rather than repaired into evidence by narration.

## GPT-5.6 matched-set restart

The owner subsequently removed free OpenRouter workers from all coordination experiments. This does
not reinterpret the GLM diagnostics; it removes their provider/runtime variable from future arms.

- Accountable lead: `gpt-5.6-sol`, medium reasoning.
- Ordinary Staff: `gpt-5.6-terra`, runtime-default reasoning.
- Direct first-party Terra probe: exact `GPT56_TERRA_READY` response in 4.5 seconds with a normal
  terminal turn and token telemetry.
- Frozen arm order remains **B1 → B0**.
- Fresh matched-set Run IDs: `v28-clh01-b1-terra` then `v28-clh01-b0-sol`.
- The scenario and evaluator hashes remain unchanged. If the first-party actor fails to produce an
  observable terminal handoff, the arm remains invalid rather than being narrated into evidence.

The OpenRouter-specific sustained-route and visible-retry gates remain required before free providers
can be evaluated again. They do not gate this first-party GPT-5.6 matched set. Artifact recovery remains
a production requirement and an attribution rule: a missing callback is `unknown`, never success.

## Founder-stopped GPT-5.6 diagnostic

`v28-clh01-b1-terra` started under the GPT-5.6 routing policy and was stopped by the founder before the
lead integrated both seams, made a final decision or submitted the candidate to the external evaluator.
It is preserved as diagnostic evidence and is not a counted B1 arm.

- Sol commissioned the Terra producer after 17 seconds.
- Terra's first Attempt, `attempt-184ce149e6`, ran for 160.4 seconds and produced clean Field Atlas
  commit `926c3ca`, but ended without a terminal callback. Its semantic result therefore remained
  `unknown` even though the workspace evidence survived.
- Sol used one bounded recovery. The second Terra Attempt, `attempt-8a21f4e2aa`, reported the preserved
  commit `926c3ca` successfully after 28.3 seconds with clean gates and the verifier present.
- Sol independently produced Squad Workshop commit `c45526c`.
- The stop occurred before canonical integration, final lead judgement, native evaluation and the
  matched B0 arm. No comparison result can be inferred.

No replacement Run ID is allocated until the founder gives the start signal for
[`exp-sprint-01-coordination-frontier.md`](../../../exp-sprints/exp-sprint-01-coordination-frontier.md).
Native evidence and any future matched interpretation will be appended without
rewriting the frozen hashes above.

## Experiment Sprint 01 allocation

The start signal was received after T0 first-party feasibility passed. The scenario/evaluator bytes and
seed remain unchanged, but the matched allocation uses a fresh order seed and Run IDs so the stopped
v28 diagnostic cannot be repaired into evidence.

- Order seed: `EXP-01:E01:C-LH-01:gpt56:2026-08-23`.
- Order SHA-256: `b0a88b7d6fb85ce3a5fda1eb7ec6039dd2430f649bf27cccfc889667980db5cc`.
- Odd first-32-bit parity selects **B1 → B0**.
- First counted Run ID: `exp01-e01-b1-terra`.
- Matched Run ID after structurally valid B1: `exp01-e01-b0-sol`.
- Sol lead and Terra producer use the frozen first-party allocation; no OpenRouter actor participates.
