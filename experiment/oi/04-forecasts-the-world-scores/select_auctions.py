#!/usr/bin/env python3
"""Draw the auctions for one batch from saved search pages, by the plan's frozen rules.

    python3 select_auctions.py --pages PAGES_DIR --batch pilot-01 --n 40 --seed 20261120 \
        --out selection.json

Reads only saved pages (see pages.py). Never fetches anything.

The draw is reproducible by anyone without this code: within each category and price band, every
eligible auction is ranked by sha256("<seed>:<item id>") and the lowest ranks are taken.
"""
import argparse
import datetime as dt
import hashlib
import json
import sys
from collections import Counter
from pathlib import Path

import pages

HERE = Path(__file__).resolve().parent
DAY = 86400


def load_rules(path=None):
    raw = Path(path or HERE / "rules.json").read_bytes()
    rules = json.loads(raw)
    rules["_sha256"] = pages.sha256_bytes(raw)
    return rules


def band_of(price, rules):
    bands = rules["price_bands"]
    for index, (low, high) in enumerate(bands):
        if low <= price < high or (index == len(bands) - 1 and price == high):
            return index
    return None


def rank(seed, item_id):
    return hashlib.sha256(f"{seed}:{item_id}".encode()).hexdigest()


def gather(pages_dir, rules):
    """Every auction seen on the saved search pages, latest sighting of each."""
    seen, used = {}, []
    for meta, html, digest in pages.load_pages(pages_dir, "search"):
        if meta.get("category") not in rules["categories"]:
            raise SystemExit(f"{meta['file']}: category {meta.get('category')!r} is not in the plan")
        captured = pages.parse_time(meta["captured_at"])
        rows = pages.parse_search(html)
        used.append({"file": meta["file"], "sha256": digest, "captured_at": pages.iso(captured),
                     "category": meta["category"], "results": len(rows)})
        for row in rows:
            row = dict(row, category=meta["category"], captured_at=captured,
                       page=meta["file"], page_sha256=digest)
            old = seen.get(row["item_id"])
            if old is None or old["captured_at"] < captured:
                seen[row["item_id"]] = row
    if not used:
        raise SystemExit(f"no saved search pages in {pages_dir}")
    return seen, used


def why_not(row, selected_at, rules):
    """The first rule this auction fails, or None if it is eligible."""
    if row["price"] is None:
        return row["currency"]  # price_range or price_unreadable
    if row["currency"] != rules["currency"]:
        return "other_currency"
    if row["bid_count"] is None:
        return "not_an_auction"
    if row["bid_count"] < rules["min_bids"]:
        return "no_bids"
    if not rules["price_min"] <= row["price"] <= rules["price_max"]:
        return "price_out_of_range"
    if row["time_left"] is None:
        return "time_left_unreadable"
    low, high = row["time_left"]
    ends_after = row["captured_at"] + dt.timedelta(seconds=low)
    ends_before = row["captured_at"] + dt.timedelta(seconds=high)
    if ends_after < selected_at + dt.timedelta(days=rules["min_days_to_end"]):
        return "ends_too_soon"
    if ends_before > selected_at + dt.timedelta(days=rules["max_days_to_end"]):
        return "ends_too_late"
    return None


def select(pages_dir, batch, n, seed, rules, exclude_ids=()):
    seen, used = gather(pages_dir, rules)
    selected_at = max(pages.parse_time(p["captured_at"]) for p in used)
    exclude_ids = set(exclude_ids)

    rejected = Counter()
    cells = {(c, b): [] for c in rules["categories"] for b in range(len(rules["price_bands"]))}
    for item_id in sorted(seen):
        row = seen[item_id]
        reason = "used_in_earlier_batch" if item_id in exclude_ids else why_not(row, selected_at, rules)
        if reason:
            rejected[reason] += 1
        else:
            cells[(row["category"], band_of(row["price"], rules))].append(row)
    for rows in cells.values():
        rows.sort(key=lambda r: rank(seed, r["item_id"]))

    # Equal share per cell; the remainder goes to cells in a seeded order. A cell that runs short
    # hands its places on, one at a time, to the next cells in that same order.
    order = sorted(cells, key=lambda cell: rank(seed, f"cell:{cell[0]}:{cell[1]}"))
    take = {cell: 0 for cell in cells}
    places, progress = n, True
    while places and progress:
        progress = False
        for cell in order:
            if places and take[cell] < len(cells[cell]):
                take[cell] += 1
                places -= 1
                progress = True

    target = {cell: n // len(cells) + (1 if i < n % len(cells) else 0) for i, cell in enumerate(order)}
    auctions = []
    for cell in sorted(cells):
        for row in cells[cell][:take[cell]]:
            low, high = row["time_left"]
            auctions.append({
                "item_id": row["item_id"],
                "category": row["category"],
                "price_band": rules["price_bands"][cell[1]],
                "title": row["title"],
                "current_bid": row["price"],
                "bid_count": row["bid_count"],
                "buy_it_now": row["buy_it_now"],
                "time_left_text": row["time_left_text"],
                "seen_at": pages.iso(row["captured_at"]),
                "ends_no_earlier_than": pages.iso(row["captured_at"] + dt.timedelta(seconds=low)),
                "ends_before": pages.iso(row["captured_at"] + dt.timedelta(seconds=high)),
                "search_page": row["page"],
                "search_page_sha256": row["page_sha256"],
            })

    return {
        "batch": batch,
        "rules_sha256": rules["_sha256"],
        "seed": str(seed),
        "selected_at": pages.iso(selected_at),
        "requested": n,
        "drawn": len(auctions),
        "short_by": places,
        "earliest_end": min((a["ends_no_earlier_than"] for a in auctions), default=None),
        "cells": [{"category": c, "price_band": rules["price_bands"][b], "eligible": len(cells[(c, b)]),
                   "equal_share": target[(c, b)], "drawn": take[(c, b)]} for c, b in sorted(cells)],
        "seen": len(seen),
        "rejected": dict(sorted(rejected.items())),
        "search_pages": used,
        "auctions": auctions,
    }


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--pages", required=True, help="directory of saved search pages")
    ap.add_argument("--batch", required=True)
    ap.add_argument("--n", type=int, required=True, help="auctions to draw")
    ap.add_argument("--seed", required=True, help="written down before the pages are saved")
    ap.add_argument("--out", required=True)
    ap.add_argument("--rules")
    ap.add_argument("--exclude", action="append", default=[],
                    help="an earlier selection.json whose auctions must not be reused (repeatable)")
    args = ap.parse_args(argv)

    exclude = [a["item_id"] for path in args.exclude for a in json.loads(Path(path).read_text())["auctions"]]
    result = select(args.pages, args.batch, args.n, args.seed, load_rules(args.rules), exclude)
    Path(args.out).write_text(json.dumps(result, indent=2) + "\n")
    print(f"{result['drawn']} of {result['requested']} auctions drawn from {result['seen']} seen; "
          f"earliest end {result['earliest_end']}; rejected {result['rejected']}")
    if result["short_by"]:
        print(f"SHORT by {result['short_by']}: not enough eligible auctions on the saved pages", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
