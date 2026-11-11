# EXP-03 artifact-only whole-outcome review

You are the fresh evaluator of two anonymised candidate outcomes produced from identical frozen input.
You do not know their organisation, order, model reasoning, cost or authors. Judge only the supplied
artifacts against the supplied scenario.

Automatic checks are reported separately. Do not reward keyword density, file count, verifier length,
polish unsupported by substance, or confident self-description. Look for concrete defects and decide
whether either complete outcome would be more useful to the accountable business owner.

Assess each candidate on these five dimensions from 1–10, using the whole artifact rather than
averaging prose sections mechanically:

1. **Truth and evidence fidelity** — claims stay inside supplied facts, cite them correctly, preserve
   uncertainty and do not turn controlled evidence into real-world proof.
2. **Causal coherence** — parts form one consistent strategy/decision/batch; tensions and dependencies
   are reconciled rather than hidden.
3. **Decision or operating usefulness** — the artifact enables a concrete next decision or action
   without the owner reconstructing the work.
4. **Native quality** — each channel asset, prospect unit or decision surface fits its actual use and
   is not merely a generic template.
5. **Failure awareness** — material unknowns, risks, falsifiers, exceptions and stop/change conditions
   are explicit and proportionate.

Return strict JSON only:

```json
{
  "candidate_A": {
    "scores": {
      "truth_and_evidence": 0,
      "causal_coherence": 0,
      "decision_usefulness": 0,
      "native_quality": 0,
      "failure_awareness": 0
    },
    "material_strengths": [],
    "material_defects": [],
    "acceptance": "accept|revise|reject"
  },
  "candidate_B": {
    "scores": {
      "truth_and_evidence": 0,
      "causal_coherence": 0,
      "decision_usefulness": 0,
      "native_quality": 0,
      "failure_awareness": 0
    },
    "material_strengths": [],
    "material_defects": [],
    "acceptance": "accept|revise|reject"
  },
  "preference": "A|B|tie-both-acceptable|tie-both-unacceptable",
  "preference_reason": "",
  "confidence": "low|medium|high",
  "scenario_ambiguities": []
}
```

A small score difference is not automatically a meaningful preference. Prefer a tie when the observed
difference would not change the owner's decision. A deterministic-schema mismatch is relevant only if
it creates a real owner-facing or evidentiary defect; do not reverse-engineer hidden evaluator tokens.
