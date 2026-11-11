#!/usr/bin/env python3
"""Run an exact-commit JavaScript proof in its prepared Company Runtime.

Codex leads run on the host while the Cosmon browser dependencies live in the
Company Runtime. This adapter keeps that already-probed boundary explicit,
exports the selected commit without Git metadata, and lets the actor verify the
actual review artifact rather than a friendlier repository checkout.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path


CONNECTION_REFUSED = re.compile(r"ERR_CONNECTION_REFUSED|ECONNREFUSED", re.I)
LOCAL_PORT = re.compile(r"127\.0\.0\.1:(\d+)")


def git(workspace: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", "-C", str(workspace), *args],
        text=True,
        capture_output=True,
        timeout=30,
    )


def container_run(
    cell: str,
    check_file: str,
    commit: str,
    ports: list[str],
) -> subprocess.CompletedProcess[str]:
    script = """
set -eu
check_file=$1
candidate=$2
shift 2
review_dir=$(mktemp -d /tmp/restless-actor-native-check.XXXXXX)
pids=''
cleanup() {
  for pid in $pids; do kill "$pid" 2>/dev/null || true; done
  for pid in $pids; do wait "$pid" 2>/dev/null || true; done
  rm -rf "$review_dir"
}
trap cleanup EXIT INT TERM
git archive "$candidate" | tar -x -C "$review_dir"
cd "$review_dir"
for port in "$@"; do
  python3 -m http.server "$port" >/tmp/restless-actor-native-check-"$port".log 2>&1 &
  pids="$pids $!"
done
if [ "$#" -gt 0 ]; then sleep 0.4; fi
node "$check_file"
"""
    return subprocess.run(
        [
            "docker",
            "exec",
            "-u",
            "company",
            "-w",
            "/workspace",
            cell,
            "sh",
            "-c",
            script,
            "native-check",
            check_file,
            commit,
            *ports,
        ],
        text=True,
        capture_output=True,
        timeout=190,
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("cell")
    parser.add_argument("workspace")
    parser.add_argument(
        "--commit",
        help="Exact candidate commit. Defaults to the canonical candidate branch.",
    )
    parser.add_argument("check_file")
    args = parser.parse_args()

    workspace = Path(args.workspace).resolve()
    relative = Path(args.check_file)
    if relative.is_absolute() or ".." in relative.parts:
        parser.error("check_file must be a relative path inside the actor workspace")
    resolved = git(workspace, "rev-parse", f"{args.commit or 'candidate'}^{{commit}}")
    if resolved.returncode:
        parser.error(f"candidate commit is not resolvable: {resolved.stderr.strip()}")
    commit = resolved.stdout.strip()
    source = git(workspace, "show", f"{commit}:{relative.as_posix()}")
    if source.returncode:
        parser.error(f"check file does not exist in {commit}: {relative}")

    first = container_run(args.cell, str(relative), commit, [])
    combined = first.stdout + "\n" + first.stderr
    result = first
    if first.returncode and CONNECTION_REFUSED.search(combined):
        ports = sorted(set(LOCAL_PORT.findall(source.stdout)))
        if ports:
            result = container_run(args.cell, str(relative), commit, ports)

    sys.stdout.write(result.stdout)
    sys.stderr.write(result.stderr)
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
