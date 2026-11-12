"""Tests on made-up saved pages (tests/make_fixtures.py). Nothing here touches the network.

Run from the experiment folder:  python3 -m unittest discover -s tests -v
"""
import datetime as dt
import json
import math
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

import build_packs  # noqa: E402
import lock_forecasts  # noqa: E402
import pages  # noqa: E402
import score  # noqa: E402
import select_auctions  # noqa: E402
import stats  # noqa: E402

PAGES = HERE / "fixtures" / "pages"
IDENTIFIED = json.loads((HERE / "fixtures" / "identified.json").read_text())
RULES = select_auctions.load_rules()
SEED = "fixture-seed"
UTC = dt.timezone.utc
cam = lambda n: str(1000000000 + n)


def selection(n=8, seed=SEED, **kw):
    return select_auctions.select(PAGES, "fixture", n, seed, RULES, **kw)


class Parsers(unittest.TestCase):
    def test_price(self):
        self.assertEqual(pages.parse_price("AU $1,234.50"), (1234.5, "AUD"))
        self.assertEqual(pages.parse_price("US $20.00"), (20.0, "USD"))
        self.assertEqual(pages.parse_price("AU $5.00 to AU $9.00"), (None, "price_range"))
        self.assertEqual(pages.parse_price("call us"), (None, "price_unreadable"))

    def test_time_left_is_a_range(self):
        self.assertEqual(pages.parse_time_left("3d 4h left"), (3 * 86400 + 4 * 3600, 3 * 86400 + 5 * 3600))
        self.assertEqual(pages.parse_time_left("5h 12m left"), (18720, 18780))
        self.assertIsNone(pages.parse_time_left("Ended"))

    def test_sold_date_both_orders(self):
        self.assertEqual(pages.parse_sold_date("Sold 10 Nov 2026"), dt.date(2026, 11, 10))
        self.assertEqual(pages.parse_sold_date("Sold Nov 2, 2026"), dt.date(2026, 11, 2))
        self.assertIsNone(pages.parse_sold_date("Sold recently"))

    def test_search_page(self):
        rows = pages.parse_search((PAGES / "search-cameras.html").read_text())
        self.assertEqual(len(rows), 17)  # the placeholder row with no real item is skipped
        row = next(r for r in rows if r["item_id"] == cam(98))
        self.assertEqual((row["price"], row["currency"], row["bid_count"], row["postage"]), (150.0, "AUD", 4, 12.0))
        self.assertIsNone(next(r for r in rows if r["item_id"] == cam(97))["bid_count"])

    def test_empty_or_changed_page_fails_loudly(self):
        with self.assertRaises(pages.PageError):
            pages.parse_search("<html><body><p>Pardon our interruption</p></body></html>")
        with self.assertRaises(pages.PageError):
            pages.parse_listing("<html><body></body></html>")


class Selection(unittest.TestCase):
    def test_every_rule_rejects_its_case(self):
        sel = selection()
        self.assertEqual(sel["rejected"], {
            "ends_too_late": 1, "ends_too_soon": 1, "no_bids": 1, "not_an_auction": 1,
            "other_currency": 1, "price_out_of_range": 2, "price_range": 1})
        self.assertEqual(sel["seen"], 41)  # the auction seen on two pages counts once
        self.assertEqual(sum(c["eligible"] for c in sel["cells"]), 33)

    def test_window_edges(self):
        row = {"price": 120.0, "currency": "AUD", "bid_count": 2,
               "captured_at": dt.datetime(2026, 11, 20, 1, tzinfo=UTC)}
        at = row["captured_at"]
        verdict = lambda text: select_auctions.why_not(dict(row, time_left=pages.parse_time_left(text)), at, RULES)
        self.assertIsNone(verdict("2d 0h left"))
        self.assertIsNone(verdict("6d 23h left"))
        self.assertEqual(verdict("1d 23h left"), "ends_too_soon")
        self.assertEqual(verdict("7d 0h left"), "ends_too_late")

    def test_page_saved_earlier_is_judged_at_selection_time(self):
        # Seen 3 hours before selection with "2d 1h" left: by selection it may have under 2 days.
        row = {"price": 120.0, "currency": "AUD", "bid_count": 2, "time_left": pages.parse_time_left("2d 1h left"),
               "captured_at": dt.datetime(2026, 11, 19, 22, tzinfo=UTC)}
        self.assertEqual(select_auctions.why_not(row, dt.datetime(2026, 11, 20, 1, tzinfo=UTC), RULES), "ends_too_soon")

    def test_later_sighting_wins(self):
        sel = selection(n=33)
        twice = next(a for a in sel["auctions"] if a["item_id"] == cam(98))
        self.assertEqual((twice["current_bid"], twice["bid_count"]), (150.0, 4))

    def test_draw_is_repeatable_and_seeded(self):
        ids = lambda s: [a["item_id"] for a in s["auctions"]]
        self.assertEqual(ids(selection()), ids(selection()))
        self.assertNotEqual(ids(selection()), ids(selection(seed="another")))

    def test_draw_follows_the_published_ranking(self):
        sel = selection(n=16)
        for cell in sel["cells"]:
            self.assertEqual(cell["drawn"], 1)
        got = next(a["item_id"] for a in sel["auctions"]
                   if a["category"] == "lego" and a["price_band"] == [30, 100])
        candidates = [str(4000000000 + n) for n in (0, 1)]
        self.assertEqual(got, min(candidates, key=lambda i: select_auctions.rank(SEED, i)))

    def test_spread_and_shortfall(self):
        sel = selection(n=8)
        self.assertEqual(max(c["drawn"] for c in sel["cells"]), 1)
        self.assertEqual((selection(n=33)["short_by"], selection(n=40)["short_by"], selection(n=40)["drawn"]), (0, 7, 33))

    def test_earlier_batches_are_not_reused(self):
        first = selection()
        used = [a["item_id"] for a in first["auctions"]]
        second = selection(exclude_ids=used)
        self.assertFalse(set(used) & {a["item_id"] for a in second["auctions"]})
        self.assertEqual(second["rejected"]["used_in_earlier_batch"], 8)

    def test_band_edges(self):
        self.assertEqual([select_auctions.band_of(p, RULES) for p in (30, 99.99, 100, 800, 2000)], [0, 0, 1, 3, 3])


class Packs(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.sel = selection()
        cls.manifest = build_packs.build_all(cls.sel, PAGES, IDENTIFIED, cls.tmp.name, RULES)
        cls.packs = score.load_packs(cls.tmp.name)

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def test_target_cross_posts_and_late_sales_removed(self):
        self.assertEqual(len(self.packs), 8)
        for pack in self.packs.values():
            titles = {s["title"] for s in pack["sold"]}
            self.assertFalse(titles & {"the auction itself", "same seller", "same photo", "sold after selection", "US sale"})
            self.assertTrue(all(s["sold_date"] <= "2026-11-20" for s in pack["sold"]))
            for reason in ("target_listing", "same_seller", "same_photo", "price_not_usable"):
                self.assertEqual(pack["removed"][reason], 1)

    def test_selection_day_sale_kept_only_if_page_predates_selection(self):
        first = self.sel["auctions"][0]["item_id"]
        for item_id, pack in self.packs.items():
            has_day = any(s["title"] == "sold on the selection day" for s in pack["sold"])
            self.assertEqual(has_day, item_id == first)
            self.assertEqual(len(pack["sold"]), 14 if item_id == first else 13)

    def test_no_seller_or_link_in_any_pack(self):
        for path in Path(self.tmp.name).glob("*.json"):
            text = path.read_text().lower()
            for word in ("seller_x", "seller_cameras_", "seller_lego_", "seller_tools_", "target_seller_",
                         "other_", "http", "ebayimg", "img" + path.stem):
                self.assertNotIn(word, text)

    def test_manifest_matches_files(self):
        lines = (Path(self.tmp.name) / "MANIFEST.sha256").read_text().splitlines()
        self.assertEqual(len(lines), 8)
        for line in lines:
            digest, name = line.split("  ")
            self.assertEqual(digest, pages.sha256_bytes((Path(self.tmp.name) / name).read_bytes()))

    def test_missing_inputs_stop_the_build(self):
        with tempfile.TemporaryDirectory() as out:
            with self.assertRaises(SystemExit):
                build_packs.build_all(self.sel, PAGES, {}, out, RULES)
            with self.assertRaises(SystemExit):  # a 16-auction draw has auctions with no saved listing page
                build_packs.build_all(selection(n=16), PAGES, {a["item_id"]: "x" for a in selection(n=16)["auctions"]}, out, RULES)


def forecasts(sel, setups, central=lambda item, setup, member, rnd: 100.0):
    out = []
    for a in sel["auctions"]:
        for s in setups:
            for m in range(1, RULES["setups"][s]["members"] + 1):
                for r in range(1, RULES["setups"][s]["rounds"] + 1):
                    c = central(a, s, m, r)
                    out.append({"item_id": a["item_id"], "setup": s, "member": m, "round": r,
                                "central": c, "low": c * 0.7, "high": c * 1.4, "model": "test-model",
                                "tokens_in": 1000, "tokens_out": 200, "tool_calls": 2,
                                "wall_seconds": 30.0, "usd_equiv": 0.02})
    return out


PILOT = RULES["pilot_setups"]
BEFORE = dt.datetime(2026, 11, 21, 0, tzinfo=UTC)


class Locking(unittest.TestCase):
    def setUp(self):
        self.sel = selection()
        self.records = forecasts(self.sel, PILOT)

    def test_lock_then_verify(self):
        locked = lock_forecasts.lock(self.sel, self.records, RULES, PILOT, BEFORE)
        self.assertEqual(locked["forecasts"], 8 * 7)
        self.assertEqual(locked["deadline"], "2026-11-22T00:00:00Z")
        self.assertEqual(lock_forecasts.verify(self.records, locked), [])
        self.assertEqual(lock_forecasts.verify(list(reversed(self.records)), locked), [])  # order is not content

    def test_any_change_is_caught(self):
        locked = lock_forecasts.lock(self.sel, self.records, RULES, PILOT, BEFORE)
        self.records[3]["central"] += 0.01
        self.assertEqual(len(lock_forecasts.verify(self.records, locked)), 2)
        self.assertTrue(lock_forecasts.verify(self.records[:-1], locked))

    def test_refuses_after_the_deadline(self):
        ok = dt.datetime(2026, 11, 22, 0, 0, tzinfo=UTC)
        late = dt.datetime(2026, 11, 22, 0, 0, 1, tzinfo=UTC)
        lock_forecasts.lock(self.sel, self.records, RULES, PILOT, ok)
        with self.assertRaises(SystemExit) as caught:
            lock_forecasts.lock(self.sel, self.records, RULES, PILOT, late)
        self.assertIn("past the deadline", str(caught.exception))

    def test_refuses_incomplete_duplicate_or_malformed(self):
        problems = lambda recs: lock_forecasts.check(recs, self.sel, RULES, PILOT)
        self.assertEqual(problems(self.records), [])
        self.assertTrue(any("missing forecast" in p for p in problems(self.records[1:])))
        self.assertTrue(any("duplicate" in p for p in problems(self.records + [self.records[0]])))
        self.assertTrue(any("low <= central" in p for p in problems([dict(self.records[0], low=500.0)] + self.records[1:])))
        self.assertTrue(any("not a number" in p for p in problems([dict(self.records[0], central="100")] + self.records[1:])))
        self.assertTrue(any("not expected" in p for p in problems(self.records + [dict(self.records[0], setup="mixed_crowd")])))
        with self.assertRaises(SystemExit):
            lock_forecasts.lock(self.sel, self.records[1:], RULES, PILOT, BEFORE)


class Statistics(unittest.TestCase):
    def test_error_is_symmetric(self):
        self.assertAlmostEqual(stats.log_error(200, 100), 100 * math.log(2))
        self.assertAlmostEqual(stats.log_error(50, 100), stats.log_error(200, 100))
        self.assertEqual(stats.log_error(100, 100), 0)

    def test_t_distribution_against_published_tables(self):
        self.assertAlmostEqual(2 * (1 - stats.t_cdf(2.0, 10)), 0.07339, places=4)
        self.assertAlmostEqual(stats.t_cdf(2.228, 10), 0.975, places=4)
        self.assertAlmostEqual(stats.t_cdf(-1.96, 1e6), 0.025, places=4)
        self.assertAlmostEqual(stats.t_cdf(12.706, 1), 0.975, places=4)
        self.assertAlmostEqual(stats.t_quantile(0.975, 39), 2.0227, places=3)

    def test_paired_and_equivalence(self):
        r = stats.paired([1, 2, 3, 4, 5], 2)
        self.assertAlmostEqual(r["t"], 3 / (math.sqrt(2.5) / math.sqrt(5)))
        self.assertAlmostEqual(r["difference_p"], 0.01324, places=4)   # t = 4.2426, 4 df
        self.assertGreater(r["equivalent_p"], 0.5)
        tight = stats.paired([0.1, -0.1, 0.2, -0.2, 0.05, -0.05] * 5, 2)
        self.assertLess(tight["equivalent_p"], 0.001)
        self.assertGreater(tight["difference_p"], 0.9)
        wide = stats.paired([30, -30, 25, -25, 10, -10], 2)
        self.assertGreater(wide["equivalent_p"], 0.05)
        self.assertGreater(wide["difference_p"], 0.05)

    def test_sample_size(self):
        self.assertEqual(stats.auctions_needed(20, 3), 349)   # ((1.96 + 0.8416) * 20 / 3)^2 = 348.9
        self.assertEqual(stats.auctions_needed(30, 3), 785)


class Scoring(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.sel = selection()
        build_packs.build_all(cls.sel, PAGES, IDENTIFIED, cls.tmp.name, RULES)
        cls.packs = score.load_packs(cls.tmp.name)

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def run_score(self, central, outcomes, uplift=None, setups=PILOT):
        records = forecasts(self.sel, setups, central)
        locked = lock_forecasts.lock(self.sel, records, RULES, setups, BEFORE)
        return score.score(self.sel, self.packs, records, locked, outcomes, uplift, RULES)

    def test_crowd_takes_medians_and_last_round(self):
        def central(a, s, m, r):
            return [10, 20, 30, 400, 5000][m - 1] * (1 if r == 1 else 2)
        records = forecasts(self.sel, ["crowd_5", "discussing_crowd"], central)
        got = score.combine(records, RULES)
        item = self.sel["auctions"][0]["item_id"]
        self.assertEqual(got[(item, "crowd_5")]["central"], 30)
        self.assertAlmostEqual(got[(item, "crowd_5")]["low"], 21)
        self.assertEqual(got[(item, "discussing_crowd")]["central"], 60)     # round 2 only
        self.assertEqual(got[(item, "discussing_crowd")]["agent_forecasts"], 10)  # both rounds are paid for
        self.assertAlmostEqual(got[(item, "discussing_crowd")]["usd_equiv"], 0.2)

    def test_full_run_from_pages_to_results(self):
        items = [a["item_id"] for a in self.sel["auctions"]]
        outcomes = {i: {"status": "sold", "final_price": 200.0} for i in items[:5]}
        outcomes[items[5]] = {"status": "unsold"}
        outcomes[items[6]] = {"status": "cancelled"}   # items[7] has no outcome recorded
        uplift = {c: {"uplift": 1.5} for c in RULES["categories"]}

        def central(a, s, m, r):   # one_agent is 2x high, the other two are exact
            return 400.0 if s == "one_agent" else 200.0
        results, rows = self.run_score(central, outcomes, uplift)

        self.assertEqual(results["outcomes"], {"cancelled": 1, "no_outcome_recorded": 1, "sold": 5, "unsold": 1})
        self.assertEqual(results["scored"], 5)
        s = results["setups"]
        self.assertAlmostEqual(s["one_agent"]["mean_error_points"], 100 * math.log(2), places=2)
        self.assertEqual(s["crowd_5"]["mean_error_points"], 0)
        self.assertEqual(s["one_agent"]["range_hit_rate"], 0.0)     # 280 to 560 misses 200
        self.assertEqual(s["crowd_5"]["range_hit_rate"], 1.0)
        self.assertAlmostEqual(s["crowd_5"]["usd_equiv_per_auction"], 0.10)
        self.assertAlmostEqual(s["crowd_5"]["usd_equiv_per_agent_forecast"], 0.02)
        self.assertEqual(s["one_agent"]["bought"], 5)               # 200 < 0.7 * 400
        self.assertEqual(s["crowd_5"]["bought"], 0)
        self.assertEqual(s["simple_rule"]["auctions"], 5)
        self.assertEqual(s["bid_rule"]["auctions"], 5)
        self.assertEqual(len(rows), 5 * 5)

        first = self.packs[items[0]]
        row = next(r for r in rows if r["item_id"] == items[0] and r["setup"] == "bid_rule")
        self.assertAlmostEqual(row["central"], round(first["listing"]["current_bid"] * 1.5, 2))
        row = next(r for r in rows if r["item_id"] == items[0] and r["setup"] == "one_agent")
        self.assertAlmostEqual(row["profit"], round(score.simple_rule(first, RULES) - 200.0, 2))

    def test_simple_rule_uses_the_ten_most_recent(self):
        pack = {"sold": [{"price": float(p)} for p in range(1, 15)]}
        self.assertEqual(score.simple_rule(pack, RULES), 5.5)
        self.assertIsNone(score.simple_rule({"sold": []}, RULES))

    def test_main_comparison_readings(self):
        items = [a["item_id"] for a in self.sel["auctions"]]
        outcomes = {i: {"status": "sold", "final_price": 100.0} for i in items}
        wobble = {i: 1 + 0.002 * (k % 3 - 1) for k, i in enumerate(items)}

        # Each setup is off by 0.2 points on alternate auctions, so the mean difference is zero.
        turn = {"crowd_5": 0, "one_agent_5x": 1}
        same = lambda a, s, m, r: 100.0 * (1.002 if items.index(a["item_id"]) % 2 == turn.get(s) else 1.0)
        main = self.run_score(same, outcomes)[0]["main_comparison"]
        self.assertEqual(main["reading"], "the same within 2 points")

        better = lambda a, s, m, r: 100.0 * wobble[a["item_id"]] * (1.0 if s == "crowd_5" else 1.3)
        main = self.run_score(better, outcomes)[0]["main_comparison"]
        self.assertEqual(main["reading"], "crowd_5 was closer")
        self.assertLess(main["mean"], 0)

        noisy = lambda a, s, m, r: 100.0 * (1 + 0.3 * (items.index(a["item_id"]) % 2) if s == "crowd_5"
                                            else 1 + 0.3 * (items.index(a["item_id"]) // 4))
        main = self.run_score(noisy, outcomes)[0]["main_comparison"]
        self.assertEqual(main["reading"], "inconclusive at this sample size")
        self.assertGreater(main["auctions_needed_for_gap"], 8)

    def test_scoring_refuses_changed_forecasts_and_unknown_status(self):
        records = forecasts(self.sel, PILOT)
        locked = lock_forecasts.lock(self.sel, records, RULES, PILOT, BEFORE)
        records[0]["central"] = 101.0
        with self.assertRaises(SystemExit):
            score.score(self.sel, self.packs, records, locked, {}, None, RULES)
        with self.assertRaises(SystemExit):
            self.run_score(lambda *a: 100.0, {self.sel["auctions"][0]["item_id"]: {"status": "won"}})

    def test_learn_uplift(self):
        items = [a["item_id"] for a in self.sel["auctions"]]
        outcomes = {i: {"status": "sold", "final_price": self.packs[i]["listing"]["current_bid"] * 2} for i in items}
        outcomes[items[0]] = {"status": "unsold"}
        got = score.learn_uplift(self.packs, outcomes)
        self.assertTrue(all(abs(v["uplift"] - 2.0) < 1e-9 for v in got.values()))
        self.assertEqual(sum(v["auctions"] for v in got.values()), 7)


class CommandLine(unittest.TestCase):
    def test_scripts_run_end_to_end(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            self.assertEqual(select_auctions.main(["--pages", str(PAGES), "--batch", "cli", "--n", "8", "--seed", SEED,
                                                   "--out", str(tmp / "selection.json")]), 0)
            sel = json.loads((tmp / "selection.json").read_text())
            build_packs.main(["--selection", str(tmp / "selection.json"), "--pages", str(PAGES),
                              "--identified", str(HERE / "fixtures" / "identified.json"), "--out", str(tmp / "packs")])
            (tmp / "forecasts.jsonl").write_text("".join(json.dumps(r) + "\n" for r in forecasts(sel, PILOT)))
            # The command-line lock reads the real clock, so it is not called here; the deadline
            # refusal is covered by Locking.test_refuses_after_the_deadline.
            locked = lock_forecasts.lock(sel, lock_forecasts.read_forecasts(tmp / "forecasts.jsonl"), RULES, PILOT, BEFORE)
            (tmp / "LOCK.json").write_text(json.dumps(locked))
            self.assertEqual(lock_forecasts.main(["verify", "--forecasts", str(tmp / "forecasts.jsonl"),
                                                  "--lock", str(tmp / "LOCK.json")]), 0)
            outcomes = {a["item_id"]: {"status": "sold", "final_price": 130.0} for a in sel["auctions"]}
            (tmp / "outcomes.json").write_text(json.dumps(outcomes))
            score.main(["learn-uplift", "--packs", str(tmp / "packs"), "--outcomes", str(tmp / "outcomes.json"),
                        "--out", str(tmp / "uplift.json")])
            score.main(["score", "--selection", str(tmp / "selection.json"), "--packs", str(tmp / "packs"),
                        "--forecasts", str(tmp / "forecasts.jsonl"), "--lock", str(tmp / "LOCK.json"),
                        "--outcomes", str(tmp / "outcomes.json"), "--uplift", str(tmp / "uplift.json"),
                        "--out", str(tmp / "results")])
            results = json.loads((tmp / "results" / "results.json").read_text())
            self.assertEqual(results["scored"], 8)
            self.assertEqual(len((tmp / "results" / "per_auction.csv").read_text().splitlines()), 1 + 8 * 5)


if __name__ == "__main__":
    unittest.main()
