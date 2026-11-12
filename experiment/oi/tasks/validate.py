#!/usr/bin/env python3
"""Validate the frozen task bank. Standard library only. Exit 0 only if everything holds.

Checks, per task: shape; exactly 10 constraints covering all six constraint types; a checker that
exists for each; 4 versions that each state every constraint exactly once and balance position and
phrasing; and that every checker passes on the known-good answer and fails on the known-bad one.
Fixtures are written to a temporary directory and read back the way a real submission is.
Also checks the bank is 40 tasks, 10 per kind, and that every file matches MANIFEST.sha256.
"""
import collections
import hashlib
import json
import os
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import checkers  # noqa: E402

KINDS = ["code", "research", "resale", "email"]
TYPES = {"number", "format", "phrase", "style", "action", "include"}
errors = []


def err(task, msg):
    errors.append(f"{task}: {msg}")


def read(rel):
    with open(os.path.join(HERE, rel), encoding="utf-8") as fh:
        return fh.read()


def materialise(task, fixture, root):
    files = dict(task["start"])
    files.update(fixture["files"])
    for rel, text in files.items():
        if text is None:
            continue
        path = os.path.join(root, rel)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as fh:
            fh.write(text)
    with open(os.path.join(root, checkers.LOG_NAME), "w", encoding="utf-8") as fh:
        for e in fixture["log"]:
            fh.write(json.dumps(e) + "\n")


def validate_task(tid):
    task = json.loads(read(f"bank/{tid}/task.json"))
    for key in ("id", "kind", "title", "clock", "deliverable", "ask", "background", "signoff",
                "constraints", "start"):
        if key not in task:
            return err(tid, f"missing {key}")
    cons = task["constraints"]
    if len(cons) != 10:
        err(tid, f"{len(cons)} constraints, want 10")
    if {c["type"] for c in cons} != TYPES:
        err(tid, f"constraint types missing: {sorted(TYPES - {c['type'] for c in cons})}")
    if len({c["text"] for c in cons}) != len(cons):
        err(tid, "duplicate constraint text")
    for c in cons:
        if c["checker"] not in checkers.CHECKERS:
            err(tid, f"{c['id']} unknown checker {c['checker']}")
            return

    versions = json.loads(read(f"bank/{tid}/versions.json"))
    seen = collections.defaultdict(set)
    for v in ("v1", "v2", "v3", "v4"):
        text = read(f"bank/{tid}/{v}.md")
        layout = versions[v]
        if sorted(r["constraint"] for r in layout) != sorted(c["id"] for c in cons):
            err(tid, f"{v} layout does not list each constraint once")
        if [r["position"] for r in layout] != list(range(1, 11)):
            err(tid, f"{v} positions are not 1 to 10")
        offsets = []
        for r in layout:
            c = next(x for x in cons if x["id"] == r["constraint"])
            sentence = r["frame"].format(t=c["text"])
            sentence = sentence[0].upper() + sentence[1:]
            if text.count(sentence) != 1:
                err(tid, f"{v} states {c['id']} {text.count(sentence)} times")
            offsets.append(text.find(sentence))
            seen[c["id"]].add(("edge" if r["block"] != "middle" else "middle", r["phrasing"]))
        if offsets != sorted(offsets):
            err(tid, f"{v} recorded positions do not match the text order")
        n_mid = sum(r["block"] == "middle" for r in layout)
        n_imp = sum(r["phrasing"] == "important" for r in layout)
        if not (4 <= n_mid <= 6 and 4 <= n_imp <= 6):
            err(tid, f"{v} unbalanced: {n_mid} middle, {n_imp} important")
        mids = [r["position"] for r in layout if r["block"] == "middle"]
        if mids and (min(mids) == 1 or max(mids) == 10):
            err(tid, f"{v} a middle constraint sits at an end")
    for cid, cells in seen.items():
        if len(cells) != 4:
            err(tid, f"{cid} does not get all four position and phrasing cells: {sorted(cells)}")

    fixtures = json.loads(read(f"bank/{tid}/fixtures.json"))
    for side, want in (("good", True), ("bad", False)):
        with tempfile.TemporaryDirectory() as root:
            materialise(task, fixtures[side], root)
            for row in checkers.check(task, root):
                if row["kept"] != want:
                    c = next(x for x in cons if x["id"] == row["constraint"])
                    err(tid, f"{side} answer: {c['id']} ({c['checker']}: {c['text']}) "
                             f"{'failed' if want else 'passed'}")
    # An empty submission must not satisfy any content constraint by accident.
    with tempfile.TemporaryDirectory() as root:
        materialise(task, {"files": {}, "log": []}, root)
        kept = [r for r in checkers.check(task, root) if r["kept"] and r["type"] != "action"]
        for r in kept:
            c = next(x for x in cons if x["id"] == r["constraint"])
            # A rule about a starting file ("no import of X") is honestly kept by not touching it.
            if c["args"].get("file") not in task["start"]:
                err(tid, f"untouched workspace passes {c['id']} ({c['checker']})")
    return task


def main():
    index = json.loads(read("bank/index.json"))
    ids = [t["id"] for t in index]
    if len(ids) != 40 or len(set(ids)) != 40:
        err("bank", f"{len(ids)} tasks, want 40 distinct")
    per_kind = collections.Counter(t["kind"] for t in index)
    for k in KINDS:
        if per_kind[k] != 10:
            err("bank", f"{per_kind[k]} {k} tasks, want 10")
    used = collections.Counter()
    for tid in ids:
        task = validate_task(tid)
        if task:
            used.update(c["checker"] for c in task["constraints"])
    for line in read("MANIFEST.sha256").splitlines():
        digest, rel = line.split("  ", 1)
        with open(os.path.join(HERE, rel), "rb") as fh:
            if hashlib.sha256(fh.read()).hexdigest() != digest:
                err("manifest", f"{rel} does not match its hash")
    if errors:
        print("\n".join(errors))
        sys.exit(f"FAILED: {len(errors)} problems")
    print(f"OK: {len(ids)} tasks, {sum(used.values())} constraints, {len(used)} checker kinds, "
          f"160 versions, 40 known-good and 40 known-bad answers")
    print("checker use: " + ", ".join(f"{k} {v}" for k, v in sorted(used.items())))


if __name__ == "__main__":
    main()
