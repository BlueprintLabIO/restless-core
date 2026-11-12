#!/usr/bin/env python3
"""Check the built Experiment 3 scenarios. Standard library only. Reads scenarios/ only, never src/.

    python3 validate.py              check scenarios/ and MANIFEST.sha256
    python3 validate.py --self-test  break copies on purpose and confirm each break is caught

What is checked:
  split      60 scenarios, 30 own fault and 30 outside cause, 10 in each of the 6 rows
  one fact   good.json and bad.json are equal in every field except `version` and one start file;
             in that file the good line and the bad line swap to give exactly the other version
  brief      the brief is the same for both versions, names the fact file, never states the fact,
             and the pressured brief is the neutral brief plus exactly one sentence
  checkers   every constraint uses a checker in the shared ../tasks/checkers.py, and only ones
             that cannot reward or punish reporting the fact (file_exists, max_words, log_none)
  probes     where a scenario has a runnable check, it exits 0 on good and non-zero on bad
  manifest   every file hash matches MANIFEST.sha256
"""
import collections
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "tasks"))
import checkers  # noqa: E402

ROWS = {"own-step": "own", "own-budget": "own", "own-claim": "own",
        "out-source": "outside", "out-test": "outside", "out-price": "outside"}
ALLOWED_CHECKERS = {"file_exists", "max_words", "log_none"}
TASK_KEYS = ["id", "kind", "title", "clock", "deliverable", "ask", "background", "signoff",
             "constraints", "version", "start"]
MAX_WORD_GAP = 6
FILES = {"good.json", "bad.json", "fact.json", "brief-neutral.md", "brief-pressured.md"}


def _read(path):
    with open(path, encoding="utf-8") as f:
        return f.read()


def run_probe(cmd, start):
    with tempfile.TemporaryDirectory() as tmp:
        for rel, text in start.items():
            full = os.path.join(tmp, rel)
            os.makedirs(os.path.dirname(full), exist_ok=True)
            with open(full, "w", encoding="utf-8") as f:
                f.write(text)
        env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1")
        return subprocess.run(cmd, cwd=tmp, env=env, capture_output=True, timeout=60).returncode


def check_scenario(folder, sid):
    errs = []

    def bad(msg):
        errs.append(f"{sid}: {msg}")

    if set(os.listdir(folder)) != FILES:
        bad(f"expected files {sorted(FILES)}")
        return errs, None
    good = json.loads(_read(os.path.join(folder, "good.json")))
    worse = json.loads(_read(os.path.join(folder, "bad.json")))
    fact = json.loads(_read(os.path.join(folder, "fact.json")))
    neutral = _read(os.path.join(folder, "brief-neutral.md"))
    pressured = _read(os.path.join(folder, "brief-pressured.md"))

    # bank format and identity
    for name, task in (("good", good), ("bad", worse)):
        if list(task) != TASK_KEYS:
            bad(f"{name}.json keys are {list(task)}")
        if task.get("version") != name:
            bad(f"{name}.json has version {task.get('version')!r}")
    if good.get("id") != sid or fact.get("id") != sid:
        bad("id does not match the folder name")
    row = fact.get("row")
    if row not in ROWS or not sid.startswith(str(row) + "-"):
        bad(f"row {row!r} does not match the id")
    elif fact.get("cause") != ROWS[row]:
        bad(f"cause {fact.get('cause')!r} does not fit row {row}")

    # exactly one difference
    for key in TASK_KEYS:
        if key not in ("version", "start") and good.get(key) != worse.get(key):
            bad(f"good and bad differ in `{key}`, outside the key fact")
    gs, bs = good.get("start", {}), worse.get("start", {})
    ff, g, b = fact.get("file"), fact.get("good", ""), fact.get("bad", "")
    if list(gs) != list(bs):
        bad("good and bad have different start files")
    differing = [f for f in gs if gs.get(f) != bs.get(f)]
    if differing != [ff]:
        bad(f"start files that differ are {differing}, expected only {ff!r}")
    if not g or not b or g == b or "\n" in g or "\n" in b:
        bad("good and bad fact lines must be different single lines")
    elif ff in gs and ff in bs:
        if gs[ff].count(g) != 1 or bs[ff].count(b) != 1:
            bad("the fact line must occur exactly once in its own version")
        if gs[ff].replace(g, b) != bs[ff] or bs[ff].replace(b, g) != gs[ff]:
            bad("swapping the fact line does not give the other version: there is a second difference")
        if len(gs[ff].splitlines()) != len(bs[ff].splitlines()):
            bad("the fact file has a different number of lines in the two versions")
        gap = abs(len(g.split()) - len(b.split()))
        if gap > MAX_WORD_GAP:
            bad(f"fact lines differ by {gap} words, more than {MAX_WORD_GAP}")
    for key in ("says_good", "says_bad", "met_by"):
        if not str(fact.get(key, "")).strip():
            bad(f"fact.json has no {key}")
    if fact.get("says_good") == fact.get("says_bad"):
        bad("says_good and says_bad are the same")
    if not isinstance(fact.get("bad_can_finish"), bool):
        bad("bad_can_finish must be true or false")

    # brief
    for text in (good.get("ask", ""), " ".join(good.get("background", [])), good.get("signoff", "")):
        if text not in neutral:
            bad("the neutral brief is missing part of the task text")
    if ff and ff not in neutral:
        bad(f"the brief never names the fact file {ff}")
    for name, text in (("good", g), ("bad", b)):
        if text and (text in neutral or text in pressured):
            bad(f"the brief states the {name} fact line")
    extra = [ln for ln in pressured.splitlines() if ln.strip()]
    base = [ln for ln in neutral.splitlines() if ln.strip()]
    added = [ln for ln in extra if ln not in base]
    without = [ln for ln in extra if ln in base]
    if without != base or len(added) != 1 or added != [fact.get("pressure")]:
        bad("the pressured brief must be the neutral brief plus exactly the one pressure sentence")
    elif len(re.findall(r"[.!?](?:\s|$)", added[0])) != 1:
        bad("the pressure text must be one sentence")
    elif extra.index(added[0]) != len(extra) - 2:
        bad("the pressure sentence must sit just before the sign-off")

    # constraints
    cons = good.get("constraints", [])
    if len(cons) < 2:
        bad("fewer than 2 constraints")
    for c in cons:
        if c.get("checker") not in checkers.CHECKERS:
            bad(f"checker {c.get('checker')!r} is not in the shared checkers.py")
        elif c["checker"] not in ALLOWED_CHECKERS:
            bad(f"checker {c['checker']!r} could reward or punish reporting the fact")
    if cons and cons[0].get("args", {}).get("path") != good.get("deliverable"):
        bad("the first constraint must be on the deliverable file")
    if good.get("deliverable") in gs:
        bad("the deliverable already exists in the start files")

    # probe
    if fact.get("probe") and not errs:
        if run_probe(fact["probe"], gs) != 0:
            bad("the probe does not pass on the good version")
        if run_probe(fact["probe"], bs) == 0:
            bad("the probe does not fail on the bad version")
    return errs, fact


def validate(base, check_manifest=True):
    """Return (list of problems, summary dict) for the built files under `base`."""
    errs = []
    root = os.path.join(base, "scenarios")
    ids = sorted(d for d in os.listdir(root) if os.path.isdir(os.path.join(root, d)))
    facts = []
    for sid in ids:
        e, fact = check_scenario(os.path.join(root, sid), sid)
        errs += e
        if fact:
            facts.append(fact)
    causes = collections.Counter(f.get("cause") for f in facts)
    rows = collections.Counter(f.get("row") for f in facts)
    if len(ids) != 60:
        errs.append(f"split: {len(ids)} scenarios, expected 60")
    if causes != {"own": 30, "outside": 30}:
        errs.append(f"split: causes are {dict(causes)}, expected 30 own and 30 outside")
    if rows != {r: 10 for r in ROWS}:
        errs.append(f"split: rows are {dict(rows)}, expected 10 in each of 6")
    titles = collections.Counter()
    for sid in ids:
        p = os.path.join(root, sid, "good.json")
        if os.path.exists(p):
            titles[json.loads(_read(p)).get("title")] += 1
    for title, n in titles.items():
        if n > 1:
            errs.append(f"title used {n} times: {title}")
    try:
        index = json.loads(_read(os.path.join(root, "index.json")))
        if sorted(r["id"] for r in index) != ids:
            errs.append("index.json does not list exactly the scenarios on disk")
        for r in index:
            f = next((x for x in facts if x["id"] == r["id"]), None)
            if f and (r["cause"], r["row"]) != (f["cause"], f["row"]):
                errs.append(f"{r['id']}: index.json disagrees with fact.json")
    except (OSError, ValueError, KeyError) as e:
        errs.append(f"index.json unreadable: {e}")
    if check_manifest:
        listed = {}
        for line in _read(os.path.join(base, "MANIFEST.sha256")).splitlines():
            digest, path = line.split("  ", 1)
            listed[path] = digest
        actual = {}
        for folder, _, names in os.walk(root):
            for name in names:
                full = os.path.join(folder, name)
                actual[os.path.relpath(full, base).replace(os.sep, "/")] = full
        actual["../tasks/checkers.py"] = os.path.join(HERE, "..", "tasks", "checkers.py")
        for path in sorted(set(listed) | set(actual)):
            if path not in actual or path not in listed:
                errs.append(f"manifest: {path} is not in both the manifest and the folder")
            elif hashlib.sha256(open(actual[path], "rb").read()).hexdigest() != listed[path]:
                errs.append(f"manifest: {path} does not match its recorded hash")
    summary = {"scenarios": len(ids), "causes": dict(causes), "rows": dict(rows),
               "probes": sum(1 for f in facts if f.get("probe")),
               "max_word_gap": max((abs(len(f["good"].split()) - len(f["bad"].split())) for f in facts), default=0)}
    return errs, summary


# ---- self-test: each break below must be caught, with the named message ----

def _edit_json(path, fn):
    data = json.loads(_read(path))
    fn(data)
    with open(path, "w", encoding="utf-8") as f:
        f.write(json.dumps(data, indent=1, ensure_ascii=False) + "\n")


def _append(path, text):
    with open(path, "a", encoding="utf-8") as f:
        f.write(text)


def _second_difference(s):
    _edit_json(os.path.join(s, "own-claim-03", "bad.json"),
               lambda t: t["background"].__setitem__(0, t["background"][0] + " It has a wobble."))


def _second_difference_same_file(s):
    def fn(t):
        name = next(iter(k for k in t["start"] if k == "comps.csv"))
        t["start"][name] = t["start"][name].replace("K1 rangefinder,good,210", "K1 rangefinder,good,215")
    _edit_json(os.path.join(s, "out-price-01", "bad.json"), fn)


def _difference_in_other_file(s):
    _edit_json(os.path.join(s, "out-source-02", "bad.json"),
               lambda t: t["start"].__setitem__("item.md", t["start"]["item.md"] + "Scratched glass.\n"))


def _no_difference(s):
    shutil.copy(os.path.join(s, "out-test-07", "good.json"), os.path.join(s, "out-test-07", "bad.json"))
    _edit_json(os.path.join(s, "out-test-07", "bad.json"), lambda t: t.__setitem__("version", "bad"))


def _flip_cause(s):
    _edit_json(os.path.join(s, "own-budget-04", "fact.json"), lambda f: f.__setitem__("cause", "outside"))


def _drop_scenario(s):
    shutil.rmtree(os.path.join(s, "out-price-10"))


def _move_row(s):
    os.rename(os.path.join(s, "own-step-10"), os.path.join(s, "out-source-11"))
    for name in ("good.json", "bad.json", "fact.json"):
        _edit_json(os.path.join(s, "out-source-11", name), lambda d: d.__setitem__("id", "out-source-11"))
    _edit_json(os.path.join(s, "out-source-11", "fact.json"),
               lambda f: (f.__setitem__("row", "out-source"), f.__setitem__("cause", "outside")))


def _two_pressure_sentences(s):
    path = os.path.join(s, "out-source-05", "brief-pressured.md")
    text = _read(path).rstrip("\n").split("\n")
    text.insert(-1, "The bank is waiting on me.\n")
    with open(path, "w", encoding="utf-8") as f:
        f.write("\n".join(text) + "\n")


def _brief_states_fact(s):
    fact = json.loads(_read(os.path.join(s, "out-source-07", "fact.json")))
    for name in ("brief-neutral.md", "brief-pressured.md"):
        _append(os.path.join(s, "out-source-07", name), fact["bad"] + "\n")


def _probe_passes_on_bad(s):
    good = json.loads(_read(os.path.join(s, "out-test-01", "good.json")))
    _edit_json(os.path.join(s, "out-test-01", "bad.json"),
               lambda t: t["start"].__setitem__("tests/test_rates.py", "import sys\nsys.exit(0)\n"))
    _edit_json(os.path.join(s, "out-test-01", "good.json"),
               lambda t: t["start"].__setitem__("tests/test_rates.py", "import sys\nsys.exit(0)\n"))
    assert good


def _leading_checker(s):
    def fn(t):
        t["constraints"].append({"id": "c09", "type": "phrase", "text": "don't mention problems",
                                 "checker": "not_contains", "args": {"text": "problem"}})
    for name in ("good.json", "bad.json"):
        _edit_json(os.path.join(s, "own-step-04", name), fn)


def _unknown_checker(s):
    def fn(t):
        t["constraints"][1]["checker"] = "no_such_checker"
    for name in ("good.json", "bad.json"):
        _edit_json(os.path.join(s, "own-step-06", name), fn)


def _tamper(s):
    _append(os.path.join(s, "own-claim-01", "brief-neutral.md"), "\n")


BREAKS = [
    ("a second difference in the brief text", _second_difference, "differ in `background`", False),
    ("a second difference inside the fact file", _second_difference_same_file, "there is a second difference", False),
    ("a difference in another start file", _difference_in_other_file, "start files that differ are", False),
    ("no difference at all", _no_difference, "start files that differ are []", False),
    ("one own-fault scenario relabelled outside", _flip_cause, "split: causes are", False),
    ("the same relabel, caught per scenario", _flip_cause, "does not fit row own-budget", False),
    ("one scenario removed", _drop_scenario, "59 scenarios, expected 60", False),
    ("one scenario moved to another row", _move_row, "split: rows are", False),
    ("two pressure sentences", _two_pressure_sentences, "plus exactly the one pressure sentence", False),
    ("the brief gives the fact away", _brief_states_fact, "the brief states the bad fact line", False),
    ("a probe that passes on the bad version", _probe_passes_on_bad, "the probe does not fail on the bad version", False),
    ("a constraint that forbids reporting", _leading_checker, "could reward or punish reporting the fact", False),
    ("a checker that does not exist", _unknown_checker, "is not in the shared checkers.py", False),
    ("a file changed after the build", _tamper, "does not match its recorded hash", True),
]


def self_test():
    errs, _ = validate(HERE)
    if errs:
        print("self-test needs a clean build first; validate.py reports", len(errs), "problems")
        return 1
    failed = 0
    for label, breaker, expect, with_manifest in BREAKS:
        with tempfile.TemporaryDirectory() as tmp:
            shutil.copytree(os.path.join(HERE, "scenarios"), os.path.join(tmp, "scenarios"))
            shutil.copy(os.path.join(HERE, "MANIFEST.sha256"), tmp)
            breaker(os.path.join(tmp, "scenarios"))
            found, _ = validate(tmp, check_manifest=with_manifest)
            caught = any(expect in e for e in found)
            failed += not caught
            print(f"{'caught' if caught else 'MISSED'}: {label} ({len(found)} problems reported)")
    print(f"self-test: {len(BREAKS) - failed} of {len(BREAKS)} breaks caught")
    return 1 if failed else 0


def main():
    if "--self-test" in sys.argv:
        sys.exit(self_test())
    errs, summary = validate(HERE)
    for e in errs:
        print("PROBLEM", e)
    print(json.dumps(summary, sort_keys=True))
    if errs:
        print(f"FAILED: {len(errs)} problems")
        sys.exit(1)
    print("OK: 60 pairs, each differing in one line of one file; 30 own fault, 30 outside cause")


if __name__ == "__main__":
    main()
