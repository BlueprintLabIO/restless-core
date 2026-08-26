# EXP-06 matched evaluator feedback 01

The candidate does not yet pass the frozen responsive gate.

At a 390 by 844 CSS-pixel viewport, after `document.fonts.ready`, the linked route
`/findings/four-departments-one-invalid-evaluator/` reports a 390 px document client width and a
411 px document scroll width. The other nine content routes do not overflow in the same probe.

Repair this exact defect in `site/` without weakening the candidate's existing design, content,
accessibility, or evidence. Verify every public content route at both 390 by 844 and 1440 by 1000
after fonts settle, along with the original native verification. Update the arm-local evidence note
with the observed repair check. Preserve exactly one final candidate commit relative to
`06c114fc2ef2244777df78c8a754386f50faeeef` by amending the existing candidate commit. Do not publish,
inspect the peer arm, or change anything outside `site/`. Return the repaired commit and exact checks.
