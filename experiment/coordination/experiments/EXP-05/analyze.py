#!/usr/bin/env python3
"""Apply EXP-05's frozen sparse gates to preserved product-run evidence."""

from __future__ import annotations

import argparse
import json
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parent
RESULTS = ROOT / "results"
CATALOG = json.loads((ROOT / "arm-catalog.json").read_text())
CONTRACT = json.loads((ROOT / "frozen-contract.json").read_text())


def read(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text())


def load_runs() -> tuple[dict[str, list[dict[str, Any]]], list[dict[str, Any]]]:
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    malformed: list[dict[str, Any]] = []
    for path in sorted(RESULTS.glob("*/run-result.json")):
        try:
            row = read(path)
        except (OSError, json.JSONDecodeError) as error:
            malformed.append({"path": str(path.relative_to(ROOT)), "error": str(error)})
            continue
        row["_path"] = str(path.relative_to(ROOT))
        arm = row.get("arm")
        if arm:
            grouped[str(arm)].append(row)
    for path in sorted(RESULTS.glob("*/run-failure.json")):
        # A finalization observer can fail after productive work is already
        # durable, then recover the same cell without replaying it. Its later
        # run-result is the cell disposition; preserve but do not double-count
        # the superseded observer diagnostic beside it.
        if (path.parent / "run-result.json").exists():
            continue
        try:
            row = read(path)
        except (OSError, json.JSONDecodeError) as error:
            malformed.append({"path": str(path.relative_to(ROOT)), "error": str(error)})
            continue
        row["_path"] = str(path.relative_to(ROOT))
        arm = row.get("arm")
        if arm:
            grouped[str(arm)].append(row)
        else:
            malformed.append({"path": row["_path"], "error": "failure evidence has no arm"})
    return grouped, malformed


def counted(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    return [row for row in rows if row.get("validity") == "counted"]


def resolved(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Terminal protocol dispositions include outcomes and explicitly stopped branches."""
    return [
        row
        for row in rows
        if row.get("validity")
        in {
            "counted",
            "counted_outcome_failure",
            "branch_stopped_infrastructure_invalid",
        }
    ]


def quality(row: dict[str, Any]) -> dict[str, Any] | None:
    blind = row.get("blind_evaluation")
    if not isinstance(blind, dict) or not blind.get("valid"):
        return None
    judgement = blind.get("judgement")
    if not isinstance(judgement, dict):
        return None
    scores = judgement.get("scores")
    worst = judgement.get("worst_unit")
    if not isinstance(scores, dict) or not isinstance(worst, dict):
        return None
    return {
        "mean_score": sum(float(value) for value in scores.values()) / len(scores),
        "scores": scores,
        "worst_unit": worst,
        "high_consequence_breach": judgement.get("high_consequence_breach"),
        "consequential_defects": judgement.get("consequential_defects"),
        "decision": judgement.get("decision"),
    }


def ratio(candidate: float | None, baseline: float | None) -> float | None:
    if candidate is None or baseline in (None, 0):
        return None
    return candidate / baseline


def sales_pair_comparison(
    demand: str,
    q1: dict[str, Any],
    q2: dict[str, Any],
) -> dict[str, Any]:
    m1 = q1["metrics"]
    m2 = q2["metrics"]
    quality1 = quality(q1)
    quality2 = quality(q2)
    value_ratio = ratio(
        m2.get("value_adjusted_units_per_request_hour"),
        m1.get("value_adjusted_units_per_request_hour"),
    )
    p90_ratio = ratio(
        m2.get("unit_latency_seconds", {}).get("p90"),
        m1.get("unit_latency_seconds", {}).get("p90"),
    )
    cost_ratio = ratio(
        m2.get("estimated_list_cost_per_accepted_unit_usd"),
        m1.get("estimated_list_cost_per_accepted_unit_usd"),
    )
    if cost_ratio is None:
        cost_ratio = ratio(
            m2.get("observed_tokens", 0) / max(1, m2.get("accepted_units", 0)),
            m1.get("observed_tokens", 0) / max(1, m1.get("accepted_units", 0)),
        )
    quality_gate = bool(
        quality1
        and quality2
        and float(quality2["worst_unit"]["score"])
        >= float(quality1["worst_unit"]["score"])
        - CONTRACT["sales"]["worst_decile_tolerance_10"]
        and not quality2["high_consequence_breach"]
        and quality2["decision"] == "accept"
    )
    gates = {
        "value_adjusted_throughput_plus_25pct": value_ratio is not None and value_ratio >= 1.25,
        "p90_improves_20pct": p90_ratio is not None and p90_ratio <= 0.80,
        "tail_quality_preserved": quality_gate,
        "cost_per_unit_within_25pct": cost_ratio is not None and cost_ratio <= 1.25,
        "exact_local_closure": all(
            row["exact_evaluation"]["exact"].get("valid")
            and row["exact_evaluation"]["attribution_valid"]
            for row in (q1, q2)
        ),
        "lead_review_inside_span": all(
            row["metrics"].get("lead_review_seconds", float("inf"))
            <= CONTRACT["lead_span"]["exception_response_seconds"]
            for row in (q1, q2)
        ),
    }
    return {
        "demand": demand,
        "ready": True,
        "q1": {"path": q1["_path"], "metrics": m1, "quality": quality1},
        "q2": {"path": q2["_path"], "metrics": m2, "quality": quality2},
        "ratios": {
            "value_adjusted_throughput": value_ratio,
            "p90_latency": p90_ratio,
            "cost_per_accepted_unit": cost_ratio,
        },
        "gates": gates,
        "q2_crossover": all(gates.values()),
    }


def sales_comparison(demand: str, grouped: dict[str, list[dict[str, Any]]]) -> dict[str, Any]:
    prefix = demand.lower()
    q1_rows = counted(grouped.get(f"sales-{prefix}-q1-r1", []))
    q2_rows = counted(grouped.get(f"sales-{prefix}-q2-r1", []))
    if not q1_rows or not q2_rows:
        return {
            "demand": demand,
            "ready": False,
            "missing": [
                name
                for name, rows in (("q1", q1_rows), ("q2", q2_rows))
                if not rows
            ],
        }
    result = sales_pair_comparison(demand, q1_rows[0], q2_rows[0])
    result["replication"] = (
        sales_pair_comparison(demand, q1_rows[1], q2_rows[1])
        if len(q1_rows) >= 2 and len(q2_rows) >= 2
        else {
            "ready": False,
            "missing": [
                name
                for name, rows in (("q1", q1_rows), ("q2", q2_rows))
                if len(rows) < 2
            ],
        }
    )
    return result


def latest_gate(pattern: str) -> dict[str, Any] | None:
    paths = sorted(RESULTS.glob(pattern), key=lambda path: path.stat().st_mtime)
    return read(paths[-1]) if paths else None


def first_counted(grouped: dict[str, list[dict[str, Any]]], arm: str) -> dict[str, Any] | None:
    rows = counted(grouped.get(arm, []))
    return rows[0] if rows else None


def first_resolved(grouped: dict[str, list[dict[str, Any]]], arm: str) -> dict[str, Any] | None:
    rows = resolved(grouped.get(arm, []))
    return rows[0] if rows else None


def matched_comparison(
    grouped: dict[str, list[dict[str, Any]]],
    baseline_arm: str,
    candidate_arm: str,
) -> dict[str, Any]:
    baseline = first_resolved(grouped, baseline_arm)
    candidate = first_resolved(grouped, candidate_arm)
    if baseline is None or candidate is None:
        return {
            "ready": False,
            "missing": [
                name
                for name, row in ((baseline_arm, baseline), (candidate_arm, candidate))
                if row is None
            ],
        }
    base = baseline.get("metrics")
    cand = candidate.get("metrics")
    baseline_summary = {
        "arm": baseline_arm,
        "path": baseline["_path"],
        "metrics": base,
        "quality": quality(baseline),
        "outcome_failure": baseline.get("outcome_failure"),
    }
    candidate_summary = {
        "arm": candidate_arm,
        "path": candidate["_path"],
        "metrics": cand,
        "quality": quality(candidate),
        "outcome_failure": candidate.get("outcome_failure"),
    }
    if not isinstance(base, dict) or not isinstance(cand, dict):
        return {
            "ready": True,
            "baseline": baseline_summary,
            "candidate": candidate_summary,
            "ratios": None,
        }
    return {
        "ready": True,
        "baseline": baseline_summary,
        "candidate": candidate_summary,
        "ratios": {
            "accepted_throughput": ratio(
                cand.get("accepted_units_per_request_hour"),
                base.get("accepted_units_per_request_hour"),
            ),
            "p90_latency": ratio(
                cand.get("unit_latency_seconds", {}).get("p90"),
                base.get("unit_latency_seconds", {}).get("p90"),
            ),
            "estimated_cost_per_unit": ratio(
                cand.get("estimated_list_cost_per_accepted_unit_usd"),
                base.get("estimated_list_cost_per_accepted_unit_usd"),
            ),
        },
    }


def analyze() -> dict[str, Any]:
    grouped, malformed = load_runs()
    deterministic = (RESULTS / "wave0-deterministic-gates.json")
    g4 = read(deterministic) if deterministic.is_file() else None
    continuity = latest_gate("wave0-continuity-*/run-result.json")
    q4_rows = counted(grouped.get("wave0-q4-admission", []))
    wave0 = {
        "G1_G3_continuity": bool(continuity and continuity.get("valid")),
        "G2_sustained_q4": bool(
            q4_rows
            and q4_rows[-1]["metrics"].get("peak_staff_attempt_concurrency", 0) >= 4
        ),
        "G4_deterministic_local_closure": bool(g4 and g4.get("valid")),
    }
    sales = [sales_comparison(demand, grouped) for demand in ("D0", "D1", "D2")]
    elastic = {
        demand: (
            {
                "path": row["_path"],
                "metrics": row["metrics"],
                "quality": quality(row),
            }
            if (row := first_counted(grouped, f"sales-{demand.lower()}-elastic-r1"))
            else None
        )
        for demand in ("D1", "D2")
    }
    support = matched_comparison(
        grouped, "support-terminal-r1", "support-causal-r1"
    )
    if support.get("ready"):
        support["change"] = {
            "terminal": (support["baseline"].get("metrics") or {}).get("support_change"),
            "causal": (support["candidate"].get("metrics") or {}).get("support_change"),
        }
    monitoring = matched_comparison(
        grouped, "monitoring-q1-r1", "monitoring-q2-r1"
    )
    wave4_rows = resolved(grouped.get("company-q1x4-r1", []))
    wave4 = (
        {
            "ready": True,
            "path": wave4_rows[0]["_path"],
            "metrics": wave4_rows[0]["metrics"],
            "exact_evaluations": wave4_rows[0]["exact_evaluations"],
            "blind_evaluations": wave4_rows[0].get("blind_evaluations"),
            "validity": wave4_rows[0].get("validity"),
            "infrastructure_failure": wave4_rows[0].get("infrastructure_failure"),
        }
        if wave4_rows
        else {"ready": False}
    )
    crossovers = [row for row in sales if row.get("q2_crossover")]
    missing_base = [
        arm for arm in CATALOG["base_order"] if not resolved(grouped.get(arm, []))
    ]
    first_crossover = crossovers[0] if crossovers else None
    replication = first_crossover.get("replication") if first_crossover else None
    replication_complete = bool(replication and replication.get("ready"))
    replicated_crossover = bool(
        replication_complete and replication.get("q2_crossover")
    )
    conditional_q4_authorized = bool(
        not missing_base
        and wave0["G2_sustained_q4"]
        and replicated_crossover
    )
    if not wave0["G1_G3_continuity"]:
        next_action = "run Wave 0 G1+G3 continuity gate"
    elif not wave0["G2_sustained_q4"]:
        next_action = "run Wave 0 sustained-Q4 admission gate"
    elif missing_base:
        next_action = f"run next frozen base arm: {missing_base[0]}"
    elif first_crossover and not replication_complete:
        next_action = f"replicate first provisional crossover with reversed order: {first_crossover['demand']} Q1/Q2"
    elif conditional_q4_authorized:
        next_action = f"run conditional Q4 arm at replicated crossover: {first_crossover['demand']}"
    elif first_crossover:
        next_action = "compile bounded unreplicated crossover result; do not activate Q4 or a wildcard"
    else:
        next_action = "compile bounded no-crossover result; do not activate Q4 or a wildcard"
    return {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "wave0": wave0,
        "base_arms": {
            arm: {
                "counted": len(counted(grouped.get(arm, []))),
                "counted_outcome_failures": len(
                    [
                        row
                        for row in grouped.get(arm, [])
                        if row.get("validity") == "counted_outcome_failure"
                    ]
                ),
                "branch_stopped": len(
                    [
                        row
                        for row in grouped.get(arm, [])
                        if row.get("validity")
                        == "branch_stopped_infrastructure_invalid"
                    ]
                ),
                "invalid": len(grouped.get(arm, [])) - len(counted(grouped.get(arm, []))),
            }
            for arm in CATALOG["base_order"]
        },
        "sales_comparisons": sales,
        "sales_elastic_arms": elastic,
        "support_change_comparison": support,
        "monitoring_comparison": monitoring,
        "company_concurrency": wave4,
        "provisional_crossovers": [row["demand"] for row in crossovers],
        "first_crossover_replication_complete": replication_complete,
        "first_crossover_replicated": replicated_crossover,
        "conditional_q4_authorized": conditional_q4_authorized,
        "wildcard_authorized": False,
        "missing_base_arms": missing_base,
        "malformed_results": malformed,
        "next_action": next_action,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    result = analyze()
    if args.write:
        path = RESULTS / "program-status.json"
        path.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
