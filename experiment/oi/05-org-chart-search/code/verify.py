#!/usr/bin/env python3
"""Check each frozen code task against the repository and record what was seen.

For every task:
  1. the hidden test patch has the recorded hash and applies cleanly to the parent tree;
  2. the parent tree and commit still match task.json.
For tasks graded with node (no packages needed):
  3. parent + hidden tests: the grading command fails;
  4. the real change + hidden tests: the grading command passes.
Tasks graded with cargo get steps 1 and 2 only. This script never builds Rust.

Writes code/verification.json. Uses one scratch worktree, removed at the end.

Usage: python3 code/verify.py
"""
import hashlib
import json
import pathlib
import shutil
import subprocess
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
SUITE = HERE.parent
ROOT = subprocess.run(["git", "rev-parse", "--show-toplevel"], cwd=HERE, capture_output=True, text=True, check=True).stdout.strip()


def run(cmd, cwd=None, data=None, timeout=300):
    try:
        p = subprocess.run(cmd, cwd=cwd, input=data, capture_output=True, timeout=timeout)
        return p.returncode
    except subprocess.TimeoutExpired:
        return 124


def out(cmd, cwd=None):
    return subprocess.run(cmd, cwd=cwd or ROOT, capture_output=True, text=True, check=True).stdout.strip()


def main():
    tasks = [json.loads(p.read_text()) for p in sorted((SUITE / "tasks" / "code").glob("code-*/task.json"))]
    scratch = pathlib.Path(tempfile.mkdtemp(prefix="oi05-verify-"))
    wt = scratch / "wt"
    subprocess.run(["git", "worktree", "add", "--detach", str(wt), tasks[0]["parent"]], check=True, capture_output=True, cwd=ROOT)
    results = []
    try:
        for t in tasks:
            patch = (SUITE / "tasks" / "code" / t["id"] / "hidden-tests.patch").read_bytes()
            r = {"id": t["id"], "runner": t["grade_runner"]}
            r["patch_hash_matches"] = hashlib.sha256(patch).hexdigest() == t["hidden_tests_sha256"]
            r["parent_is_parent"] = out(["git", "log", "-1", "--format=%P", t["commit"]]) == t["parent"]
            r["parent_tree_matches"] = out(["git", "rev-parse", t["parent"] + "^{tree}"]) == t["parent_tree"]

            def reset(rev):
                run(["git", "checkout", "-f", "--detach", rev], cwd=wt)
                run(["git", "clean", "-fdq"], cwd=wt)

            reset(t["parent"])
            r["patch_applies_to_parent"] = run(["git", "apply", "--check", "-"], cwd=wt, data=patch) == 0
            if t["grade_runner"] == "node":
                run(["git", "apply", "-"], cwd=wt, data=patch)
                r["exit_at_parent_with_tests"] = run(t["grade_command"], cwd=wt)
                reset(t["commit"])
                r["exit_at_change"] = run(t["grade_command"], cwd=wt)
                r["fails_before_passes_after"] = r["exit_at_parent_with_tests"] != 0 and r["exit_at_change"] == 0
            else:
                r["fails_before_passes_after"] = None
            results.append(r)
            print(r["id"], r["runner"], r["patch_applies_to_parent"], r["fails_before_passes_after"], flush=True)
    finally:
        subprocess.run(["git", "worktree", "remove", "--force", str(wt)], capture_output=True, cwd=ROOT)
        shutil.rmtree(scratch, ignore_errors=True)
    node_version = out(["node", "--version"])
    (HERE / "verification.json").write_text(json.dumps({"node": node_version, "tasks": results}, indent=1) + "\n")
    ran = [r for r in results if r["runner"] == "node"]
    print(f"{sum(r['patch_applies_to_parent'] for r in results)}/{len(results)} patches apply; "
          f"{sum(bool(r['fails_before_passes_after']) for r in ran)}/{len(ran)} node tasks fail before and pass after; "
          f"{len(results) - len(ran)} cargo tasks not run")


if __name__ == "__main__":
    main()
