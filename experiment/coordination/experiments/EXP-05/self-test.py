#!/usr/bin/env python3
"""Falsify EXP-05's exact evaluator and order-independent local composition."""

from __future__ import annotations

import hashlib
import json
import random
import subprocess
import tempfile
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


def main() -> None:
    records = []
    with tempfile.TemporaryDirectory(prefix="restless-exp05-self-test-") as directory:
        base = Path(directory)
        for domain in ("sales", "support-v2", "monitoring", "operations"):
            domain_root = base / domain
            domain_root.mkdir()
            records.append(check_domain(domain_root, domain))
    print(json.dumps({"valid": True, "records": records}, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
