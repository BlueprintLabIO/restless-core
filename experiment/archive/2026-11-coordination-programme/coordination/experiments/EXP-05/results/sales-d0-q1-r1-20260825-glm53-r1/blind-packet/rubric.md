# EXP-05 blinded semantic evaluation

The fresh GLM-5.3 evaluator receives only the frozen owner contract, authoritative fictional
sources, exact deterministic index and native artifacts. It must not see topology, actor names,
model traces, usage, cost, arm labels or producer identity.

Score 0–10 with evidence for: usefulness, grounding, safe actionability, tail/exception handling,
uncertainty calibration and native-review readiness. Identify every consequential defect. Exact
population verification remains authoritative for coverage and policy fields; this review must not
replace it. Return exactly one JSON object with this shape and no additional top-level keys:

```json
{
  "scores": {
    "usefulness": 0,
    "grounding": 0,
    "safe_actionability": 0,
    "tail_handling": 0,
    "uncertainty_calibration": 0,
    "native_review_readiness": 0
  },
  "worst_unit": {"id": "exact unit id", "score": 0, "defect": "concise or none"},
  "high_consequence_breach": false,
  "consequential_defects": [],
  "evidence": ["concise artifact-grounded observation"],
  "decision": "accept"
}
```

Every score may be an integer or decimal from 0 through 10. `decision` is exactly `accept`, `repair`,
or `reject`. Private reasoning, topology guesses, markdown fences and surrounding prose are invalid.
