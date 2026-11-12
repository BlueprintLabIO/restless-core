# Details the plan left open

The plan fixes the question, the setups, the eligibility rule and the measures. Writing the scripts
needed the details below, which the plan does not state. Each is settled one way in the code so the
tests can run. None is accepted yet: the owner accepts, changes or rejects each before the pilot,
and the accepted list is then recorded as an amendment to the plan.

Items 17 and 18 are the two where a different reading of the plan would change a reported number,
and need an explicit answer.

## Choosing auctions

1. **Price bands.** $30 to under $100, $100 to under $300, $300 to under $800, $800 to $2,000
   inclusive. Four categories by four bands gives 16 cells.
2. **The price that is banded** is the current bid at the time the search page was saved, in
   Australian dollars, before postage. Listings priced in another currency or as a range are
   rejected.
3. **The random draw.** Within a cell, auctions are ordered by `sha256("<seed>:<item id>")` and
   taken from the lowest. Places are dealt one at a time around the cells, in an order also set by
   the seed, so a cell with too few auctions hands its places to the others. The seed should be
   fixed and written down before pages are saved. A shortfall is recorded and never filled by
   relaxing a rule.
4. **Selection time** is the latest save time among the search pages. An auction seen twice uses
   the later sighting.
5. **The 2 to 7 day window.** The site shows time left in two units ("3d 4h"), so the true end lies
   within a range. An auction is eligible only if that whole range, measured from selection time,
   lies between 2 and 7 days.
6. **Buy-it-now alongside bidding** does not exclude an auction; the pack says it was offered.
7. **Later batches** never reuse an auction from an earlier selection.

## Evidence packs

8. **Contents.** Title, category, condition, item specifics, current bid, bid count, time left,
   when seen, whether buy-it-now was offered, the identified item, and the sold list (title, price,
   postage, condition, sold date). No seller names, no links, no photos, no description.
9. **Removing the auction itself and cross-posts.** A sold row is removed if it has the same item
   id, the same seller name ignoring case, or the same lead photo. This is a rule of thumb: it can
   miss a cross-post by another account with a new photo, and can remove an honest second sale by
   the same seller. Counts of each removal are kept in the pack.
10. **Sold before selection.** Sold dates carry a day and no time. A sale dated on the selection
    day (Sydney time) is kept only if the sold page was saved at or before selection time.
    Sales with no readable date or price are removed.
11. **Item identification** is one step shared by every setup, supplied as a file. How it is done
    (a model call, a person, a lookup) is not decided.
12. **The simple rule's value is not in the pack**, so forecasters cannot anchor on it.

## Locking

13. **Deadline.** The earliest moment any selected auction could end, less 60 minutes.
14. **What is locked.** A hash of each forecast record and one hash over all of them, written to
    `LOCK.json` and committed with the forecasts. Proof of time beyond a git commit is not chosen.
15. **The sceptic team** is recorded as a single forecast (the lead's), carrying the whole team's
    cost.

## Scoring

16. **Error measure.** 100 × |ln(forecast ÷ final price)|, called points. Being twice too high and
    half too low score the same (69.3). The plan's "3 points" gap and "2 points" margin are read in
    these units, which are close to percentage points for small errors.
17. **Needs confirmation: decision value.** The plan says "bought when the forecast says the price
    is below 70% of the comparable median", which a forecast of the final price cannot say directly.
    The code buys when the final price is below 70% of the setup's forecast, and counts profit as
    the pack's comparable median less the final price, before fees and postage. The other reading
    is to buy when the forecast itself is below 70% of the comparable median.
18. **Needs confirmation: the two baselines.** The simple rule is the median of the 10 most recent
    sold prices in the pack, before postage. The bid rule is the current bid times a factor per
    category, where the factor is the median of final ÷ current bid in a separate earlier batch.
    That earlier batch costs extra auctions that the plan's count of 40 and 300 does not include.
19. **A crowd's forecast** is the median of its members' central values, and likewise of their low
    and high values, from the final round. Cost adds every member and every round.
20. **The main comparison** (five agents against one agent at five times the effort) is a paired
    test on per-auction error. "Closer" needs p below 0.05; "the same within 2 points" needs both
    one-sided tests to pass at 0.05; anything else reads "inconclusive at this sample size". The
    95% range on the difference is the one already fixed in the method.
21. **Auctions needed** to see a 3 point gap: ((1.96 + 0.84) × spread of the paired differences
    ÷ 3)², rounded up, for 80% power.
22. **Excluded auctions** (ended early, cancelled, relisted, unsold, or no outcome recorded) are
    counted and reported, never scored.
23. **Range hit rate** counts a final price equal to either end of the range as inside.

## Format

24. **Saved pages** are an HTML file with a small companion file giving kind, address, save time
    and category or item id. The collection step must write this format.
