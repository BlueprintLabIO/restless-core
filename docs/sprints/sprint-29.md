# Sprint 29 — Make exceptional the inherited outcome standard

**Status:** Draft, awaiting founder alignment

**Date:** 31 August 2026

**Target:** [`ARCHITECTURE.md`](../../ARCHITECTURE.md) §2.4 / §4.4 / §9 / §16 ·
[`owner-cockpit`](../specs/owner-cockpit.md) §4 / §6 / §10 / §14 ·
[Sprint 28](sprint-28.md) ·
[outcome-quality-enforcer dogfood](../dogfood/outcome-quality-enforcer-landing-test.md)

**Independent of:** S25–S27 deployment work. This sprint changes how owner ambition becomes an
operating contract inside one company. It does not widen network entry, authority, external effects
or the company spend ceiling.

---

## Why this sprint exists

The quality-enforcer dogfood demonstrated a material capability and a material gap. From a sparse
owner mandate, an accountable lead eventually assembled reference research, native implementation,
independent criticism and repeated revision into a much stronger landing page. The owner supplied
one creative mandate rather than managing the team. But the run took roughly two hours and 963,659
tracked tokens, two critic attempts exceeded their context payload, and the lead discovered the
effective process only while doing the work.

Restless can therefore reach exceptional work, but exceptional is not yet an explicit company
standard. Quality ambition is distributed across owner prose, lead taste, prompts, available time,
spend capacity and luck. The owner cannot set a durable default once, see what standard a request
inherited, or ask for a deliberately faster or more frontier-seeking result without restating a
process. Leads have doctrine, but no compact contract that tells them how aggressively to explore,
compare, challenge and iterate for this outcome.

Three independent quality/time/cost sliders would encode a false model. Quality is not mechanically
produced by a duration or token count, and the three variables are not independent. The product needs
one semantic statement of ambition, inherited by default, with time and spend exposed only as honest
limits around it.

> **The owner chooses the standard of outcome once. Restless adapts the method to the consequence,
> reports the live frontier honestly and returns only when the standard is met or a real boundary
> requires owner judgement.**

## Outcome

Every newly commissioned outcome receives one durable, inspectable **Outcome Standard**:

| Standard        | Owner promise                                                                 | Operating posture                                                                                 |
| --------------- | ----------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| **Fast**        | The smallest correct, usable result                                           | Direct path; native verification; stop when the outcome is safely useful                          |
| **Thorough**    | A production-ready result with strong evidence                               | Cover material cases; compare meaningful alternatives; add independent review when consequence warrants |
| **Exceptional** | A result that is clearly superior, not merely acceptable                     | Strong references, purposeful exploration, independent native evaluation and root-cause revision |
| **Frontier**    | Seek a new ceiling where the best answer is not yet known                    | Broader experiments, greater uncertainty tolerance and explicit diminishing-return judgement     |

`Exceptional` is the product default for new companies. The owner can change the company default in
Company settings or override it from one compact composer control. A continuation of the same
commissioned outcome keeps its existing standard unless the owner explicitly changes it.

The standard governs ambition, not ceremony. An exceptional typo fix may still require one small
edit and one exact check. An exceptional migration may require rehearsal and recovery proof. An
exceptional research decision may require disconfirming evidence. An exceptional landing page may
require reference-led visual exploration, motion, comparative critique and native browser review.

## Founders' implementation decision

### One primary control, not three sliders

The owner-facing control is one compact value:

```text
Outcome standard · Exceptional ▾
```

It appears in the owner composer with the inherited value visible but does not demand interaction on
every request. Its menu explains the outcome promise in owner language, not actor counts or model
settings. A progressively disclosed **Limits** area may show a target deadline and an ask-before-
crossing model-spend envelope where the product can measure them honestly.

The existing company spend ceiling remains the hard Authority-owned fuse. A per-outcome envelope is
advisory until the source can attribute spend to that outcome exactly: it changes when the lead must
ask, not whether the company may spend beyond its hard ceiling. The UI must never label an estimate
or advisory envelope as a guaranteed budget.

### One source and a frozen inheritance rule

There is one `OutcomeStandard` concept. It is not duplicated as prompt prose, UI state and team
metadata with divergent meanings.

For a new outcome:

1. an explicit owner composer selection wins;
2. otherwise the Exec may interpret an unambiguous natural-language instruction such as “quick safe
   pass” or “push the frontier”, using accountable model judgement rather than keywords;
3. otherwise the current company default applies; and
4. the Exec records the effective standard, source and any limits in the commissioned team's charter
   and first Work context.

For a continuation, feedback or revision of that outcome, the recorded standard remains in force.
An inferred override records `owner_language` as its source, preserves the supporting owner message
and is shown back immediately so the owner can correct it through ordinary conversation. Ambiguous
language inherits the company default. Natural-language urgency may also inform execution inside the
selected standard, but deterministic keyword matching must never mutate policy.

There is no project preference or generic policy engine in this sprint. Repeated observation may
later earn another scope.

### The standard changes judgement policy, not topology

The standard does not map to a fixed number of agents, critics, loops or tokens. The Exec and lead
must translate it into an outcome-specific quality contract covering:

- what exceptional fitness means for this outcome;
- the strongest available ground truth or mature references;
- the material alternatives or experiments worth exploring;
- which native environment proves the result;
- who creates and who independently challenges consequential claims;
- what evidence would justify another loop or a clean-room reset; and
- what would make further work lower-value than returning, escalating or stopping.

The lead remains accountable for team design, delegation and convergence. Runtime attempt limits
remain local crash guards, not quality policy. The standard may produce one actor and one pass when
that is sufficient; it may produce multiple specialists and revision loops when the outcome demands
them.

### Quality floors never move down

`Fast` cannot relax safety, truth, authority, privacy, accessibility, reversibility or source-owned
acceptance criteria. It reduces optional exploration and polish. It never licenses an unsafe,
unverified or misleading shortcut.

### A live frontier, not a fake quality score

The owner can inspect a compact, source-backed status for the active outcome:

```text
Exceptional · evaluating
Strong: correctness, native operation
Open: first-use clarity
Next loop: likely material improvement
Limits: within company policy
```

The projection may report the selected standard, phase, observed strengths, consequential gaps,
next-loop rationale, elapsed time, attributable spend and envelope state. It must not invent a
numeric quality score, percent complete or deterministic “S-tier” badge. Model-authored judgement is
attributed to the accountable lead and linked to native evidence.

The owner is interrupted only when:

- the selected standard appears infeasible under a real limit;
- continuing would cross an explicit ask-before-crossing envelope;
- a material trade-off requires irreducible taste or authority; or
- the lead recommends stopping because expected improvement has become immaterial.

## Success contract

The sprint passes when one integrated `_test` company and a frozen heterogeneous corpus demonstrate:

1. **Set once, inherit normally.** A company default applies to new outcomes without repeated owner
   input; an explicit or clearly inferred request override is durable for that outcome and its
   revisions.
2. **One concept, one meaning.** API, config, charter, actor context and cockpit project the same
   four-value standard and source. Refresh, restart and handoff do not change it.
3. **Adaptive intensity.** The same `Exceptional` default produces materially different plans for a
   trivial correction, consequential migration, research decision and creative outcome.
4. **No ceremony quota.** No mode mechanically requires an actor count, critic count, loop count,
   tool, reference count, token allocation or clean-room reset.
5. **Outcome-specific excellence.** Each charter defines fitness and evidence appropriate to its
   outcome; it does not paste a landing-page rubric into unrelated work.
6. **Independent challenge where earned.** Consequential claims are evaluated separately from their
   creation when that can change the decision; trivial work remains direct.
7. **Native proof.** Acceptance occurs in the environment where the result must work. Self-report and
   prompt presence are not acceptance evidence.
8. **Honest limits.** Company ceiling, attributable spend, estimate and advisory outcome envelope are
   visibly distinct. The system never claims hard per-outcome enforcement it does not possess.
9. **Useful frontier status.** The owner can see why work continues or returns without receiving
   process spam, fake precision or a quality score.
10. **Rare, consequential interruption.** The owner is not asked to choose implementation tactics.
    Every interruption in the run maps to infeasibility, envelope crossing, authority or irreducible
    taste.
11. **Lower modes retain floors.** Fast results still pass applicable safety, truth, authority and
    native correctness checks.
12. **Quality compounds.** Each non-trivial run returns at least one reusable reference, evaluation
    method, primitive, rubric or observed lesson when one was genuinely earned; it does not manufacture
    registry content to satisfy a quota.
13. **Behaviour, not branding.** Independent evaluators can distinguish Fast, Exceptional and
    Frontier behaviour from the evidence, while the product makes no guarantee that a label alone
    creates exceptional output.
14. **No policy sprawl.** The closing audit finds no second quality lifecycle, generic preference
    engine, quality-scoring service, project entity or duplicated prompt-only mode map.

## Frozen acceptance corpus

T0 freezes current inputs and baselines before implementation:

- a one-line product copy correction with an exact expected result;
- a consequential data or configuration migration with rehearsal and recovery requirements;
- a research brief whose value depends on decision-changing and disconfirming evidence;
- a creative public artifact requiring product-language fidelity and native visual review;
- one continuation after owner feedback;
- one explicit `Fast` override;
- one explicit `Frontier` override; and
- one unambiguous natural-language override plus one ambiguous request that retains the default; and
- one outcome whose desired standard cannot fit an owner-set limit.

For every case, preserve the owner message, inherited company setting, existing Exec/team behaviour,
native evidence, elapsed time, attributable model spend where available, owner interruptions and
final independent judgement. Synthetic external effects remain labelled; no test authorises real
publication, outreach, payment or deployment.

## Measures

- owner interactions required before an outcome can proceed;
- standard and source retained across refresh, restart, handoff and revision;
- time to first native evidence and time to accepted outcome;
- attributable model spend, unattributed spend and confidence in that attribution;
- creation/evaluation context failures and rework caused by oversized payloads;
- consequential defects found after the lead first considered the work complete;
- evaluator judgement of correctness, usefulness, distinctiveness and outcome fitness;
- whether a further loop changed the evaluator's decision materially;
- interruptions by reason and whether each needed owner judgement;
- safety/truth/authority-floor violations; and
- reusable quality capital actually reused by a later corpus case.

Token count, wall time, team size and loop count are diagnostic measures. None is a proxy for
quality. A single aggregate quality score is forbidden.

## Slice per layer

**Company configuration / owner authority.** Own the stable company default and its authenticated
owner change. Reuse the existing company configuration seam. Do not treat the standard as permission
to widen spend, providers, credentials or external effects.

**OrgIntel.** Preserve the effective standard, selection source and honest limits with the existing
commissioned outcome/team context. Reuse the team charter, Work, owner messages and handoffs; do not
create a quality run or project lifecycle.

**Exec and lead.** Translate the standard into an outcome-specific quality contract and team shape.
The accountable lead owns convergence, root-cause revision, reset decisions and the recommendation to
continue, return or escalate.

**Runtime.** Supply native resources, evidence and attributable usage already observed by Runtime.
Do not hard-code mode-to-model, mode-to-agent or mode-to-attempt mappings.

**Owner projection and cockpit.** Expose one inherited composer control, progressive limits and a
compact frontier account. The projection reads source state and attributed lead judgement; the
browser does not generate quality conclusions.

**Evaluation.** Use heterogeneous native outcomes, independent judgement and paired mode exercises.
Do not accept self-report, screenshots alone or doctrine text as proof.

## Ticket decomposition

Status lives only in this checklist.

| Status | Ticket | Slice | Outcome or friction served | Prior machinery made deletable |
| ------ | ------ | ----- | -------------------------- | ------------------------------- |
| [ ] | [S29-T0 · Freeze the standard, corpus and evaluation grammar](sprint-29/t0-standard-corpus-and-grammar.md) | Product contract + Evaluation | “Exceptional” can otherwise become an unfalsifiable brand label | Remembered examples and quality-by-vibe acceptance |
| [ ] | [S29-T1 · Give the company one owner-set default](sprint-29/t1-company-default.md) | Company config + owner action | Owners must currently restate ambition in prose | Prompt-only company quality preference and browser-local default |
| [ ] | [S29-T2 · Preserve an outcome's effective standard and limits](sprint-29/t2-outcome-envelope.md) | Owner input + OrgIntel | Owner ambition can disappear between message, commission and revision | Keyword parsing and duplicated team-specific quality flags |
| [ ] | [S29-T3 · Make the Exec and lead operationalise ambition](sprint-29/t3-adaptive-lead-policy.md) | Exec + lead + Runtime | The successful quality process was discovered ad hoc during dogfood | Fixed team recipes, actor quotas and prompt fragments that restate the mode map |
| [ ] | [S29-T4 · Expose one sane owner control](sprint-29/t4-owner-control.md) | Owner cockpit | Three sliders would demand repeated tuning and imply false independence | Hidden defaults, mode selection encoded only in prose and decorative controls |
| [ ] | [S29-T5 · Project the live frontier and escalate only real boundaries](sprint-29/t5-frontier-and-escalation.md) | OrgIntel + owner projection | Owners cannot tell why work continues, stops or asks for attention | Process spam, fake completion scores and tactical owner menus |
| [ ] | [S29-T6 · Prove adaptive exceptional work and purge](sprint-29/t6-dogfood-and-purge.md) | Full slice + Evaluation | One strong landing page does not prove a general quality system | Losing policy branches, prompt-only claims and unexercised compatibility paths |

Expected order: **T0 → T1/T2 → T3/T4 → T5 → T6**. T1 and T2 share the frozen enum and
inheritance contract. T3 and T4 may proceed in parallel only after that contract lands. T6 evaluates
the integrated behaviour, not isolated UI or prompt demos.

## Risks and dispositions

| Risk | Disposition | Treatment |
| ---- | ----------- | --------- |
| Exceptional becomes expensive ceremony | **Invariant** | Outcome-specific contract; no fixed team, critic, reference or loop quota |
| Fast becomes permission to cut safety or truth | **Invariant** | Non-negotiable floors are mode-independent and separately tested |
| Frontier work never stops | **Guarded** | Lead reports expected next-loop value and recommends stop/escalation at diminishing returns or real limits |
| Natural language is over-interpreted | **Invariant** | Model judgement may infer only clear instructions, records the source message and exposes the result immediately; ambiguity inherits default; no keyword rules |
| Per-outcome budget appears harder than accounting allows | **Invariant** | Distinguish hard company ceiling, attributed actuals, estimates and advisory ask boundary |
| A quality score launders subjective judgement | **Invariant** | No aggregate score or percent complete; expose attributed claims, gaps and native evidence |
| The default makes trivial tasks slow | **Guarded** | Heterogeneous dogfood requires Exceptional to remain minimal for exact low-consequence work |
| Leads optimize to the test corpus | **Guarded** | Freeze varied cases, hold back evaluator prompts and include one unannounced transfer case |
| Preference scope proliferates | **Accepted** | Company default plus commissioned-outcome override only; no project/workstream preference in this sprint |
| More quality always costs more | **Rejected premise** | Measure both; better decomposition, reuse and earlier critique may improve quality and efficiency together |

## Non-goals

- guaranteeing “S-tier” output from a label;
- three continuously tuned quality/time/cost sliders;
- automatic model, reasoning-effort, team-size or loop-count selection tables;
- a generic policy, project, workflow or preference subsystem;
- changing the company spend ceiling through a quality selection;
- authorising deployment, publication, outreach, payment or credentials;
- scoring prose, design or code with one universal metric;
- requiring independent critics or reusable registry entries for every trivial task; or
- replacing accountable lead judgement with deterministic orchestration.

## Salvage

No legacy control-plane machinery. Reuse company configuration, authenticated owner actions, ordinary
owner messages, team charters, Work, actor contexts, spend accounting, Attention and native review
targets. Re-validate each seam against T0 before extending it. The existing accountable quality
doctrine and its dogfood are observed evidence, not an implementation to copy blindly.

## Closing evidence

T6 publishes:

1. the frozen corpus and pre-sprint baseline;
2. the exact standard/inheritance contract and generated bindings;
3. restart, handoff and revision persistence traces;
4. owner composer and Company-setting captures at desktop and narrow widths;
5. the adaptive quality contracts and team decisions for every corpus case;
6. native acceptance evidence and blinded evaluator accounts;
7. time, attributable/unattributed spend and owner-interruption results;
8. one honest infeasibility or envelope-crossing escalation;
9. one demonstrated reuse of earned quality capital; and
10. rejected branches, deleted machinery and remaining uncertainty.
