#!/usr/bin/env python3
"""Hash and commit a batch's forecasts before its earliest auction ends.

    python3 lock_forecasts.py lock --selection selection.json --forecasts forecasts.jsonl \
        --out LOCK.json [--setups one_agent,one_agent_5x,crowd_5] [--commit]
    python3 lock_forecasts.py verify --forecasts forecasts.jsonl --lock LOCK.json

`forecasts.jsonl` has one forecast per line:

    {"item_id": "...", "setup": "crowd_5", "member": 1, "round": 1,
     "central": 120.0, "low": 90.0, "high": 160.0,
     "model": "...", "tokens_in": 0, "tokens_out": 0, "tool_calls": 0,
     "wall_seconds": 0.0, "usd_equiv": 0.0}

`lock` refuses when a forecast is missing, duplicated or malformed, and when it is run later than
the earliest possible auction end less the margin in rules.json. With --commit it commits the
forecasts and the lock file to git in one commit.

A git commit carries the clock of the machine that made it. It shows order within the repository,
not proof to an outsider. Proof to an outsider needs the root hash posted somewhere public before
the auctions end; that is the owner's step and is not done here.
"""
import argparse
import datetime as dt
import json
import subprocess
import sys
from pathlib import Path

import pages
from select_auctions import load_rules

NUMBERS = ("central", "low", "high")
COSTS = ("tokens_in", "tokens_out", "tool_calls", "wall_seconds", "usd_equiv")


def read_forecasts(path):
    return [json.loads(line) for line in Path(path).read_text().splitlines() if line.strip()]


def key(record):
    return (record["item_id"], record["setup"], record["member"], record["round"])


def check(records, selection, rules, setups):
    """Every problem that stops these forecasts being locked."""
    problems, seen = [], set()
    items = {a["item_id"] for a in selection["auctions"]}
    expected = {(i, s, m, r) for i in items for s in setups
                for m in range(1, rules["setups"][s]["members"] + 1)
                for r in range(1, rules["setups"][s]["rounds"] + 1)}
    for n, record in enumerate(records, 1):
        missing = [f for f in ("item_id", "setup", "member", "round", "model") + NUMBERS + COSTS if f not in record]
        if missing:
            problems.append(f"line {n}: missing {', '.join(missing)}")
            continue
        k = key(record)
        if k in seen:
            problems.append(f"line {n}: duplicate forecast {k}")
        seen.add(k)
        if k not in expected:
            problems.append(f"line {n}: not expected for this batch {k}")
        values = [record[f] for f in NUMBERS + COSTS]
        if any(isinstance(v, bool) or not isinstance(v, (int, float)) or v != v for v in values):
            problems.append(f"line {n}: a number field is not a number")
        elif not 0 < record["low"] <= record["central"] <= record["high"] < float("inf"):
            problems.append(f"line {n}: need 0 < low <= central <= high")
    for k in sorted(expected - seen):
        problems.append(f"missing forecast {k}")
    return problems


def hashes(records):
    lines = sorted(f"{pages.sha256_bytes(pages.canonical(r))}  {'/'.join(map(str, key(r)))}" for r in records)
    return lines, pages.sha256_bytes(("\n".join(lines) + "\n").encode())


def lock(selection, records, rules, setups, now):
    problems = check(records, selection, rules, setups)
    if problems:
        raise SystemExit("not locked:\n  " + "\n  ".join(problems[:40])
                         + (f"\n  and {len(problems) - 40} more" if len(problems) > 40 else ""))
    earliest = pages.parse_time(selection["earliest_end"])
    deadline = earliest - dt.timedelta(minutes=rules["lock_margin_minutes"])
    if now > deadline:
        raise SystemExit(f"not locked: it is {pages.iso(now)}, past the deadline {pages.iso(deadline)} "
                         f"(earliest auction end {pages.iso(earliest)}). This batch cannot be used.")
    lines, root = hashes(records)
    return {
        "batch": selection["batch"],
        "selection_sha256": pages.sha256_bytes(pages.canonical(selection)),
        "rules_sha256": rules["_sha256"],
        "setups": setups,
        "locked_at": pages.iso(now),
        "deadline": pages.iso(deadline),
        "earliest_end": pages.iso(earliest),
        "forecasts": len(records),
        "root_sha256": root,
        "hashes": lines,
    }


def verify(records, locked):
    """Differences between the forecasts on disk and the lock. Empty means untouched."""
    lines, root = hashes(records)
    old, new = set(locked["hashes"]), set(lines)
    problems = [f"changed or added: {line.split('  ')[1]}" for line in sorted(new - old)]
    problems += [f"changed or removed: {line.split('  ')[1]}" for line in sorted(old - new)]
    if not problems and root != locked["root_sha256"]:
        problems.append("root hash does not match its own list")
    return problems


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("lock")
    p.add_argument("--selection", required=True)
    p.add_argument("--forecasts", required=True)
    p.add_argument("--out", required=True)
    p.add_argument("--setups", help="comma list; default is every setup in rules.json")
    p.add_argument("--rules")
    p.add_argument("--commit", action="store_true", help="git add and commit the forecasts and the lock")
    v = sub.add_parser("verify")
    v.add_argument("--forecasts", required=True)
    v.add_argument("--lock", required=True)
    args = ap.parse_args(argv)

    records = read_forecasts(args.forecasts)
    if args.cmd == "verify":
        problems = verify(records, json.loads(Path(args.lock).read_text()))
        print("\n".join(problems) if problems else f"{len(records)} forecasts match the lock")
        return 1 if problems else 0

    rules = load_rules(args.rules)
    setups = args.setups.split(",") if args.setups else list(rules["setups"])
    unknown = [s for s in setups if s not in rules["setups"]]
    if unknown:
        raise SystemExit(f"unknown setups: {unknown}")
    selection = json.loads(Path(args.selection).read_text())
    locked = lock(selection, records, rules, setups, dt.datetime.now(dt.timezone.utc))
    Path(args.out).write_text(json.dumps(locked, indent=2) + "\n")
    print(f"locked {locked['forecasts']} forecasts at {locked['locked_at']}, root {locked['root_sha256']}")
    if args.commit:
        subprocess.run(["git", "add", "--", args.forecasts, args.out], check=True)
        subprocess.run(["git", "commit", "-m", f"Lock forecasts for {locked['batch']}: root {locked['root_sha256']}",
                        "--", args.forecasts, args.out], check=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
