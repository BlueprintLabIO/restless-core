#!/usr/bin/env python3
"""Falsify EXP-05's exact evaluator and order-independent local composition."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import random
import subprocess
import sys
import tempfile
from datetime import datetime, timedelta, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parent


def write(path: Path, value: object) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def exact_units(domain: str) -> list[dict[str, object]]:
    if domain == "sales":
        expected = json.loads((ROOT / "hidden-evaluation/sales-expected.json").read_text())
        return [
            {
                **{key: row[key] for key in ("id", "qualification", "disposition", "action_type", "follow_up_days", "claim_code")},
                "evidence": ["employees=source", "fit_score=source", "intent=source", "region=source", "consent_hold=source"],
                "next_action": f"Use {row['required_signal']} in the unsent next action.",
                "uncertainty": "No uncertainty beyond the frozen dossier.",
                "follow_up_trigger": "Re-evaluate on a new verified account signal.",
            }
            for row in expected
        ]
    if domain.startswith("support-"):
        version = int(domain[-1])
        expected = json.loads((ROOT / f"hidden-evaluation/support-expected-v{version}.json").read_text())
        return [
            {
                "id": row["id"],
                "policy_version": version,
                "action": row["action"],
                "customer_safe_draft": "Prepared fictional response; not sent.",
                "system_action_plan": "Prepared fictional system step; not applied.",
                "next_state": "prepared-for-review",
                "evidence": ["frozen-case", f"policy-v{version}"],
                "uncertainty": "No external customer state was observed.",
            }
            for row in expected
        ]
    if domain == "monitoring":
        expected = json.loads((ROOT / "hidden-evaluation/monitoring-expected.json").read_text())
        return [{**row, "uncertainty": "Authoritative sources agree; future change remains possible."} for row in expected]
    if domain == "operations":
        expected = json.loads((ROOT / "hidden-evaluation/operations-expected.json").read_text())
        return [{**row, "evidence": ["invoice_total", "ledger_total"], "uncertainty": "No payment was applied."} for row in expected]
    raise ValueError(domain)


def check_domain(root: Path, domain: str) -> dict[str, object]:
    key = "entity" if domain == "monitoring" else "id"
    units = exact_units(domain)
    buckets = [units[index::4] for index in range(4)]
    outputs: list[Path] = []
    ownership: dict[str, list[object]] = {}
    for index, bucket in enumerate(buckets, start=1):
        path = root / f"worker-{index}.json"
        write(path, bucket)
        outputs.append(path)
        ownership[path.name] = [row[key] for row in bucket]
    ownership_path = root / "ownership.json"
    write(ownership_path, ownership)

    hashes: set[str] = set()
    for seed in range(20):
        ordered = outputs.copy()
        random.Random(seed).shuffle(ordered)
        index_path = root / f"index-{seed}.json"
        result = subprocess.run(
            ["python3", str(ROOT / "verify.py"), domain, *map(str, ordered), "--ownership", str(ownership_path), "--index", str(index_path)],
            check=True,
            capture_output=True,
            text=True,
        )
        reported = json.loads(result.stdout)
        digest = hashlib.sha256(index_path.read_bytes()).hexdigest()
        if reported["sha256"] != digest:
            raise AssertionError(f"{domain}: verifier digest disagrees with written index")
        hashes.add(digest)
    if len(hashes) != 1:
        raise AssertionError(f"{domain}: completion order changed deterministic index")

    # Wave 4 deliberately evaluates a frozen subset of three domains. Its
    # ownership manifest is the exact population contract; hidden expectations
    # still judge every listed unit and must reject an invented identifier.
    subset = units[: max(1, len(units) // 4)]
    subset_path = root / "subset.json"
    subset_ownership = root / "subset-ownership.json"
    write(subset_path, subset)
    write(subset_ownership, {subset_path.name: [row[key] for row in subset]})
    subprocess.run(
        ["python3", str(ROOT / "verify.py"), domain, str(subset_path), "--ownership", str(subset_ownership)],
        check=True,
        capture_output=True,
        text=True,
    )
    write(subset_ownership, {subset_path.name: [*[row[key] for row in subset], "EXP05-UNKNOWN-ID"]})
    unknown = subprocess.run(
        ["python3", str(ROOT / "verify.py"), domain, str(subset_path), "--ownership", str(subset_ownership)],
        capture_output=True,
        text=True,
    )
    if unknown.returncode == 0 or "unknown" not in unknown.stderr:
        raise AssertionError(f"{domain}: unknown subset ownership was not rejected")

    # Preserve population while moving one unit into the wrong worker file.
    left = json.loads(outputs[0].read_text())
    right = json.loads(outputs[1].read_text())
    left[0], right[0] = right[0], left[0]
    write(outputs[0], left)
    write(outputs[1], right)
    rejected = subprocess.run(
        ["python3", str(ROOT / "verify.py"), domain, *map(str, outputs), "--ownership", str(ownership_path)],
        capture_output=True,
        text=True,
    )
    if rejected.returncode == 0 or "cross-unit ownership" not in rejected.stderr:
        raise AssertionError(f"{domain}: ownership mutation was not rejected")
    visible_sources: list[Path]
    visible_domain = domain
    if domain == "sales":
        visible_sources = sorted((ROOT / "fixtures/sales/runtime-inputs/D2").glob("*.json"))
    elif domain == "support-v2":
        visible_sources = sorted((ROOT / "fixtures/support/runtime-inputs").glob("*.json"))
    elif domain == "monitoring":
        visible_sources = sorted((ROOT / "fixtures/monitoring/runtime-inputs").glob("*.json"))
    else:
        visible_sources = [ROOT / "fixtures/operations/data/invoices.json"]
    unit_key = "entity" if domain == "monitoring" else "id"
    unit_map = {row[unit_key]: row for row in units}
    visible_checks = 0
    for index, source in enumerate(visible_sources, start=1):
        source_rows = json.loads(source.read_text())
        source_ids = sorted({row[unit_key] for row in source_rows})
        output_path = root / f"visible-{index:02d}.json"
        write(output_path, [unit_map[unit_id] for unit_id in source_ids])
        subprocess.run(
            ["python3", str(ROOT / "validate-visible.py"), visible_domain, str(source), str(output_path)],
            check=True,
            capture_output=True,
            text=True,
        )
        visible_checks += 1
    broken = json.loads(output_path.read_text())
    corruption_field = {
        "sales": "qualification",
        "support-v2": "action",
        "monitoring": "event_code",
        "operations": "delta",
    }[domain]
    broken[0][corruption_field] = "deliberately-wrong"
    write(output_path, broken)
    rejected_visible = subprocess.run(
        ["python3", str(ROOT / "validate-visible.py"), visible_domain, str(visible_sources[-1]), str(output_path)],
        capture_output=True,
        text=True,
    )
    if rejected_visible.returncode == 0:
        raise AssertionError(f"{domain}: visible gate accepted a corrupted outcome")
    return {
        "domain": domain,
        "units": len(units),
        "orders": 20,
        "index_sha256": next(iter(hashes)),
        "ownership_mutation_rejected": True,
        "visible_gate_checks": visible_checks,
        "visible_corruption_rejected": True,
        "wave4_subset_contract": True,
    }


def check_runner_contracts() -> dict[str, bool]:
    spec = importlib.util.spec_from_file_location("exp05_product_runner_test", ROOT / "product-runner.py")
    if spec is None or spec.loader is None:
        raise AssertionError("could not load product runner")
    runner = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = runner
    spec.loader.exec_module(runner)
    score_fields = (
        "usefulness", "grounding", "safe_actionability", "tail_handling",
        "uncertainty_calibration", "native_review_readiness",
    )
    good = {
        "scores": {name: 9 for name in score_fields},
        "worst_unit": {"id": "A001", "score": 8, "defect": "none"},
        "high_consequence_breach": False,
        "consequential_defects": [],
        "evidence": ["exact artifact observation"],
        "decision": "accept",
    }
    if runner.validate_blind_judgement(good):
        raise AssertionError("valid frozen blind judgement was rejected")
    if not runner.validate_blind_judgement({**good, "decision": "maybe"}):
        raise AssertionError("invalid blind decision was accepted")
    start = datetime(2026, 1, 1, tzinfo=timezone.utc)
    receipts = [
        {
            "unit": f"sales batch {batch:02d}",
            "accepted_at": (start + timedelta(seconds=(batch - 1) * 45 + 300)).isoformat(),
            "arrival_offset_seconds": (batch - 1) * 45,
            "validator": {"units": 40},
        }
        for batch in range(1, 7)
    ]
    d0 = runner.sales_value_metrics(
        runner.Arm("d0", 1, "sales", "q1", 1, demand="D0"),
        receipts,
        start.timestamp(),
    )
    d2 = runner.sales_value_metrics(
        runner.Arm("d2", 1, "sales", "q1", 1, demand="D2"),
        receipts,
        start.timestamp(),
    )
    if d0["value_adjusted_units"] != 240:
        raise AssertionError("flat demand unexpectedly decayed")
    if not 0 < d2["value_adjusted_units"] < 240:
        raise AssertionError("service-pressure value curve did not decay")
    if runner.interval_summary([(0, 10), (5, 12), (20, 25)]) != {"summed": 22, "union": 17}:
        raise AssertionError("overlapping actor time was composed incorrectly")
    session_history = []
    event_id = 1
    for actor, start_second, end_second in (
        ("worker-1", 0, 10),
        ("worker-2", 1, 11),
        ("worker-3", 2, 12),
        ("worker-4", 3, 13),
    ):
        for kind, second in (("model_session_ready", start_second), ("turn_usage", end_second)):
            session_history.append({
                "id": event_id,
                "kind": kind,
                "actor_id": actor,
                "created_at": (start + timedelta(seconds=second)).isoformat(),
            })
            event_id += 1
    peak, unterminated = runner.peak_model_session_concurrency(
        list(reversed(session_history)),
        {"worker-1", "worker-2", "worker-3", "worker-4"},
    )
    if peak != 4 or unterminated != 0:
        raise AssertionError("actual model-session overlap was not measured exactly")
    session_history.pop()
    if runner.peak_model_session_concurrency(
        session_history, {"worker-1", "worker-2", "worker-3", "worker-4"}
    )[1] != 1:
        raise AssertionError("a missing terminal model-usage callback was not exposed")
    interrupted_history = [
        {
            "id": 1,
            "kind": "model_session_ready",
            "actor_id": "worker-1",
            "created_at": start.isoformat(),
        },
        {
            "id": 2,
            "kind": "attempt_process_ended",
            "actor_id": "worker-1",
            "created_at": (start + timedelta(seconds=3)).isoformat(),
        },
    ]
    if runner.peak_model_session_concurrency(
        interrupted_history, {"worker-1"}
    ) != (1, 0):
        raise AssertionError("an interrupted supervised process left a false open model session")
    review_history = [
        {"id": 1, "kind": "turn_usage", "actor_id": "lead", "created_at": (start + timedelta(seconds=10)).isoformat(), "body": {}},
        {"id": 2, "kind": "wake", "actor_id": "exec", "created_at": (start + timedelta(seconds=9)).isoformat(), "body": {"reason": "message from lead"}},
        {"id": 3, "kind": "wake_end", "actor_id": "exec", "created_at": (start + timedelta(seconds=12)).isoformat(), "body": {}},
    ]
    pair = runner.final_review_pair(list(reversed(review_history)), "lead", start.timestamp())
    if pair is None or pair[0]["kind"] != "turn_usage" or pair[1]["kind"] != "wake_end":
        raise AssertionError("material lead-to-Exec closure depended on cosmetic actor_wake_end")
    runner.experiment_spend = lambda: {
        "ceiling_usd": 100.0, "accounted_usd": 93.0,
        "committed_usd": 93.0, "unknown_reserve_usd": 0.0, "companies": []
    }
    try:
        runner.admit_program_cell(8)
    except runner.RunFailure:
        pass
    else:
        raise AssertionError("programme overspend was admitted")
    runner.experiment_spend = lambda: {
        "ceiling_usd": 100.0, "accounted_usd": 92.0,
        "committed_usd": 92.0, "unknown_reserve_usd": 0.0, "companies": []
    }
    if runner.admit_program_cell(8)["accounted_usd"] != 92.0:
        raise AssertionError("exact programme-boundary admission was rejected")
    runner.experiment_spend = lambda: {
        "ceiling_usd": 100.0, "accounted_usd": 1.0,
        "committed_usd": 8.0, "unknown_reserve_usd": 7.0, "companies": []
    }
    if runner.admit_program_cell(92)["committed_usd"] != 8.0:
        raise AssertionError("bounded unknown metering was not reserved at the company ceiling")
    actor_ids = {
        actor_id
        for domain in ("sales", "support", "monitoring", "capacity", "operations", "continuity")
        for actor_id in (
            runner.actor_ids(domain, 4)[0],
            *runner.actor_ids(domain, 4)[1],
        )
    } | {
        "account-owner", "case-owner", "monitoring-owner",
        "operations-direction", "reconciliation-owner", "continuity-control",
    }
    displays = [runner.actor_display(actor_id) for actor_id in sorted(actor_ids)]
    normalise = lambda value: "".join(character for character in value.lower() if character.isalnum())
    if len(displays) != len(set(displays)):
        raise AssertionError("frozen colleague identities are not unique")
    if any(normalise(actor_id) == normalise(display) for actor_id, display in zip(sorted(actor_ids), displays)):
        raise AssertionError("a frozen display repeats its machine actor id")
    retry_one = runner.company_name("wave0-continuity", "20260825-glm53-r1")
    retry_two = runner.company_name("wave0-continuity", "20260825-glm53-r2")
    if retry_one == retry_two or len(retry_one) > 63 or len(retry_two) > 63:
        raise AssertionError("counted retry company names are not unique and bounded")
    return {
        "actor_identities_are_product_valid": True,
        "blind_schema_rejects_drift": True,
        "flat_and_decaying_value_curves_separate": True,
        "overlap_math_is_exact": True,
        "model_session_overlap_is_not_attempt_overlap": True,
        "material_review_survives_missing_wake_end": True,
        "programme_spend_guard_is_fail_closed": True,
        "unknown_spend_reserves_company_ceiling": True,
        "retry_company_names_are_unique": True,
    }


def main() -> None:
    records = []
    with tempfile.TemporaryDirectory(prefix="restless-exp05-self-test-") as directory:
        base = Path(directory)
        for domain in ("sales", "support-v2", "monitoring", "operations"):
            domain_root = base / domain
            domain_root.mkdir()
            records.append(check_domain(domain_root, domain))
    print(
        json.dumps(
            {"valid": True, "records": records, "runner_contracts": check_runner_contracts()},
            indent=2,
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
