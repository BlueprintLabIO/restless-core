# Experiment 4: Forecasts the world scores

**Status:** plan, not started · **Can run without the owner:** yes, but publishing the locked
predictions in advance needs the owner's approval

## Question

Do AI teams predict real outcomes better than one agent with the same budget, when the answer comes
from the world after the predictions are locked?

## Why it matters

Most agent results are graded by other models or replayed on the past, and readers discount both
("paper money", "backtested"). An auction's final price is a real outcome nobody knows in advance. It
also tests a classic finding about human groups: averaging independent guesses beats most individuals,
and some kinds of discussion make groups worse.

## What we expect, and what would prove us wrong

- The average of 5 independent agents beats one agent. Wrong if the gap is within the range of noise.
- Letting agents see each other's answers before revising helps less than keeping them independent
  (discussion makes guesses converge on each other, losing the crowd's independence).
- **A single agent given 5 times the budget does as well as the team.** This is the check that the team
  isn't winning only because it spent more. If it holds, "team" means nothing beyond "more compute".
- Mixing model families beats a crowd of one family.

## Ground truth

Auctions on eBay (Australia, plus US if AU volume is too low) in categories the resale tools know:
cameras and lenses, consumer electronics, tools, LEGO. Selection rule, frozen in advance:

- auction format, ending 2 to 7 days after selection, at least one bid at selection;
- current price between $30 and $2,000;
- a random draw from search results, stratified by category and price band, by script;
- recorded at selection: listing snapshot, current bid, bid count, time left.

**Exclusions, declared now:** auctions that end early, are cancelled, relisted, or end without a sale
are removed and counted in the results. There is no way to detect shill bidding; this is stated as a
limit.

## Rules for the forecasters

- Each forecast gives a central price and an 80% range. For crowds, the central price is the median of
  the members' central prices, and the range runs from the median of their lower ends to the median of
  their upper ends.
- **No live web access.** At selection, a script freezes one evidence pack per auction: the listing
  snapshot, and sold-item results for the identified item up to the selection timestamp only. Same-item
  cross-posts and the target listing itself are removed. Forecasters see only the pack. This stops
  answers leaking and keeps marketplace traffic to one fetch per item.
- **Locking:** all forecasts for a batch are written, hashed and committed before the earliest auction
  in the batch ends. With owner approval, the list of hashes is posted publicly at lock time so anyone
  can verify later that nothing changed. If the owner prefers not to post, the hash file is stamped by a
  public timestamping service (such as OpenTimestamps), which proves the time without publishing
  anything readable.
- Data collection follows the marketplace pacing rules and runs from the owner's machine, not from the
  Hetzner plane (marketplaces block datacenter addresses).

## Setups

| Setup | What it is |
| --- | --- |
| Simple rule | median of the last 10 sold prices for the identified item |
| Bid rule | current bid × a per-category uplift learned from a separate earlier batch |
| One agent | one agent, standard budget |
| One agent, 5× budget | one agent with 5 times the token and tool-call caps (caps set from the pilot; actual spend recorded) |
| Crowd of 5 | 5 independent agents, answers averaged (median) |
| Discussing crowd | 5 agents give answers, see the others' answers once, revise, then averaged |
| Team with a sceptic | a lead, two researchers and one agent assigned to argue against the leading estimate; the lead decides |
| Mixed crowd | 5 independent agents from 3 model families, averaged |

Every auction gets a forecast from every setup (paired).

**The main comparison, named in advance:** crowd of 5 against one agent with 5× budget.

## Measures

- **Main:** average error, measured as the percentage gap between forecast and final price (on a log
  scale, so a 2x miss counts the same both ways).
- How often the 80% range contains the final price (it should be about 80%).
- Decision value: had the setup been used to decide "buy if listed below 70% of forecast", how much
  would it have made or lost on these items (descriptive, not a trading claim).
- Cost per forecast.

## Sample size

Pilot: 40 auctions with one agent, one agent at 5× budget and the crowd of 5. It sets the budget caps,
estimates the uplift factor for the bid rule, measures time and cost per forecast, and measures how
much the paired difference in error varies between auctions. Pilot items are not reused.

Main run: 400 auctions unless the pilot shows more are needed. Every setup forecasts the same auctions,
and the sample is the number of auctions. The run is sized for 80% power on the main comparison at the
smallest gap worth acting on, set before the pilot at 3 points of average error.

**Equivalence margin for "teams are just compute":** the crowd and the 5× agent count as the same only
if a formal equivalence test shows their average errors are within 2 points of each other. Not finding
a difference is not enough.

## Timeline

About 25 agent forecasts per auction including the crowds: about 10,000 forecasts for 400 auctions,
each due before its batch's first auction ends, with at most 3 agents at once. Forecasts work only from
the evidence pack, so each should take about 1 to 3 minutes. At that speed 10,000 forecasts take
roughly 3 to 7 days of continuous running. Batches are sized from the pilot's measured time so each
one finishes well before its lock. Expect about 4 to 6 weeks in total. If the pilot shows forecasts take
longer, cut the discussing crowd and the team with a sceptic first.

## Budget

Estimated $400 to $1,200 equivalent, re-estimated from the pilot. Ceiling $1,200.

## Decision rules

- If a crowd beats the 5× single agent: Sydney Resale's appraisals switch to a crowd, and Restless
  offers "ask several, combine" as a pattern for judgement calls.
- If they pass the equivalence test: publish "teams are just compute" for this task, and appraisals
  stay single-agent.
- If neither: publish that the result is inconclusive at this sample size.

## What we publish

- **Title idea:** "We made AI teams predict 400 eBay auctions, then waited for the hammer to fall."
- **The one chart:** average error per setup, with ranges, the simple rules shown as reference lines.
- **Public kit:** the locked forecasts, their hashes, the final prices and the scoring script.

## Limits to state

- One market and a handful of categories.
- Auction prices are noisy; some error cannot be predicted by anyone.
- Results are for the models tested on the date tested.
