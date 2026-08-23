#!/usr/bin/env python3
"""Bounded first-party conformance probe for W01 session mitosis."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import time
from pathlib import Path


HERE = Path(__file__).resolve().parent
WORK_ROOT = HERE / "workdir"


def run(argv: list[str], *, cwd: Path, prompt: str | None = None, timeout: int = 900) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        argv,
        cwd=cwd,
        input=prompt,
        text=True,
        capture_output=True,
        timeout=timeout,
        env=os.environ.copy(),
    )
    if result.returncode:
        raise RuntimeError(f"command failed ({result.returncode}): {' '.join(argv)}\n{result.stderr[-2000:]}\n{result.stdout[-2000:]}")
    return result


def parse_events(raw: str) -> dict[str, object]:
    events = []
    for line in raw.splitlines():
        try:
            events.append(json.loads(line))
        except json.JSONDecodeError:
            continue
    started = [event for event in events if event.get("type") == "thread.started"]
    completed = [event for event in events if event.get("type") == "turn.completed"]
    return {
        "thread_id": started[-1]["thread_id"] if started else None,
        "usage": completed[-1].get("usage", {}) if completed else {},
        "tool_calls": sum(
            1 for event in events if event.get("type") == "item.started"
            and event.get("item", {}).get("type") in {"command_execution", "mcp_tool_call", "file_change"}
        ),
        "event_count": len(events),
    }


def init_repo(path: Path, role: str) -> None:
    path.mkdir()
    (path / "README.md").write_text(f"# W01 {role} branch\n")
    run(["git", "init", "-q", "-b", "candidate"], cwd=path)
    run(["git", "config", "user.name", "Restless Experiment"], cwd=path)
    run(["git", "config", "user.email", "experiment@localhost"], cwd=path)
    run(["git", "add", "README.md"], cwd=path)
    run(["git", "commit", "-q", "-m", "seed"], cwd=path)


def codex_base(model: str, effort: str = "low") -> list[str]:
    return [
        "codex", "exec", "--ignore-user-config", "--ignore-rules", "--skip-git-repo-check",
        "--json", "-m", model, "-c", f'model_reasoning_effort="{effort}"',
        "-c", 'approval_policy="never"', "-c", 'sandbox_mode="workspace-write"',
    ]


def git_value(repo: Path, *args: str) -> str:
    return run(["git", *args], cwd=repo).stdout.strip()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("run_id")
    args = parser.parse_args()
    run_dir = WORK_ROOT / args.run_id
    if run_dir.exists():
        raise SystemExit(f"refusing to overwrite existing probe: {run_dir}")
    run_dir.mkdir(parents=True)
    lead_repo = run_dir / "lead"
    worker_repo = run_dir / "worker"
    init_repo(lead_repo, "lead")
    init_repo(worker_repo, "worker")

    nonce = hashlib.sha256(f"{args.run_id}:shared-history".encode()).hexdigest()[:20]
    contract = (
        "W01 shared-prefix conformance. Preserve this secret task fact across the fork: "
        f"SHARED_NONCE={nonce}. Lead and worker will later receive different roles. Do not edit files."
    )
    contract_path = run_dir / "contract.md"
    contract_path.write_text(contract + "\n")
    common_prompt = f"""Read `contract.md` and inspect the two seeded Git repositories `lead/` and `worker/`.
This is the completed common kickoff history for a session-fork experiment. Do not edit any file or
commit. State the shared nonce and confirm both repositories are clean, then end with
`COMMON_PREFIX_READY`.
"""
    kickoff_last = run_dir / "kickoff-last.md"
    kickoff_argv = codex_base("gpt-5.6-sol") + [
        "-C", str(run_dir), "--sandbox", "workspace-write", "-o", str(kickoff_last), "-"
    ]
    started = time.monotonic()
    kickoff = run(kickoff_argv, cwd=run_dir, prompt=common_prompt)
    kickoff_elapsed = time.monotonic() - started
    kickoff_meta = parse_events(kickoff.stdout)
    parent = kickoff_meta["thread_id"]
    if not parent or "COMMON_PREFIX_READY" not in kickoff_last.read_text():
        raise RuntimeError("kickoff did not produce a persisted parent session and completion marker")
    if nonce not in kickoff_last.read_text():
        raise RuntimeError("kickoff did not retain the shared nonce")
    contract_hash = hashlib.sha256(contract_path.read_bytes()).hexdigest()
    contract_path.unlink()

    role_prompt = {
        "lead": """You are now the accountable lead child. Use the shared nonce from inherited history;
do not search outside `lead/`. In `lead/`, create `sentinel.txt` with exactly two lines:
`role=lead` and `shared_nonce=<the inherited nonce>`. Commit it with message `w01 lead sentinel`.
Do not touch `worker/`. End with `LEAD_CHILD_COMPLETE` and the commit SHA.
""",
        "worker": """You are now the bounded producer child. Use the shared nonce from inherited history;
do not search outside `worker/`. In `worker/`, create `sentinel.txt` with exactly two lines:
`role=worker` and `shared_nonce=<the inherited nonce>`. Commit it with message `w01 worker sentinel`.
Do not touch `lead/`. End with `WORKER_CHILD_COMPLETE` and the commit SHA.
""",
    }
    processes: dict[str, tuple[subprocess.Popen[str], float, Path]] = {}
    for role, model in (("lead", "gpt-5.6-sol"), ("worker", "gpt-5.6-terra")):
        last = run_dir / f"{role}-last.md"
        argv = codex_base(model)[:2] + ["fork"] + codex_base(model)[2:] + ["-o", str(last), str(parent), "-"]
        process = subprocess.Popen(
            argv,
            cwd=run_dir,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env=os.environ.copy(),
        )
        assert process.stdin is not None
        process.stdin.write(role_prompt[role])
        process.stdin.close()
        processes[role] = (process, time.monotonic(), last)

    children: dict[str, dict[str, object]] = {}
    for role, (process, role_started, last) in processes.items():
        assert process.stdout is not None and process.stderr is not None
        stdout = process.stdout.read()
        stderr = process.stderr.read()
        code = process.wait(timeout=900)
        if code:
            raise RuntimeError(f"{role} fork failed ({code}): {stderr[-2000:]}\n{stdout[-2000:]}")
        children[role] = {
            **parse_events(stdout),
            "elapsed_seconds": time.monotonic() - role_started,
            "last_message": last.read_text(),
            "stdout_sha256": hashlib.sha256(stdout.encode()).hexdigest(),
        }

    expected = {
        "lead": f"role=lead\nshared_nonce={nonce}\n",
        "worker": f"role=worker\nshared_nonce={nonce}\n",
    }
    repositories = {"lead": lead_repo, "worker": worker_repo}
    audit: dict[str, dict[str, object]] = {}
    for role, repo in repositories.items():
        changed = git_value(repo, "diff-tree", "--no-commit-id", "--name-only", "-r", "HEAD").splitlines()
        sentinel = repo / "sentinel.txt"
        audit[role] = {
            "sentinel_exact": sentinel.is_file() and sentinel.read_text() == expected[role],
            "changed_paths": changed,
            "only_role_artifact": changed == ["sentinel.txt"],
            "clean": git_value(repo, "status", "--porcelain") == "",
            "commit": git_value(repo, "rev-parse", "HEAD"),
            "commit_count": int(git_value(repo, "rev-list", "--count", "HEAD")),
        }

    checks = {
        "distinct_children": children["lead"]["thread_id"] not in {None, parent, children["worker"]["thread_id"]}
        and children["worker"]["thread_id"] not in {None, parent},
        "common_prefix_removed_before_children": not contract_path.exists(),
        "lead_retained_nonce": audit["lead"]["sentinel_exact"],
        "worker_retained_nonce": audit["worker"]["sentinel_exact"],
        "role_branches_isolated": all(item["only_role_artifact"] and item["clean"] for item in audit.values()),
        "one_child_commit_each": all(item["commit_count"] == 2 for item in audit.values()),
        "role_models_diverged": True,
    }
    proof = {
        "run": args.run_id,
        "mechanism": "W01 session mitosis",
        "passed": all(checks.values()),
        "checks": checks,
        "common": {
            "parent_thread_id": parent,
            "contract_sha256": contract_hash,
            "prompt_sha256": hashlib.sha256(common_prompt.encode()).hexdigest(),
            "elapsed_seconds": kickoff_elapsed,
            **kickoff_meta,
        },
        "children": children,
        "repositories": audit,
        "models": {"common": "gpt-5.6-sol", "lead": "gpt-5.6-sol", "worker": "gpt-5.6-terra"},
        "authority_note": "Both isolated children inherit the common run-root sandbox; branch separation is audited, not capability-enforced.",
    }
    (run_dir / "proof.json").write_text(json.dumps(proof, indent=2, sort_keys=True))
    print(json.dumps(proof, indent=2, sort_keys=True))
    raise SystemExit(0 if proof["passed"] else 1)


if __name__ == "__main__":
    main()
