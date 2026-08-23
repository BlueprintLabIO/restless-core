#!/usr/bin/env python3
"""Run one W01 common-prefix fork → work → artifact reunion experiment arm."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import time
from pathlib import Path

from probe_w01 import codex_base, git_value, parse_events, run
from runner import LabRun, MODE_LEAD, SEED, prepare, safe_name


HERE = Path(__file__).resolve().parent
WORK_ROOT = HERE / "workdir"


def write_result(run_dir: Path, name: str, result: subprocess.CompletedProcess[str]) -> dict[str, object]:
    (run_dir / f"{name}.jsonl").write_text(result.stdout)
    (run_dir / f"{name}.stderr").write_text(result.stderr)
    return {**parse_events(result.stdout), "stdout_sha256": hashlib.sha256(result.stdout.encode()).hexdigest()}


def run_fork(
    run_dir: Path,
    parent: str,
    role: str,
    model: str,
    prompt: str,
) -> tuple[subprocess.Popen[str], object, object, object, float]:
    prompt_path = run_dir / f"{role}-prompt.md"
    stdout_path = run_dir / f"{role}.jsonl"
    stderr_path = run_dir / f"{role}.stderr"
    last_path = run_dir / f"{role}-last.md"
    prompt_path.write_text(prompt)
    stdin = prompt_path.open()
    stdout = stdout_path.open("w")
    stderr = stderr_path.open("w")
    base = codex_base(model, "medium")
    argv = base[:2] + ["fork"] + base[2:] + ["-o", str(last_path), parent, "-"]
    process = subprocess.Popen(argv, cwd=run_dir, stdin=stdin, stdout=stdout, stderr=stderr, text=True, env=os.environ.copy())
    return process, stdin, stdout, stderr, time.monotonic()


def wait_fork(run_dir: Path, role: str, handles: tuple[subprocess.Popen[str], object, object, object, float]) -> dict[str, object]:
    process, stdin, stdout, stderr, started = handles
    code = process.wait(timeout=7200)
    stdin.close(); stdout.close(); stderr.close()
    raw = (run_dir / f"{role}.jsonl").read_text()
    error = (run_dir / f"{role}.stderr").read_text()
    if code:
        raise RuntimeError(f"{role} fork failed ({code}): {error[-3000:]}\n{raw[-3000:]}")
    return {
        **parse_events(raw),
        "elapsed_seconds": time.monotonic() - started,
        "stdout_sha256": hashlib.sha256(raw.encode()).hexdigest(),
        "last_message": (run_dir / f"{role}-last.md").read_text(),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("run_id")
    parser.add_argument("--scenario-file", required=True)
    parser.add_argument("--evaluator-file", required=True)
    args = parser.parse_args()
    scenario_path = Path(args.scenario_file).resolve()
    evaluator_path = Path(args.evaluator_file).resolve()
    if not scenario_path.is_file() or not evaluator_path.is_file():
        parser.error("scenario and evaluator must be files")
    run_dir = WORK_ROOT / args.run_id
    if run_dir.exists():
        raise SystemExit(f"refusing to overwrite existing W01 run: {run_dir}")

    prepare(
        args.run_id,
        mode=MODE_LEAD,
        lead_model="gpt-5.6-sol",
        worker_pool=[],
        spend_ceiling_usd=6,
        wall_clock_seconds=14_400,
        drain_grace_seconds=120,
        scenario_text=scenario_path.read_text(),
        team_worker_actor="world-content",
        evaluator_files=[],
        actor_max_time="none",
    )
    canonical = run_dir / "canonical"
    worker = run_dir / "worker"
    run(["git", "clone", "--no-local", str(canonical), str(worker)], cwd=run_dir)
    run(["git", "-C", str(worker), "checkout", "-b", f"w01/{safe_name(args.run_id)}/worker", SEED], cwd=run_dir)
    run(["git", "-C", str(worker), "config", "user.name", "Restless world-content"], cwd=run_dir)
    run(["git", "-C", str(worker), "config", "user.email", "world-content@restless.invalid"], cwd=run_dir)
    run(["git", "-C", str(canonical), "config", "user.name", "Restless studio-lead"], cwd=run_dir)
    run(["git", "-C", str(canonical), "config", "user.email", "studio-lead@restless.invalid"], cwd=run_dir)

    lab = LabRun(args.run_id)
    try:
        cell = lab.coordinator.workspaces.ensure_coordination_cell("studio-lead", read_only=False)
    finally:
        lab.coordinator.close()

    scenario_hash = hashlib.sha256(scenario_path.read_bytes()).hexdigest()
    kickoff_prompt = f"""# W01 common kickoff — inspection and seam agreement only

The company Exec delegated the owner outcome and immediately returned to availability. You are forming
one accountable lead plus one bounded producer below it. Read `context/scenario.md` completely and
inspect the exact seed in `canonical/`. The identical seed in `worker/` is the future producer branch.

Do not edit or commit anything in this kickoff. Build one shared causal model of the current game,
identify the smallest independently useful pure cavern-scene seam for the producer, and reserve runtime
integration, input, HUD, battle return, native proof and final judgement for the lead. Name exact files,
interfaces, coordinate/state risks and verification obligations. The frozen task hash is
`{scenario_hash}`. End with `COMMON_KICKOFF_COMPLETE`.
"""
    kickoff_last = run_dir / "kickoff-last.md"
    kickoff_argv = codex_base("gpt-5.6-sol", "medium") + [
        "-C", str(run_dir), "--sandbox", "workspace-write", "-o", str(kickoff_last), "-"
    ]
    overall_started = time.monotonic()
    kickoff_started = time.monotonic()
    kickoff_result = run(kickoff_argv, cwd=run_dir, prompt=kickoff_prompt, timeout=3600)
    kickoff_meta = write_result(run_dir, "kickoff", kickoff_result)
    kickoff_meta["elapsed_seconds"] = time.monotonic() - kickoff_started
    parent = kickoff_meta["thread_id"]
    if not parent or "COMMON_KICKOFF_COMPLETE" not in kickoff_last.read_text():
        raise RuntimeError("common kickoff did not close with a persisted parent session")
    if git_value(canonical, "status", "--porcelain") or git_value(worker, "status", "--porcelain"):
        raise RuntimeError("common kickoff mutated a product branch")

    lead_prompt = f"""# W01 lead child

You are the accountable Sol lead forked from the completed shared kickoff. Work only in `canonical/`.
Retain the whole product model and implement the runtime integration, input/HUD state, battle/return
coherence, snapshot truth and focused native proof. The Terra producer simultaneously owns the pure
cavern scene-builder seam agreed in shared history on `worker/`; do not edit that module or poll the
producer. It is acceptable for your first-phase branch to await that exact module. Make complementary
progress, commit your lead-owned work cleanly, and end `LEAD_PHASE_COMPLETE`. The hidden evaluator is
not present. Prepared native checks can be run with `{HERE / 'native_check.py'} {cell} {canonical} <file>`.
"""
    worker_prompt = """# W01 bounded producer child

You are the Terra producer forked from the completed shared kickoff. Work only in `worker/`. Implement
the independently useful pure cavern scene-builder seam agreed in shared history: cohesive primitive
geometry/material roles and semantic handles/state operations needed by the lead, without taking over
Game input, HUD, battle lifecycle, canonical integration or final product judgement. Add a focused
module-level proof if useful. Leave exactly one clean meaningful commit ahead of the seed and end
`WORKER_CHILD_COMPLETE` with the SHA. Do not inspect or touch `canonical/`.
"""
    fork_handles = {
        "lead": run_fork(run_dir, str(parent), "lead", "gpt-5.6-sol", lead_prompt),
        "worker": run_fork(run_dir, str(parent), "worker", "gpt-5.6-terra", worker_prompt),
    }
    lead_meta = wait_fork(run_dir, "lead", fork_handles["lead"])
    worker_meta = wait_fork(run_dir, "worker", fork_handles["worker"])
    if lead_meta["thread_id"] == worker_meta["thread_id"] or not lead_meta["thread_id"] or not worker_meta["thread_id"]:
        raise RuntimeError("fork lineage did not produce distinct child sessions")

    worker_commit = git_value(worker, "rev-parse", "HEAD")
    worker_clean = git_value(worker, "status", "--porcelain") == ""
    worker_advance = int(git_value(worker, "rev-list", "--count", f"{SEED}..HEAD"))
    reunion_prompt = f"""# W01 artifact reunion

The bounded producer child reached terminal process state. Its exact workspace is `{worker}` and exact
HEAD is `{worker_commit}`; clean={str(worker_clean).lower()}, commits-ahead={worker_advance}. Inspect the
actual diff and treat its narration as a claim. Fetch/cherry-pick or reject it by product judgement,
then finish the entire frozen owner outcome in `canonical/`. Resolve all coordinate/state interfaces,
run the combined native proof and unchanged regression suites, and leave exactly one clean meaningful
candidate commit ahead of `{SEED}` (squash experiment commits if needed). Do not seek owner help. End
with `W01_RUN_COMPLETE`, candidate SHA, and explicit `producer=accepted|repaired|rejected`. The hidden
external evaluator remains unavailable.
"""
    reunion_last = run_dir / "reunion-last.md"
    reunion_base = codex_base("gpt-5.6-sol", "medium")
    reunion_argv = reunion_base[:2] + ["resume"] + reunion_base[2:] + [
        "-o", str(reunion_last), str(lead_meta["thread_id"]), "-"
    ]
    reunion_started = time.monotonic()
    reunion_result = run(reunion_argv, cwd=run_dir, prompt=reunion_prompt, timeout=7200)
    reunion_meta = write_result(run_dir, "reunion", reunion_result)
    reunion_meta["elapsed_seconds"] = time.monotonic() - reunion_started

    candidate = git_value(canonical, "rev-parse", "HEAD")
    clean = git_value(canonical, "status", "--porcelain") == ""
    commits_ahead = int(git_value(canonical, "rev-list", "--count", f"{SEED}..HEAD"))
    changed = git_value(canonical, "diff", "--stat", f"{SEED}..HEAD")
    child_paths = {
        "lead": git_value(canonical, "diff", "--name-only", f"{SEED}..HEAD").splitlines(),
        "worker": git_value(worker, "diff", "--name-only", f"{SEED}..HEAD").splitlines(),
    }
    phases = {"kickoff": kickoff_meta, "lead": lead_meta, "worker": worker_meta, "reunion": reunion_meta}
    total_tokens = sum(
        int(meta.get("usage", {}).get("input_tokens", 0)) + int(meta.get("usage", {}).get("output_tokens", 0))
        for meta in phases.values()
    )
    cached_tokens = sum(int(meta.get("usage", {}).get("cached_input_tokens", 0)) for meta in phases.values())
    tool_calls = sum(int(meta.get("tool_calls", 0)) for meta in phases.values())
    protocol = {
        "valid": bool(clean and commits_ahead == 1 and candidate != SEED and worker_clean and worker_advance == 1),
        "required": "one common parent, distinct Sol/Terra forks, one producer commit, lead reunion, one clean candidate commit",
        "parent": parent,
        "lead_child": lead_meta["thread_id"],
        "worker_child": worker_meta["thread_id"],
        "worker_commit": worker_commit,
        "worker_clean": worker_clean,
        "worker_commits_ahead": worker_advance,
    }

    evaluation_dir = run_dir / "evaluation"
    evaluation_dir.mkdir(exist_ok=True)
    evaluator_name = safe_name(evaluator_path.name)
    shutil.copyfile(evaluator_path, evaluation_dir / evaluator_name)
    manifest_path = run_dir / "manifest.json"
    manifest = json.loads(manifest_path.read_text())
    manifest["experimental_mechanism"] = "w01_session_mitosis"
    manifest["declared_evaluators"] = [{
        "name": evaluator_name,
        "sha256": hashlib.sha256(evaluator_path.read_bytes()).hexdigest(),
    }]
    manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True))

    summary = {
        "run": args.run_id,
        "mode": "session_mitosis",
        "models": {"kickoff": "gpt-5.6-sol", "lead": "gpt-5.6-sol", "worker": "gpt-5.6-terra"},
        "scenario_sha256": scenario_hash,
        "evaluator_sha256": hashlib.sha256(evaluator_path.read_bytes()).hexdigest(),
        "wall_seconds": time.monotonic() - overall_started,
        "total_tokens": total_tokens,
        "cached_input_tokens": cached_tokens,
        "tool_calls": tool_calls,
        "phases": phases,
        "protocol": protocol,
        "candidate_evidence": {
            "candidate_commit": candidate,
            "checkout_clean": clean,
            "commits_ahead": commits_ahead,
            "changed_from_seed": changed,
            "changed_paths": child_paths,
        },
        "reunion_result": reunion_last.read_text(),
        "authority_note": "Both children inherited the isolated common run-root sandbox; branch separation was audited, not capability-enforced.",
    }
    (run_dir / "summary.json").write_text(json.dumps(summary, indent=2, sort_keys=True))
    print(json.dumps(summary, indent=2, sort_keys=True))
    raise SystemExit(0 if protocol["valid"] else 1)


if __name__ == "__main__":
    main()
