#!/usr/bin/env python3
"""Freeze one evidence pack per selected auction. Forecasters see the pack and nothing else.

    python3 build_packs.py --selection selection.json --pages PAGES_DIR \
        --identified identified.json --out packs/

Needs, per auction, one saved listing page and at least one saved sold-results page for the
identified item (meta "item_id" = the auction they belong to). `identified.json` maps each item id
to the name of the item the sold search was run for.

A pack holds the listing snapshot and the sold items dated up to the selection time. Removed
before freezing: the target listing itself, anything from the same seller or with the same lead
photo (same-item cross-posts), anything sold after selection, and every seller name and link.
Never fetches anything.
"""
import argparse
import json
import sys
from collections import Counter
from pathlib import Path
from zoneinfo import ZoneInfo

import pages
from select_auctions import load_rules


def sold_cutoff(selected_at, captured_at, zone):
    """Last sold date allowed, and whether that day itself is allowed.

    Sold dates carry no time of day. A page saved at or before selection cannot show a later sale,
    so the selection day counts. A page saved after selection could, so that day is dropped.
    """
    day = selected_at.astimezone(zone).date()
    return day, captured_at <= selected_at


def filter_sold(rows, target, cutoff_day, same_day_ok, rules):
    kept, removed = [], Counter()
    for row in rows:
        if row["item_id"] == target["item_id"]:
            reason = "target_listing"
        elif row["seller"] and target.get("seller") and row["seller"].lower() == target["seller"].lower():
            reason = "same_seller"
        elif row["image_key"] and row["image_key"] == target.get("image_key"):
            reason = "same_photo"
        elif row["price"] is None or row["currency"] != rules["currency"]:
            reason = "price_not_usable"
        elif row["sold_date"] is None:
            reason = "no_sold_date"
        elif row["sold_date"] > cutoff_day or (row["sold_date"] == cutoff_day and not same_day_ok):
            reason = "sold_after_selection"
        else:
            reason = None
        if reason:
            removed[reason] += 1
        else:
            kept.append(row)
    return kept, removed


def build_pack(auction, listing_pages, sold_pages, identified, selected_at, rules):
    zone = ZoneInfo(rules["timezone"])
    if len(listing_pages) != 1:
        raise SystemExit(f"{auction['item_id']}: need exactly one listing page, found {len(listing_pages)}")
    meta, html, digest = listing_pages[0]
    listing = pages.parse_listing(html)
    target = {"item_id": auction["item_id"], "seller": listing["seller"], "image_key": listing["image_key"]}

    if not sold_pages:
        raise SystemExit(f"{auction['item_id']}: no sold-results page")
    sold, removed, sources, seen = [], Counter(), [{"file": meta["file"], "sha256": digest}], set()
    for s_meta, s_html, s_digest in sold_pages:
        sources.append({"file": s_meta["file"], "sha256": s_digest})
        day, same_day_ok = sold_cutoff(selected_at, pages.parse_time(s_meta["captured_at"]), zone)
        rows = [r for r in pages.parse_sold(s_html) if r["item_id"] not in seen]
        seen.update(r["item_id"] for r in rows)
        kept, gone = filter_sold(rows, target, day, same_day_ok, rules)
        sold += kept
        removed += gone
    sold.sort(key=lambda r: (r["sold_date"], r["item_id"]), reverse=True)

    return {
        "item_id": auction["item_id"],
        "selected_at": pages.iso(selected_at),
        "currency": rules["currency"],
        "listing": {
            "title": listing["title"],
            "category": auction["category"],
            "condition": listing["condition"],
            "specifics": listing["specifics"],
            "current_bid": listing["price"] if listing["price"] is not None else auction["current_bid"],
            "bid_count": listing["bid_count"] if listing["bid_count"] is not None else auction["bid_count"],
            "time_left": listing["time_left_text"] or auction["time_left_text"],
            "seen_at": meta["captured_at"],
            "buy_it_now_offered": auction["buy_it_now"],
        },
        "identified_item": identified,
        "sold": [{"title": r["title"], "price": r["price"], "postage": r["postage"],
                  "condition": r["condition"], "sold_date": r["sold_date"].isoformat()} for r in sold],
        "removed": dict(sorted(removed.items())),
        "sources": sources,
    }


def build_all(selection, pages_dir, identified, out_dir, rules):
    if selection["rules_sha256"] != rules["_sha256"]:
        raise SystemExit("rules.json changed since the selection was drawn")
    selected_at = pages.parse_time(selection["selected_at"])
    by_item = {}
    for kind in ("listing", "sold"):
        for page in pages.load_pages(pages_dir, kind):
            by_item.setdefault((kind, page[0].get("item_id")), []).append(page)

    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    manifest = []
    for auction in selection["auctions"]:
        item_id = auction["item_id"]
        if not identified.get(item_id):
            raise SystemExit(f"{item_id}: no identified item")
        pack = build_pack(auction, by_item.get(("listing", item_id), []), by_item.get(("sold", item_id), []),
                          identified[item_id], selected_at, rules)
        data = json.dumps(pack, indent=2, sort_keys=True, ensure_ascii=False).encode() + b"\n"
        (out_dir / f"{item_id}.json").write_bytes(data)
        manifest.append((pages.sha256_bytes(data), f"{item_id}.json", len(pack["sold"])))
    (out_dir / "MANIFEST.sha256").write_text("".join(f"{h}  {name}\n" for h, name, _ in manifest))
    return manifest


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--selection", required=True)
    ap.add_argument("--pages", required=True)
    ap.add_argument("--identified", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--rules")
    args = ap.parse_args(argv)
    manifest = build_all(json.loads(Path(args.selection).read_text()), args.pages,
                         json.loads(Path(args.identified).read_text()), args.out, load_rules(args.rules))
    thin = [name for _, name, count in manifest if count < 3]
    print(f"{len(manifest)} packs written to {args.out}; {len(thin)} have fewer than 3 sold items")
    return 0


if __name__ == "__main__":
    sys.exit(main())
