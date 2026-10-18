---
name: go-to-market
description: Plan how the company reaches its first or next customers - who exactly, why they would choose it, what to say, where to find them, what to charge, and a first month of measured experiments. Use for a launch, a new audience or a stalled channel; do not use to run campaigns or spend money (that needs the owner's approval, per action).
---

# Go to market

A plan is a set of bets about who will buy and why, cheap to test before anything is spent. Build it
from what the company already knows, test the riskiest bet first, and keep everything that leaves
the company in the owner's hands.

## 1. Gather what is known

Read the charter, Brand & voice and Goals (`restless charter show`, `restless identity show`,
`restless goal list`). If there is no competitor research for this market from the last few months,
run the competitor-research skill first. Note what the owner already tried and what happened.

## 2. Who, exactly

Describe the best-fit customer so a stranger could find ten of them:

- who they are (role, kind of business or household, size, place);
- the trigger that makes them look now (a new job, a failure, a season, a rule change);
- what they use today instead, and what annoys them about it;
- where they already spend attention (communities, searches, events, publications).

Name the two or three groups worth testing; do not try to reach everyone.

## 3. Why us: positioning

Work it out in this order, each in a sentence, from evidence:

1. **The alternatives** the customer would otherwise use (from the competitor research).
2. **What only we have** compared with those alternatives.
3. **The value** that gives the customer, in their terms (time, money, risk, status).
4. **Who cares most** about that value (the best-fit customer above).
5. **The category** that makes the value obvious ("a mobile bike mechanic", not "a platform").

Then the message: a headline, a one-line explanation, and three proof points, in the company's
voice. Offer two or three alternative headlines for the owner to choose between.

## 4. Where and how much

- **Channels:** list candidates, then rank them by how fast they give a signal, effort, cost and
  fit with where the customer already is. Pick two to test first.
- **Price:** a hypothesis anchored on the alternatives' prices and the value, with the cheapest
  way to test it (a pricing page, a pre-order, three sales conversations). Say what would change
  it.

## 5. Launch and a first month of experiments

- **Launch plan:** the assets needed (a landing page, a deck, an announcement, a listing), in
  order, with who makes each. Hand assets to the landing-page-design and presentation-deck skills.
- **Experiments:** a sheet (`restless sheet create "Go-to-market experiments"`) with one row per
  bet, added with an `import_csv` action (`restless sheet edit <sheet> @rows.json`). Columns:
  hypothesis, action, metric, success threshold, cost, effort, owner, start and end. Order by the
  riskiest assumption first. Five to eight is enough for a month.
- **Measure:** propose one to three Goals for the month (`restless goal add` only once the owner
  agrees), each with an observable finish line.

## 6. Write it up

Publish a Library document (`restless document create --kind plan`):

1. **The plan in five lines:** who, why us, the message, the two channels, the price to test.
2. **Customer, positioning, message** with the evidence behind each.
3. **Channels and price** with the ranking and its reasons.
4. **Launch and experiments** linking the sheet.
5. **Risks and what we will learn first.**
6. **Sources.**

Ask the owner to choose the headline and confirm the two channels and the price to test, with
those as quick replies.

## Rules

- Draft, never send. Posts, emails, outreach, ads, listings, sign-ups and any spend go through the
  owner's approval for each action, by the company's normal authority path.
- No claims the company cannot back up, no fake reviews or testimonials, no buying followers or
  contact lists, no messaging people who did not ask, and respect each platform's rules.
- Facts about markets and competitors follow the research skill's sourcing rules.
- The company's voice applies to every line a customer will read (`restless identity show`).

Credits: the positioning steps follow the widely used order of alternatives, unique attributes,
value, best-fit customers and market category (as popularised by April Dunford's "Obviously
Awesome"), written in our own words.
