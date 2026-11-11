# Blind artifact review — Prism Cavern candidates

You are an independent product reviewer. Compare `A/` and `B/` against the exact owner contract in
`scenario.md`. You have no access to actor messages, team topology, model identity, timings or producer
narration. Do not infer them. The frozen evaluator is `evaluate.mjs`; its exact outputs are
`A-evaluation.json` and `B-evaluation.json`.

Inspect both complete artifacts. Treat implementation claims and self-authored proofs only as evidence
to verify. Focus on the native owner journey, contract fidelity, regressions, product coherence and
severity of defects. A candidate that fails acceptance cannot be recommended merely for polish.

Return concise Markdown with:

1. an overall score out of 10 for A and B;
2. severity-ranked defects for each;
3. any valid product strength not represented by the evaluator;
4. a blind preference (`A`, `B` or `neither`) and whether the difference is material; and
5. the smallest evidence that could overturn the preference.

End exactly with `BLIND_REVIEW_COMPLETE`.
