# Case study: a frontend team that can produce a credible Aris landing page

## Why the recent result failed

The poor landing page is evidence of a workflow failure, not merely a weak CSS pass. The coding actor
was effectively asked to discover product positioning, invent art direction, implement a responsive
site, judge its own taste, and know when the final browser outcome was convincing. A generic
"frontend designer" label or a design skill does not create the missing separation.

The missing loop was:

```text
commercial brief
-> explicit visual direction and references
-> bounded implementation
-> live desktop/mobile outcome
-> independent visual criticism
-> revision against concrete evidence
-> lead and owner acceptance
```

## Smallest team that fits this work

The accountable lead should keep the roster as small as possible and combine roles when one actor has
proven competence.

| Responsibility | Difference it contributes | Required output |
|---|---|---|
| **Product/creative lead** | Holds the commercial goal and makes trade-offs | One chosen direction, brief, priorities, final synthesis |
| **Art director / visual designer** | Visual taste and reference synthesis without implementation tunnel vision | Mood/reference board and a concrete visual system |
| **Frontend design engineer** | Converts the direction into a working responsive surface | Live implementation in the real design system |
| **Offer/copy specialist** | Plain-language buyer comprehension and offer hierarchy | Section copy and CTA matched to tutoring centres |
| **Independent visual critic** | Sees the rendered outcome without producer reasoning | Specific evidence-based objections and pass/revise recommendation |

This is not a permanent five-person department. The lead may use three actors by combining creative
lead with copy and combining visual design with implementation. The critic should remain independent
for a judged external outcome.

## The role of a frontend-design skill

The public [Anthropic frontend-design skill](https://skills.sh/anthropics/skills/frontend-design) is a
promising candidate because it explicitly asks for intentional aesthetic direction, typography,
composition, motion, and avoidance of generic AI-looking layouts. That makes it better procedural
context than "make it look good."

It is still only a candidate:

- its popularity is install telemetry, not outcome evidence;
- its publisher provenance does not prove fit with Aris;
- a model may follow it differently; and
- it cannot see whether the page works unless the actor receives browser evidence.

Vercel's [web-design-guidelines skill](https://skills.sh/vercel-labs/agent-skills/web-design-guidelines)
serves a different function: it is primarily an audit/standards critic. It should not be mistaken for
art direction. It also fetches current guidance, which makes results drift unless the fetched version
or resulting rules are captured.

## Visual references are not optional for a taste target

OpenAI's current [responsive frontend workflow](https://learn.chatgpt.com/use-cases/frontend-designs)
starts from screenshots or visual references, reuses the repository's design system, opens the result
in a real browser, and compares desktop and mobile views until they match the intended direction.

This vendor guide does not prove visual excellence, but its causal logic is sound:

- a model asked for "wow" must infer an enormous hidden target;
- references turn part of that target into observable evidence;
- a real browser exposes layout, hierarchy, cropping, and responsive failures; and
- iteration against screenshots is more reliable than judging source code.

The prior review-target decision in Restless is therefore important: for websites, the primary review
artifact should be the actual running site, with screenshots or reference comparison alongside it.
The code diff is supporting evidence.

## Proposed work sequence for a future Aris run

### 1. Lead defines the commercial outcome

The page should make a tutoring-centre operator understand, within one screen:

- Aris supplies full selective-entry practice papers;
- papers cover thinking skills, reading, maths, and writing;
- complete answers are included;
- delivery can be cheap, fast, and regular; and
- centres can optionally license a digital student environment.

The owner reviews this positioning only if judgement is needed. The lead retains ordinary team
coordination.

### 2. Art director produces visible direction before implementation

The brief should include:

- two or three relevant visual references, with the exact qualities to borrow;
- a restrained palette and typographic hierarchy;
- section rhythm, sample-paper treatment, and image/art direction;
- what should make the page feel credible to tutoring centres; and
- anti-goals, including generic SaaS gradients, vague AI claims, fake dashboards, and consumer copy
  that makes centres feel displaced.

One direction is selected and the unused branches are purged.

### 3. Copy specialist produces the offer structure

Copy is written for a reader with limited English:

- short sentences;
- concrete product nouns;
- one idea per section;
- no inflated AI language;
- clear sample PDFs for each subject; and
- a simple reply-to-email or contact action appropriate to the campaign.

### 4. Design engineer implements the selected direction

The actor receives the chosen visual brief, copy, repository design system, and exact responsive
targets. It does not reopen the product strategy unless implementation uncovers a real contradiction.

### 5. Critic reviews the final surface

The critic receives the commercial criteria, selected references, and live desktop/mobile page, but
not the producer's reasoning. Its report must cite visible evidence. Useful questions include:

- Can the buyer identify the product and audience without scrolling?
- Do the four paper sections feel substantial rather than decorative?
- Are sample PDFs visibly real and easy to inspect?
- Does the digital option read as optional support, not competition with the centre?
- Is the visual hierarchy deliberate at desktop and mobile sizes?
- Does any part still look like a generic AI-generated SaaS template?

### 6. Lead revises Work and presents the outcome

The lead converts objections into bounded revisions, resolves conflicts between copy and design, and
presents the live site to the owner. The owner is asked for taste or commercial judgement, not to
coordinate the team.

## Validation

A future run should compare the team process with the previous result using the same buyer and offer.
Evidence should include:

- live desktop and mobile URLs or captured browser sessions;
- the selected visual references;
- critic findings before and after revision;
- owner accept/revise decision and number of interventions;
- time and model cost; and
- a simple comprehension check by someone not involved in production.

The team process wins only if the finished surface is materially better and the owner coordinates
less. A more elaborate team that produces the same page at higher cost falsifies the hypothesis.

