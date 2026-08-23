#!/usr/bin/env python3
"""Run a workspace-native JavaScript proof in its prepared Company Runtime.

Codex leads run on the host while the Cosmon browser dependencies live in the
Company Runtime. This adapter keeps that already-probed boundary explicit and
lets the actor verify the current working tree without rediscovering a host CDP
stack or committing first.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path


CONNECTION_REFUSED = re.compile(r"ERR_CONNECTION_REFUSED|ECONNREFUSED", re.I)
LOCAL_PORT = re.compile(r"127\.0\.0\.1:(\d+)")


def container_run(cell: str, check_file: str, ports: list[str]) -> subprocess.CompletedProcess[str]:
    script = """
set -eu
check_file=$1
shift
cd /workspace
pids=''
cleanup() {
  for pid in $pids; do kill "$pid" 2>/dev/null || true; done
  for pid in $pids; do wait "$pid" 2>/dev/null || true; done
}
trap cleanup EXIT INT TERM
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
    parser.add_argument("check_file")
    args = parser.parse_args()

    workspace = Path(args.workspace).resolve()
    relative = Path(args.check_file)
    if relative.is_absolute() or ".." in relative.parts:
        parser.error("check_file must be a relative path inside the actor workspace")
    source_path = workspace / relative
    if not source_path.is_file():
        parser.error(f"check file does not exist: {source_path}")

    first = container_run(args.cell, str(relative), [])
    combined = first.stdout + "\n" + first.stderr
    result = first
    if first.returncode and CONNECTION_REFUSED.search(combined):
        ports = sorted(set(LOCAL_PORT.findall(source_path.read_text(errors="replace"))))
        if ports:
            result = container_run(args.cell, str(relative), ports)

    sys.stdout.write(result.stdout)
    sys.stderr.write(result.stderr)
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
