#!/usr/bin/env python3
"""Check the suite and its split. Exit 0 only if every check passes.

Checks
  files      every file listed in MANIFEST.sha256 exists with that hash, and no unlisted file sits under tasks/
  counts     30 code, 30 research, 20 operations, 20 forecast; ids unique
  content    every hash in suite.json matches the file it names (code briefs and patches, research questions, keys, documents)
  keys       every research key is a non-empty list of names from that world's register, smaller than the register
  forecast   20 empty slots, 5 per category, every price band present in every category, no auction filled in
  bank       operations file hashes equal the task bank manifest at its commit (skipped, and said so, if the commit is absent)
  split      60 search / 40 held out, 18/12, 18/12, 12/8, 12/8, every task on exactly one side, split matches suite.json
  leak       no group (commit, research world, bank task, slot) on both sides; no content hash on both sides
  rerun      split.py run again with the same inputs gives the same split
  final      with --final: the split must be difficulty balanced (it is not until calibration labels exist)

Also reports, without failing, how many search code tasks start from a snapshot
that already contains a held-out change. See DECISIONS.md, item 9.

Usage: python3 validate.py [--final] [--suite suite.json] [--split split.json]
"""
import argparse
import hashlib
import json
import pathlib
import subprocess
import sys

SUITE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(SUITE))
import split as splitter  # noqa: E402

FAMILY = {"code": 30, "research": 30, "operations": 20, "forecast": 20}
SEARCH = {"code": 18, "research": 18, "operations": 12, "forecast": 12}


def file_sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git(*args):
    return subprocess.run(["git", *args], capture_output=True, text=True, cwd=SUITE)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--final", action="store_true")
    ap.add_argument("--suite", default=str(SUITE / "suite.json"))
    ap.add_argument("--split", default=str(SUITE / "split.json"))
    args = ap.parse_args()
    fails, notes = [], []

    def check(name, ok, detail=""):
        print(("ok    " if ok else "FAIL  ") + name + (f": {detail}" if detail and not ok else ""))
        if not ok:
            fails.append(name)

    suite_bytes = pathlib.Path(args.suite).read_bytes()
    tasks = json.loads(suite_bytes)["tasks"]
    sp = json.loads(pathlib.Path(args.split).read_text())
    by_id = {t["id"]: t for t in tasks}

    # files
    listed = {}
    for line in (SUITE / "MANIFEST.sha256").read_text().splitlines():
        digest, path = line.split("  ", 1)
        listed[path] = digest
    wrong = [p for p, d in listed.items() if not (SUITE / p).is_file() or file_sha(SUITE / p) != d]
    check("files: manifest hashes", not wrong, f"{len(wrong)} differ, first {wrong[:1]}")
    on_disk = {str(p.relative_to(SUITE)) for p in (SUITE / "tasks").rglob("*") if p.is_file()}
    extra = sorted(on_disk - set(listed))
    check("files: nothing unlisted under tasks/", not extra, f"{extra[:2]}")

    # counts
    ids = [t["id"] for t in tasks]
    check("counts: 100 tasks, unique ids", len(ids) == 100 and len(set(ids)) == 100, f"{len(ids)} entries, {len(set(ids))} unique")
    per = {f: sum(t["family"] == f for t in tasks) for f in FAMILY}
    check("counts: 30 code, 30 research, 20 operations, 20 forecast", per == FAMILY, str(per))

    # content
    bad = []
    for t in tasks:
        d = SUITE / "tasks" / t["family"] / t["id"]
        c = t["content"]
        try:
            if t["family"] == "code":
                tj = json.loads((d / "task.json").read_text())
                ok = (file_sha(d / "brief.md") == c["brief"] and file_sha(d / "hidden-tests.patch") == c["hidden_tests"]
                      and (d / "hidden-tests.patch").stat().st_size > 0 and "commit:" + tj["commit"] == t["group"]
                      and c["reference_change"] == tj["reference_change_sha256"])
            elif t["family"] == "research":
                tj = json.loads((d / "task.json").read_text())
                w = SUITE / "tasks" / "research" / "worlds" / tj["world"]
                ok = (file_sha(d / "question.md") == c["question"] and file_sha(d / "key.json") == c["key"]
                      and "world:" + tj["world"] == t["group"]
                      and all(file_sha(w / k[4:]) == v for k, v in c.items() if k.startswith("doc:"))
                      and sum(k.startswith("doc:") for k in c) == len(list(w.iterdir())))
            else:
                ok = True
        except (OSError, KeyError, ValueError):
            ok = False
        if not ok:
            bad.append(t["id"])
    check("content: hashes in suite.json match the task files", not bad, f"{bad[:3]}")

    # keys
    bad = []
    for t in tasks:
        if t["family"] != "research":
            continue
        try:
            d = SUITE / "tasks" / "research" / t["id"]
            names = json.loads((d / "key.json").read_text())["names"]
            reg = (SUITE / "tasks" / "research" / "worlds" / t["group"][6:] / "01-register.md").read_text()
            known = {l.split("|")[2].strip() for l in reg.splitlines() if l.startswith("| ") and not l.startswith("| Code")}
            if not names or len(set(names)) != len(names) or not set(names) < known:
                bad.append(t["id"])
        except (OSError, KeyError, ValueError, IndexError):
            bad.append(t["id"])
    check("keys: every research key is a non-empty proper subset of its register", not bad, f"{bad[:3]}")

    # forecast
    fc = json.loads((SUITE / "tasks" / "forecast" / "slots.json").read_text())
    slots = fc["slots"]
    cats = {}
    for s in slots:
        cats.setdefault(s["category"], []).append(tuple(s["price_band"]))
    ok = (len(slots) == 20 and len(cats) == 4 and all(len(v) == 5 and len(set(v)) == 4 for v in cats.values())
          and all(s["auction"] is None for s in slots) and fc["fill_rule"]["status_when_filled"] == "open"
          and fc["fill_rule"]["min_days_to_end"] >= 1)
    check("forecast: 20 empty slots, 5 per category, all 4 bands in each, open auctions only", ok)

    # bank
    ops = json.loads((SUITE / "tasks" / "operations" / "index.json").read_text())
    src = ops["source"]
    shown = git("show", f"{src['commit']}:experiment/oi/tasks/MANIFEST.sha256")
    if shown.returncode == 0:
        theirs = {}
        for line in shown.stdout.splitlines():
            digest, path = line.split(None, 1)
            theirs[path] = digest
        bad = [j["id"] for j in ops["jobs"] if any(theirs.get(f"bank/{j['bank_task']}/{f}") != h for f, h in j["files"].items())]
        blobs = [j["id"] for j in ops["jobs"] for f, h in j["files"].items()
                 if hashlib.sha256(subprocess.run(["git", "show", f"{src['commit']}:{src['root']}/{j['bank_task']}/{f}"],
                                                  capture_output=True, cwd=SUITE).stdout).hexdigest() != h]
        check("bank: operations hashes equal the task bank files at its commit", not bad and not blobs, f"{(bad + blobs)[:3]}")
    else:
        notes.append(f"bank check SKIPPED: commit {src['commit'][:8]} is not in this repository")
    check("bank: 20 jobs from 20 different bank tasks", len({j["bank_task"] for j in ops["jobs"]}) == 20 and len(ops["jobs"]) == 20)

    # split
    search, held = sp["search"], sp["held_out"]
    both = set(search) & set(held)
    check("split: 60 search, 40 held out", len(search) == 60 and len(held) == 40 and len(set(search)) == 60 and len(set(held)) == 40,
          f"{len(search)}/{len(held)}")
    check("split: every task on exactly one side", not both and set(search) | set(held) == set(ids), f"both={sorted(both)[:3]}")
    per_s = {f: sum(by_id[i]["family"] == f for i in search if i in by_id) for f in FAMILY}
    check("split: 18/12 code, 18/12 research, 12/8 operations, 12/8 forecast", per_s == SEARCH, str(per_s))
    check("split: made from this suite.json", sp.get("suite_sha256") == hashlib.sha256(suite_bytes).hexdigest())

    # leak
    side = {i: "search" for i in search}
    side.update({i: "held_out" for i in held})
    groups, hashes = {}, {}
    for t in tasks:
        s = side.get(t["id"])
        groups.setdefault(t["group"], set()).add(s)
        for k, h in t["content"].items():
            if t["family"] == "research" and k.startswith("doc:"):
                continue  # documents are shared inside a world; the group check covers them
            hashes.setdefault(h, set()).add(s)
    straddle = sorted(g for g, s in groups.items() if len(s) > 1)
    check("leak: no commit, research world, bank task or slot on both sides", not straddle, f"{straddle[:3]}")
    shared = [h for h, s in hashes.items() if len(s) > 1]
    check("leak: no brief, key, hidden test or file hash on both sides", not shared, f"{len(shared)} shared")
    docs = {}
    for t in tasks:
        if t["family"] == "research":
            for k, h in t["content"].items():
                if k.startswith("doc:"):
                    docs.setdefault(h, set()).add(side.get(t["id"]))
    check("leak: no research document on both sides", not [h for h, s in docs.items() if len(s) > 1])

    # rerun
    labels = None
    if sp.get("difficulty_balanced"):
        lp = SUITE / "labels.json"
        labels = json.loads(lp.read_text()) if lp.is_file() else None
    if sp.get("difficulty_balanced") and labels is None:
        check("rerun: split.py gives the same split", False, "split says balanced but labels.json is absent")
    else:
        again = splitter.split(tasks, labels)
        check("rerun: split.py gives the same split", sp.get("seed") == splitter.SEED
              and sorted(i for i, s in again.items() if s == "search") == sorted(search))

    if args.final:
        check("final: split is difficulty balanced", bool(sp.get("difficulty_balanced")), "draft split, no calibration labels yet")
    elif not sp.get("difficulty_balanced"):
        notes.append("split is a DRAFT: not difficulty balanced, because no calibration has run. --final fails until it is.")

    # report only: search snapshots that already contain a held-out change
    code = {t["id"]: json.loads((SUITE / "tasks" / "code" / t["id"] / "task.json").read_text()) for t in tasks if t["family"] == "code"}
    if git("cat-file", "-e", next(iter(code.values()))["commit"]).returncode == 0:
        pairs = [(s, h) for s in search if s in code for h in held if h in code
                 if git("merge-base", "--is-ancestor", code[h]["commit"], code[s]["parent"]).returncode == 0]
        notes.append(f"{len(pairs)} pairs where a search code task starts from a snapshot that already contains a held-out change "
                     f"({len({s for s, _ in pairs})} search tasks, {len({h for _, h in pairs})} held-out changes). Accepted for now, see DECISIONS.md.")
    else:
        notes.append("snapshot overlap report SKIPPED: the Restless commits are not in this repository")

    for n in notes:
        print("note  " + n)
    print(f"\n{'PASS' if not fails else 'FAIL'}: {len(fails)} failed")
    sys.exit(1 if fails else 0)


if __name__ == "__main__":
    main()
