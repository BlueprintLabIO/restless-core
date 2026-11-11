# T5 — Source-complete product investment decision

Prepare one decision-ready research memo for the fictional Cosmon `_test` company using only the
frozen corpus below. This tests evidence breadth and synthesis, not live market research. Do not browse,
contact anyone, publish anything or treat controlled evidence as real demand.

## Decision

Cosmon has one eight-week milestone available before its next owner review. Decide how it should
allocate that milestone between:

- **Option A — content expansion:** complete and ship a second explorable biome with additional
  creature encounters; or
- **Option B — first-session comprehension and performance:** improve battle readability,
  onboarding and performance on the already implemented Sunleaf Basin loop.

A staged hybrid is allowed only if it names one primary bet, exact sequencing, resource feasibility
and a falsifiable gate. “Do both” without a real trade-off is not a decision.

## Frozen corpus — region A: behaviour, product and delivery evidence

### A-01 — controlled first-session funnel

In one fictional internal playtest, 120 invited desktop testers received access, 96 loaded the build,
72 began exploration, 54 completed one Resonance Bond, 31 entered a battle, 18 completed that battle,
and 6 returned within seven days. These are stage counts from one controlled cohort, not causal effects
or market conversion estimates. No comparison cohort exists.

### A-02 — observed abandonment coding

Researchers reviewed recordings for 24 of the 41 participants who stopped after bonding but before
battle completion. Thirteen recordings showed difficulty reading battle state or choosing a command,
six showed repeated frame-rate degradation during battle effects, and five ended for reasons the
recording did not establish. The sample is incomplete and the codes can overlap with unobserved causes.

### A-03 — reproducible performance probe

The frozen browser probe covered 12 supported desktop configurations. All 12 loaded Sunleaf Basin;
four fell below 30 frames per second during stacked battle effects, while none fell below 30 during
plain exploration. The probe did not cover arbitrary consumer hardware and did not measure perceived
fun.

### A-04 — current product surface

The current build has one explorable biome, 12 base species, six evolved forms, visible wild-creature
temperaments, Resonance Bond capture, hybrid action/command combat and elemental interactions. The
existing loop is executable, but no supplied evidence establishes commercial polish, retention or
content sufficiency.

### A-05 — delivery estimates

The game lead's frozen estimate for Option A is four to six engineering weeks plus three art/content
weeks, with integration depending on the unfinished encounter-authoring tool. Option B is estimated at
three to four engineering weeks plus one design week; its battle telemetry and effect-stress harness
already run in the repository. Estimates are planning evidence, not guarantees.

## Frozen corpus — region B: player meaning, strategic constraint and uncertainty

### B-01 — coded interviews

Sixteen fictional playtest interviews were coded. Eleven participants wanted another place to explore,
nine highlighted creature behaviour or personality, ten described battle commands or effects as hard
to understand initially, and four explicitly said the current session felt too short. Counts may
overlap and must not be converted into population percentages.

### B-02 — artifact-only expert review

Three independent reviewers played the same frozen build without seeing the team's reasoning. All
three could complete a battle after learning the controls; two said the first battle communicated
enemy intent poorly; two said Sunleaf Basin and the bonding interaction were distinctive enough to
support another iteration. This is expert judgement over one build, not user-retention evidence.

### B-03 — milestone constraint

For the next eight weeks the fictional team has one gameplay engineer, one generalist
designer/technical artist and no additional contractor budget. Public launch, multiplayer, payment and
mobile support are outside the milestone. The owner wants the next review to reduce uncertainty about
whether the core loop deserves further investment.

### B-04 — content readiness

The second-biome concept has an approved visual direction and 40 percent of its environment blockout,
but no production creature set or complete encounter script. The encounter-authoring tool has two
known manual steps. This is production readiness, not evidence that players need the biome first.

### B-05 — evidence boundary

Every source in this corpus is fictional or controlled. There is no supplied evidence of real demand,
payment, public conversion, statistically representative preference, causal retention improvement or
the effect of either option after implementation.

## Required analysis

The final decision must:

1. reconstruct the relevant funnel with correct denominators rather than flattening stage counts;
2. distinguish observations, estimates, expert judgement, assumptions and unknowns;
3. reconcile the genuine tension between requests for more exploration/content and evidence of
   first-session battle friction;
4. compare both options on learning value, product value, feasibility, reversibility and downside;
5. choose one primary milestone or a genuinely gated sequence;
6. name what evidence would falsify or reverse the recommendation;
7. state the next executable action and the exact owner decision, if any, that remains.

## Required artifact and exact schema

All changes must be confined to a new `research-decision/` directory and new root file
`verify-research-decision.mjs`. Do not edit game/product files. Produce one clean commit containing:

1. `research-decision/manifest.json`, valid JSON with:

   - `schema_version`: exactly `exp03-t5-v1`;
   - `company`: `{ "id": "cosmon_test", "fictional": true }`;
   - `decision`: object with non-empty `primary_bet`, `recommendation`, `rationale`,
     `rejected_or_deferred`, `confidence`, `falsifier`, and `next_action`;
   - `claims`: at least eight unique objects with `id`, non-empty `text`, `kind` chosen from
     `observation`, `estimate`, `judgement`, `assumption`, or `unknown`, and non-empty `source_ids`;
   - `options`: exactly two objects, one each for `A` and `B`, with non-empty `benefits`, `risks`,
     `learning_value`, and `feasibility`;
   - `contradictions`: non-empty array; every item has `tension`, `source_ids` spanning both source
     regions, and `resolution`;
   - non-empty `uncertainties` and `decision_gates` arrays;
   - `nothing_external`: exactly `true`.

2. `research-decision/decision.md`: concise executive decision memo containing the recommendation,
   funnel, option comparison, contradictions, evidence limits, falsifier, next action and source
   citations in exact `[A-01]`…`[B-05]` form.
3. `research-decision/evidence-map.md`: every source exactly once as a source heading, the claims it
   supports, its evidence kind and its limitations. It must make cross-source contradictions visible.
4. `research-decision/review.html`: self-contained readable owner review of the decision, option
   comparison, decisive evidence, uncertainty, gate and next action. No script or remote resource.
5. `verify-research-decision.mjs`: dependency-free Node verifier using built-ins only. It must verify
   schema, complete source coverage, valid citations, numeric transcription, option and contradiction
   coverage, fictional/controlled boundaries, review surface and no edits outside allowed paths. It
   prints explicit PASS lines and exits nonzero precisely on failure.

The verifier covers checkable fidelity, not whether the strategic judgement is wise. Verify the final
artifact from a clean checkout. Finish with `outcome_met`, the exact commit ID and exact verifier
output. If the corpus does not determine a confident decision, make a bounded recommendation under
explicit uncertainty instead of inventing evidence.
