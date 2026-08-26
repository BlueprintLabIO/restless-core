# Sprint 20 - Dogfood 4: publish the research record

**Status:** Draft for founder alignment; specification only, no implementation or run started

**Date:** 26 August 2026

**Depends on:** EXP-07's founder-accepted Restless greenfield site; the completed EXP-01 through
EXP-07 evidence corpus; Sprint 19's scenario-native delivery path; current non-producing lead and
native-review contracts.

**Design baseline:** Restless EXP-07 candidate branch `experiment/exp-07-candidate-a` at commit
`4e454bc2a36eab3afbf7d91a070e732cf657a847`. The incumbent site is not a design input.

**Spec refs:** `ARCHITECTURE.md` §2, §4, §5, §7.3, §11 and §16;
`docs/COORDINATION_THEORY.md`; `evaluation-dogfood` §1-§8, §16-§21 and §25;
`docs/FRONTEND_DESIGN_REFERENCES.md`.

---

## Why this sprint exists

Restless has accumulated decision-changing experimental evidence, but most of it remains in sprint
reports, run notes and synthesis files written for the founders. The accepted public site contains a
small journal, not the research publication the evidence now supports.

The useful company outcome is not "add a blog." It is to turn the experimental record into a pointed,
readable and defensible public body of work without laundering provisional findings into doctrine or
flattening every experiment into a chronology.

This is a company dogfood and publication sprint. It tests whether Restless can use its successful
organisation to produce rigorous editorial work and a coherent live site. It does not test whether a
reusable team playbook caused that success; EXP-08 isolates that later.

## Outcome

Using Restless as the operating company, expand the accepted Restless site into a complete research
publication in which:

- every material finding from EXP-01 through EXP-07 has an explicit public home or a recorded reason
  it should remain unpublished;
- each article advances one sharp thesis through a narrative grounded in exact evidence;
- an independent peer reviewer can reject unsupported, generic or poorly written work;
- the journal, research index, product explanation and related reading form one calm, coherent site;
  and
- the owner receives a live-probed candidate plus one bounded publication decision, not a folder of
  drafts or an agent-management task.

External publication, replacement of the current hosted site, branch promotion and distribution are
prepared effects. This draft does not authorise them.

## Frozen organisation and intelligent freedom

```text
owner
  -> available Exec
      -> Research Publication lead - non-producing, accountable for the complete publication
          -> article owner(s) - one end-to-end Staff writer-researcher per article
          -> independent peer reviewer - Staff, critiques exact drafts against evidence
          -> web producer only if site integration is a stable, separately useful production seam
```

- Exec delegates the outcome and returns to portfolio availability.
- The publication lead sets the editorial thesis, commissions, calibrates, judges and accepts. It does
  not draft articles, rewrite weak prose or repair the site privately.
- One writer-researcher owns an article from source inspection through final revision. Articles are
  locally closing units; after one calibration article, the lead may add writers if a real backlog
  and provider capacity justify parallel production.
- The peer reviewer receives the rendered draft, claim map and authoritative sources, but not the
  writer's private reasoning or persuasive justification. The reviewer returns `accept`, `revise` or
  `reject` with exact defects and does not silently become the final author.
- The lead decides whether site implementation stays with an article owner or warrants a distinct web
  producer. No fixed department pipeline is prescribed.

The counted model envelope is GPT-5.6 Sol with no fallback: medium effort for Exec, lead and producers;
high effort for the fresh peer-review responsibility. Same-model correlation is reported. Reviewer
independence comes from withheld production context, an adversarial publication-quality mandate and
direct access to sources, not from pretending model identity is irrelevant.

## Editorial contract

The publication lead chooses titles and grouping, but every accepted article must make this structure
legible without turning it into a visible template:

1. the consequential belief, tension or practical question;
2. what the team expected and why;
3. the smallest experiment that could change the decision;
4. what actually happened, including failures and conflicting evidence;
5. the explanation that best fits the observation;
6. what changed in Restless or its operating theory;
7. where the conclusion stops, what remains unknown and what would falsify it; and
8. exact links to the underlying experiment evidence.

An article has one primary thesis. It may combine experiments when they answer the same question, but
it may not become a summary of summaries. Public claims distinguish observation, inference, owner
decision, product hypothesis and open question. Controlled `_test` outcomes are never presented as
market or customer facts.

The writing bar is direct, specific and narrative-driven. Reject abstract openings, throat-clearing,
generic AI prose, false universality, repetitive conclusions, ornamental eyebrow labels, em dashes,
card grids used as default layout, decorative gradients and unsupported superlatives. A rigorous
article remains enjoyable to read; an evidence appendix is not a substitute for a point of view.

## Success contract

Sprint 20 passes only when all of the following are observed in one versioned run.

### Research completeness and truth

1. **Corpus map.** A versioned map links every material EXP-01 through EXP-07 finding to an accepted
   article, an explicitly deferred article or a reason the finding should remain internal. File count
   is not coverage.
2. **Pointed articles.** Every published candidate page has one defensible thesis, substantive
   narrative, counter-case or boundary, and exact evidence locators. No page exists merely to hit an
   experiment count.
3. **Epistemic integrity.** Observed facts, inferences, product decisions, hypotheses and unknowns are
   distinguishable. Citations entail the public claim at the scope stated.
4. **Honest limits.** Negative results, harness failures, model/provider conditions, single-run limits
   and changed conclusions remain visible where they materially affect interpretation.

### Independent editorial review

5. **Fresh peer review.** Every article receives an attributable fresh-context review against the
   exact draft and sources. The review can genuinely reject and records unsupported claims, missing
   counterevidence, causal overreach and prose defects.
6. **Bounded revision.** The article owner answers one consolidated revision packet. A second repair
   is allowed only when the lead names the remaining consequential defect; otherwise the article is
   accepted, deferred or rejected rather than polished indefinitely.
7. **Lead judgement.** The publication lead inspects the rendered final article and its review record,
   then makes the exact acceptance decision. Reviewer approval alone does not close the outcome.

### Native site outcome

8. **One coherent publication.** Research and journal navigation, article pages, related reading and
   the product story feel native to the accepted Restless design rather than a bolted-on content
   system or a second visual identity.
9. **Prepared native review.** Every route builds from a clean install and is live-probed at desktop
   and mobile. The primary ReviewTarget opens the exact candidate site. There is no horizontal
   overflow, broken route, inaccessible navigation, console error or motion-only meaning.
10. **Writing in context.** The peer reviewer and lead inspect rendered pages, not Markdown alone.
    Typography, line length, hierarchy, negative space and related-page flow support long-form reading.
11. **Owner attention.** Ordinary source discovery, writing, review, revision, integration and
    verification require no owner rescue. The owner supplies taste and final publication judgement
    only.
12. **Prepared effect.** The exact commit, deployment target, rollback point and public change are
    prepared. Nothing is merged, replaced, announced or distributed without the separately governed
    authority required at that boundary.

## Evidence package

The run leaves:

1. frozen starting commit, corpus snapshot, model/effort envelope and spend ceiling;
2. the finding-to-article corpus map;
3. source and claim map for every article;
4. writer, reviewer, revision and lead-acceptance attribution;
5. rendered desktop/mobile review targets and route/build/browser checks;
6. owner attention, elapsed time, model usage, repair loops and deferred/rejected article count;
7. publication and rollback preparation; and
8. one after-action report separating editorial problems from Restless harness friction.

## Layer slices

| Concern | Owner | Sprint responsibility |
| --- | --- | --- |
| Publication responsibility, article Work, review and acceptance | OrgIntel | Preserve sparse article ownership, independent review, exact outcome judgement and Exec availability |
| Repositories, sources, site, builds and rendered pages | Company Runtime | Produce ordinary files and commits; preserve evidence locators and the native site target |
| Model access, spend and publication authority | Authority Plane | Enforce the frozen model/spend envelope and govern any deployment or public replacement separately |
| Final site and publication judgement | Owner cockpit | Open the candidate and exact decision without exposing coordination machinery by default |
| Coverage, owner attention and run classification | Evaluation | Record the corpus map, review outcomes and truthful accepted/rejected/product-invalid result |

No new Kernel entity, asset lifecycle, article database, workflow engine, reviewer state machine or
content-management system is introduced merely to run this sprint.

## Risks and dispositions

| Risk | Disposition | Treatment |
| --- | --- | --- |
| Restless converts its own beliefs into polished mythology | **Guarded** | Exact evidence locators, epistemic labels, adversarial peer review and visible negative results |
| Same-model reviewer echoes the writer | **Guarded** | Fresh responsibility, withheld production reasoning, direct sources and a mandate that rewards rejection; correlation remains a limitation |
| Parallel writers fragment voice and design | **Guarded** | One calibration article first; publication lead judges the whole native site and may keep one writer if calibration cost dominates |
| Review becomes an unbounded rewrite loop | **Guarded** | One consolidated repair by default, one consequential exception, then accept/defer/reject |
| The article count rewards shallow pages | **Accepted** | Coverage is claim-based and lead-judged; no fixed article quota is a success criterion |
| Publishing reveals internal material that should remain private | **Invariant at effect boundary** | Source classification before drafting and exact owner-governed publication effect |
| Public traffic or revenue does not appear during the sprint | **Accepted** | This sprint proves a public research asset, not market demand; later operation records real usage |

## Non-goals

- proving that the publication team topology beats another topology;
- implementing reusable team templates, a playbook registry or automatic department creation;
- inventing experiments or smoothing contradictions to create a stronger story;
- publishing every internal log, prompt, trace or private reasoning artifact;
- replacing the accepted visual direction with another redesign;
- adding a general CMS, analytics platform or content workflow; and
- claiming that publication traffic, adoption or revenue exists before external evidence does.

## Proposed ticket decomposition

Ticket files are created only after founder alignment. Status will live only in this checklist.

| Status | Proposed ticket | Slice | Outcome or friction served | Prior machinery made deletable |
| --- | --- | --- | --- | --- |
| [ ] | **S20-T0 - Freeze the Restless site and evidence corpus** | Evaluation + Runtime | The publication can silently change its source record or design base | Ad hoc source gathering and mutable acceptance claims |
| [ ] | **S20-T1 - Commission and calibrate the publication** | OrgIntel + Runtime | Many articles can create a content factory before voice and truth are calibrated | Up-front article pipeline and fixed writer count |
| [ ] | **S20-T2 - Prove one article through adversarial peer review** | Full outcome slice | Reviewer agreement may carry no information and Markdown may hide weak native reading | Same-context self-review and prose-only acceptance |
| [ ] | **S20-T3 - Close the material findings backlog** | OrgIntel + Runtime | The experimental record remains inaccessible to readers | Per-experiment summaries and compulsory model fan-in |
| [ ] | **S20-T4 - Integrate and live-probe the research publication** | Runtime + owner surface | Good drafts can still form a cramped, inconsistent or broken site | Parallel page variants and a second visual system |
| [ ] | **S20-T5 - Prepare publication, classify and report** | Authority + Evaluation | A green build can be mistaken for a public outcome | Unattributed completion claims and unrecoverable deployment steps |

## Entry, stop and exit gates

**Entry:** founders approve this contract, the exact Restless design baseline, the evidence corpus,
GPT-5.6 Sol envelope, spend ceiling, source-privacy boundary and no-publication-without-authority rule.

**Stop:** stop for source/privacy uncertainty, a candidate that cannot trace material claims, model or
spend-envelope drift, uncontrolled public replacement, branch/data corruption risk or founder stop.
An article rejection is ordinary editorial evidence, not a sprint stop.

**Exit:** the live candidate, corpus map, review records, evidence package and truthful terminal
classification are complete. An informative rejection may complete the sprint as evidence; only an
owner-accepted native publication satisfies the product outcome.
