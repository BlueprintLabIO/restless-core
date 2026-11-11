# Organisational Intelligence: charter

**Status:** draft for the owner, 12 November 2026

## The idea

A model is intelligent. An organisation of models can be more or less intelligent than the models in
it. Human companies have known this for a century: structure, handoffs, reviews and memory decide
whether a group of smart people acts smart. AI organisations are new, cheap to copy and easy to
measure, so their rules can be studied far faster than human ones.

**Organisational Intelligence (OI)** is the study of what makes a group of AI agents, working for a
person, produce better outcomes than its members could alone, and what makes it worse.

Restless has two layers:

- **The product:** the UI and features people use to run their AI companies.
- **OI underneath it:** the research that decides how those companies are organised. Every default in
  Restless (who reviews, how many workers, what gets escalated to the owner, what is remembered) should
  trace back to an OI finding, and say so.

## Why this and not "AI runs your company"

Research on what does well on Hacker News (12 November 2026) found that "an AI runs your company"
posts are met with hostility and accusations of astroturfing. What earns attention is a controlled
measurement, a real result anyone can check, honest limits and negative results. OI lets Restless talk
about what it knows, not what it claims.

## The questions

1. **Information:** how much of what the owner says survives each handoff, and what survives
   selectively (does bad news get softened on the way up)?
2. **Checking:** do reviews catch mistakes, or only add cost?
3. **Judgement:** do teams make better calls than individuals, scored by the real world?
4. **Structure:** what org chart is best for which kind of work, and does the answer change as models
   get stronger?
5. **Attention:** how little of the owner's attention can a company need while staying right?
6. **Memory:** does an organisation learn from its own mistakes?

The theory to test across all of them comes from Ronald Coase's theory of the firm: an organisation
grows until the cost of coordinating inside it exceeds the benefit. For AI agents, that cost is
concrete: the tokens spent handing context over and the information lost at each handoff. Stronger
models hold more in one head, so the prediction is that **the best AI organisation gets flatter as
models get stronger.** It is a prediction that can fail, which is what makes it worth testing.

## Rules for the work

1. **Plain words.** Anything published must make sense to a smart reader who has never seen Restless.
   No internal labels (no "R1", "Q2", "arm") in public text.
2. **Plan before data.** Every experiment's question, setups, measures and decision rule are written
   and dated before the first counted run. Changes after that are listed as amendments.
3. **Publish what we find, including when Restless loses.** A result where one agent beats a team is a
   finding, not an embarrassment.
4. **Ground truth over opinion.** Prefer measures a machine or the world can check (tests, actual sale
   prices, planted mistakes) over model graders. Where a grader is needed, check it against the owner
   on a sample.
5. **Ranges, not single numbers.** Report how sure we are. One run per condition is a pilot, never a
   result.
6. **Something anyone can rerun.** Each public result ships with a script or task set others can run on
   their own setup.
7. **No live harm.** Experiments run in sandbox companies on restless-dev. Nothing contacts a real
   person, spends real money or touches a live company without the owner's separate approval.
8. **The owner writes the public prose.** Agents prepare data, charts and drafts of facts.

## How it shows up in the product and the voice

- The research page on restless.run lists findings, each linked to its data.
- Settings that come from a finding say so in one line, with a link ("Reviews use a different model.
  Why: reviewers on the same model missed X of Y planted mistakes.").
- Posts lead with the question and the finding. Restless is named once, as where the data came from.
- Old claims are retired in public when new evidence replaces them. The first is EXP-17 (see
  [the archive note](../archive/2026-11-coordination-programme/ARCHIVED.md)).
