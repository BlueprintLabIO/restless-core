#!/usr/bin/env python3
"""Exact population verifier and deterministic local composer for EXP-05."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parent


def load_unit_files(paths: list[Path]) -> dict[str, list[dict[str, object]]]:
    files: dict[str, list[dict[str, object]]] = {}
    for path in paths:
        value = json.loads(path.read_text())
        if not isinstance(value, list):
            raise ValueError(f"{path}: expected a JSON array")
        if path.name in files:
            raise ValueError(f"duplicate output filename: {path.name}")
        files[path.name] = value
    return files


def fail(errors: list[str]) -> None:
    if errors:
        raise SystemExit(json.dumps({"valid": False, "errors": errors}, sort_keys=True))


def verify_sales(units: list[dict[str, object]], expected_ids: set[str] | None = None) -> list[str]:
    expected = {row["id"]: row for row in json.loads((ROOT / "hidden-evaluation/sales-expected.json").read_text())}
    population = set(expected) if expected_ids is None else expected_ids
    errors: list[str] = []
    if not population <= set(expected): errors.append("ownership contains unknown account ids")
    ids = [row.get("id") for row in units]
    if len(ids) != len(set(ids)): errors.append("duplicate account ids")
    if set(ids) != population: errors.append(f"coverage expected={len(population)} observed={len(set(ids))}")
    for row in units:
        target = expected.get(row.get("id"))
        if not target: continue
        for key in ("qualification", "disposition", "action_type", "follow_up_days", "claim_code"):
            if row.get(key) != target[key]: errors.append(f"{row.get('id')}:{key}")
        evidence = row.get("evidence") if isinstance(row.get("evidence"), list) else []
        for prefix in ("employees=", "fit_score=", "intent=", "region=", "consent_hold="):
            if not any(str(item).startswith(prefix) for item in evidence): errors.append(f"{row.get('id')}:evidence:{prefix}")
        if target["required_signal"] not in str(row.get("next_action", "")): errors.append(f"{row.get('id')}:personalization")
        for key in ("uncertainty", "follow_up_trigger"):
            if not str(row.get(key, "")).strip(): errors.append(f"{row.get('id')}:{key}")
    return errors


def verify_support(
    units: list[dict[str, object]],
    policy: int,
    expected_ids: set[str] | None = None,
) -> list[str]:
    expected_path = ROOT / f"hidden-evaluation/support-expected-v{policy}.json"
    expected = {row["id"]: row for row in json.loads(expected_path.read_text())}
    population = set(expected) if expected_ids is None else expected_ids
    errors: list[str] = []
    if not population <= set(expected): errors.append("ownership contains unknown case ids")
    ids = [row.get("id") for row in units]
    if len(ids) != len(set(ids)): errors.append("duplicate case ids")
    if set(ids) != population: errors.append(f"coverage expected={len(population)} observed={len(set(ids))}")
    for row in units:
        target = expected.get(row.get("id"))
        if not target: continue
        if row.get("policy_version") != policy: errors.append(f"{row.get('id')}:policy_version")
        if row.get("action") != target["action"]: errors.append(f"{row.get('id')}:action")
        for key in ("customer_safe_draft", "system_action_plan", "next_state", "evidence", "uncertainty"):
            if row.get(key) in (None, "", []): errors.append(f"{row.get('id')}:{key}")
    return errors


def verify_monitoring(units: list[dict[str, object]], expected_ids: set[str] | None = None) -> list[str]:
    expected = {row["entity"]: row for row in json.loads((ROOT / "hidden-evaluation/monitoring-expected.json").read_text())}
    population = set(expected) if expected_ids is None else expected_ids
    errors: list[str] = []
    if not population <= set(expected): errors.append("ownership contains unknown entity ids")
    ids = [row.get("entity") for row in units]
    if len(ids) != len(set(ids)): errors.append("duplicate entities")
    if set(ids) != population: errors.append(f"coverage expected={len(population)} observed={len(set(ids))}")
    for row in units:
        target = expected.get(row.get("entity"))
        if not target: continue
        for key in ("event_code", "severity", "follow_up_trigger"):
            if row.get(key) != target[key]: errors.append(f"{row.get('entity')}:{key}")
        if sorted(row.get("source_ids") or []) != sorted(target["source_ids"]): errors.append(f"{row.get('entity')}:source_ids")
        if not str(row.get("uncertainty", "")).strip(): errors.append(f"{row.get('entity')}:uncertainty")
    return errors


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("domain", choices=["sales", "support-v1", "support-v2", "monitoring", "operations"])
    parser.add_argument("files", nargs="+", type=Path)
    parser.add_argument("--index")
    parser.add_argument("--ownership", help="JSON object mapping output filename to exact owned unit ids")
    args = parser.parse_args()
    unit_files = load_unit_files(args.files)
    units = [unit for filename in sorted(unit_files) for unit in unit_files[filename]]
    key = "entity" if args.domain == "monitoring" else "id"
    units.sort(key=lambda row: str(row.get(key, "")))
    ownership_errors: list[str] = []
    expected_population: set[str] | None = None
    if args.ownership:
        ownership = json.loads(Path(args.ownership).read_text())
        if set(ownership) != set(unit_files):
            ownership_errors.append("ownership filenames do not match output filenames")
        for filename, rows in unit_files.items():
            observed = [row.get(key) for row in rows]
            expected_ids = ownership.get(filename, [])
            if len(observed) != len(set(observed)) or sorted(observed) != sorted(expected_ids):
                ownership_errors.append(f"{filename}:cross-unit ownership")
        expected_population = {
            str(unit_id)
            for expected_ids in ownership.values()
            for unit_id in expected_ids
        }
    if args.domain == "monitoring":
        errors = verify_monitoring(units, expected_population)
    elif args.domain.startswith("support-"):
        errors = verify_support(units, int(args.domain[-1]), expected_population)
    elif args.domain == "operations":
        expected = {row["id"]: row for row in json.loads((ROOT / "hidden-evaluation/operations-expected.json").read_text())}
        errors = []
        ids = [row.get("id") for row in units]
        if len(ids) != len(set(ids)): errors.append("duplicate invoice ids")
        population = set(expected) if expected_population is None else expected_population
        if not population <= set(expected): errors.append("ownership contains unknown invoice ids")
        if set(ids) != population: errors.append(f"coverage expected={len(population)} observed={len(set(ids))}")
        for row in units:
            target = expected.get(row.get("id"))
            if not target: continue
            for field in ("disposition", "action", "delta"):
                if row.get(field) != target[field]: errors.append(f"{row.get('id')}:{field}")
            for field in ("evidence", "uncertainty"):
                if row.get(field) in (None, "", []): errors.append(f"{row.get('id')}:{field}")
    else:
        errors = verify_sales(units, expected_population)
    fail(ownership_errors + errors)
    payload = (json.dumps(units, indent=2, sort_keys=True) + "\n").encode()
    if args.index:
        Path(args.index).write_bytes(payload)
    print(json.dumps({"valid": True, "domain": args.domain, "units": len(units), "sha256": hashlib.sha256(payload).hexdigest()}, sort_keys=True))


if __name__ == "__main__":
    main()
