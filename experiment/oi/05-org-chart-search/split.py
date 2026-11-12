#!/usr/bin/env python3
"""Split the 100 tasks into 60 for the search and 40 held out.

The split is made by this script and nobody chooses a task's side by hand.
Tasks that share material (the two questions on one research world) move
together. Each family is split 60/40 on its own.

Without difficulty labels the result is a DRAFT: the plan asks for the split
to be balanced by difficulty, and difficulty comes from the stage 1
calibration runs, which have not happened. After calibration, run

    python3 split.py --labels labels.json

where labels.json maps task id to true (hard) or false. The seed is fixed
below and must not be changed after any calibration result has been seen.

Usage: python3 split.py [--labels labels.json] [--out split.json]
"""
import argparse
import hashlib
import json
import pathlib

SUITE = pathlib.Path(__file__).resolve().parent
SEED = "oi-05-split-2026-11-12"
HELD_OUT = {"code": 12, "research": 12, "operations": 8, "forecast": 8}


def order_key(group):
    return hashlib.sha256(f"{SEED}|{group}".encode()).hexdigest()


def split(tasks, labels=None):
    sides = {}
    for family, quota in HELD_OUT.items():
        groups = {}
        for t in tasks:
            if t["family"] == family:
                groups.setdefault(t["group"], []).append(t["id"])
        sizes = {len(v) for v in groups.values()}
        assert len(sizes) == 1, f"{family}: groups of unequal size"
        size = sizes.pop()
        assert quota % size == 0, f"{family}: quota does not divide into groups"
        want = quota // size

        def hard_count(g):
            return sum(bool(labels[i]) for i in groups[g]) if labels else 0

        # Hardest groups first, shuffled by the seed within the same difficulty.
        # Then every n-th group is held out, so each difficulty level is held out in proportion.
        # Within a difficulty level, tasks of one sort (test runner, job kind, auction category) sit together,
        # so each sort is also held out in proportion.
        stratum = {t["group"]: t.get("stratum", "") for t in tasks}
        ordered = sorted(groups, key=lambda g: (-hard_count(g), stratum[g], order_key(g)))
        n = len(ordered)
        for i, g in enumerate(ordered):
            held = (i + 1) * want // n > i * want // n
            for tid in groups[g]:
                sides[tid] = "held_out" if held else "search"
    return sides


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--labels")
    ap.add_argument("--out", default=str(SUITE / "split.json"))
    args = ap.parse_args()
    suite_bytes = (SUITE / "suite.json").read_bytes()
    tasks = json.loads(suite_bytes)["tasks"]
    labels = None
    if args.labels:
        labels = json.loads(pathlib.Path(args.labels).read_text())
        missing = [t["id"] for t in tasks if t["id"] not in labels]
        if missing:
            raise SystemExit(f"labels missing for {len(missing)} tasks, first {missing[0]}")
    sides = split(tasks, labels)
    out = {
        "seed": SEED,
        "suite_sha256": hashlib.sha256(suite_bytes).hexdigest(),
        "difficulty_balanced": labels is not None,
        "status": "final" if labels is not None else "draft: no difficulty labels yet, do not use for stage 2",
        "search": sorted(i for i, s in sides.items() if s == "search"),
        "held_out": sorted(i for i, s in sides.items() if s == "held_out"),
    }
    if labels is not None:
        out["hard"] = {side: sum(bool(labels[i]) for i in out[side]) for side in ("search", "held_out")}
    pathlib.Path(args.out).write_text(json.dumps(out, indent=1) + "\n")
    print(f"{out['status']}: {len(out['search'])} search, {len(out['held_out'])} held out")


if __name__ == "__main__":
    main()
