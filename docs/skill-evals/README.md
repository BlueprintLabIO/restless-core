# Skill evaluations (S97)

Each built-in skill ships only when it beats no skill on its own evaluation set. The sets live here,
outside the company image, so agents never see the answer key.

## Protocol

1. **Same everything but the skill.** One `_test` company, the same model and effort, the same
   charter and Brand. Run each case twice: once with the skill selected for the message or Work,
   once without (the skill disabled for that company).
2. **Fresh session per run**, so one run cannot learn from the other.
3. **Blind review.** An independent reviewer (a separate agent with the outcome-quality-enforcement
   skill, or a person) scores both outputs against the case's checklist without knowing which used
   the skill. Each checklist item is pass or fail; a case's score is the share it passes.
4. **Record cost.** Tokens and spend per run, from Company › Tokens.
5. **Ship rule.** The skill's average score must beat no-skill by at least 15 points with no case
   worse by more than 10, at no more than 1.5× the tokens. Otherwise revise the skill and rerun.
6. **Keep the evidence.** Save each run's output, score sheet and token count under
   `docs/skill-evals/runs/<skill>/<date>/`.

Re-run a skill's set whenever its SKILL.md changes, and when the default model changes.

## Status

Written in S97 with the skills. The first scored runs need a real model connection on a `_test`
company; they have not been run yet.
