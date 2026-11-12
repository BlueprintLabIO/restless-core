#!/usr/bin/env python3
"""List the Restless changes that could become code tasks.

Reads git history only. A change qualifies when it is a single-parent commit
in the window, its files split cleanly into test files and source files with
both present, the source change is 40 to 400 changed lines in at most 8 files,
it adds at least 15 test lines in at most 4 test files, and it carries a
written description of at least 200 characters. The 30 chosen and the
reserves were picked by hand from this list; see selection.json.

Usage: python3 code/mine.py [--ref origin/main]
"""
import argparse
import json
import re
import subprocess
import sys

TEST = re.compile(r"(^|/)tests?/|\.test\.(ts|js|mjs)$|\.spec\.(ts|js|mjs)$|_test\.rs$|(^|/)test_[^/]*\.py$")
ROOT = subprocess.run(["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True, check=True).stdout.strip()
NOT_SOURCE = re.compile(r"^(docs/|experiment/|output/|tmp/|release/)|\.md$|\.lock$|package-lock\.json$|\.snap$|\.png$|\.svg$")


def git(*args):
    return subprocess.run(["git", *args], capture_output=True, text=True, check=True, cwd=ROOT).stdout


def classify(parent, commit):
    """Return (tests, source, other) path lists, or None for a binary change."""
    tests, source, other = [], [], []
    for line in git("diff", "--numstat", "--no-renames", parent, commit).splitlines():
        added, deleted, path = line.split("\t")
        if added == "-":
            return None
        row = (path, int(added), int(deleted))
        if TEST.search(path):
            tests.append(row)
        elif NOT_SOURCE.search(path):
            other.append(row)
        else:
            source.append(row)
    return tests, source, other


def description(commit):
    text = git("log", "-1", "--format=%s%n%b", commit)
    lines = [l for l in text.splitlines() if not l.startswith("Co-Authored")]
    return lines[0], "\n".join(lines[1:]).strip()


def candidates(ref, since, until):
    out = []
    for line in git("log", ref, "--no-merges", f"--since={since}", f"--until={until} 23:59:59", "--format=%H %P").splitlines():
        parts = line.split()
        if len(parts) != 2:
            continue
        commit, parent = parts
        split = classify(parent, commit)
        if not split:
            continue
        tests, source, _ = split
        if not tests or not source:
            continue
        src_lines = sum(a + d for _, a, d in source)
        test_added = sum(a for _, a, _ in tests)
        if not (40 <= src_lines <= 400 and len(source) <= 8 and test_added >= 15 and len(tests) <= 4):
            continue
        subject, body = description(commit)
        if re.match(r"(WIP|Revert)", subject) or len(body) < 200:
            continue
        out.append({"commit": commit, "subject": subject, "source_lines": src_lines, "test_lines_added": test_added})
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ref", default="origin/main")
    ap.add_argument("--since", default="2026-10-01")
    ap.add_argument("--until", default="2026-11-12")
    args = ap.parse_args()
    rows = candidates(args.ref, args.since, args.until)
    json.dump(rows, sys.stdout, indent=1)
    print(f"\n{len(rows)} candidates", file=sys.stderr)


if __name__ == "__main__":
    main()
