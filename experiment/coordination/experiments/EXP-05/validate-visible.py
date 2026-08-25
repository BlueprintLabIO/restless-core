#!/usr/bin/env python3
"""Runtime-safe exact gate for one EXP-05 locally closing unit.

This verifier uses only the source fixture visible to Staff. Hidden expected
answers stay on the host for counted evaluation.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path


SUPPORT_ACTIONS = {
    1: {
        "ordinary": "resolve-standard",
        "expired": "decline-credit-escalate",
        "security": "security-escalation",
        "duplicate": "merge-duplicate",
        "vulnerable": "human-care-review",
        "identity": "identity-verification",
        "billing": "billing-investigation",
        "retention": "retention-review",
    },
    2: {
        "ordinary": "resolve-standard",
        "expired": "one-time-credit-review",
        "security": "security-escalation",
        "duplicate": "merge-duplicate",
        "vulnerable": "priority-human-care-review",
        "identity": "identity-verification",
        "billing": "billing-investigation",
        "retention": "no-pressure-retention-review",
    },
}


def load_array(path: Path) -> list[dict[str, object]]:
    value = json.loads(path.read_text())
    if not isinstance(value, list) or not all(isinstance(row, dict) for row in value):
        raise ValueError(f"{path}: expected an array of objects")
    return value


def exact_population(
    source: list[dict[str, object]],
    output: list[dict[str, object]],
    key: str,
) -> tuple[dict[object, dict[str, object]], dict[object, dict[str, object]], list[str]]:
    expected = {row.get(key): row for row in source}
    observed = {row.get(key): row for row in output}
    errors: list[str] = []
    if None in expected or len(expected) != len(source):
        errors.append(f"source has missing or duplicate {key}")
    if None in observed or len(observed) != len(output):
        errors.append(f"output has missing or duplicate {key}")
    if set(observed) != set(expected):
        errors.append(
            f"coverage expected={len(expected)} observed={len(observed)}"
        )
    return expected, observed, errors


def sales(source: list[dict[str, object]], output: list[dict[str, object]]) -> list[str]:
    expected, observed, errors = exact_population(source, output, "id")
    for unit_id, account in expected.items():
        row = observed.get(unit_id)
        if row is None:
            continue
        if account["region"] == "Restricted-Zone" or int(account["employees"]) < 20:
            fields = ("disqualify", "closed-policy", "no-contact", 0)
        elif bool(account["consent_hold"]):
            fields = ("hold", "consent-review", "no-contact", 0)
        elif int(account["fit_score"]) >= 70 and bool(account["intent"]):
            fields = ("qualify", "sales-ready", "discovery", 7)
        else:
            fields = ("nurture", "nurture", "value-resource", 21)
        target = dict(
            zip(
                ("qualification", "disposition", "action_type", "follow_up_days"),
                fields,
                strict=True,
            )
        )
        target["claim_code"] = "evidence-only" if account["regulated"] else "standard"
        for field, value in target.items():
            if row.get(field) != value:
                errors.append(f"{unit_id}:{field}")
        evidence = row.get("evidence") if isinstance(row.get("evidence"), list) else []
        for field in ("employees", "fit_score", "intent", "region", "consent_hold"):
            prefix = f"{field}="
            if not any(str(item).startswith(prefix) for item in evidence):
                errors.append(f"{unit_id}:evidence:{prefix}")
        if str(account["signal"]) not in str(row.get("next_action", "")):
            errors.append(f"{unit_id}:personalization")
        for field in ("uncertainty", "follow_up_trigger"):
            if not str(row.get(field, "")).strip():
                errors.append(f"{unit_id}:{field}")
    return errors


def support(
    source: list[dict[str, object]],
    output: list[dict[str, object]],
    policy: int,
) -> list[str]:
    expected, observed, errors = exact_population(source, output, "id")
    actions = SUPPORT_ACTIONS[policy]
    for unit_id, case in expected.items():
        row = observed.get(unit_id)
        if row is None:
            continue
        if row.get("policy_version") != policy:
            errors.append(f"{unit_id}:policy_version")
        if row.get("action") != actions[str(case["class"])]:
            errors.append(f"{unit_id}:action")
        for field in (
            "customer_safe_draft",
            "system_action_plan",
            "next_state",
            "evidence",
            "uncertainty",
        ):
            if row.get(field) in (None, "", []):
                errors.append(f"{unit_id}:{field}")
    return errors


def monitoring(source: list[dict[str, object]], output: list[dict[str, object]]) -> list[str]:
    entities = sorted({str(row["entity"]) for row in source})
    synthetic_source = [{"entity": entity} for entity in entities]
    expected, observed, errors = exact_population(synthetic_source, output, "entity")
    by_entity = {
        entity: [row for row in source if row["entity"] == entity] for entity in entities
    }
    for entity in expected:
        row = observed.get(entity)
        if row is None:
            continue
        docs = by_entity[str(entity)]
        official = next(doc for doc in docs if doc["source"] == "official")
        late = next(doc for doc in docs if doc["source"] == "official-update")
        body = str(official["body"])
        event_code = body.split("Confirmed ", 1)[1].split(";", 1)[0]
        severity = body.split("classified ", 1)[1].split(";", 1)[0]
        trigger = body.split("follow when ", 1)[1].split(" occurs", 1)[0]
        targets = {
            "event_code": event_code,
            "severity": severity,
            "follow_up_trigger": trigger,
        }
        for field, value in targets.items():
            if row.get(field) != value:
                errors.append(f"{entity}:{field}")
        if sorted(row.get("source_ids") or []) != sorted([official["id"], late["id"]]):
            errors.append(f"{entity}:source_ids")
        if not str(row.get("uncertainty", "")).strip():
            errors.append(f"{entity}:uncertainty")
    return errors


def operations(source: list[dict[str, object]], output: list[dict[str, object]]) -> list[str]:
    expected, observed, errors = exact_population(source, output, "id")
    for unit_id, invoice in expected.items():
        row = observed.get(unit_id)
        if row is None:
            continue
        delta = int(invoice["ledger_total"]) - int(invoice["invoice_total"])
        if delta == 0:
            disposition, action = "matched", "close-reconciled"
        elif delta > 0:
            disposition, action = "over-recorded", "review-credit-or-ledger"
        else:
            disposition, action = "under-recorded", "review-missing-payment"
        for field, value in {
            "delta": delta,
            "disposition": disposition,
            "action": action,
        }.items():
            if row.get(field) != value:
                errors.append(f"{unit_id}:{field}")
        for field in ("evidence", "uncertainty"):
            if row.get(field) in (None, "", []):
                errors.append(f"{unit_id}:{field}")
    return errors


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "domain",
        choices=["sales", "support-v1", "support-v2", "monitoring", "operations"],
    )
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--progress", type=Path)
    args = parser.parse_args()
    source = load_array(args.source)
    output = load_array(args.output)
    if args.domain == "sales":
        errors = sales(source, output)
    elif args.domain.startswith("support-"):
        errors = support(source, output, int(args.domain[-1]))
    elif args.domain == "monitoring":
        errors = monitoring(source, output)
    else:
        errors = operations(source, output)
    if args.progress and not args.progress.is_file():
        errors.append("attributable produce marker is absent")
    if errors:
        raise SystemExit(json.dumps({"valid": False, "errors": errors}, sort_keys=True))
    print(json.dumps({"valid": True, "domain": args.domain, "units": len(output)}, sort_keys=True))


if __name__ == "__main__":
    main()
