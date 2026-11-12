#!/usr/bin/env python3
"""Freeze the chosen Restless changes into code tasks.

For each commit in selection.json this writes tasks/code/<id>/:
  task.json           the commit, its parent, the parent tree, paths, hashes, the grading command
  brief.md            what the agent is asked to do
  hidden-tests.patch  the change's own test files, withheld from the agent

The agent's starting point is the parent tree. Nothing here runs a model.

Usage: python3 code/freeze.py      (from the suite folder; needs the commits in the local repo)
"""
import hashlib
import json
import pathlib
import re
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
SUITE = HERE.parent
ROOT = subprocess.run(["git", "rev-parse", "--show-toplevel"], cwd=HERE, capture_output=True, text=True, check=True).stdout.strip()
sys.path.insert(0, str(HERE))
from mine import classify  # noqa: E402

DROP_PARAGRAPH = re.compile(r"^(Observed|Checked|Seen|Verified|Local A/B|Claude-Session|Co-Authored|Not run)", re.I)


def git(*args, binary=False):
    out = subprocess.run(["git", *args], capture_output=True, check=True, cwd=ROOT).stdout
    return out if binary else out.decode()


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def brief_text(commit):
    """The change's own description with the paragraphs that report checks removed."""
    text = git("log", "-1", "--format=%s%n%n%b", commit)
    paragraphs = [p.strip() for p in re.split(r"\n\s*\n", text) if p.strip()]
    kept = [p for p in paragraphs if not DROP_PARAGRAPH.match(p)]
    kept = ["\n".join(l for l in p.splitlines() if not DROP_PARAGRAPH.match(l)) for p in kept]
    return kept[0], [p for p in kept[1:] if p.strip()]


def imports(path, text):
    if path.endswith(".rs"):
        return [re.sub(r"\s+", " ", m) for m in re.findall(r"^use [^;]+;", text, re.M)]
    return [re.sub(r"\s+", " ", m) for m in re.findall(r"^import[^;]*?from\s+['\"][^'\"]+['\"];?", text, re.M)]


def grade_command(tests):
    if all(re.search(r"\.(ts|mjs|js)$", t) for t in tests):
        return ["node", "--test", *tests], "node"
    cmds = []
    for t in tests:
        m = re.match(r"crates/([^/]+)/tests/([^/]+)\.rs$", t)
        if not m:
            raise SystemExit(f"unexpected test path {t}")
        cmds.append(["cargo", "test", "-p", m.group(1), "--test", m.group(2)])
    if len(cmds) != 1:
        raise SystemExit(f"more than one Rust test target: {tests}")
    return cmds[0], "cargo"


def main():
    sel = json.loads((HERE / "selection.json").read_text())
    rows = []
    for short in sel["chosen"]:
        commit = git("rev-parse", short).strip()
        parents = git("log", "-1", "--format=%P", commit).split()
        assert len(parents) == 1, short
        date = git("log", "-1", "--format=%cs", commit).strip()
        rows.append((date, commit, parents[0]))
    rows.sort()
    out_root = SUITE / "tasks" / "code"
    index = []
    for n, (date, commit, parent) in enumerate(rows, 1):
        tid = f"code-{n:02d}"
        tests, source, other = classify(parent, commit)
        test_paths = sorted(p for p, _, _ in tests)
        source_paths = sorted(p for p, _, _ in source)
        patch = git("diff", "--binary", "--no-renames", "--full-index", parent, commit, "--", *test_paths, binary=True)
        reference = git("diff", "--binary", "--no-renames", "--full-index", parent, commit, "--", *source_paths, binary=True)
        subject, body = brief_text(commit)
        iface = []
        for p in test_paths:
            lines = imports(p, git("show", f"{commit}:{p}"))
            iface.append(f"`{p}`\n\n```\n" + "\n".join(lines) + "\n```")
        brief = (
            f"# {subject}\n\n" + "\n\n".join(body) + "\n\n"
            "## How it is checked\n\n"
            "Your work is checked by test files you cannot see. They are placed at the paths below after you finish, "
            "and these are the lines they use to load your code, so the names and locations must match.\n\n"
            + "\n\n".join(iface) + "\n"
        )
        command, runner = grade_command(test_paths)
        d = out_root / tid
        d.mkdir(parents=True, exist_ok=True)
        (d / "brief.md").write_text(brief)
        (d / "hidden-tests.patch").write_bytes(patch)
        task = {
            "id": tid,
            "family": "code",
            "commit": commit,
            "commit_date": date,
            "parent": parent,
            "parent_tree": git("rev-parse", f"{parent}^{{tree}}").strip(),
            "source_paths": source_paths,
            "test_paths": test_paths,
            "other_paths": sorted(p for p, _, _ in other),
            "source_lines_changed": sum(a + d_ for _, a, d_ in source),
            "test_lines_added": sum(a for _, a, _ in tests),
            "grade_command": command,
            "grade_runner": runner,
            "brief_sha256": sha256(brief.encode()),
            "hidden_tests_sha256": sha256(patch),
            "reference_change_sha256": sha256(reference),
        }
        (d / "task.json").write_text(json.dumps(task, indent=1) + "\n")
        index.append({"id": tid, "commit": commit, "runner": runner, "title": subject})
    (out_root / "index.json").write_text(json.dumps(index, indent=1) + "\n")
    print(f"wrote {len(index)} code tasks")


if __name__ == "__main__":
    main()
