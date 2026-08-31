# Sprint 28 — Make company output readable without making it uniform

**Status:** Implemented and visually verified; blinded-reader acceptance pending

**Date:** 31 August 2026

**Target:** [`ARCHITECTURE.md`](../../ARCHITECTURE.md) §2.4 / §4.4 / §6 / §9.1 / §9.5 /
§16 · [`owner-cockpit`](../specs/owner-cockpit.md) §2 / §4 / §5 / §6.6 / §7.4 / §10.3 /
§12 / §14.3 · [Sprint 07](sprint-07.md) · [Sprint 19](sprint-19.md)

**Independent of:** S25–S27 deployment work. This sprint changes the meaning and presentation of
existing owner-visible projections; it does not widen network entry, authority or company
capability.

---

## Why this sprint exists

Sprint 07 established that the accountable actor, not the cockpit, authors owner meaning. Sprint 19
then told every actor that Work titles, outcomes, resolutions, artifact labels and gate names are
writing addressed to a person. Those were the correct cheapest interventions. They are no longer the
whole solution.

The current Attention contract carries separate prose fields for `what_happened`, `why_it_matters`,
`recommendation`, `requested_action` and `if_no_action`. The focused folio renders most of them as
paragraphs and Markdown, while an action's actual consequence is often available only as button
metadata. A truthful item can therefore repeat one idea five times while leaving the executable
choice less legible than its explanation.

The same boundary appears elsewhere:

- a Work outcome still combines a human opening and an exact machine contract in one authored text;
- artifact and gate labels are readable only when the actor remembered who would see them;
- ordinary agent messages need expressive prose, but completion, owner need and next ownership are
  not consistently distinguishable from background explanation;
- the same event is retold differently in Attention, chat, Work and decision history, so the owner
  must reconstruct causality from several plausible summaries; and
- Markdown formatting is carrying meaning that should instead come from source state and typed
  actions.

This is not evidence for a universal document schema, a presentation agent or a readability
algorithm. Communication varies too much for one template. It is evidence for a stronger boundary:

> **Constrain meaning more than expression. Source systems own facts and consequences; accountable
> actors author interpretation; narrow semantic contracts preserve the distinction; the cockpit
> chooses the readable form.**

## Outcome

Every consequential owner-visible Restless item has a clear reading path:

```text
what this is
→ what changed
→ why it matters
→ what the company recommends
→ whether the owner is needed
→ what each real choice causes
→ what happens next or while waiting
→ where the evidence lives
```

Attention decisions render as decisions rather than essays. Work and artifacts open with their
human meaning while preserving the exact execution contract and source evidence. Agent conversation
remains natural, but consequential replies can declare outcome, owner need, next ownership and
artifact references without encoding those meanings in prose formatting.

The result is not shorter text at any cost. It is a source-faithful narrative whose hierarchy comes
from semantics, with prose reserved for the context and judgement that actually require prose.

## Founders' implementation decision

Readability is enforced by several owners, not by one prompt:

| Concern                                                | Owner                                      | Enforcement                                                       |
| ------------------------------------------------------ | ------------------------------------------ | ----------------------------------------------------------------- |
| Facts, current state, evidence and allowed actions     | Existing source plane                      | Typed source records and live projection                          |
| Consequence of a control                               | Plane that performs the action             | Source-owned action semantics; never inferred from button copy    |
| Meaning, recommendation and relevance                  | Accountable actor                          | Shared authoring instruction and attributed authored fields       |
| Required semantic roles and provenance                 | Narrow source record / projection contract | Schema and structural validation at write time                    |
| Language quality and appropriate form                  | Accountable actor, then observed review    | Model judgement plus behavioural evaluation; no readability score |
| Lists, controls, comparisons, timelines and disclosure | Owner cockpit                              | Deterministic rendering from semantic type                        |
| Approval, acceptance, direction and other consequences | Existing explicit operation                | Controls only; prose never performs the transition                |

The cockpit does not summarize on read. A validator does not silently rewrite an accountable actor's
words. If an objective contract is invalid, the write is rejected with the exact defect. If the
language is poor but truthful, the accountable author revises it; dogfood measures whether the
authoring path actually produces readable results.

## Problem classification

### Deterministic and enumerable

- which plane owns a fact or transition;
- whether a required field, source reference or action consequence is absent;
- whether an action id maps to a real source-owned operation;
- whether choices are mutually exclusive or independently composable when the source declares that;
- whether evidence is primary, supporting or unavailable;
- whether an owner action was recorded and what source state followed it; and
- which renderer corresponds to an already-declared semantic shape.

These belong in types, validation and projection code.

### Judgement and open-ended

- what changed materially;
- why it matters to this owner now;
- which facts are relevant enough to foreground;
- what course the accountable actor recommends;
- whether uncertainty could change that recommendation;
- whether prose, bullets or a comparison best explains the context; and
- whether the final communication is concise without omitting truth.

These remain model-authored and are tested behaviourally. Do not replace them with word lists,
readability scores, character limits, sentence counting, semantic-similarity thresholds or a second
model call in the browser/BFF path.

## Communication profiles, not one universal template

The common grammar is applied in proportion to consequence:

| Surface                   | Constraint         | Default presentation                                                             |
| ------------------------- | ------------------ | -------------------------------------------------------------------------------- |
| Attention                 | Strong             | Focused decision/review/human-step composition with explicit controls            |
| Decision continuation     | Strong             | Recorded choice → released work → current owner → observed outcome               |
| Work outcome              | Moderate           | Human summary first; exact contract and evidence retained                        |
| Artifact reference        | Moderate           | Recognisable name, purpose/context, observed availability and native open action |
| Consequential agent reply | Light and optional | Direct answer plus declared outcome/next step/owner need when present            |
| Ordinary conversation     | Minimal            | Natural prose, attachments and optional supporting detail                        |
| Raw evidence              | Exact              | Native artifact or disclosed technical detail; no simplifying rewrite            |

A field is introduced only where the current source record cannot carry the required distinction
without parsing prose. There is no `UserVisibleArtifact` database, universal message DSL, generic
workflow form or renderer catalogue.

## Attention semantic envelope

The first implementation starts from the existing `OwnerBrief` and `AttentionItem`; it does not add a
parallel lifecycle. The projection should make these roles explicit enough to render without
reverse-parsing Markdown:

```text
kind and source reference
headline
observed trigger / why now
material effects[]
recommendation
decision {
  mode
  recommended action id, when one exists
  source-owned actions[] {
    id
    label
    immediate consequence
    next observable state
  }
}
no-action consequence
material uncertainties[]
deadline, only when delay changes the consequence
native target and supporting evidence[]
accountable author and source snapshot
```

This is a semantic envelope, not a visual template. A renderer may use one sentence, bullets, a
comparison or a causal sequence according to the declared data. It must not turn an arbitrary array
into checkboxes merely to reduce text.

Choice controls follow actual semantics:

- mutually exclusive actions use buttons or a single-choice control;
- independently composable source choices may use multi-select;
- a requested adjustment uses bounded input tied to the existing feedback/direction operation;
- outcome review uses accept/request-changes semantics;
- an irreducible browser or identity step opens the prepared last mile; and
- informational context with no owner action has no fake decision control.

Multi-select is not earned by this spec alone. T3 may implement it only if T0 freezes a real source
case whose independent selections have distinct source-owned consequences. Otherwise the contract
records the future distinction and the selected implementation remains single-choice.

## Authoring and validation discipline

The accountable emitting actor remains the only author of interpretation. Its shared instruction
must require:

1. lead with the answer, outcome or decision, not process narration;
2. give each authored field one job;
3. distinguish observation, interpretation, recommendation and uncertainty;
4. recommend one course when owner judgement remains;
5. state only source-supported effects and consequences of waiting;
6. use familiar business language unless the technical term is itself decision-relevant;
7. keep machine contracts, paths, gates, logs and raw evidence outside the primary explanation;
8. use a list only for separate parallel facts, a comparison only for real alternatives and prose
   where qualification or narrative is necessary;
9. remove repetition across semantic roles; and
10. privately map every factual clause to a source observation before submitting.

Write-time validation is deliberately narrower than this judgement. It can reject missing roles,
blank optional fields, ungrounded source/action identifiers, actions without consequences, invalid
choice composition, stale source fingerprints and a claimed transition unsupported by source state.
It does not declare prose "simple" or "good" by algorithm. Structural failure is returned to the
same accountable actor for revision; the validator never becomes an anonymous copywriter.

## Success contract

The sprint passes when one integrated `_test` company and the frozen varied corpus demonstrate all of
the following:

1. **Five-second orientation.** From each Attention queue row and focused first view, a reader can
   identify what the item is and whether the owner is needed without opening evidence.
2. **Ten-second decision account.** For a decision item, the reader can state what changed, the
   recommendation, the real choices and what happens after acting or waiting. The account matches
   source evidence.
3. **Controls carry consequences.** Every consequential control displays its immediate effect and
   maps to an existing source-owned operation. Prose, a checkbox state or a conversational reply
   cannot itself cross the boundary.
4. **Form follows semantics.** Approval, outcome review, bounded decision and human step render
   differently where their actions differ. Lists, comparisons and selection controls correspond to
   actual content structure rather than paragraph length.
5. **One recommendation, legitimate alternatives.** The accountable actor recommends a course. The
   UI shows only alternatives the source can truthfully execute and never hands implementation
   research back to the owner as an options menu.
6. **Truth survives compression.** Material uncertainty, counter-evidence, reversibility and a real
   deadline remain visible at the point they could change a choice. Technical evidence remains
   complete behind deliberate disclosure.
7. **Work separates audiences.** A human can understand the Work outcome without first reading its
   execution contract, while the actor receives the exact unchanged constraints and evidence rules.
8. **Artifacts are recognisable.** Important artifact references say what the artifact is, whether it
   is available and why it matters in the current Work without exposing a command-produced label as
   the primary name.
9. **Conversation remains conversation.** Ordinary messages remain free-form. A consequential reply
   can expose outcome, next ownership, owner need and linked artifacts without requiring the renderer
   to infer those meanings from Markdown.
10. **One causal narrative.** After an owner action, Attention, decision history, Work and the
    responsible actor's next message agree on: recorded choice → what it released → who owns the next
    step → observed result. They project the same source facts rather than copying one another's prose.
11. **No prompt-only claim.** Automated tests prove structural/source/action invariants. A blinded
    reader exercise over the frozen corpus proves comprehension. Neither is reported as proving the
    other.
12. **No universal presentation subsystem.** The closing deletion audit finds no new authoritative
    presentation store, BFF model call, generic workflow/form engine, readability heuristic or
    parallel action lifecycle.

## Frozen acceptance corpus

T0 freezes source-backed examples before the schema or renderer is selected:

- one Authority approval with a bounded external consequence;
- one owner decision with at least two legitimate mutually exclusive choices;
- one outcome review with a native ReviewTarget and revision path;
- one irreducible human/browser step;
- one material uncertainty that changes how a recommendation should be read;
- one completed Work with a long exact execution contract;
- one important artifact in each currently supported native review family available in the fixture;
- one concise ordinary conversation;
- one verbose but truthful consequential agent reply; and
- one decision continuation from recorded action to observed outcome.

Each fixture preserves the source record, current owner-visible rendering, intended reader question
and exact facts that must survive. Synthetic content is allowed only where labelled and where the
source operation is real inside `_test`; no simulated external effect is reported as real.

## Measures

- correct identification of item purpose and owner need;
- correct prediction of each control's consequence;
- correct account of what happens after acting and waiting;
- unsupported or omitted material claims;
- time to first correct account;
- evidence expansions needed to make the decision;
- clarification questions caused by presentation;
- duplicate factual clauses across the primary view;
- internal identifiers or implementation terms required for comprehension;
- desktop/mobile overflow, focus order and control labelling; and
- cross-surface contradictions in the causal continuation.

Word count, sentence count and generic readability grade may be recorded for diagnosis but cannot be
pass/fail measures.

## Slice per layer

**OrgIntel.** Preserve accountable authored meaning, source fingerprint, optional message semantics,
Work owner summary where earned and causal decision continuation. Reuse owner handoffs, Work,
messages, artifact references and decisions. Do not create a presentation lifecycle or second
artifact record.

**Authority Plane.** Continue to own approval facts, available decisions, exact party/amount/scope,
consequence, receipt and transition. Supply projection-safe action semantics; never accept an
OrgIntel-authored consequence as authority truth.

**Owner projection.** Combine source-owned facts with attributed authored meaning into narrow view
contracts. It may select already-declared fields and current health; it does not generate, rewrite or
persist meaning.

**Owner cockpit.** Render semantics through existing Restless primitives and visual language. Keep
the native outcome primary, evidence progressively disclosed and action consequences visible. No
second component library or generic schema-driven form builder.

**Runtime.** Preserve native ReviewTargets and exact artifacts. Actor contexts receive the shared
authoring discipline at the point each user-visible field is written. No presentation daemon or
additional always-on model.

## Ticket decomposition

Status lives only in this checklist; ticket files contain scope and evidence contracts, not another
status system.

| Status | Ticket                                                                                                            | Slice                              | Outcome or friction served                                                                 | Prior machinery made deletable                                                          |
| ------ | ----------------------------------------------------------------------------------------------------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------- |
| [ ]    | [S28-T0 · Freeze the reader corpus and semantic grammar](sprint-28/t0-corpus-and-grammar.md)                      | Product contract + Evaluation      | One long Attention example can overfit the whole communication model                       | Ad hoc screenshots and remembered prose examples as the acceptance basis                |
| [ ]    | [S28-T1 · Preserve meaning in narrow source-backed contracts](sprint-28/t1-semantic-contracts.md)                 | OrgIntel + Authority + projection  | Prose and Markdown currently carry distinctions the source already knows                   | Renderer parsing of prose; duplicated UI-invented action meaning                        |
| [ ]    | [S28-T2 · Make accountable authorship revisable and checkable](sprint-28/t2-authoring-validation.md)              | Actor context + write paths        | Prompt-only guidance is neither structural enforcement nor proof of readability            | Anonymous rewriting; objective defects discovered only in the browser                   |
| [ ]    | [S28-T3 · Render Attention as the decision it contains](sprint-28/t3-attention-composition.md)                    | Owner cockpit                      | Five prose regions bury real choices and their consequences                                | Wall-of-text folio; action consequence hidden in tooltip-only metadata                  |
| [ ]    | [S28-T4 · Give Work and artifacts a human opening without weakening the contract](sprint-28/t4-work-artifacts.md) | OrgIntel + Runtime + owner cockpit | Human meaning and machine instruction still share one text block                           | Prompt-parsed owner opening; command-produced artifact labels as primary UI copy        |
| [ ]    | [S28-T5 · Keep messages expressive while declaring consequential meaning](sprint-28/t5-message-semantics.md)      | OrgIntel messages + owner cockpit  | Chat must remain varied, but outcome and owner need are buried in arbitrary prose          | Markdown inference for completion, next ownership or owner need                         |
| [ ]    | [S28-T6 · Prove one readable causal narrative and purge](sprint-28/t6-dogfood-and-purge.md)                       | Full slice + Evaluation            | Individually improved screens can still contradict each other or remain hard to understand | Losing schema/renderer branches; duplicate presentation copy; prompt-only quality claim |

Expected order: **T0 → T1 → T2 → T3/T4/T5 → T6**. T3–T5 may proceed in parallel after the shared
contract lands, but T6 evaluates the combined narrative rather than accepting three isolated demos.

## Risks and dispositions

| Risk                                                 | Disposition   | Treatment                                                                                                                           |
| ---------------------------------------------------- | ------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| Structure makes every communication sound the same   | **Guarded**   | Strong contract only for consequential items; optional semantics for chat; prose form remains authored judgement                    |
| The schema becomes a universal presentation ontology | **Invariant** | Extend existing source records narrowly; no generic user-visible entity, document lifecycle or form engine                          |
| Simple language removes decision-relevant truth      | **Invariant** | Source facts/actions remain exact; uncertainty and evidence cannot be dropped by presentation; exact contract remains reachable     |
| A validator replaces judgement with heuristics       | **Invariant** | Validate structure and source relations only; behavioural review judges language; no readability gate or content keyword classifier |
| A second model silently rewrites accountable meaning | **Invariant** | Same accountable actor revises rejected writes; no model call in the projection or render path                                      |
| Widgets create choices the source cannot perform     | **Invariant** | Controls require source-owned action ids and consequences; no UI-only resolution                                                    |
| Multi-select is implemented speculatively            | **Accepted**  | Implement only if the frozen corpus contains a real independently composable source choice                                          |
| Historic records remain verbose                      | **Accepted**  | Do not rewrite history; new/current records use the contract and old evidence remains inspectable                                   |
| Concision becomes a cosmetic redesign                | **Guarded**   | Acceptance measures comprehension, consequence prediction, evidence fidelity and causal continuity, not screenshot preference alone |

## Non-goals

- rewriting historic messages, Work or owner briefs;
- a universal content-management, artifact or document system;
- a generic schema-driven form builder;
- automatic summarisation when a page is opened;
- semantic parsing of arbitrary Markdown to discover actions or state;
- banning technical language that the owner genuinely needs for the decision;
- enforcing prose quality with length, grade-level, keyword or similarity thresholds;
- turning ordinary conversation into status forms;
- adding a new approval, review, directive or decision lifecycle; or
- changing Restless's visual identity beyond the compositions needed to express the new hierarchy.

## Salvage

No legacy control-plane machinery. Reuse the current `OwnerBrief`, Attention projection, Work and
message records, artifact references, ReviewTarget gateway, generated TypeScript binding seam and
existing Svelte design system. Re-validate each against the frozen corpus before extending it; do not
lift a universal content schema, notification format, document renderer or presentation service.

## Closing evidence

T6 publishes:

1. the frozen before/after corpus with source references;
2. generated wire contracts and structural validation results;
3. desktop and mobile captures of every consequential Attention shape;
4. the native Work/artifact and conversation views used in the run;
5. blinded reader accounts and measured owner effort;
6. one end-to-end causal trace from attention through action, resumed Work and observed outcome;
7. source-fidelity and contradiction audit;
8. accessibility and keyboard-use results;
9. rejected branches and deleted code paths; and
10. remaining cases where free-form communication is preferable to more structure.
