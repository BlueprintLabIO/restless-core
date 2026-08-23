# C-LH-01 run index

Status: frozen; B1 infrastructure corrected, counted arm queued first

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
- Counted B1 Run ID: `v27-clh01-b1-glm-r3` — queued first after the liveness repair.
- B0 Run ID: `v27-clh01-b0-sol` — queued second after B1 closes.

Native evidence and the matched interpretation will be appended without rewriting frozen hashes.
