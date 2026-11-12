#!/usr/bin/env python3
"""Lay out one code task for a run, or its grading copy.

  python3 code/materialise.py code-07 /path/to/empty/dir
      writes the parent tree (no .git, so no history) and BRIEF.md. This is what the agent gets.

  python3 code/materialise.py code-07 /path/to/agent/dir --add-hidden-tests
      applies the withheld test files onto a finished agent directory, for grading.
      Then run the task's grade_command from that directory; exit 0 is a pass.

The hidden tests overwrite any file the agent left at the same paths.
"""
import argparse
import io
import json
import pathlib
import subprocess
import sys
import tarfile

SUITE = pathlib.Path(__file__).resolve().parent.parent
ROOT = subprocess.run(["git", "rev-parse", "--show-toplevel"], cwd=SUITE, capture_output=True, text=True, check=True).stdout.strip()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("task")
    ap.add_argument("dest")
    ap.add_argument("--add-hidden-tests", action="store_true")
    args = ap.parse_args()
    tdir = SUITE / "tasks" / "code" / args.task
    task = json.loads((tdir / "task.json").read_text())
    dest = pathlib.Path(args.dest)
    if args.add_hidden_tests:
        for rel in task["test_paths"]:
            (dest / rel).unlink(missing_ok=True)
        files = subprocess.run(["git", "archive", task["commit"], "--", *task["test_paths"]], capture_output=True, check=True, cwd=ROOT).stdout
        tarfile.open(fileobj=io.BytesIO(files)).extractall(dest, filter="data")
        print(json.dumps(task["grade_command"]))
        return
    dest.mkdir(parents=True, exist_ok=True)
    if any(dest.iterdir()):
        sys.exit(f"{dest} is not empty")
    tree = subprocess.run(["git", "archive", task["parent"]], capture_output=True, check=True, cwd=ROOT).stdout
    tarfile.open(fileobj=io.BytesIO(tree)).extractall(dest, filter="data")
    (dest / "BRIEF.md").write_text((tdir / "brief.md").read_text())
    print(f"{args.task}: parent {task['parent'][:12]} written to {dest}")


if __name__ == "__main__":
    main()
