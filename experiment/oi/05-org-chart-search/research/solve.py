#!/usr/bin/env python3
"""Answer every research task from the written documents alone and compare with the key.

This does not import build.py and never sees the generator's records. It reads
the markdown in tasks/research/worlds/, the question settings in task.json, and
works the answer out the way a careful reader would. It also works out the
answers a careless reader would get (skipping one rule at a time) and reports
whether each of those differs from the key.

Writes research/solve-report.json. Exit 1 if any answer differs from its key.

Usage: python3 research/solve.py
"""
import json
import pathlib
import re
import sys

SUITE = pathlib.Path(__file__).resolve().parent.parent
R = SUITE / "tasks" / "research"


def rows(path):
    out = []
    for line in path.read_text().splitlines():
        if line.startswith("|") and not line.startswith("|---"):
            out.append([c.strip() for c in line.strip().strip("|").split("|")])
    return out[1:]


def figure(cell, factor):
    if "†" in cell:
        return int(cell.replace("†", "").strip()) * factor
    return int(cell)


def read_world(wdir):
    reg = {code: {"name": name, "region": region, "category": cat} for code, name, region, cat in rows(wdir / "01-register.md")}
    name_to_code = {v["name"]: c for c, v in reg.items()}
    factor = int(re.search(r"is (\d+) ", (wdir / "03-units-note.md").read_text()).group(1))
    first = {code: figure(cell, factor) for code, cell in rows(wdir / "02-annual-return.md")}
    final = dict(first)
    for name, cell in rows(wdir / "04-correction-notice.md"):
        final[name_to_code[name]] = figure(cell, factor)
    withdrawn = {name_to_code[l[2:].strip()] for l in (wdir / "06-withdrawals.md").read_text().splitlines() if l.startswith("- ")}
    former = {name: code for name, code in rows(wdir / "07-former-names.md")}
    log = [(int(n), date, ref) for n, date, ref in rows(wdir / "08-log.md")]
    void_line = [l for l in (wdir / "09-void-entries.md").read_text().splitlines() if re.fullmatch(r"[\d, ]+", l)][0]
    void = {int(x) for x in void_line.split(",")}
    return {"reg": reg, "name_to_code": name_to_code, "first": first, "final": final, "withdrawn": withdrawn,
            "former": former, "log": log, "void": void}


def threshold(w, p, corrections=True, withdrawals=True):
    figs = w["final"] if corrections else w["first"]
    return sorted(v["name"] for c, v in w["reg"].items() if v["region"] == p["region"] and figs[c] > p["threshold"]
                  and not (withdrawals and c in w["withdrawn"]))


def count(w, p, former=True, voids=True, withdrawals=True):
    seen = {}
    for n, date, ref in w["log"]:
        if not (p["from"] <= date <= p["to"]) or (voids and n in w["void"]):
            continue
        code = ref if ref in w["reg"] else w["name_to_code"].get(ref) or (w["former"].get(ref) if former else None)
        if code:
            seen[code] = seen.get(code, 0) + 1
    return sorted(v["name"] for c, v in w["reg"].items() if v["category"] == p["category"] and seen.get(c, 0) >= p["at_least"]
                  and not (withdrawals and c in w["withdrawn"]))


def main():
    report, bad = [], 0
    worlds = {}
    for tdir in sorted(R.glob("research-*")):
        task = json.loads((tdir / "task.json").read_text())
        key = json.loads((tdir / "key.json").read_text())["names"]
        w = worlds.setdefault(task["world"], read_world(R / "worlds" / task["world"]))
        p = task["params"]
        if task["kind"] == "threshold":
            got = threshold(w, p)
            shortcuts = {"ignores corrections": threshold(w, p, corrections=False), "ignores withdrawals": threshold(w, p, withdrawals=False)}
        else:
            got = count(w, p)
            shortcuts = {"ignores former names": count(w, p, former=False), "ignores void entries": count(w, p, voids=False),
                         "ignores withdrawals": count(w, p, withdrawals=False)}
        ok = got == key
        bad += not ok
        report.append({"id": task["id"], "matches_key": ok, "key_size": len(key),
                       "shortcuts_that_give_the_key": sorted(k for k, v in shortcuts.items() if v == key)})
    (SUITE / "research" / "solve-report.json").write_text(json.dumps(report, indent=1) + "\n")
    easy = [r["id"] for r in report if r["shortcuts_that_give_the_key"]]
    print(f"{len(report) - bad}/{len(report)} answers match the key; {len(easy)} tasks where a shortcut still gives the key")
    sys.exit(1 if bad or easy else 0)


if __name__ == "__main__":
    main()
