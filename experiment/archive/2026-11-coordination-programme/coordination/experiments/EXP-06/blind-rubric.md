# EXP-06 blind outcome rubric

Reviewers receive only two production builds labelled Candidate A and Candidate B. They may inspect
the rendered desktop and mobile sites and public page source. They do not receive actor names,
topology, prompts, traces, cost, timing, commit authorship or arm identity until the score is locked.

## Gate checks

A candidate is not rankable until it builds, serves every linked route, has no horizontal overflow at
390 px or 1440 px, exposes keyboard focus, respects reduced motion and contains no em dash. A material
product falsehood or fabricated evidence is a rejection, not a small writing deduction.

## Weighted score

| Dimension | Weight | Question |
| --- | ---: | --- |
| Immediate clarity | 15 | Can an owner/operator understand what Restless does, for whom and why it matters within one screen? |
| Product truth and differentiation | 20 | Is the promise faithful to the repository, meaningfully distinct from assistants and agent frameworks, and supported without hedging it away? |
| Writing | 20 | Is the prose specific, economical, natural and free of generic AI cadence or technical self-absorption? |
| Visual direction | 20 | Does the site have a coherent, distinctive identity, strong hierarchy, negative space and one memorable signature without template clichés? |
| Information architecture | 10 | Do the distinct pages form a persuasive journey rather than a cramped long page or fragmented brochure? |
| Interaction and responsive polish | 10 | Are motion, navigation, mobile behaviour, focus and details deliberate and robust? |
| Evidence and trust | 5 | Are experiments, limitations, comparisons and calls to action presented honestly and usefully? |

Score each dimension from 0 to 10, multiply by its weight, and report the weighted total out of 100.
For each candidate, name the three strongest choices and every defect that would block publication.
Then choose one of `A wins`, `B wins`, `tie`, or `neither publishable`, with a concise evidence-based
reason. The owner remains the final judge.

## Process measures, revealed only after blind judgement

- elapsed time from owner request to review-ready commit;
- model turns, input/cache/output usage and provider failures where observable;
- number and role of productive actors;
- supervision interventions, repair loops and unaccepted attempts;
- files and lines changed, dependency delta and production bundle size; and
- whether the arm completed its own visual inspection and evidence note.

Process efficiency is explanatory and breaks a near tie. It does not rescue a weaker published
outcome.
