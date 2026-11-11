# EXP-02 A4 — recovery capsule screen

**Status:** provisional-loss after the first valid live pair. Both arms recovered correctly; the
capsule improved exact provenance but failed the frozen targeted-churn gate. No replication runs.

## Question

Does one bounded recovery capsule help an accountable Sol lead recover useful work after missing
semantic completion more accurately and efficiently than the current generic `unknown + workspace`
notice?

## Hypothesis and mechanism

Both arms receive the same frozen Work, Attempt and preserved candidate. Baseline receives only the
generic unknown-outcome statement and workspace locator. A4 additionally receives facts already owned
by the substrate: actor, Work/Attempt, immutable input fingerprint, start/end Git observation, exact
available artifact locators and the permitted next judgements.

The capsule never claims success, recommends a content decision or adds a workflow state. The lead
must inspect and decide.

## Relevant canonical claims

- `CL-006`: material events and artifact observation drive resumption; elapsed time is not completion.
- `CL-007`: useful workspace evidence survives missing callback as unknown.
- `CL-013`: natural leadership sits above minimal factual Work evidence.

## Baseline and changed variable

| Arm | Lead input after identical producer failure |
|---|---|
| R0 | Work title, generic `unknown`, preserved workspace path |
| A4 | R0 plus bounded process/workspace/artifact recovery capsule |

Only the recovery context changes. Same Sol model, tools, candidate bytes, source corpus, Work/Attempt,
lead mandate, resource envelope and evaluator.

## Scenario and native success contract

Use P5. The lead must:

1. inspect the exact preserved candidate rather than rerun the producer;
2. compare it with the frozen Q01–Q08 corpus and acceptance contract;
3. record an evidence-backed accept, revise, resume, reassign or abandon decision;
4. preserve truthful attribution and unknown outcome until that judgement; and
5. produce a compact recovery decision artifact with exact locators.

The negative control contains no producer file and must not be accepted or described as recovered.

## Models, tools and envelope

- lead: `gpt-5.6-sol`;
- producer fixture: one Terra-produced candidate frozen once, then copied byte-for-byte into both
  isolated recovery arms;
- ordinary Git/files/search plus the existing seven lab commands;
- no network, live company or consequential effect;
- identical generous outer operating envelope, never used as semantic evidence.

## Measurements

- correct native judgement and planted-defect detection;
- time/tool/tokens to first candidate inspection and final recovery decision;
- repeated discovery and file reads;
- duplicate producer launch or implementation;
- unsupported claims about completion or artifact provenance;
- decision/artifact accepted blind.

## Discriminating result

A4 wins only if quality is non-inferior, no epistemic/attribution rule regresses, the lead inspects the
same candidate without duplicate production, and recovery/reconstruction effort materially falls.
Merely repeating the capsule text is not a win.

## Stop rules

- conformance must prove identical candidate bytes and that neither arm infers success;
- a first clear live loss rejects A4 and deletes its harness path;
- a win remains provisional until two coding shapes and one non-coding shape pass;
- if R0 already recovers cleanly at equal cost, add no product capsule;
- any capsule that exposes hidden context, becomes a checklist or mistakes Git change for acceptance
  loses regardless of speed.

## Result

See [`a4-result.md`](a4-result.md). Run `exp02-a4-p5-r1` is infrastructure-invalid because the Codex
CLI rejected incompatible launcher flags before model work. Fresh run `exp02-a4-p5-r2` is valid.
