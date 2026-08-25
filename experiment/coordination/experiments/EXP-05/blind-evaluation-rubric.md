# EXP-05 blinded semantic evaluation

The fresh GPT-5.6 Sol evaluator receives only the frozen owner contract, authoritative fictional
sources, exact deterministic index and native artifacts. It must not see topology, actor names,
model traces, usage, cost, arm labels or producer identity.

Score 0–10 with evidence for: usefulness, grounding, safe actionability, tail/exception handling,
uncertainty calibration and native-review readiness. Identify every consequential defect. Exact
population verification remains authoritative for coverage and policy fields; this review must not
replace it. Return structured JSON plus a short decision: `accept`, `repair`, or `reject`.
