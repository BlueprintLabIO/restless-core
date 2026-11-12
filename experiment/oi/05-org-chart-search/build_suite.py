#!/usr/bin/env python3
"""Write the operations list, the forecast slots and suite.json (the 100 tasks in one list).

Operations jobs are the 20 resale and email tasks of the experiment 2 task bank,
named by id and file hashes at a fixed commit. They are not copied here.

Forecast slots are empty places. Each is filled later with one auction that is
still open when a run starts, under experiment 4's rules. No auction with a
known final price may ever fill a slot.

Usage: python3 build_suite.py      (needs the two source commits in the local repo)
"""
import hashlib
import json
import pathlib
import subprocess

SUITE = pathlib.Path(__file__).resolve().parent
BANK_COMMIT = "51f18bf6"
BANK_ROOT = "experiment/oi/tasks"
RULES_COMMIT = "fa229c99"
RULES_PATH = "experiment/oi/04-forecasts-the-world-scores/rules.json"
EXTRA_BAND = {"cameras": 0, "electronics": 1, "tools": 2, "lego": 3}


def git(*args):
    return subprocess.run(["git", *args], capture_output=True, text=True, check=True, cwd=SUITE).stdout


def sha(text):
    return hashlib.sha256(text.encode()).hexdigest()


def dump(path, obj):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(obj, indent=1) + "\n")


def operations():
    commit = git("rev-parse", BANK_COMMIT).strip()
    manifest = git("show", f"{commit}:{BANK_ROOT}/MANIFEST.sha256")
    files = {}
    for line in manifest.splitlines():
        digest, path = line.split(None, 1)
        parts = path.split("/")
        if parts[0] == "bank" and len(parts) == 3:
            files.setdefault(parts[1], {})[parts[2]] = digest
    titles = {r["id"]: r for r in json.loads(git("show", f"{commit}:{BANK_ROOT}/bank/index.json"))}
    jobs = []
    for kind in ("resale", "email"):
        for n in range(1, 11):
            bid = f"{kind}-{n:02d}"
            assert titles[bid]["kind"] == kind and len(files[bid]) == 7, bid
            jobs.append({"id": f"ops-{len(jobs) + 1:02d}", "family": "operations", "bank_task": bid, "kind": kind,
                         "title": titles[bid]["title"], "files": dict(sorted(files[bid].items()))})
    out = {"source": {"branch": "research/oi-task-bank", "commit": commit, "root": BANK_ROOT + "/bank"},
           "note": "Each job is one bank task with all four of its versions. Which version an arm gets is set by the run design, not here.",
           "jobs": jobs}
    dump(SUITE / "tasks" / "operations" / "index.json", out)
    return jobs


def forecasts():
    commit = git("rev-parse", RULES_COMMIT).strip()
    raw = git("show", f"{commit}:{RULES_PATH}")
    rules = json.loads(raw)
    slots = []
    for cat in rules["categories"]:
        for band in [0, 1, 2, 3, EXTRA_BAND[cat]]:
            lo, hi = rules["price_bands"][band]
            slots.append({"id": f"forecast-{len(slots) + 1:02d}", "family": "forecast", "category": cat,
                          "price_band": [lo, hi], "auction": None})
    out = {
        "rules_source": {"branch": "research/oi-04-forecast-preparation", "commit": commit, "path": RULES_PATH, "sha256": sha(raw)},
        "site": rules["site"], "currency": rules["currency"],
        "fill_rule": {
            "status_when_filled": "open",
            "min_days_to_end": rules["min_days_to_end"], "max_days_to_end": rules["max_days_to_end"], "min_bids": rules["min_bids"],
            "price_band_is_judged_on": "current bid when the slot is filled",
            "never": "an auction that has ended, or whose final price is known to anyone or anything in the run",
            "taken_from": "experiment 4's open batches, collected at the start of the run that uses the slot",
        },
        "slots": slots,
    }
    dump(SUITE / "tasks" / "forecast" / "slots.json", out)
    return slots


def main():
    entries = []
    for p in sorted((SUITE / "tasks" / "code").glob("code-*/task.json")):
        t = json.loads(p.read_text())
        entries.append({"id": t["id"], "family": "code", "group": "commit:" + t["commit"], "stratum": t["grade_runner"],
                        "content": {"brief": t["brief_sha256"], "hidden_tests": t["hidden_tests_sha256"],
                                    "reference_change": t["reference_change_sha256"]}})
    for p in sorted((SUITE / "tasks" / "research").glob("research-*/task.json")):
        t = json.loads(p.read_text())
        entries.append({"id": t["id"], "family": "research", "group": "world:" + t["world"], "stratum": "",
                        "content": {"question": t["question_sha256"], "key": t["key_sha256"],
                                    **{"doc:" + k: v for k, v in t["documents"].items()}}})
    for j in operations():
        entries.append({"id": j["id"], "family": "operations", "group": "bank:" + j["bank_task"], "stratum": j["kind"],
                        "content": {"file:" + k: v for k, v in j["files"].items()}})
    for s in forecasts():
        entries.append({"id": s["id"], "family": "forecast", "group": "slot:" + s["id"], "stratum": s["category"],
                        "content": {"slot": sha(json.dumps([s["id"], s["category"], s["price_band"]]))}})
    dump(SUITE / "suite.json", {"version": 1, "tasks": entries})
    print(f"suite.json: {len(entries)} tasks")


if __name__ == "__main__":
    main()
