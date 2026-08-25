#!/usr/bin/env python3
"""Fail closed until every EXP-05 deliverable has authoritative evidence."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[3]
RESULTS = ROOT / "results"
sys.path.insert(0, str(ROOT))
import analyze  # noqa: E402


def read(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text())


def check(name: str, passed: bool, evidence: str) -> dict[str, object]:
    return {"requirement": name, "passed": bool(passed), "evidence": evidence}


def latest(pattern: str) -> Path | None:
    paths = sorted(RESULTS.glob(pattern), key=lambda path: path.stat().st_mtime)
    return paths[-1] if paths else None


def contains(path: Path, text: str) -> bool:
    return path.is_file() and text in path.read_text()


def main() -> None:
    status = analyze.analyze()
    grouped, malformed = analyze.load_runs()
    checks: list[dict[str, object]] = []
    preflight_path = RESULTS / "wave0-preflight.json"
    preflight = read(preflight_path) if preflight_path.is_file() else {}
    checks.append(
        check(
            "exact frozen-model preflight",
            bool(
                preflight.get("valid")
                and preflight.get("exact_execution_probe") == "passed"
            ),
            str(preflight_path.relative_to(REPO)) if preflight else "missing",
        )
    )
    continuity_path = latest("wave0-continuity-*/run-result.json")
    continuity = read(continuity_path) if continuity_path else {}
    checks.append(
        check(
            "G1 and G3 live continuity/cancellation",
            bool(continuity.get("valid")),
            str(continuity_path.relative_to(REPO)) if continuity_path else "missing",
        )
    )
    q4 = analyze.counted(grouped.get("wave0-q4-admission", []))
    checks.append(
        check(
            "G2 sustained Q4 admission",
            bool(q4 and q4[-1]["metrics"].get("peak_staff_attempt_concurrency", 0) >= 4),
            q4[-1]["_path"] if q4 else "missing",
        )
    )
    deterministic_path = RESULTS / "wave0-deterministic-gates.json"
    deterministic = read(deterministic_path) if deterministic_path.is_file() else {}
    checks.append(
        check(
            "G4 deterministic local closure and evaluator falsification",
            bool(deterministic.get("valid")),
            str(deterministic_path.relative_to(REPO)) if deterministic else "missing",
        )
    )
    base_complete = not status["missing_base_arms"]
    checks.append(
        check(
            "all frozen base arms counted",
            base_complete,
            ", ".join(status["missing_base_arms"]) if not base_complete else "results/program-status.json",
        )
    )
    crossovers = status["provisional_crossovers"]
    replicated = True
    for demand in crossovers:
        prefix = demand.lower()
        replicated &= len(analyze.counted(grouped.get(f"sales-{prefix}-q1-r1", []))) >= 2
        replicated &= len(analyze.counted(grouped.get(f"sales-{prefix}-q2-r1", []))) >= 2
    checks.append(
        check(
            "first crossover replicated or bounded not-found",
            base_complete and (not crossovers or replicated),
            f"crossovers={crossovers}; replicated={replicated}",
        )
    )
    deliverables = {
        "sales demand/capacity result": ROOT / "t01-sales-demand-capacity.md",
        "customer-operations change result": ROOT / "t02-support-change.md",
        "monitoring breadth result": ROOT / "t03-monitoring-breadth.md",
        "concurrent company result": ROOT / "t04-company-concurrency.md",
        "demand/team-size guide": ROOT / "demand-team-size-guide.md",
        "supervisor-span guide": ROOT / "supervisor-span-guide.md",
        "fan-in and wildcard dispositions": ROOT / "fan-in-wildcard-dispositions.md",
        "final result and adapter disposition": ROOT / "t05-final-results.md",
    }
    for name, path in deliverables.items():
        required_text = "Experiment-only adapter disposition" if name.startswith("final result") else "EXP-05"
        checks.append(
            check(
                name,
                contains(path, required_text),
                str(path.relative_to(REPO)) if path.is_file() else "missing",
            )
        )
    systematic = [
        REPO / "experiment/coordination/CANON.md",
        REPO / "experiment/coordination/EVIDENCE.md",
        REPO / "experiment/coordination/PROGRAM.md",
        REPO / "experiment/coordination/REGISTRY.md",
    ]
    checks.append(
        check(
            "systematic knowledge base updated",
            all(contains(path, "EXP-05") for path in systematic),
            ", ".join(str(path.relative_to(REPO)) for path in systematic),
        )
    )
    counted_runs = [row for rows in grouped.values() for row in analyze.counted(rows)]
    telemetry_complete = bool(counted_runs) and all(
        isinstance(row.get("metrics", {}).get("configured_efforts"), list)
        and isinstance(row.get("metrics", {}).get("models"), list)
        and row.get("metrics", {}).get("unit_latency_seconds") is not None
        and row.get("metrics", {}).get("worker_active_seconds") is not None
        for row in counted_runs
    )
    checks.append(
        check(
            "effort, usage, latency and active-time evidence",
            telemetry_complete,
            f"counted_runs={len(counted_runs)}",
        )
    )
    spec = REPO / "experiment/exp-sprints/exp-sprint-05-demand-and-elastic-capacity.md"
    checks.append(
        check(
            "sprint marked complete only after evidence",
            contains(spec, "**Status:** Complete"),
            str(spec.relative_to(REPO)),
        )
    )
    result = {
        "complete": all(row["passed"] for row in checks) and not malformed,
        "checks": checks,
        "malformed_results": malformed,
    }
    print(json.dumps(result, indent=2, sort_keys=True))
    if not result["complete"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
