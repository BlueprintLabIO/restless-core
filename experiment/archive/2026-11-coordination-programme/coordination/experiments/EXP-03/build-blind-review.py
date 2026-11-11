#!/usr/bin/env python3
"""Build an anonymised, artifact-only EXP-03 review prompt from exact run commits."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path


HERE = Path(__file__).resolve().parent
LAB_WORK = HERE.parents[2] / "coordination-lab" / "v2" / "workdir"


def git(repo: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(repo), *args],
        check=True,
        text=True,
        capture_output=True,
    ).stdout


def candidate(label_run: str, include: str) -> dict[str, object]:
    label, run_id = label_run.split("=", 1)
    run_dir = LAB_WORK / run_id
    summary = json.loads((run_dir / "summary.json").read_text())
    protocol = summary.get("protocol") or {}
    evidence_source = "terminal_summary"
    terminal_protocol_valid = bool(protocol.get("valid"))
    if not terminal_protocol_valid:
        postflight_path = run_dir / "postflight-evidence.json"
        if not postflight_path.exists():
            raise RuntimeError(
                f"{run_id} terminal protocol is invalid and has no explicit postflight recertification"
            )
        postflight = json.loads(postflight_path.read_text())
        if not postflight.get("valid"):
            raise RuntimeError(f"{run_id} postflight evidence is not valid")
        protocol = postflight.get("protocol") or {}
        evidence_source = "postflight_recertification"
    conformance = protocol.get("supervisor_conformance") or {}
    commit = conformance.get("candidate_commit")
    if not protocol.get("valid") or not isinstance(commit, str):
        raise RuntimeError(f"{run_id} is not a valid completed supervisor candidate")
    repo = run_dir / "canonical"
    files = [
        line
        for line in git(repo, "ls-tree", "-r", "--name-only", commit, "--", include).splitlines()
        if line
    ]
    if not files:
        raise RuntimeError(f"{run_id} commit {commit} has no files under {include!r}")
    artifact = []
    for path in files:
        body = git(repo, "show", f"{commit}:{path}")
        artifact.append(f"### `{path}`\n\n```text\n{body}\n```\n")
    return {
        "label": label,
        "run": run_id,
        "commit": commit,
        "files": files,
        "artifact": "\n".join(artifact),
        "evidence_source": evidence_source,
        "terminal_protocol_valid": terminal_protocol_valid,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("task")
    parser.add_argument("--scenario", required=True)
    parser.add_argument("--include", required=True)
    parser.add_argument("--candidate", action="append", required=True)
    parser.add_argument("--output-dir", required=True)
    args = parser.parse_args()
    if len(args.candidate) != 2:
        raise SystemExit("exactly two --candidate LABEL=RUN arguments are required")

    scenario = Path(args.scenario).resolve().read_text()
    rubric = (HERE / "blind-review-rubric.md").read_text()
    candidates = [candidate(value, args.include) for value in args.candidate]
    order_seed = hashlib.sha256(
        (hashlib.sha256(scenario.encode()).hexdigest() + "|" + "|".join(sorted(str(item["commit"]) for item in candidates))).encode()
    ).hexdigest()
    candidates.sort(key=lambda item: str(item["commit"]), reverse=int(order_seed[:2], 16) % 2 == 1)

    prompt = (
        f"{rubric}\n\n"
        f"# Frozen scenario\n\n{scenario}\n\n"
        f"# Candidate A\n\n{candidates[0]['artifact']}\n\n"
        f"# Candidate B\n\n{candidates[1]['artifact']}\n"
    )
    output_dir = Path(args.output_dir).resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    prompt_path = output_dir / "prompt.md"
    prompt_path.write_text(prompt)
    mapping = {
        "task": args.task,
        "scenario_sha256": hashlib.sha256(scenario.encode()).hexdigest(),
        "rubric_sha256": hashlib.sha256(rubric.encode()).hexdigest(),
        "order_seed": order_seed,
        "prompt_sha256": hashlib.sha256(prompt.encode()).hexdigest(),
        "candidate_A": {
            key: candidates[0][key]
            for key in (
                "label",
                "run",
                "commit",
                "files",
                "evidence_source",
                "terminal_protocol_valid",
            )
        },
        "candidate_B": {
            key: candidates[1][key]
            for key in (
                "label",
                "run",
                "commit",
                "files",
                "evidence_source",
                "terminal_protocol_valid",
            )
        },
    }
    (output_dir / "mapping.json").write_text(json.dumps(mapping, indent=2, sort_keys=True) + "\n")
    print(json.dumps(mapping, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
