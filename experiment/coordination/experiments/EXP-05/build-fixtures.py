#!/usr/bin/env python3
"""Build the frozen fictional fixtures for EXP-05 before any counted model call."""

from __future__ import annotations

import hashlib
import json
import shutil
from pathlib import Path


ROOT = Path(__file__).resolve().parent
FIXTURES = ROOT / "fixtures"
HIDDEN = ROOT / "hidden-evaluation"


def write(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if isinstance(value, str):
        path.write_text(value)
    else:
        path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sales_expected(account: dict[str, object]) -> dict[str, object]:
    if account["region"] == "Restricted-Zone" or int(account["employees"]) < 20:
        qualification, disposition, action, days = "disqualify", "closed-policy", "no-contact", 0
    elif bool(account["consent_hold"]):
        qualification, disposition, action, days = "hold", "consent-review", "no-contact", 0
    elif int(account["fit_score"]) >= 70 and bool(account["intent"]):
        qualification, disposition, action, days = "qualify", "sales-ready", "discovery", 7
    else:
        qualification, disposition, action, days = "nurture", "nurture", "value-resource", 21
    return {
        "id": account["id"],
        "qualification": qualification,
        "disposition": disposition,
        "action_type": action,
        "follow_up_days": days,
        "claim_code": "evidence-only" if account["regulated"] else "standard",
        "required_signal": account["signal"],
        "consequence_class": account["consequence_class"],
        "initial_value": account["initial_value"],
        "response_half_life_seconds": account["response_half_life_seconds"],
    }


def build_sales() -> None:
    root = FIXTURES / "sales"
    accounts: list[dict[str, object]] = []
    high_consequence = {17, 41, 63, 88, 109, 127, 151, 177, 203, 229}
    restricted = {14, 38, 74, 98, 134, 158, 194, 218}
    micro = {9, 31, 69, 91, 129, 171, 209, 231}
    consent = {23, 53, 83, 113, 143, 173, 213, 233}
    for index in range(1, 241):
        consequence = "high" if index in high_consequence else "standard"
        account = {
            "id": f"A{index:03d}",
            "company": f"Fictional Meridian {index:03d}",
            "employees": 12 if index in micro else 24 + (index * 17) % 980,
            "fit_score": 42 + (index * 13) % 57,
            "intent": index % 3 != 0,
            "region": "Restricted-Zone" if index in restricted else ["AU", "US", "EU"][index % 3],
            "regulated": index % 7 == 0 or index in high_consequence,
            "consent_hold": index in consent,
            "signal": f"SIG-{index:03d}-{['MIGRATION', 'CAPACITY', 'COMPLIANCE', 'LAUNCH'][index % 4]}",
            "note": f"The fictional intake explicitly mentioned {['migration timing', 'capacity pressure', 'compliance review', 'a new launch'][index % 4]}.",
            "consequence_class": consequence,
            "initial_value": 12 if consequence == "high" else 3 + index % 5,
            "response_half_life_seconds": 240 if consequence == "high" else 1_800 + (index % 4) * 600,
        }
        accounts.append(account)

    schedules: dict[str, list[dict[str, object]]] = {"D0": [], "D1": [], "D2": []}
    for position, account in enumerate(accounts):
        base = {
            "unit_id": account["id"],
            "batch": position // 40 + 1,
            "arrival_offset_seconds": 0 if position < 40 else (position // 40) * 45,
        }
        schedules["D0"].append({**base, "arrival_offset_seconds": 0, "value_curve": "flat"})
        schedules["D1"].append({**base, "value_curve": "ordinary"})
        schedules["D2"].append(
            {
                **base,
                "value_curve": "high-consequence" if account["consequence_class"] == "high" else "ordinary",
                "initial_value": account["initial_value"],
                "response_half_life_seconds": account["response_half_life_seconds"],
            }
        )

    write(root / "data" / "accounts.json", accounts)
    for name, schedule in schedules.items():
        write(root / "arrivals" / f"{name}.json", schedule)
    write(
        root / "README.md",
        """# EXP-05 fictional account queue (_test only)

The 240 dossiers are grouped into six arrival batches of forty. A Work batch is real sparse
accountability; individual accounts are not Work nodes. Each Staff owner returns one locally complete
state per owned account: `id`, `qualification`, `disposition`, `action_type`, `follow_up_days`,
`claim_code`, `evidence` (array), `next_action` (unsent draft), `uncertainty`, and `follow_up_trigger`.
Nothing is sent. Run `python3 verify.py sales <files...>` before handoff.
""",
    )
    write(
        root / "POLICY.md",
        """# Frozen sales policy

- Restricted-Zone or fewer than 20 employees: disqualify / closed-policy / no-contact / day 0.
- A consent hold: hold / consent-review / no-contact / day 0.
- Otherwise fit >= 70 and intent=true: qualify / sales-ready / discovery / day 7.
- All others: nurture / nurture / value-resource / day 21.
- Regulated accounts use claim_code=evidence-only; all others use standard.
- Evidence must cite employees, fit_score, intent, region and consent_hold.
- The unsent next action must mention the exact signal. Never invent a send, meeting or customer fact.
""",
    )
    write(HIDDEN / "sales-expected.json", [sales_expected(account) for account in accounts])
    by_id = {account["id"]: account for account in accounts}
    for demand, schedule in schedules.items():
        for batch in range(1, 7):
            rows = [
                {**by_id[item["unit_id"]], "arrival": item}
                for item in schedule
                if item["batch"] == batch
            ]
            write(root / "runtime-inputs" / demand / f"batch-{batch:02d}.json", rows)


SUPPORT_ACTION_V1 = {
    "ordinary": "resolve-standard",
    "expired": "decline-credit-escalate",
    "security": "security-escalation",
    "duplicate": "merge-duplicate",
    "vulnerable": "human-care-review",
    "identity": "identity-verification",
    "billing": "billing-investigation",
    "retention": "retention-review",
}

SUPPORT_ACTION_V2 = {
    **SUPPORT_ACTION_V1,
    "expired": "one-time-credit-review",
    "vulnerable": "priority-human-care-review",
    "retention": "no-pressure-retention-review",
}


def build_support() -> None:
    root = FIXTURES / "support"
    classes = list(SUPPORT_ACTION_V1)
    cases: list[dict[str, object]] = []
    for index in range(1, 97):
        case_class = classes[(index - 1) % len(classes)]
        cases.append(
            {
                "id": f"C{index:03d}",
                "class": case_class,
                "summary": f"Fictional {case_class} support case {index}",
                "signal": f"CASE-SIGNAL-{index:03d}",
                "harm_weight": 10 if case_class in {"security", "vulnerable", "identity"} else 3,
                "arrival_offset_seconds": (index - 1) // 8 * 45,
            }
        )

    write(root / "data" / "cases.json", cases)
    write(
        root / "POLICY-v1.md",
        "# Support policy v1\n\n"
        + "\n".join(f"- `{name}` → `{action}`" for name, action in SUPPORT_ACTION_V1.items())
        + "\n",
    )
    write(
        root / "POLICY-v2.md",
        "# Material support policy v2\n\n"
        "Effective for every unresolved or unaccepted case after the controlled event. Preserve prior work, "
        "but re-verify it before handoff.\n\n"
        + "\n".join(f"- `{name}` → `{action}`" for name, action in SUPPORT_ACTION_V2.items())
        + "\n",
    )
    policy_v2_hash = sha256_bytes((root / "POLICY-v2.md").read_bytes())
    write(
        root / "events" / "material-policy-change.json",
        {
            "event_id": "exp05-support-policy-v2",
            "kind": "controlled_test_policy_change",
            "transport_authenticated": False,
            "sender_content_trusted": True,
            "policy_version": 2,
            "policy_sha256": policy_v2_hash,
            "affected_classes": ["expired", "vulnerable", "retention"],
            "acceptance_rule": "No post-event handoff may use policy_version=1.",
        },
    )
    write(
        root / "events" / "worker-obstacle.json",
        {
            "event_id": "exp05-support-obstacle-c057",
            "kind": "controlled_test_worker_obstacle",
            "transport_authenticated": False,
            "case_id": "C057",
            "fact": "The fixture deliberately withholds the duplicate parent reference.",
            "required_supervisor_choice": "assign-parent-reference-or-safe-escalation",
        },
    )
    write(
        root / "README.md",
        """# EXP-05 fictional support queue (_test only)

The 96 cases arrive in twelve batches of eight and contain eight planted exception classes. Staff
owns only the exact case IDs in its Work and writes one locally complete resolution package per case:
`id`, `policy_version`, `action`, `customer_safe_draft`, `system_action_plan`, `next_state`, `evidence`,
and `uncertainty`. Nothing is sent or applied. A controlled policy event may arrive while production
is active; after that event every unresolved package must use policy v2. C057 contains a worker-local
obstacle whose safe resolution must remain attributable.
""",
    )
    write(
        HIDDEN / "support-expected-v1.json",
        [{"id": case["id"], "policy_version": 1, "action": SUPPORT_ACTION_V1[str(case["class"])], "harm_weight": case["harm_weight"]} for case in cases],
    )
    write(
        HIDDEN / "support-expected-v2.json",
        [{"id": case["id"], "policy_version": 2, "action": SUPPORT_ACTION_V2[str(case["class"])], "harm_weight": case["harm_weight"]} for case in cases],
    )
    for batch in range(1, 13):
        write(
            root / "runtime-inputs" / f"batch-{batch:02d}.json",
            cases[(batch - 1) * 8 : batch * 8],
        )


def monitoring_expected(index: int) -> dict[str, object]:
    return {
        "entity": f"Entity-{index:02d}",
        "event_code": f"EVENT-{index:02d}",
        "severity": "high" if index % 5 == 0 else "medium" if index % 2 == 0 else "low",
        "source_ids": [f"D{index:02d}-OFFICIAL", f"D{index:02d}-LATE"],
        "follow_up_trigger": f"TRIGGER-{index:02d}",
    }


def build_monitoring() -> None:
    root = FIXTURES / "monitoring"
    documents: list[dict[str, object]] = []
    expected: list[dict[str, object]] = []
    for index in range(1, 41):
        expected.append(monitoring_expected(index))
        severity = expected[-1]["severity"]
        entity = f"Entity-{index:02d}"
        documents.extend(
            [
                {"id": f"D{index:02d}-BASE", "entity": entity, "date": "2026-07-01", "source": "filing", "body": "Routine baseline with no material change."},
                {"id": f"D{index:02d}-NOISE", "entity": entity, "date": "2026-07-04", "source": "blog", "body": "Unrelated hiring and office trivia."},
                {"id": f"D{index:02d}-RUMOR", "entity": entity, "date": "2026-07-06", "source": "anonymous", "body": f"Uncorroborated rumor mentions EVENT-{index:02d}."},
                {"id": f"D{index:02d}-OLD", "entity": entity, "date": "2026-07-08", "source": "archive", "body": f"Outdated note denies EVENT-{index:02d}; superseded later."},
                {"id": f"D{index:02d}-OFFICIAL", "entity": entity, "date": "2026-07-12", "source": "official", "body": f"Confirmed EVENT-{index:02d}; classified {severity}; follow when TRIGGER-{index:02d} occurs."},
                {"id": f"D{index:02d}-DUP", "entity": entity, "date": "2026-07-12", "source": "syndicated", "body": f"Duplicate of confirmed EVENT-{index:02d}."},
                {"id": f"D{index:02d}-LATE", "entity": entity, "date": "2026-07-19", "source": "official-update", "body": f"Late update corroborates EVENT-{index:02d} and keeps severity {severity}."},
            ]
        )
    write(root / "corpus" / "documents.json", documents)
    write(
        root / "README.md",
        """# EXP-05 fictional monitoring corpus (_test only)

The corpus contains 280 dated documents across 40 fictional entities. Staff owns disjoint entities
and returns one locally complete alert per entity with `entity`, `event_code`, `severity`, `source_ids`,
`uncertainty`, and `follow_up_trigger`. Prefer authoritative late evidence. Rumor, stale contradiction
and noise are evidence to reject, not separate alerts. The product is an alert feed/index; there is no
summary memo or cognitive fan-in.
""",
    )
    write(HIDDEN / "monitoring-expected.json", expected)
    for territory in range(1, 5):
        owned = {f"Entity-{index:02d}" for index in range((territory - 1) * 10 + 1, territory * 10 + 1)}
        write(
            root / "runtime-inputs" / f"territory-{territory:02d}.json",
            [document for document in documents if document["entity"] in owned],
        )


def build_operations() -> None:
    """Freeze the small fourth-function outcome used only by Wave 4."""
    root = FIXTURES / "operations"
    invoices: list[dict[str, object]] = []
    expected: list[dict[str, object]] = []
    for index in range(1, 33):
        billed = 800 + index * 37
        delta = 0 if index % 4 else 75 if index % 8 else -50
        ledger = billed + delta
        if delta == 0:
            disposition, action = "matched", "close-reconciled"
        elif delta > 0:
            disposition, action = "over-recorded", "review-credit-or-ledger"
        else:
            disposition, action = "under-recorded", "review-missing-payment"
        invoices.append(
            {
                "id": f"I{index:03d}",
                "fictional_customer": f"Test Customer {index:02d}",
                "invoice_total": billed,
                "ledger_total": ledger,
                "currency": "AUD",
                "reference": f"INV-TEST-{index:03d}",
            }
        )
        expected.append({"id": f"I{index:03d}", "disposition": disposition, "action": action, "delta": delta})
    write(root / "data" / "invoices.json", invoices)
    write(
        root / "README.md",
        """# EXP-05 fictional operations reconciliation (_test only)

This is Wave 4's fourth bounded owner request. Reconcile all 32 fictional invoice/ledger pairs and
return one `reconciliation.json` array with `id`, `disposition`, `action`, `delta`, `evidence`, and
`uncertainty`. Nothing is posted, paid or changed. The exact verifier is authoritative for population
and arithmetic; the Staff worker owns the productive package and native review target.
""",
    )
    write(HIDDEN / "operations-expected.json", expected)
    for territory in range(1, 5):
        write(
            root / "runtime-inputs" / f"territory-{territory:02d}.json",
            invoices[(territory - 1) * 8 : territory * 8],
        )


def write_verifier() -> None:
    write(
        ROOT / "verify.py",
        '''#!/usr/bin/env python3
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
    payload = (json.dumps(units, indent=2, sort_keys=True) + "\\n").encode()
    if args.index:
        Path(args.index).write_bytes(payload)
    print(json.dumps({"valid": True, "domain": args.domain, "units": len(units), "sha256": hashlib.sha256(payload).hexdigest()}, sort_keys=True))


if __name__ == "__main__":
    main()
''',
    )


def write_blind_rubric() -> None:
    write(
        ROOT / "blind-evaluation-rubric.md",
        """# EXP-05 blinded semantic evaluation

The fresh GPT-5.6 Sol evaluator receives only the frozen owner contract, authoritative fictional
sources, exact deterministic index and native artifacts. It must not see topology, actor names,
model traces, usage, cost, arm labels or producer identity.

Score 0–10 with evidence for: usefulness, grounding, safe actionability, tail/exception handling,
uncertainty calibration and native-review readiness. Identify every consequential defect. Exact
population verification remains authoritative for coverage and policy fields; this review must not
replace it. Return exactly one JSON object with this shape and no additional top-level keys:

```json
{
  "scores": {
    "usefulness": 0,
    "grounding": 0,
    "safe_actionability": 0,
    "tail_handling": 0,
    "uncertainty_calibration": 0,
    "native_review_readiness": 0
  },
  "worst_unit": {"id": "exact unit id", "score": 0, "defect": "concise or none"},
  "high_consequence_breach": false,
  "consequential_defects": [],
  "evidence": ["concise artifact-grounded observation"],
  "decision": "accept"
}
```

Every score may be an integer or decimal from 0 through 10. `decision` is exactly `accept`, `repair`,
or `reject`. Private reasoning, topology guesses, markdown fences and surrounding prose are invalid.
""",
    )


def write_contract() -> None:
    write(
        ROOT / "frozen-contract.json",
        {
            "version": 1,
            "models": {
                "exec": {"selector": "openai-codex/gpt-5.6-sol", "effort": "medium"},
                "lead": {"selector": "openai-codex/gpt-5.6-sol", "effort": "medium"},
                "staff": {"selector": "openai-codex/gpt-5.6-terra", "effort": "medium"},
                "blind_evaluator": {"selector": "openai-codex/gpt-5.6-sol", "effort": "high"},
            },
            "sales": {
                "batches": 6,
                "units_per_batch": 40,
                "arrival_offsets_seconds": [0, 45, 90, 135, 180, 225],
                "q2_mapping": "alternating whole batches after a seeded random arm-order draw",
                "material_p90_improvement_fraction": 0.20,
                "throughput_gate_fraction": 0.25,
                "worst_decile_tolerance_10": 0.5,
                "cost_per_unit_tolerance_fraction": 0.25,
                "accepted_timestamp": "exact batch validator receipt",
                "sample_ids": ["A009", "A014", "A017", "A041", "A053", "A088", "A109", "A127", "A151", "A177", "A203", "A229", "A240"],
            },
            "support": {
                "event_trigger": "after at least two Staff Attempts have begun and the first attributable output progress marker is observed",
                "event_effective_time": "the same frozen trigger in both arms; only delivery to the lead differs",
                "terminal_arm_delivery": "withhold the same policy and obstacle until terminal review",
                "causal_arm_delivery": "address the same policy and obstacle immediately at the frozen trigger",
                "safe_redirect_window_seconds": 180,
                "sample_ids": ["C003", "C005", "C011", "C029", "C057", "C061", "C077", "C093", "C096"],
            },
            "monitoring": {"entities": 40, "documents": 280, "sample_entities": ["Entity-01", "Entity-05", "Entity-17", "Entity-25", "Entity-40"]},
            "lead_span": {"exception_response_seconds": 180, "max_needless_intervention_ratio": 0.25},
            "timing": {
                "request_start": "first Staff Attempt after Exec delegation; Exec-to-Staff dispatch is reported separately",
                "unit_arrival": "frozen schedule timestamp",
                "queue_complete": "all expected deterministic validator receipts",
                "worker_active": "report both summed actor time and union window",
                "operator_pauses": "reported separately and excluded from actor/provider latency",
            },
            "cost": {"subscription_usd_zero_or_missing": "non-discriminating; preserve unknown and use usage, latency, cadence and accepted value"},
        },
    )


def manifest() -> None:
    records = []
    for base in (FIXTURES, HIDDEN):
        for path in sorted(candidate for candidate in base.rglob("*") if candidate.is_file()):
            records.append({"path": str(path.relative_to(ROOT)), "bytes": path.stat().st_size, "sha256": sha256_bytes(path.read_bytes())})
    for path in (
        ROOT / "verify.py",
        ROOT / "blind-evaluation-rubric.md",
        ROOT / "frozen-contract.json",
        ROOT / "self-test.py",
        ROOT / "validate-visible.py",
        ROOT / "arm-catalog.json",
        ROOT / "product-runner.py",
        ROOT / "analyze.py",
        ROOT / "completion-audit.py",
    ):
        records.append({"path": str(path.relative_to(ROOT)), "bytes": path.stat().st_size, "sha256": sha256_bytes(path.read_bytes())})
    write(ROOT / "fixture-manifest.json", {"fictional_test_data": True, "version": 1, "files": records})


def main() -> None:
    for path in (FIXTURES, HIDDEN):
        if path.exists():
            shutil.rmtree(path)
    build_sales()
    build_support()
    build_monitoring()
    build_operations()
    write_verifier()
    write_blind_rubric()
    write_contract()
    manifest()


if __name__ == "__main__":
    main()
