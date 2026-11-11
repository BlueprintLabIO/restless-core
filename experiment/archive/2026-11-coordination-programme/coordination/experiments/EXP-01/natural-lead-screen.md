# Natural-lead behavioural screen

Status: frozen before live execution on 2026-08-23.

## Question

Can a strong accountable lead, encouraged to coordinate naturally and allowed to choose whether to
delegate, outperform both a forced ordinary handoff and a fresh lead working alone on the same bounded
product outcome?

This is a prompt-and-behaviour experiment, not a proposal for more coordination machinery. The lead
retains the writable candidate and may commission zero or one Staff contribution. Zero is a valid team-
sizing judgement, not protocol failure.

## Frozen task and evidence

- Workload: `W01-G-WORLD-01`, semantic repair of Cosmon's bridge return/bonding path.
- Scenario: `../W01-G-WORLD-01/scenario-r2.md`.
- Scenario SHA-256: `4f2419a67acf80b784132af9ec930303ccc89e7075a028b7df92ce7a2247aae0`.
- External evaluator: `../W01-G-WORLD-01/evaluate.mjs`.
- Evaluator SHA-256: `6481712fe3c265fef669569a540b45c0c6ab4af5c2266c3b1a0ce1a7545a2d93`.
- Seed commit: `514b7b3d0a65e093af608b08ca142344412181f4`.
- Lead model: `gpt-5.6-sol` for both fresh arms.
- Available Staff model: `gpt-5.6-terra` in the natural arm.
- Available Staff role: `world-content`.
- No mid-run owner intervention.

The evaluator remains hidden from producers and is run externally after the run. The frozen scenario
contains the exact public acceptance schema established for the valid r2 comparison.

## Arms

1. `B1 existing`: the already-valid forced ordinary-handoff run
   `exp01-w01-gworld-r2-b1-terra`. It required exactly one producer Work before lead edits.
2. `N1 natural`: a fresh accountable lead with one available Staff member. It may work alone or
   commission at most one genuine responsibility after understanding the outcome. There is no required
   first action, cadence, shared-state form, polling loop, critic, or handoff template.
3. `B0 fresh`: the same lead model working alone with no Staff.

The existing B1 artifact is frozen and will not be rerun or modified. N1 and B0 are fresh runs from the
same seed. The deterministic ordering seed is
`EXP-01:NATURAL-LEAD:G-WORLD-R2:gpt56:2026-08-23`, whose SHA-256 begins with odd hex `d`; the frozen
rule is odd = N1 first, B0 second.

## Natural-lead manipulation

The lead is told:

- own the whole outcome and first build a causal understanding of it;
- choose the effective team size, including zero, rather than delegate to satisfy a topology;
- if collaboration is useful, communicate purpose, current understanding, important unknowns, a stable
  ownership seam, and observable proof—not a mechanical checklist;
- invite material challenge and update the other person only when information changes their work;
- continue complementary work, then personally inspect, integrate, run, and judge the combined result;
- treat every report and artifact as a claim until observed in the whole product.

These are behavioural principles. They do not add a document schema, message cadence, work graph,
shared scratchpad, critic, or deterministic task decomposition.

## Measurements

### Outcome

- Frozen evaluator pass count out of 18.
- Material defects found in a topology-blind review of all three runnable artifacts.
- Blind quality score and preference.
- Clean, advanced, executable candidate.

### Efficiency

- Wall-clock time, model turns, tool calls, and token usage.
- Staff Work and Attempt count.
- Retry, polling, duplicate-work, and integration churn.

### Coordination behaviour

- Whether N1 chose zero or one Staff member, when, and why.
- Whether a delegated responsibility had a real product seam and usable context.
- Whether Staff materially challenged or improved the lead's causal model.
- Whether communication occurred only at useful state changes.
- Whether the lead performed complementary work and independently judged the returned artifact.
- Whether the handoff reduced rediscovery or merely displaced it.

## Decision rule

N1 is promising only if it preserves the 18/18 semantic floor and either:

- beats B1's artifact quality without materially worse efficiency; or
- matches the better artifact with meaningfully less coordination cost or churn.

A topology-blind reviewer must prefer N1 or find fewer material defects; self-reported completion does
not count. If N1 chooses no delegation, compare it primarily with fresh B0 as an adaptive routing
decision. One screen can justify replication, not an architectural conclusion.

## Purge rule

The new harness mode exists only to make zero-or-one delegation observable. Do not add protocols around
it during the run. If the manipulation is invalid, fix only the invalidity and rerun. If it is valid but
shows no credible benefit, retain the learning and remove the special mode rather than accumulating
another permanent coordination architecture.

## Manipulation repair log

The first N1 execution (`exp01-natural-gworld-r2-n1`) was outcome-valid but coordination-invalid. Its
lead narrated commissioning and accepting a Staff room, while the factual trace contained zero
`commission` calls, Work, Attempts, Staff turns, or callbacks; every file change was made by the lead.
The artifact remains evidence about solo lead capability but cannot answer the teamwork question.

Before one repaired rerun, the natural prompt received one interface clarification only: collaboration
must begin with a real OrgIntel `commission`, only its Work → Attempt → artifact callback counts as a
Staff contribution, and a run with no Work must be described as solo. No handoff form, required
delegation, first-action rule, message cadence, or deterministic guard was added. The frozen task,
models, evaluator, baselines, and decision rule remain unchanged.
