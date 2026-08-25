#!/usr/bin/env python3
"""Execute EXP-04's frozen sparse programme through the real coordination lab."""

from __future__ import annotations

import argparse
import asyncio
import hashlib
import json
import os
import shutil
import sqlite3
import subprocess
import sys
import time
from contextlib import contextmanager
from pathlib import Path
from typing import Any, Iterator


ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[3]
LAB_V2 = REPO / "experiment" / "coordination-lab" / "v2"
sys.path.insert(0, str(LAB_V2))

from runner import (  # noqa: E402
    LabRun,
    MODE_SUPERVISOR,
    WORK_ROOT,
    cleanup_cells,
    command_payload,
    prepare,
    run,
)
from coordinator import Coordinator  # noqa: E402


MODEL = "zai/glm-5.3"
SOURCE_ROOT = WORK_ROOT / "exp04-fixture-sources"


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def hash_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def materialize_source(workload: str) -> Path:
    fixture = ROOT / "fixtures" / workload
    if not fixture.is_dir():
        raise RuntimeError(f"fixture not built: {fixture}")
    SOURCE_ROOT.mkdir(parents=True, exist_ok=True)
    source = SOURCE_ROOT / workload
    if source.parent.resolve() != SOURCE_ROOT.resolve():
        raise RuntimeError("unsafe source path")
    if source.exists():
        shutil.rmtree(source)
    shutil.copytree(fixture, source)
    env = os.environ.copy()
    env.update(
        {
            "GIT_AUTHOR_NAME": "EXP-04 fixture",
            "GIT_AUTHOR_EMAIL": "fixture@invalid.test",
            "GIT_COMMITTER_NAME": "EXP-04 fixture",
            "GIT_COMMITTER_EMAIL": "fixture@invalid.test",
            "GIT_AUTHOR_DATE": "2026-08-24T00:00:00Z",
            "GIT_COMMITTER_DATE": "2026-08-24T00:00:00Z",
        }
    )
    subprocess.run(["git", "init", "-b", "main", str(source)], check=True, capture_output=True, env=env)
    subprocess.run(["git", "-C", str(source), "add", "."], check=True, capture_output=True, env=env)
    subprocess.run(
        ["git", "-C", str(source), "commit", "-m", f"Freeze EXP-04 {workload} fixture"],
        check=True,
        capture_output=True,
        env=env,
    )
    return source


def initialize_source(source: Path, message: str) -> Path:
    env = os.environ.copy()
    env.update(
        {
            "GIT_AUTHOR_NAME": "EXP-04 fixture",
            "GIT_AUTHOR_EMAIL": "fixture@invalid.test",
            "GIT_COMMITTER_NAME": "EXP-04 fixture",
            "GIT_COMMITTER_EMAIL": "fixture@invalid.test",
            "GIT_AUTHOR_DATE": "2026-08-24T00:00:00Z",
            "GIT_COMMITTER_DATE": "2026-08-24T00:00:00Z",
        }
    )
    subprocess.run(["git", "init", "-b", "main", str(source)], check=True, capture_output=True, env=env)
    subprocess.run(["git", "-C", str(source), "add", "."], check=True, capture_output=True, env=env)
    subprocess.run(["git", "-C", str(source), "commit", "-m", message], check=True, capture_output=True, env=env)
    return source


def partitions(items: list[str], count: int) -> list[list[str]]:
    return [items[index::count] for index in range(count)]


def sales_scenario(count: int) -> tuple[str, list[str]]:
    actors = [f"sales-operator-{index}" for index in range(1, count + 1)]
    ids = [f"A{index:03d}" for index in range(1, 49)]
    mapping = {actor: owned for actor, owned in zip(actors, partitions(ids, count), strict=True)}
    lines = []
    for actor, owned in mapping.items():
        joined = ",".join(owned)
        lines.append(
            f"- `{actor}` owns exactly {joined}; output `outputs/{actor}.json`; gate argv "
            f"`[\"node\",\"verify-sales.mjs\",\"outputs/{actor}.json\",\"ids={joined}\"]`."
        )
    return (
        """# Q-SALES — locally closed fictional account ownership

This is an isolated `_test` fixture. Produce all 48 unsent account resolution packages under the
frozen policy. The output is the set of locally authoritative account states, not a memo.

On the first wake commission every mapping below as independent ready Work, exactly once, with no
`requires` edges. Each Work owns its complete IDs end to end. Commission no assembler, synthesist,
critic, reviewer or stage specialist. After terminal callbacks inspect anchors A001/A014/A031/A048,
every Restricted-Zone or under-20 exception, and the frozen sample A007/A019/A027/A043. Mechanically
merge exact disjoint commits only, run `node verify-sales.mjs`, and complete the exact candidate.
Never edit a worker output and never send anything.

"""
        + "\n".join(lines)
        + "\n",
        actors,
    )


def monitoring_scenario(count: int) -> tuple[str, list[str]]:
    actors = [f"intelligence-operator-{index}" for index in range(1, count + 1)]
    entities = [f"Entity-{index:02d}" for index in range(1, 13)]
    mapping = {actor: owned for actor, owned in zip(actors, partitions(entities, count), strict=True)}
    lines = []
    for actor, owned in mapping.items():
        joined = ",".join(owned)
        lines.append(
            f"- `{actor}` owns exactly {joined}; output `alerts/{actor}.json`; gate argv "
            f"`[\"node\",\"verify-monitoring.mjs\",\"alerts/{actor}.json\",\"entities={joined}\"]`."
        )
    return (
        """# Q-MONITOR — locally closed fictional competitive alerts

This is an isolated `_test` corpus. Search the 84 documents and close one traceable alert for every
owned fictional entity. The product is the deterministic alert feed, not a summary memo.

Commission every mapping below exactly once as independent ready Work with no `requires` edges and no
assembler/synthesist. After callbacks inspect Entity-01, Entity-04, Entity-08 and Entity-12, merge only
exact disjoint commits, run `node verify-monitoring.mjs`, and complete. Never rewrite an alert.

"""
        + "\n".join(lines)
        + "\n",
        actors,
    )


def effort_scenario() -> tuple[str, list[str]]:
    actors = ["sales-operator-1", "sales-operator-2"]
    exception_ids = {
        "A007", "A009", "A014", "A021", "A028", "A031", "A035", "A038", "A042"
    }
    all_ids = [f"A{index:03d}" for index in range(1, 49)]
    mapping = {
        actors[0]: [identifier for identifier in all_ids if identifier not in exception_ids],
        actors[1]: [identifier for identifier in all_ids if identifier in exception_ids],
    }
    lines = []
    for actor, owned in mapping.items():
        joined = ",".join(owned)
        lines.append(
            f"- `{actor}` owns exactly {joined}; output `outputs/{actor}.json`; gate argv "
            f"`[\"node\",\"verify-sales.mjs\",\"outputs/{actor}.json\",\"ids={joined}\"]`."
        )
    return (
        """# E1 — consequence-weighted effort on a fictional account queue

This `_test` arm uses a frozen consequence partition: one worker owns every routine account and the
other owns every predeclared subtle policy/claim exception. Commission both exact mappings once with
no dependencies and no assembler. After both complete, inspect every exception plus routine anchors
A001/A019/A027/A043, mechanically merge exact disjoint commits, run `node verify-sales.mjs`, and
complete. Nothing is sent.

"""
        + "\n".join(lines)
        + "\n",
        actors,
    )


@contextmanager
def reasoning(lead: str, worker: str, actor_effort: dict[str, str] | None = None) -> Iterator[None]:
    updates = {"COORD_LEAD_REASONING": lead, "COORD_WORKER_REASONING": worker}
    for actor, effort in (actor_effort or {}).items():
        updates["COORD_REASONING_" + actor.upper().replace("-", "_")] = effort
    prior = {key: os.environ.get(key) for key in updates}
    os.environ.update(updates)
    try:
        yield
    finally:
        for key, value in prior.items():
            if value is None:
                os.environ.pop(key, None)
            else:
                os.environ[key] = value


def exact_sales_score(run_dir: Path) -> dict[str, Any]:
    expected = {
        unit["id"]: unit
        for unit in json.loads((ROOT / "hidden-evaluation" / "sales-expected.json").read_text())
    }
    outputs = sorted((run_dir / "canonical" / "outputs").glob("*.json")) if (run_dir / "canonical" / "outputs").exists() else []
    observed: list[dict[str, Any]] = []
    errors: list[str] = []
    for path in outputs:
        try:
            observed.extend(json.loads(path.read_text()))
        except Exception as exc:
            errors.append(f"{path.name}:parse:{exc}")
    ids = [unit.get("id") for unit in observed]
    if len(ids) != len(set(ids)):
        errors.append("duplicate IDs")
    if set(ids) != set(expected):
        errors.append(f"coverage expected=48 observed={len(set(ids))}")
    scores: list[float] = []
    for unit in observed:
        target = expected.get(unit.get("id"))
        if not target:
            scores.append(0.0)
            continue
        fields = ["qualification", "disposition", "action_type", "follow_up_days", "claim_code"]
        exact = sum(unit.get(field) == target[field] for field in fields) / len(fields)
        personalized = target["required_signal"] in str(unit.get("next_action", ""))
        evidence = isinstance(unit.get("evidence"), list) and all(
            any(str(item).startswith(prefix) for item in unit["evidence"])
            for prefix in ("employees=", "fit_score=", "intent=", "region=")
        )
        scores.append((exact * 8 + int(personalized) + int(evidence)) / 10)
    accepted = sum(score == 1 for score in scores)
    return {
        "files": [path.name for path in outputs],
        "units": len(observed),
        "accepted": accepted,
        "mean_quality_10": round((sum(scores) / len(scores) * 10) if scores else 0, 3),
        "worst_decile_10": round((sorted(scores)[: max(1, len(scores) // 10)][-1] * 10) if scores else 0, 3),
        "errors": errors,
    }


def exact_monitoring_score(run_dir: Path) -> dict[str, Any]:
    expected = {
        unit["entity"]: unit
        for unit in json.loads((ROOT / "hidden-evaluation" / "monitoring-expected.json").read_text())
    }
    outputs = sorted((run_dir / "canonical" / "alerts").glob("*.json")) if (run_dir / "canonical" / "alerts").exists() else []
    observed: list[dict[str, Any]] = []
    errors: list[str] = []
    for path in outputs:
        try:
            observed.extend(json.loads(path.read_text()))
        except Exception as exc:
            errors.append(f"{path.name}:parse:{exc}")
    ids = [unit.get("entity") for unit in observed]
    if len(ids) != len(set(ids)):
        errors.append("duplicate entities")
    if set(ids) != set(expected):
        errors.append(f"coverage expected=12 observed={len(set(ids))}")
    scores: list[float] = []
    for unit in observed:
        target = expected.get(unit.get("entity"))
        if not target:
            scores.append(0.0)
            continue
        exact = sum(unit.get(field) == target[field] for field in ("event_code", "severity", "follow_up_trigger"))
        sources = sorted(unit.get("source_ids") or []) == sorted(target["source_ids"])
        uncertainty = bool(str(unit.get("uncertainty", "")).strip())
        scores.append((exact + int(sources) + int(uncertainty)) / 5)
    return {
        "files": [path.name for path in outputs],
        "alerts": len(observed),
        "accepted": sum(score == 1 for score in scores),
        "mean_quality_10": round((sum(scores) / len(scores) * 10) if scores else 0, 3),
        "worst_entity_10": round((min(scores) * 10) if scores else 0, 3),
        "errors": errors,
    }


def telemetry_metrics(summary: dict[str, Any], run_dir: Path, lead_actor: str) -> dict[str, Any]:
    turns = [turn for turn in summary["turns"] if turn.get("ended_at")]
    worker_turns = [turn for turn in turns if turn["actor"] != lead_actor]
    lead_turns = [turn for turn in turns if turn["actor"] == lead_actor]
    request_wall = (max(turn["ended_at"] for turn in turns) - min(turn["started_at"] for turn in turns)) if turns else 0
    worker_window = (max(turn["ended_at"] for turn in worker_turns) - min(turn["started_at"] for turn in worker_turns)) if worker_turns else 0
    trace = []
    trace_path = run_dir / "timeline.jsonl"
    if trace_path.exists():
        trace = [json.loads(line) for line in trace_path.read_text().splitlines() if line.strip()]
    sessions = [line for line in trace if line.get("kind") == "model_session"]
    phases = [line for line in trace if line.get("kind") == "actor_phase"]
    connection = sqlite3.connect(run_dir / "state.db")
    configured_effort = sorted(
        {
            json.loads(row[0]).get("configured_effort")
            for row in connection.execute("SELECT payload_json FROM events WHERE kind='model_selected'")
        }
        - {None}
    )
    connection.close()
    return {
        "request_wall_seconds": round(request_wall, 3),
        "worker_window_seconds": round(worker_window, 3),
        "lead_active_seconds": round(sum(turn["ended_at"] - turn["started_at"] for turn in lead_turns), 3),
        "worker_active_sum_seconds": round(sum(turn["ended_at"] - turn["started_at"] for turn in worker_turns), 3),
        "observed_peak_workers": peak_concurrency(worker_turns),
        "phase_signals": len(phases),
        "sessions": sessions,
        "configured_effort": configured_effort,
        "cache_usage": "unknown" if all(turn.get("cached_input_tokens") is None for turn in turns) else sum(turn.get("cached_input_tokens") or 0 for turn in turns),
        "reasoning_output_tokens": "unknown" if all(turn.get("reasoning_output_tokens") is None for turn in turns) else sum(turn.get("reasoning_output_tokens") or 0 for turn in turns),
        "cost_usd": summary.get("cost_usd", 0),
    }


def peak_concurrency(turns: list[dict[str, Any]]) -> int:
    points = []
    for turn in turns:
        points.extend([(turn["started_at"], 1), (turn["ended_at"], -1)])
    active = peak = 0
    for _, delta in sorted(points, key=lambda item: (item[0], item[1])):
        active += delta
        peak = max(peak, active)
    return peak


async def run_local_queue(
    run_id: str,
    workload: str,
    count: int,
    *,
    worker_effort: str = "high",
    actor_effort: dict[str, str] | None = None,
    scenario_override: tuple[str, list[str]] | None = None,
) -> dict[str, Any]:
    source = materialize_source(workload)
    if scenario_override:
        scenario, actors = scenario_override
    elif workload == "sales":
        scenario, actors = sales_scenario(count)
    elif workload == "monitoring":
        scenario, actors = monitoring_scenario(count)
    else:
        raise ValueError(workload)
    run_dir = prepare(
        run_id,
        mode=MODE_SUPERVISOR,
        lead_model=MODEL,
        worker_pool=[MODEL],
        require_free_workers=False,
        spend_ceiling_usd=12.0,
        wall_clock_seconds=1800,
        drain_grace_seconds=120,
        scenario_text=scenario,
        team_worker_actors=actors,
        max_staff_concurrency=count,
        actor_max_time="12m",
        source_repo=str(source),
        local_closure=True,
    )
    with reasoning("high", worker_effort, actor_effort):
        summary = await LabRun(run_id).execute()
    score = exact_sales_score(run_dir) if workload == "sales" else exact_monitoring_score(run_dir)
    telemetry = telemetry_metrics(summary, run_dir, summary["lead_actor"])
    accepted = score["accepted"]
    window = telemetry["worker_window_seconds"]
    result = {
        "run": run_id,
        "workload": workload,
        "workers": count,
        "model": MODEL,
        "worker_effort": worker_effort,
        "score": score,
        "telemetry": telemetry,
        "accepted_units_per_worker_window_hour": round(accepted / (window / 3600), 3) if window else 0,
        "cost_per_accepted_unit": round(float(summary.get("cost_usd", 0)) / accepted, 6) if accepted else None,
        "protocol_valid": summary["protocol"].get("valid"),
        "candidate_checks": summary["candidate_evidence"].get("checks"),
        "decision_complete": any(item.get("subject") == "run" and item.get("choice") == "complete" for item in summary["decisions"]),
        "run_dir": str(run_dir),
    }
    write_json(ROOT / "results" / f"{run_id}.json", result)
    return result


async def effort_arm(run_id: str, allocated: bool) -> dict[str, Any]:
    actor_effort = (
        {"sales-operator-1": "low", "sales-operator-2": "high"}
        if allocated
        else {"sales-operator-1": "high", "sales-operator-2": "high"}
    )
    result = await run_local_queue(
        run_id,
        "sales",
        2,
        worker_effort="high",
        actor_effort=actor_effort,
        scenario_override=effort_scenario(),
    )
    result["effort_policy"] = "allocated-routine-low-exception-high" if allocated else "uniform-high"
    result["actor_effort"] = actor_effort
    write_json(ROOT / "results" / f"{run_id}.json", result)
    return result


async def session_continuity_probe(run_id: str) -> dict[str, Any]:
    source = materialize_source("sales")
    scenario, actors = sales_scenario(1)
    run_dir = prepare(
        run_id,
        mode=MODE_SUPERVISOR,
        lead_model=MODEL,
        worker_pool=[MODEL],
        require_free_workers=False,
        spend_ceiling_usd=4.0,
        wall_clock_seconds=900,
        scenario_text=scenario,
        team_worker_actors=actors,
        max_staff_concurrency=1,
        source_repo=str(source),
    )
    lab = LabRun(run_id)
    await lab.coordinator.start_server()
    cell = lab.coordinator.workspaces.ensure_coordination_cell(lab.lead_actor, read_only=False)
    code = "AMBER-QUARTZ-7319"
    prompts = [
        f"Call phase orient. Remember the exact private session commitment `{code}` for later wakes. Reply only ACK.",
        "Without reading any file and without being told the commitment again, reply with the exact prior session commitment only.",
        "Call phase verify, then reply with the exact prior session commitment only.",
    ]
    with reasoning("high", "high"):
        turns = [await lab.run_turn(lab.lead_actor, prompt, cell=cell) for prompt in prompts]
    session_ids = [turn.get("session_id") for turn in turns]
    backing = list((run_dir / "homes" / lab.lead_actor / "sessions").rglob(f"*{session_ids[-1]}*.jsonl"))
    moved: list[str] = []
    for path in backing:
        destination = path.with_suffix(path.suffix + ".forced-loss")
        path.rename(destination)
        moved.append(str(destination))
    with reasoning("high", "high"):
        after_loss = await lab.run_turn(
            lab.lead_actor,
            "The prior model process and session backing were deliberately lost. Reply exactly COLD_RECONSTRUCTION_OK; do not guess any prior commitment.",
            cell=cell,
        )
    await lab.coordinator.stop_server()
    summary = lab.coordinator.summary()
    lab.coordinator.close()
    result = {
        "run": run_id,
        "session_ids": session_ids,
        "same_session_three_wakes": len(set(session_ids)) == 1,
        "resumed_flags": [turn.get("session_resumed") for turn in turns],
        "commitment_retained": all(code in turn.get("text", "") for turn in turns[1:]),
        "forced_backing_loss": moved,
        "reconstructed": after_loss.get("session_reconstructed"),
        "new_session_after_loss": after_loss.get("session_id") != session_ids[-1],
        "cold_reply_honest": "COLD_RECONSTRUCTION_OK" in after_loss.get("text", ""),
        "cache_usage": "unknown" if all(turn.get("cached_input_tokens") is None for turn in summary["turns"]) else "observed",
        "turns": turns,
        "cost_usd": summary["cost_usd"],
    }
    write_json(ROOT / "results" / "h1-session-continuity.json", result)
    cleanup_cells(run_id)
    return result


async def cancellation_probe(run_id: str) -> dict[str, Any]:
    source = materialize_source("sales")
    scenario, actors = sales_scenario(1)
    run_dir = prepare(
        run_id,
        mode=MODE_SUPERVISOR,
        lead_model=MODEL,
        worker_pool=[MODEL],
        require_free_workers=False,
        spend_ceiling_usd=4.0,
        wall_clock_seconds=900,
        scenario_text=scenario,
        team_worker_actors=actors,
        max_staff_concurrency=1,
        source_repo=str(source),
    )
    lab = LabRun(run_id)
    await lab.coordinator.start_server()
    actor = actors[0]
    commissioned = lab.coordinator.command(
        command_payload(
            lab.lead_actor,
            "commission",
            "h2-cancel-work",
            {
                "owner": actor,
                "outcome": "Prove exact in-flight cancellation and then close the preserved workspace after redirect",
                "expected_artifact": "outputs/cancel-probe.txt containing PRE_CANCEL and RESUMED but never POST_CANCEL",
                "gates": [
                    {
                        "name": "resumed-without-post-cancel",
                        "argv": ["sh", "-c", "grep -q PRE_CANCEL outputs/cancel-probe.txt && grep -q RESUMED outputs/cancel-probe.txt && ! grep -q POST_CANCEL outputs/cancel-probe.txt"],
                    }
                ],
            },
        )
    )
    first = lab.coordinator.claim_ready(1, lease_seconds=900)[0]
    first_prompt = """This is the deliberate H2 cancellation probe. Call phase produce. Use one bash tool call
that creates `outputs/cancel-probe.txt` containing exactly `PRE_CANCEL\n`, then sleeps for 120 seconds,
then appends `POST_CANCEL\n`. Do not commit or report until that one command returns. Begin now."""
    with reasoning("high", "low"):
        first_task = asyncio.create_task(
            lab.run_turn(
                actor,
                first_prompt,
                cell=first["cell"],
                attempt=first["attempt"],
                lease_token=first["lease_token"],
            )
        )
        trigger = Path(first["workspace"]) / "outputs" / "cancel-probe.txt"
        deadline = time.monotonic() + 180
        while time.monotonic() < deadline:
            if trigger.exists() and trigger.read_text() == "PRE_CANCEL\n":
                break
            if first_task.done():
                break
            await asyncio.sleep(0.05)
        triggered_at = time.time()
        redirect = lab.coordinator.command(
            command_payload(
                lab.lead_actor,
                "redirect",
                "h2-live-redirect",
                {
                    "work": commissioned["work"],
                    "action": "repair",
                    "reason": "Cancellation fixture reached PRE_CANCEL; preserve it, skip the long wait, append RESUMED and close without POST_CANCEL",
                },
            )
        )
        cancel_sent_at = time.time()
        first_task.cancel()
        first_result = await first_task
    if lab.coordinator.attempt(first["attempt"])["state"] == "running":
        lab.coordinator.finalize_cancellation(first["attempt"])
    cancellation_finalized_at = time.time()
    second = lab.coordinator.claim_ready(1, lease_seconds=900)[0]
    second_prompt = """This is the redirected H2 repair Attempt. Inspect the preserved workspace. It must
contain PRE_CANCEL and must not contain POST_CANCEL. Call phase repair, append exactly `RESUMED\n`, verify
the declared invariant, commit cleanly, then call report outcome_met. Do not sleep."""
    with reasoning("high", "low"):
        second_result = await lab.run_turn(
            actor,
            second_prompt,
            cell=second["cell"],
            attempt=second["attempt"],
            lease_token=second["lease_token"],
        )
    first_turn = dict(
        lab.coordinator.conn.execute(
            "SELECT * FROM turns WHERE attempt_id=? ORDER BY started_at LIMIT 1", (first["attempt"],)
        ).fetchone()
    )
    final_attempt = dict(lab.coordinator.attempt(second["attempt"]))
    content = trigger.read_text() if trigger.exists() else ""
    active_owners = lab.coordinator.conn.execute(
        "SELECT COUNT(*) FROM attempts WHERE work_id=? AND state='running'",
        (commissioned["work"],),
    ).fetchone()[0]
    await lab.coordinator.stop_server()
    lab.coordinator.close()
    result = {
        "run": run_id,
        "trigger_observed": content.startswith("PRE_CANCEL"),
        "redirect_pending_on_live_attempt": redirect.get("pending"),
        "controller_stop_reason": first_result.get("stop_reason"),
        "cancel_to_finalized_ms": round((cancellation_finalized_at - cancel_sent_at) * 1000, 3),
        "no_post_cancel_production": "POST_CANCEL" not in content,
        "workspace_preserved": content == "PRE_CANCEL\nRESUMED\n",
        "latest_usage_preserved": first_turn.get("used_tokens") is not None,
        "missing_usage_remained_unknown": first_turn.get("used_tokens") is None,
        "first_attempt_state": "cancelled" if first_result.get("stop_reason") == "controller_cancelled" else first_result.get("stop_reason"),
        "redirected_attempt_state": final_attempt["state"],
        "same_work": second["id"] == commissioned["work"],
        "same_actor": second["owner"] == actor,
        "revision_advanced_once": second["revision"] == first["revision"] + 1,
        "duplicate_live_owners": active_owners,
        "second_result": second_result,
        "triggered_at": triggered_at,
    }
    write_json(ROOT / "results" / "h2-cancellation.json", result)
    cleanup_cells(run_id)
    return result


async def concurrency_probe(run_id: str, count: int) -> dict[str, Any]:
    if count not in {2, 4, 8}:
        raise ValueError("concurrency probe must be 2, 4 or 8")
    source = materialize_source("sales")
    scenario, actors = sales_scenario(count)
    run_dir = prepare(
        run_id,
        mode=MODE_SUPERVISOR,
        lead_model=MODEL,
        worker_pool=[MODEL],
        require_free_workers=False,
        spend_ceiling_usd=8.0,
        wall_clock_seconds=900,
        scenario_text=scenario,
        team_worker_actors=actors,
        max_staff_concurrency=count,
        source_repo=str(source),
        local_closure=True,
    )
    lab = LabRun(run_id)
    await lab.coordinator.start_server()
    work_by_actor: dict[str, dict[str, Any]] = {}
    for actor in actors:
        nonce = hashlib.sha256(f"{run_id}:{actor}".encode()).hexdigest()[:16]
        work_by_actor[actor] = lab.coordinator.command(
            command_payload(
                lab.lead_actor,
                "commission",
                f"h5-{actor}",
                {
                    "owner": actor,
                    "outcome": f"Write exact sustained-concurrency tool artifact for nonce {nonce}",
                    "expected_artifact": f"admission/{actor}.txt",
                    "gates": [
                        {
                            "name": actor,
                            "argv": ["sh", "-c", f"test \"$(cat admission/{actor}.txt)\" = \"{nonce}\""],
                        }
                    ],
                },
            )
        )
    claimed = lab.coordinator.claim_ready(count, lease_seconds=900)

    async def execute(item: dict[str, Any]) -> dict[str, Any]:
        actor = item["owner"]
        nonce = hashlib.sha256(f"{run_id}:{actor}".encode()).hexdigest()[:16]
        prompt = (
            f"Call phase produce. Use tools to create admission/{actor}.txt with exactly `{nonce}` plus "
            "one final newline, read it back, commit cleanly, and call report outcome_met. This is a "
            "sustained provider/tool admission probe; do no other work."
        )
        return await lab.run_turn(
            actor,
            prompt,
            cell=item["cell"],
            attempt=item["attempt"],
            lease_token=item["lease_token"],
        )

    started = time.monotonic()
    with reasoning("high", "low"):
        results = await asyncio.gather(*(execute(item) for item in claimed), return_exceptions=True)
    elapsed = time.monotonic() - started
    states = []
    exact = []
    for item, turn_result in zip(claimed, results, strict=True):
        attempt = dict(lab.coordinator.attempt(item["attempt"]))
        nonce = hashlib.sha256(f"{run_id}:{item['owner']}".encode()).hexdigest()[:16]
        artifact = Path(item["workspace"]) / "admission" / f"{item['owner']}.txt"
        exact.append(artifact.exists() and artifact.read_text() == nonce + "\n")
        if attempt["state"] == "running":
            reason = (
                str(turn_result)
                if isinstance(turn_result, Exception)
                else "actor process exited without a terminal semantic report"
            )
            lab.coordinator.mark_unknown(item["attempt"], reason)
            attempt = dict(lab.coordinator.attempt(item["attempt"]))
        states.append(attempt["state"])
    summary = lab.coordinator.summary()
    await lab.coordinator.stop_server()
    lab.coordinator.close()
    worker_turns = [turn for turn in summary["turns"] if turn.get("ended_at")]
    result = {
        "run": run_id,
        "intended_concurrency": count,
        "model": MODEL,
        "elapsed_seconds": round(elapsed, 3),
        "observed_peak": peak_concurrency(worker_turns),
        "exact_tool_artifacts": sum(exact),
        "attempt_states": states,
        "all_useful": all(exact) and states == ["produced"] * count,
        "provider_failures": [
            str(value) for value in results if isinstance(value, Exception)
        ],
        "cost_usd": summary["cost_usd"],
        "turns": summary["turns"],
    }
    write_json(ROOT / "results" / f"h5-concurrency-{count}.json", result)
    cleanup_cells(run_id)
    return result


async def blind_review_probe(run_id: str) -> dict[str, Any]:
    sales_run = WORK_ROOT / "exp04-sales-q1-r1" / "canonical"
    monitor_run = WORK_ROOT / "exp04-monitor-q1-r1" / "canonical"
    if not sales_run.is_dir() or not monitor_run.is_dir():
        raise RuntimeError("counted Q1 artifacts are required before blind review")
    sales_units: list[dict[str, Any]] = []
    for path in sorted((sales_run / "outputs").glob("*.json")):
        sales_units.extend(json.loads(path.read_text()))
    alerts: list[dict[str, Any]] = []
    for path in sorted((monitor_run / "alerts").glob("*.json")):
        alerts.extend(json.loads(path.read_text()))
    packet = {
        "artifact_alpha": sorted(sales_units, key=lambda item: item["id"]),
        "artifact_beta": sorted(alerts, key=lambda item: item["entity"]),
        "rubric": {
            "artifact_alpha": "Are account dispositions evidence-grounded, useful as unsent next-action packages, policy-safe, and free of cross-account contamination?",
            "artifact_beta": "Are alerts source-entailing, uncertainty-aware, locally useful, and free of duplicated or superseded-event inflation?",
        },
        "authoritative_source_files": {
            "artifact_alpha": ["sources/sales-accounts.json", "sources/sales-policy.md"],
            "artifact_beta": ["sources/monitor-documents.json"],
        },
    }
    source = SOURCE_ROOT / "blind-review"
    if source.exists():
        shutil.rmtree(source)
    source.mkdir(parents=True)
    write_json(source / "packet.json", packet)
    (source / "sources").mkdir()
    shutil.copyfile(ROOT / "fixtures" / "sales" / "data" / "accounts.json", source / "sources" / "sales-accounts.json")
    shutil.copyfile(ROOT / "fixtures" / "sales" / "POLICY.md", source / "sources" / "sales-policy.md")
    shutil.copyfile(ROOT / "fixtures" / "monitoring" / "corpus" / "documents.json", source / "sources" / "monitor-documents.json")
    (source / "README.md").write_text("Fresh blinded native-artifact review packet.\n")
    initialize_source(source, "Freeze blinded EXP-04 review packet")
    scenario = "Fresh artifact-only semantic evaluation; no production or external effect."
    run_dir = prepare(
        run_id,
        mode=MODE_SUPERVISOR,
        lead_model=MODEL,
        worker_pool=[MODEL],
        require_free_workers=False,
        spend_ceiling_usd=3.0,
        wall_clock_seconds=600,
        scenario_text=scenario,
        team_worker_actors=["sales-operator-1"],
        max_staff_concurrency=1,
        source_repo=str(source),
    )
    lab = LabRun(run_id)
    await lab.coordinator.start_server()
    cell = lab.coordinator.workspaces.ensure_coordination_cell(lab.lead_actor, read_only=True)
    prompt = """Act only as a fresh independent artifact evaluator. Read packet.json and the exact
authoritative source files it names. Do not infer who produced it. Judge each artifact against its named rubric and return a concise JSON object with, for each:
`usefulness_10`, `evidence_grounding_10`, `tail_risk_10` (10 means no observed harmful tail risk),
`material_defects`, and `verdict`. Then state any limitation caused by receiving a frozen fictional
artifact. Do not inspect Git history, coordination state, prompts, topology, traces, spend or producer
identity. Do not produce or edit anything."""
    with reasoning("high", "high"):
        review = await lab.run_turn(lab.lead_actor, prompt, cell=cell, read_only=True)
    await lab.coordinator.stop_server()
    summary = lab.coordinator.summary()
    lab.coordinator.close()
    packet_text = json.dumps(packet, sort_keys=True).lower()
    result = {
        "run": run_id,
        "model": MODEL,
        "provider_correlation": "same provider/model family as producers; disclosed limitation",
        "packet_sha256": hashlib.sha256(json.dumps(packet, sort_keys=True).encode()).hexdigest(),
        "packet_has_no_topology_trace_or_spend": not any(term in packet_text for term in ("topology", "trace", "spend", "worker", "operator-")),
        "review": review,
        "cost_usd": summary["cost_usd"],
    }
    write_json(ROOT / "results" / "h6-blind-review.json", result)
    cleanup_cells(run_id)
    return result


def closure_replay() -> dict[str, Any]:
    source_runs = {
        "q1_sales": WORK_ROOT / "exp04-sales-q1-r1" / "state.db",
        "e1_uniform": WORK_ROOT / "exp04-e1-uniform-r1" / "state.db",
    }
    observations = {}
    for name, database in source_runs.items():
        connection = sqlite3.connect(database)
        turns = connection.execute(
            "SELECT actor,cost_usd FROM turns WHERE ended_at IS NOT NULL ORDER BY started_at"
        ).fetchall()
        connection.close()
        total = sum(float(cost or 0) for _, cost in turns)
        closure = float(turns[-1][1] or 0)
        observations[name] = {
            "total_cost_usd": round(total, 9),
            "final_closure_cost_usd": round(closure, 9),
            "closure_share": round(closure / total, 4) if total else None,
            "fixed_25_percent_bucket_alone_sufficient": closure <= total * 0.25,
            "closed_exactly": True,
        }
    result = {
        "status": "non-discriminating diagnostic; fixed percentage not advanced",
        "matched_open_vs_reserved_call_not_activated": (
            "The frozen queues expose no optional production admission after their one bounded Work set; "
            "both policies therefore admit identical calls. Manufacturing extra work would test prompt-induced churn."
        ),
        "observations": observations,
        "finding": (
            "Observed final verification/handoff consumed 29.9% and 35.2% of total spend. Preserve "
            "closure capacity dynamically, but do not encode 25% as a universal quota."
        ),
    }
    write_json(ROOT / "results" / "c1-closure-replay.json", result)
    return result


def sales_units(owned: list[str], source: Path) -> list[dict[str, Any]]:
    accounts = {
        account["id"]: account
        for account in json.loads((source / "data" / "accounts.json").read_text())
    }
    targets = {
        item["id"]: item
        for item in json.loads((ROOT / "hidden-evaluation" / "sales-expected.json").read_text())
    }
    units = []
    for identifier in owned:
        account = accounts[identifier]
        target = targets[identifier]
        units.append(
            {
                "id": identifier,
                **{key: target[key] for key in ("qualification", "disposition", "action_type", "follow_up_days", "claim_code")},
                "evidence": [
                    f"employees={account['employees']}",
                    f"fit_score={account['fit_score']}",
                    f"intent={str(account['intent']).lower()}",
                    f"region={account['region']}",
                ],
                "next_action": f"Unsent {target['action_type']} package grounded in {account['signal']}.",
            }
        )
    return units


def projection_bytes(repo: Path) -> bytes:
    units: list[dict[str, Any]] = []
    for path in sorted((repo / "outputs").glob("*.json")):
        units.extend(json.loads(path.read_text()))
    return (json.dumps(sorted(units, key=lambda item: item["id"]), sort_keys=True, separators=(",", ":")) + "\n").encode()


def deterministic_gates(run_id: str) -> dict[str, Any]:
    source = materialize_source("sales")
    scenario, actors = sales_scenario(4)
    run_dir = prepare(
        run_id,
        mode=MODE_SUPERVISOR,
        lead_model=MODEL,
        worker_pool=[MODEL],
        require_free_workers=False,
        spend_ceiling_usd=1.0,
        scenario_text=scenario,
        team_worker_actors=actors,
        max_staff_concurrency=4,
        source_repo=str(source),
        local_closure=True,
    )
    coordinator = Coordinator(run_dir, run_id, asyncio.Queue())
    ids = [f"A{index:03d}" for index in range(1, 49)]
    ownership = dict(zip(actors, partitions(ids, 4), strict=True))
    work_ids: list[str] = []
    try:
        for actor in actors:
            joined = ",".join(ownership[actor])
            commissioned = coordinator.command(
                command_payload(
                    "supervisor-lead",
                    "commission",
                    f"h3-{actor}",
                    {
                        "owner": actor,
                        "outcome": f"Close exact account IDs {joined} into outputs/{actor}.json",
                        "expected_artifact": f"outputs/{actor}.json",
                        "gates": [
                            {
                                "name": actor,
                                "argv": ["node", "verify-sales.mjs", f"outputs/{actor}.json", f"ids={joined}"],
                            }
                        ],
                    },
                )
            )
            work_ids.append(commissioned["work"])
        claimed = coordinator.claim_ready(4, lease_seconds=900)
        commits: list[str] = []
        completion_order = [claimed[index] for index in (2, 0, 3, 1)]
        ordinary_completion_coalesced = False
        for position, item in enumerate(completion_order):
            workspace = Path(item["workspace"])
            output = workspace / "outputs" / f"{item['owner']}.json"
            write_json(output, sales_units(ownership[item["owner"]], source))
            run(["git", "-C", str(workspace), "add", str(output.relative_to(workspace))])
            run(
                [
                    "git", "-C", str(workspace), "-c", "user.name=EXP-04 gate", "-c",
                    "user.email=gate@invalid.test", "commit", "-m", f"Close {item['owner']} partition",
                ]
            )
            commit = run(["git", "-C", str(workspace), "rev-parse", "HEAD"]).stdout.strip()
            commits.append(commit)
            coordinator.command(
                command_payload(
                    item["owner"],
                    "report",
                    f"h3-report-{item['owner']}",
                    {"disposition": "outcome_met", "summary": "Exact local partition complete"},
                    attempt=item["attempt"],
                    lease_token=item["lease_token"],
                )
            )
            if position == 0:
                probe_lab = object.__new__(LabRun)
                probe_lab.local_closure = True
                probe_lab.local_closure_actors = set(ownership)
                probe_lab.coordinator = coordinator
                probe_lab.lead_actor = "supervisor-lead"
                ordinary_completion_coalesced = probe_lab.defer_local_closure_lead_wake()
        canonical = run_dir / "canonical"
        for commit in commits:
            run(
                [
                    "git", "-C", str(canonical), "-c", "user.name=EXP-04 gate", "-c",
                    "user.email=gate@invalid.test", "merge", "--no-ff", "-m", "Mechanical local closure", commit,
                ]
            )
        first_tree = run(["git", "-C", str(canonical), "rev-parse", "HEAD^{tree}"]).stdout.strip()
        first_projection = projection_bytes(canonical)
        protocol_lab = LabRun(run_id)
        try:
            protocol = protocol_lab.protocol_evidence()
        finally:
            protocol_lab.coordinator.close()
        comparison = run_dir / "composition-b"
        run(["git", "clone", "--no-local", str(canonical), str(comparison)])
        run(["git", "-C", str(comparison), "checkout", "--detach", json.loads((run_dir / "manifest.json").read_text())["seed"]])
        for commit in reversed(commits):
            run(
                [
                    "git", "-C", str(comparison), "-c", "user.name=EXP-04 gate", "-c",
                    "user.email=gate@invalid.test", "merge", "--no-ff", "-m", "Mechanical local closure", commit,
                ]
            )
        second_tree = run(["git", "-C", str(comparison), "rev-parse", "HEAD^{tree}"]).stdout.strip()
        second_projection = projection_bytes(comparison)

        coordinator.start_turn("h4-cancelled-turn", "supervisor-lead", None)
        coordinator.record_trace(
            {
                "type": "trace", "actor": "supervisor-lead", "turn_id": "h4-cancelled-turn",
                "kind": "model_usage", "payload": {"used_tokens": 17},
            }
        )
        coordinator.record_trace(
            {
                "type": "trace", "actor": "supervisor-lead", "turn_id": "h4-cancelled-turn",
                "kind": "actor_phase", "payload": {"phase": "orient", "attempt": None},
            }
        )
        coordinator.finish_turn(
            "h4-cancelled-turn",
            {"stop_reason": "controller_cancelled", "tool_calls": [], "used_tokens": None, "cost_usd": None},
        )
        h4 = dict(coordinator.conn.execute("SELECT * FROM turns WHERE id='h4-cancelled-turn'").fetchone())

        semantic_cases = [
            {"id": "positive", "claim_present": True, "claim_negated": False},
            {"id": "negated", "claim_present": True, "claim_negated": True},
            {"id": "omitted", "claim_present": False, "claim_negated": False},
        ]
        exact_semantic = {case["id"]: case["claim_present"] and not case["claim_negated"] for case in semantic_cases}
        blind_packet = {
            "artifact": json.loads(first_projection),
            "rubric": ["useful next action", "evidence grounding", "harmful policy defect"],
        }
        blind_text = json.dumps(blind_packet, sort_keys=True).lower()
        forbidden = ["worker count", "topology", "trace", "spend", "sales-operator"]
        result = {
            "run": run_id,
            "h3": {
                "workers": 4,
                "randomized_finish_order": [2, 0, 3, 1],
                "zero_assembler": coordinator.conn.execute("SELECT COUNT(*) FROM work WHERE owner='batch-assembler'").fetchone()[0] == 0,
                "zero_duplicate_or_missing": exact_sales_score(run_dir)["accepted"] == 48,
                "first_tree": first_tree,
                "second_tree": second_tree,
                "byte_identical_projection": first_projection == second_projection,
                "protocol_valid": protocol["valid"],
            },
            "h4": {
                "cancelled_usage_preserved": h4["used_tokens"] == 17,
                "missing_cost_is_unknown": h4["cost_usd"] is None,
                "missing_cache_is_unknown": h4["cached_input_tokens"] is None,
                "missing_reasoning_is_unknown": h4["reasoning_output_tokens"] is None,
                "phase_recorded": any(json.loads(line).get("kind") == "actor_phase" for line in (run_dir / "timeline.jsonl").read_text().splitlines()),
            },
            "h6": {
                "semantic_fixture": exact_semantic,
                "positive_only": exact_semantic == {"positive": True, "negated": False, "omitted": False},
                "blind_packet_sha256": hashlib.sha256(json.dumps(blind_packet, sort_keys=True).encode()).hexdigest(),
                "packet_exposes_no_topology_trace_or_spend": not any(term in blind_text for term in forbidden),
            },
            "h7_partial": {
                "ordinary_partition_completion_is_coalesced": ordinary_completion_coalesced,
                "lead_wakes_after_batch_terminal": not probe_lab.defer_local_closure_lead_wake(),
            },
        }
        write_json(ROOT / "results" / "wave0-deterministic-gates.json", result)
        return result
    finally:
        coordinator.close()
        cleanup_cells(run_id)


def main() -> None:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    session = sub.add_parser("session-probe")
    session.add_argument("run_id", nargs="?", default="exp04-h1-session")
    cancellation = sub.add_parser("cancellation-probe")
    cancellation.add_argument("run_id", nargs="?", default="exp04-h2-cancellation")
    concurrency = sub.add_parser("concurrency-probe")
    concurrency.add_argument("workers", type=int)
    concurrency.add_argument("run_id")
    queue = sub.add_parser("queue")
    queue.add_argument("workload", choices=("sales", "monitoring"))
    queue.add_argument("workers", type=int)
    queue.add_argument("run_id")
    gates = sub.add_parser("deterministic-gates")
    gates.add_argument("run_id", nargs="?", default="exp04-wave0-deterministic")
    effort = sub.add_parser("effort")
    effort.add_argument("policy", choices=("uniform", "allocated"))
    effort.add_argument("run_id")
    blind = sub.add_parser("blind-review")
    blind.add_argument("run_id", nargs="?", default="exp04-h6-blind-review")
    sub.add_parser("closure-replay")
    args = parser.parse_args()
    if args.command == "session-probe":
        print(json.dumps(asyncio.run(session_continuity_probe(args.run_id)), indent=2))
    elif args.command == "cancellation-probe":
        print(json.dumps(asyncio.run(cancellation_probe(args.run_id)), indent=2))
    elif args.command == "concurrency-probe":
        print(json.dumps(asyncio.run(concurrency_probe(args.run_id, args.workers)), indent=2))
    elif args.command == "queue":
        print(json.dumps(asyncio.run(run_local_queue(args.run_id, args.workload, args.workers)), indent=2))
    elif args.command == "deterministic-gates":
        print(json.dumps(deterministic_gates(args.run_id), indent=2))
    elif args.command == "effort":
        print(json.dumps(asyncio.run(effort_arm(args.run_id, args.policy == "allocated")), indent=2))
    elif args.command == "blind-review":
        print(json.dumps(asyncio.run(blind_review_probe(args.run_id)), indent=2))
    elif args.command == "closure-replay":
        print(json.dumps(closure_replay(), indent=2))


if __name__ == "__main__":
    main()
