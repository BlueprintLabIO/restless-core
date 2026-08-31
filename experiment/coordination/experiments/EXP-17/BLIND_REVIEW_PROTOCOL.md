# Frozen blind outcome-review protocol

Each opaque candidate receives two independent, fresh source-blind GPT-5.6 reviewers. The controller
copies only these files into a new isolated review directory:

- `OWNER_BRIEF.md`, byte-identical to the producer brief;
- `FROZEN_RUBRIC.json`, containing only that task's locked rubric;
- `candidate/`, the immutable terminal artifact;
- `NATIVE_GATE.json`, the visible and hidden native result without process identity; and
- `REVIEW_TASK.md`, the exact instruction below.

The directory contains no arm label mapping, transcript, session, cost, elapsed time, organisational
state, sibling candidate, prior score, hidden solution or reviewer note. Reviewers may read but not edit
the candidate. They write only `REVIEW.json`.

## Exact reviewer instruction

Act as an exacting independent reviewer for the named consumer, not as a producer or copy editor. Read
the owner brief, locked rubric, complete candidate and native-gate receipt. Inspect concrete artifact
evidence. Do not infer how the artifact was produced and do not reward verbosity, code volume, process
claims or stylistic polish that does not improve the consumer outcome.

Write valid JSON to `REVIEW.json` with this shape:

```json
{
  "schema": "restless.exp17.blind-review.v1",
  "criterion_scores": {
    "<locked criterion>": {"score": 0, "evidence": ["specific artifact fact"]}
  },
  "weighted_score": 0,
  "serious_blockers": [
    {"kind": "locked blocker or none", "evidence": "specific native/artifact fact"}
  ],
  "consumer_decision": "accept|revise|reject",
  "strongest_quality": "one specific sentence",
  "most_material_gap": "one specific sentence",
  "uncertainty": "one bounded sentence"
}
```

Scores are 0–100 per criterion; `weighted_score` is their exact weighted mean on 0–100. A failed
non-negotiable native gate must produce the matching serious blocker and cannot be averaged away. An
empty blocker list means the reviewer positively observed none. Finish only after parsing `REVIEW.json`
and recomputing the weighted score.

## Lock and adjudication

The controller hashes both review files before revealing the sealed arm mapping. Criterion differences
over 15 points or disagreement on a blocker trigger one narrow adjudication of only the disputed
criterion against the same blind bundle. No reviewer sees the sibling artifact until every score for
the pair is locked.
