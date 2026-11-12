#!/usr/bin/env python3
"""Score locked forecasts against final prices.

    python3 score.py score --selection selection.json --packs packs/ --forecasts forecasts.jsonl \
        --lock LOCK.json --outcomes outcomes.json [--uplift uplift.json] --out results/
    python3 score.py learn-uplift --packs packs/ --outcomes outcomes.json --out uplift.json

`outcomes.json` maps each item id to what happened:

    {"1234": {"status": "sold", "final_price": 152.5},
     "5678": {"status": "unsold"}}

Status is "sold" or one of the exclusions named in the plan (rules.json, exclusion_statuses).
Scoring refuses to run if the forecasts no longer match the lock. Nothing here uses the network.
"""
import argparse
import csv
import json
import statistics
import sys
from collections import Counter, defaultdict
from pathlib import Path

import lock_forecasts
import stats
from select_auctions import load_rules

BASELINES = ("simple_rule", "bid_rule")


def load_packs(packs_dir):
    return {p.stem: json.loads(p.read_text()) for p in sorted(Path(packs_dir).glob("*.json"))}


def simple_rule(pack, rules):
    """Median of the most recent sold prices in the pack (the pack lists newest first)."""
    prices = [s["price"] for s in pack["sold"][:rules["simple_rule_last_n"]]]
    return statistics.median(prices) if prices else None


def learn_uplift(packs, outcomes):
    """Per category, the median of final price / bid at selection, over auctions that sold."""
    ratios = defaultdict(list)
    for item_id, pack in packs.items():
        outcome = outcomes.get(item_id, {})
        if outcome.get("status") == "sold":
            ratios[pack["listing"]["category"]].append(outcome["final_price"] / pack["listing"]["current_bid"])
    return {c: {"uplift": statistics.median(r), "auctions": len(r)} for c, r in sorted(ratios.items())}


def combine(records, rules):
    """One forecast and one cost per (auction, setup). Crowds take medians of the last round."""
    groups = defaultdict(list)
    for r in records:
        groups[(r["item_id"], r["setup"])].append(r)
    out = {}
    for (item_id, setup), group in groups.items():
        final = [r for r in group if r["round"] == rules["setups"][setup]["rounds"]]
        out[(item_id, setup)] = {
            "central": statistics.median(r["central"] for r in final),
            "low": statistics.median(r["low"] for r in final),
            "high": statistics.median(r["high"] for r in final),
            "usd_equiv": sum(r["usd_equiv"] for r in group),
            "tokens": sum(r["tokens_in"] + r["tokens_out"] for r in group),
            "tool_calls": sum(r["tool_calls"] for r in group),
            "wall_seconds": sum(r["wall_seconds"] for r in group),
            "agent_forecasts": len(group),
        }
    return out


def score(selection, packs, records, locked, outcomes, uplift, rules, seed=0):
    problems = lock_forecasts.verify(records, locked)
    if problems:
        raise SystemExit("forecasts do not match the lock:\n  " + "\n  ".join(problems[:20]))
    if locked["selection_sha256"] != lock_forecasts.pages.sha256_bytes(lock_forecasts.pages.canonical(selection)):
        raise SystemExit("selection.json is not the one the forecasts were locked against")

    statuses = Counter()
    scored = []
    for auction in selection["auctions"]:
        outcome = outcomes.get(auction["item_id"])
        status = outcome["status"] if outcome else "no_outcome_recorded"
        if status != "sold" and status not in rules["exclusion_statuses"] and outcome:
            raise SystemExit(f"{auction['item_id']}: unknown status {status!r}")
        statuses[status] += 1
        if status == "sold":
            scored.append((auction, outcome["final_price"]))

    combined = combine(records, rules)
    setups = list(BASELINES) + locked["setups"]
    rows, errors = [], defaultdict(dict)
    for auction, final in scored:
        item_id, pack = auction["item_id"], packs[auction["item_id"]]
        value = simple_rule(pack, rules)
        factor = (uplift or {}).get(auction["category"], {}).get("uplift")
        forecasts = {
            "simple_rule": {"central": value} if value else None,
            "bid_rule": {"central": pack["listing"]["current_bid"] * factor} if factor else None,
        }
        forecasts.update({s: combined[(item_id, s)] for s in locked["setups"]})
        for setup in setups:
            f = forecasts[setup]
            if f is None:
                continue
            error = stats.log_error(f["central"], final)
            errors[setup][item_id] = error
            bought = final < rules["buy_threshold"] * f["central"]
            rows.append({
                "item_id": item_id, "category": auction["category"], "setup": setup,
                "final_price": final, "central": round(f["central"], 2),
                "low": f.get("low", ""), "high": f.get("high", ""),
                "error_points": round(error, 3),
                "in_range": int(f["low"] <= final <= f["high"]) if "low" in f else "",
                "bought": int(bought),
                "profit": round(value - final, 2) if bought and value else "",
                "usd_equiv": f.get("usd_equiv", ""), "tokens": f.get("tokens", ""),
                "tool_calls": f.get("tool_calls", ""), "wall_seconds": f.get("wall_seconds", ""),
                "agent_forecasts": f.get("agent_forecasts", ""),
            })

    per_setup = {}
    for setup in setups:
        mine = [r for r in rows if r["setup"] == setup]
        if not mine:
            per_setup[setup] = {"auctions": 0}
            continue
        mean, ci = stats.bootstrap_mean([r["error_points"] for r in mine], seed)
        entry = {"auctions": len(mine), "mean_error_points": mean, "mean_error_ci95": ci,
                 "median_error_points": statistics.median(r["error_points"] for r in mine),
                 "bought": sum(r["bought"] for r in mine),
                 "profit_on_bought": round(sum(r["profit"] or 0 for r in mine), 2)}
        if setup not in BASELINES:
            n_agent = sum(r["agent_forecasts"] for r in mine)
            entry.update({
                "range_hit_rate": statistics.fmean(r["in_range"] for r in mine),
                "usd_equiv_per_auction": statistics.fmean(r["usd_equiv"] for r in mine),
                "usd_equiv_per_agent_forecast": sum(r["usd_equiv"] for r in mine) / n_agent,
                "tokens_per_auction": statistics.fmean(r["tokens"] for r in mine),
                "tool_calls_per_auction": statistics.fmean(r["tool_calls"] for r in mine),
                "wall_seconds_per_auction": statistics.fmean(r["wall_seconds"] for r in mine),
            })
        per_setup[setup] = entry

    a, b = rules["main_comparison"]
    main = None
    if a in errors and b in errors:
        diffs = [errors[a][i] - errors[b][i] for i in sorted(errors[a]) if i in errors[b]]
        main = stats.paired(diffs, rules["equivalence_margin_points"])
        main["difference"] = f"{a} error minus {b} error, in points; negative means {a} was closer"
        if main.get("sd"):
            main["auctions_needed_for_gap"] = stats.auctions_needed(main["sd"], rules["smallest_gap_points"])
            if main["difference_p"] < 0.05:
                main["reading"] = f"{a if main['mean'] < 0 else b} was closer"
            elif main["equivalent_p"] < 0.05:
                main["reading"] = f"the same within {rules['equivalence_margin_points']} points"
            else:
                main["reading"] = "inconclusive at this sample size"

    return {
        "batch": selection["batch"],
        "lock_root_sha256": locked["root_sha256"],
        "selected": len(selection["auctions"]),
        "outcomes": dict(sorted(statuses.items())),
        "scored": len(scored),
        "bootstrap_seed": seed,
        "setups": per_setup,
        "main_comparison": main,
    }, rows


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("score")
    for name in ("selection", "packs", "forecasts", "lock", "outcomes", "out"):
        s.add_argument(f"--{name}", required=True)
    s.add_argument("--uplift")
    s.add_argument("--rules")
    u = sub.add_parser("learn-uplift")
    for name in ("packs", "outcomes", "out"):
        u.add_argument(f"--{name}", required=True)
    args = ap.parse_args(argv)

    read = lambda path: json.loads(Path(path).read_text())
    if args.cmd == "learn-uplift":
        uplift = learn_uplift(load_packs(args.packs), read(args.outcomes))
        Path(args.out).write_text(json.dumps(uplift, indent=2) + "\n")
        print(json.dumps(uplift))
        return 0

    results, rows = score(read(args.selection), load_packs(args.packs),
                          lock_forecasts.read_forecasts(args.forecasts), read(args.lock), read(args.outcomes),
                          read(args.uplift) if args.uplift else None, load_rules(args.rules))
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    (out / "results.json").write_text(json.dumps(results, indent=2) + "\n")
    with open(out / "per_auction.csv", "w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0]) if rows else ["item_id"])
        writer.writeheader()
        writer.writerows(rows)
    print(f"scored {results['scored']} of {results['selected']} auctions; outcomes {results['outcomes']}")
    for setup, entry in results["setups"].items():
        if entry["auctions"]:
            print(f"  {setup:18} error {entry['mean_error_points']:.1f} points over {entry['auctions']} auctions")
    if results["main_comparison"]:
        print(f"  main comparison: {results['main_comparison'].get('reading', 'too few auctions')}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
