# Experiment Sprint 08 - Evidence-backed capability playbooks

**Status:** Draft for founder approval; no arm or implementation started

**Decision owner:** Founder

**Date:** 26 August 2026

**Depends on:** EXP-07's successful Restless greenfield web outcome and Sprint 20's completed research
publication dogfood. If Sprint 20 has not reached a terminal evidence report, EXP-08 does not start.

## Decision this sprint must produce

Determine whether a thin, evidence-backed web-production playbook lets a fresh Restless team produce
better new web outcomes with less rediscovery and owner attention than the same Restless organisation
working naturally without the playbook.

The decision is not whether templates sound useful. It is whether the observed Restless web pattern
transfers across materially different work without turning intelligent leadership into a rigid
workflow or cargo-cult visual template.

## Hypothesis

> A versioned capability playbook containing validated priors, capability probes, native-review
> expectations, known failure modes and evidence from prior runs will improve orientation, first-pass
> completeness and repair cost on analogous work, while a lead retains authority to adapt or reject
> it.

The credible null is that a strong lead and worker infer the same pattern from the mission and repo,
so the playbook adds prompt weight, anchoring and visual sameness without changing accepted outcomes.

## What a playbook is in this experiment

The treatment artifact is an ordinary, readable file bundle. It may contain:

- the work shapes for which prior evidence applies and the boundary where it stops;
- the default accountability shape: Exec dispatch, non-producing lead, one end-to-end producer;
- conditions that previously justified a specialist or independent reviewer;
- capability and source probes that prevent guessing;
- useful design-reference discovery and anti-jank reminders;
- native ReviewTarget and evidence expectations;
- observed model/runtime combinations and dated results;
- known failures, repair patterns and explicit unknowns; and
- instructions to adapt, override or reject the prior when the actual work differs.

It may not contain a fixed sequence of stages, mandatory handoff prose, generated project scaffold,
visual theme, article template, hard-coded team size, actor prompt transcript, task router or automatic
department-spawning rule.

The playbook records a prior. It is not an invariant and does not guarantee a result. Its evidence
certificate is versioned by task shape, model/runtime and observed runs rather than reduced to a
permanent scalar score.

## Matched treatment

Every counted task runs through the current Restless product path with GPT-5.6 Sol, matched effort,
tools, starting artifacts, authority, spend ceiling and native evaluation.

```text
Arm N - natural
  owner mission -> Exec -> accountable lead -> Staff chosen by lead
  no web-production playbook

Arm P - playbook
  same mission -> Exec -> accountable lead -> Staff chosen by lead
  exact frozen web-production playbook available and named as a revisable prior
```

The lead in either arm remains a non-producing supervisor and uses one end-to-end Staff producer by
default. The experiment does not force identical actor counts after launch: whether the playbook
improves staffing judgement is part of the treatment. Every deviation and its factual rationale are
recorded. Arms use fresh responsibility-scoped sessions and cannot inspect each other.

## Sparse transfer workloads

Run three paired workloads in randomised arm order. None may reuse the Restless site, its copy or its
visual identity.

1. **Greenfield proposition site.** Create a production-runnable public site for an unfamiliar,
   evidence-rich product from an empty repository.
2. **Existing application surface.** Extend a mature design system with one interactive owner-facing
   product outcome whose quality must be judged in the running application, not from a screenshot.
3. **Long-form evidence publication.** Add a coherent research/documentation section to an unfamiliar
   existing site while preserving its native identity and source truth.

Run workload 1 first. If the playbook arm is clearly worse through anchoring, scope confusion or
quality loss, stop and revise or reject the treatment before spending on transfer. Workloads 2 and 3
test transfer, not repeated mastery of one landing-page shape.

## Validity gates

1. The playbook is frozen before either arm and cites only evidence that actually exists.
2. Both arms use the same model family, configured effort, source pack, tools, provider conditions,
   spend ceiling and objective outcome checks.
3. The evaluator sees native outcomes and source evidence, not arm, playbook, actor, topology, trace,
   timing or spend.
4. Deterministic checks judge routes, builds, accessibility and exact claims only. Fresh blinded
   reviewers and the founder judge usefulness, writing, visual quality and whole-outcome coherence.
5. A lead using the playbook may reject it. Mechanical adherence is not scored as success.
6. Known EXP-07 callback, review-locator or verification-output failures must either pass preflight or
   be classified as product-invalid rather than attributed to the playbook.

## Measures

For every arm record:

- founder acceptance and blinded native quality judgement;
- active owner minutes and intervention type;
- time to first useful native artifact and accepted outcome;
- lead and producer newly processed input, model usage and cost;
- orientation work before production;
- missing capability or source assumptions;
- first-attempt completeness and material repair loops;
- coordination overhead and lead interventions that changed the result;
- playbook retrieval, applied prior, explicit rejection and override rationale;
- visual or structural anchoring to prior work; and
- whether a previous failure was avoided or mechanically repeated.

Do not collapse these dimensions into one team-template score.

## Decision rule

Promote the playbook toward a product default only if:

1. Arm P is accepted on at least two materially different workloads;
2. it improves owner attention, time to useful output or material repair burden on at least two;
3. no accepted task suffers a consequential quality or truth loss relative to Arm N;
4. the leads demonstrably adapt or reject inapplicable guidance rather than reproducing one site;
5. at least one prior lesson changes a later decision and helps; and
6. the benefit remains after counting the playbook's context and review cost.

If it helps only one work shape, publish a scoped playbook for that shape. If it is neutral, keep the
underlying evidence notes and do not add product machinery. If it harms transfer, archive the
treatment and preserve the natural-lead baseline.

One experiment does not justify pre-creating a permanent web department in every company. A winning
playbook becomes available by default and is instantiated when relevant work appears. A lead may
remain durable after real use; idle bureaucracy is not a product outcome.

## Deliverables

1. frozen natural and playbook contracts for all three workloads;
2. the exact candidate playbook and evidence certificate;
3. blinded native outcomes and comparable run reports;
4. transfer, owner-attention, cost, quality and repair results;
5. a disposition: `promote`, `scope`, `revise`, `reject` or `inconclusive`;
6. if promoted, the smallest proposed retrieval/product slice and what it makes deletable; and
7. a canonical note explaining what the playbook does not guarantee.

## Risks and dispositions

| Risk | Disposition | Treatment |
| --- | --- | --- |
| The playbook copies the winning site's aesthetics | **Guarded** | Exclude visual assets/theme and test unfamiliar identities; blinded reviewers penalise anchoring |
| The natural arm rediscovers the playbook from repo evidence | **Guarded** | Isolated source packs expose only task-native context; shared product invariants remain fair |
| Team-template language hardens a contingent topology | **Guarded** | Encode applicability and override, not a router or actor count |
| The three workloads are still all web work | **Accepted** | This sprint scopes only the web-production playbook; non-coding transfer requires a later playbook |
| One model dominates the result | **Accepted and recorded** | The evidence certificate names GPT-5.6 Sol and date; it is not universalised |
| A winning playbook causes speculative registry work | **Guarded** | Product promotion requires the smallest file-first retrieval slice and a separate implementation sprint |

## Stop boundary

This sprint authorises only isolated `_test` runs, candidate playbook files and evaluation evidence.
It does not authorise a playbook registry, database, automatic team spawning, default department,
production site change, public deployment or modification of the Restless organisation invariants.
