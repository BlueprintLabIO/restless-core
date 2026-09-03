# Sprint 34 — Make culture observable operating identity

**Status:** Implemented and locally verified; matched live-model comparison provider-blocked

**Date:** 31 August 2026

**Programme:** [Company Identity](./company-identity-programme.md)

**Depends on:** Sprint 31. May run beside Sprints 32 and 33 after the common release contract freezes.

## Why this sprint exists

Culture prose is easy to generate and easy to ignore. Values such as “ownership”, “candour” and
“customer obsession” are not evidence that a company behaves that way. Restless already observes
decisions, handoffs, corrections, disagreement, incidents and customer effects. Repeated, approved
patterns can become durable behavioural identity without turning slogans into policy.

## Outcome

One `_test` company handles a disagreement, uncertain incident, customer-visible mistake, quality
trade-off and hiring brief in ways consistent with an owner-approved Culture release. Independent
reviewers can cite exact decisions and conduct—not value words—to explain the consistency.

## Culture contract

Culture evidence is an attributed observation about behaviour under conditions:

- situation and consequence;
- actors and authority boundary;
- decision or conduct observed;
- evidence and outcome;
- owner judgement or repeated corroboration;
- scope, confidence and counterexample; and
- the operational implication, if promoted.

Culture may inform how Staff surface uncertainty, disagree, correct errors, treat customers, define
finished work and escalate irreducible judgement. It cannot grant capability, override owner authority,
force personality, suppress dissent or evaluate employment performance in this programme.

## Success contract

1. Every promoted norm has observed evidence or an explicit owner founding decision.
2. Abstract value words alone cannot enter a released Culture pillar.
3. Relevant norms compile into actor and team context without replaying unrelated company history.
4. A norm includes boundary conditions and counterexamples; it is not an unconditional slogan.
5. Disagreement may produce a different decision while preserving the cultural method.
6. Uncertainty is reported honestly and correction remains visible rather than reputation-managed away.
7. Customer treatment is tested through an actual bounded communication and recovery action.
8. A hiring brief describes expected conduct through scenarios rather than personality or demographic
   proxies.
9. Culture review distinguishes behaviour from polished explanatory prose.
10. No employee scoring, sentiment surveillance, personality typing, generic rules engine or model-led
    disciplinary system is introduced.

## Slice per layer

**Authority / OrgIntel.** Retain founding decisions, observed conduct, counterexamples, proposals and
retirement through Company Identity. Culture does not become employment adjudication.

**Runtime.** Compile only consequence-relevant norms into actor/team context and capture exact decision
and communication evidence. It never infers protected traits, sentiment or personality.

**Exec / accountable lead.** Apply cultural method while preserving disagreement, role authority and
outcome judgement. Actors remain free to surface contradiction and choose different defensible answers.

**Cockpit.** Let the owner inspect and govern evidence-backed norms and exceptions without employee
profiles, leaderboards or routine behavior alerts.

## Salvage

No unverified salvage lift. Historical decisions and dogfood incidents enter the corpus only after T0
confirms their exact context, consequence, authority and counterevidence.

## Ticket index

| Status | Ticket | Outcome |
| --- | --- | --- |
| [x] | [S34-T0](./sprint-34/t0-behaviour-corpus.md) | Freeze decisions, dissent, incidents and counterexamples |
| [x] | [S34-T1](./sprint-34/t1-culture-evidence.md) | Define promotable observed-behaviour evidence |
| [x] | [S34-T2](./sprint-34/t2-bounded-posture.md) | Compile relevant norms into actor/team posture |
| [x] | [S34-T3](./sprint-34/t3-decisions-and-corrections.md) | Exercise disagreement, uncertainty and visible correction |
| [x] | [S34-T4](./sprint-34/t4-customer-and-hiring.md) | Apply culture to customer recovery and scenario-based hiring |
| [x] | [S34-T5](./sprint-34/t5-owner-governance.md) | Govern norms, counterexamples, exceptions and retirement |
| [x] | [S34-T6](./sprint-34/t6-dogfood-and-purge.md) | Prove conduct, reject slogans and purge surveillance shapes |

Expected order: **T0 → T1/T5 → T2 → T3/T4 → T6**.

## Terminal decision

- **Pass:** the exercised company demonstrates stable behavioural method across all five cases without
  suppressing disagreement or requiring tactical owner coordination.
- **Revise once:** repair one bounded evidence, retrieval or authority defect.
- **Stop negative:** if culture cannot be distinguished from generated prose or drifts into worker
  scoring and surveillance, retain the negative finding and do not integrate it in Sprint 35.
