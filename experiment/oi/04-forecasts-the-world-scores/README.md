# Experiment 4 preparation: forecasts the world scores

Scripts for [the plan](../04-forecasts-the-world-scores.md). They choose auctions, build the
evidence each forecaster sees, lock the forecasts before any auction ends, and score them once the
auctions have ended.

Nothing here fetches a web page. Every script reads pages that were saved to disk by a separate
collection step, which does not exist yet (see "Not built").

## Not checked against the real site

**No page from eBay has been read by this code.** This work was done from a machine that must not
fetch the marketplace, so:

- the class names in `pages.SELECTORS` were written from memory and are unverified;
- every page under `tests/fixtures/` is invented by `tests/make_fixtures.py` using those same class
  names, so the tests prove the selection, filtering, locking and scoring logic, and do not prove
  that the parsers fit eBay's pages;
- the wording of time left ("3d 4h left"), sold dates ("Sold 10 Nov 2026"), postage and bid counts
  is likewise assumed.

The first job wherever collection runs is to save one search page, one listing page and one sold
page by hand, run the parsers on them, and correct `pages.SELECTORS`. The parsers stop with an error
when they find nothing, rather than returning an empty list, so a changed or blocked page cannot
pass silently as "no auctions".

## Order of use

Run from this folder. Python 3.12, standard library only.

1. Save pages (collection step, not built). Each page is `name.html` beside `name.meta.json`:

   ```json
   {"kind": "search", "url": "...", "captured_at": "2026-11-20T01:00:00Z", "category": "cameras"}
   {"kind": "listing", "url": "...", "captured_at": "...", "item_id": "1234567890"}
   {"kind": "sold", "url": "...", "captured_at": "...", "item_id": "1234567890"}
   ```

   `category` is one of the four in `rules.json`. A sold page is the search of recently sold
   comparable items for the auction named by `item_id`.

2. Choose auctions:

   ```
   python3 select_auctions.py --pages PAGES --batch pilot --n 40 --seed SEED --out selection.json
   ```

   Add `--exclude earlier-selection.json` so a later batch never reuses an auction. Exit code 2
   means fewer eligible auctions than asked for. The output counts every rejection by reason.

3. Identify each item (same step for every setup, not built): write `identified.json`, a map from
   item id to the item's name.

4. Build evidence packs:

   ```
   python3 build_packs.py --selection selection.json --pages PAGES --identified identified.json --out packs
   ```

   One `packs/<item_id>.json` per auction and `packs/MANIFEST.sha256`.

5. Run the forecasters (in a sandbox company, not here) and gather `forecasts.jsonl`, one record per
   line: `item_id, setup, member, round, central, low, high, model, tokens_in, tokens_out,
   tool_calls, wall_seconds, usd_equiv`.

6. Lock before the earliest auction can end:

   ```
   python3 lock_forecasts.py lock --selection selection.json --forecasts forecasts.jsonl --out LOCK.json --setups one_agent,one_agent_5x,crowd_5 --commit
   python3 lock_forecasts.py verify --forecasts forecasts.jsonl --lock LOCK.json
   ```

   It refuses if any forecast is missing, duplicated or malformed, or if the clock is past the
   deadline (earliest possible end, less 60 minutes).

7. After the auctions end, record `outcomes.json` (collection step, not built):
   `{"<item_id>": {"status": "sold", "final_price": 212.5}}`, or a status of `ended_early`,
   `cancelled`, `relisted` or `unsold`.

8. Score:

   ```
   python3 score.py learn-uplift --packs EARLIER/packs --outcomes EARLIER/outcomes.json --out uplift.json
   python3 score.py score --selection selection.json --packs packs --forecasts forecasts.jsonl --lock LOCK.json --outcomes outcomes.json --uplift uplift.json --out results
   ```

   Scoring refuses forecasts that no longer match the lock. It writes `results/results.json` and
   `results/per_auction.csv`.

## Files

| File | What it does |
|---|---|
| `rules.json` | The frozen rules: categories, price bounds and bands, time window, setups, margins |
| `pages.py` | Reads saved search, listing and sold pages |
| `select_auctions.py` | Eligibility and the seeded stratified draw |
| `build_packs.py` | Evidence packs, with the target auction and likely cross-posts removed |
| `lock_forecasts.py` | Checks, hashes and commits forecasts; verifies them later |
| `stats.py` | Error measure, paired test, equivalence test, bootstrap, sample size |
| `score.py` | Baselines, crowd aggregation, accuracy, range hit rate, cost, the main comparison |
| `DECISIONS.md` | Every detail the plan left open and how it was settled here, for the owner to accept |
| `tests/` | `make_fixtures.py` writes the invented pages; `test_pipeline.py` has 34 tests |

## Tests

```
python3 -m unittest discover -s tests
```

Last run in this work: `Ran 34 tests ... OK`. They use only the invented pages and never the
network.

## Not built

- **The collection step.** No fetcher, no pacing, no page saving. Where it may run is the owner's
  decision, the pacing rules are not written down in this repository, and whether eBay's terms
  allow automated fetching at all has not been checked.
- **Reading ended-auction pages.** Final prices come in through `outcomes.json`.
- **Item identification.** `identified.json` is an input.
- **Listing descriptions.** Packs carry title, condition and item specifics only.
- **Public timestamping.** A git commit time is set by the committer, so `--commit` alone does not
  prove when the lock was made. Pushing the lock commit somewhere public, or stamping the root hash
  with a third party, is still to be chosen.
- **Charts.** No plotting library is installed here.
- **Category ids and search addresses.** None are hard-coded; they need to be chosen against the
  live site.
- **The forecasters themselves**, and any run. This folder has produced no data.
