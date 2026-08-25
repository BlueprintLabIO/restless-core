#!/usr/bin/env python3
"""Build deterministic, fictional EXP-04 fixtures before any model call."""

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


def sales_expected(account: dict[str, object]) -> dict[str, object]:
    if account["region"] == "Restricted-Zone" or int(account["employees"]) < 20:
        qualification, disposition, action, days = "disqualify", "closed-policy", "no-contact", 0
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
    }


def build_sales() -> None:
    root = FIXTURES / "sales"
    accounts: list[dict[str, object]] = []
    for index in range(1, 49):
        accounts.append(
            {
                "id": f"A{index:03d}",
                "company": f"Fictional Meridian {index:02d}",
                "employees": 12 if index in {9, 31} else 24 + (index * 17) % 480,
                "fit_score": 42 + (index * 13) % 57,
                "intent": index % 3 != 0,
                "region": "Restricted-Zone" if index in {14, 38} else ["AU", "US", "EU"][index % 3],
                "regulated": index % 7 == 0,
                "signal": f"SIG-{index:03d}-{['MIGRATION', 'CAPACITY', 'COMPLIANCE', 'LAUNCH'][index % 4]}",
                "note": f"The account explicitly mentioned {['migration timing', 'capacity pressure', 'compliance review', 'a new launch'][index % 4]} in its fictional intake.",
            }
        )
    expected = [sales_expected(account) for account in accounts]
    write(root / "data" / "accounts.json", accounts)
    write(
        root / "README.md",
        """# Fictional account-ownership queue (_test only)

This repository contains 48 fictional account dossiers. A worker owns the exact IDs in its Work.
Write one JSON array to the exact `outputs/<actor>.json` path named by the supervisor. Each unit needs:
`id`, `qualification`, `disposition`, `action_type`, `follow_up_days`, `claim_code`, `evidence` (array),
`next_action` (unsent draft). Apply the rules in `POLICY.md`, personalize with the dossier's exact signal,
run the declared verifier, commit cleanly, and report. Never send anything.
""",
    )
    write(
        root / "POLICY.md",
        """# Frozen sales policy

- Restricted-Zone or fewer than 20 employees: disqualify / closed-policy / no-contact / day 0.
- Otherwise fit >= 70 and intent=true: qualify / sales-ready / discovery / day 7.
- All others: nurture / nurture / value-resource / day 21.
- Regulated accounts use claim_code=evidence-only; all others use standard.
- Evidence must cite employees, fit_score, intent and region. The unsent next action must mention the exact signal.
""",
    )
    write(root / "verify-sales.mjs", SALES_VERIFIER)
    write(HIDDEN / "sales-expected.json", expected)


SALES_VERIFIER = r'''#!/usr/bin/env node
import fs from "node:fs";
const accounts = JSON.parse(fs.readFileSync("data/accounts.json", "utf8"));
const byId = new Map(accounts.map(a => [a.id, a]));
const requestedFiles = process.argv.slice(2).filter(x => x.endsWith(".json"));
const requestedIdsArg = process.argv.slice(2).find(x => x.startsWith("ids="));
const files = requestedFiles.length ? requestedFiles : (fs.existsSync("outputs") ? fs.readdirSync("outputs").filter(x => x.endsWith(".json")).map(x => `outputs/${x}`) : []);
const units = files.flatMap(file => JSON.parse(fs.readFileSync(file, "utf8")));
const expectedIds = requestedIdsArg ? requestedIdsArg.slice(4).split(",").filter(Boolean) : accounts.map(a => a.id);
const ids = units.map(u => u.id);
const errors = [];
if (new Set(ids).size !== ids.length) errors.push("duplicate IDs");
if ([...ids].sort().join() !== [...expectedIds].sort().join()) errors.push(`ownership mismatch expected=${expectedIds.length} observed=${ids.length}`);
for (const unit of units) {
  const a = byId.get(unit.id); if (!a) { errors.push(`unknown ${unit.id}`); continue; }
  let q, d, t, days;
  if (a.region === "Restricted-Zone" || a.employees < 20) [q,d,t,days] = ["disqualify","closed-policy","no-contact",0];
  else if (a.fit_score >= 70 && a.intent) [q,d,t,days] = ["qualify","sales-ready","discovery",7];
  else [q,d,t,days] = ["nurture","nurture","value-resource",21];
  const exact = {qualification:q, disposition:d, action_type:t, follow_up_days:days, claim_code:a.regulated ? "evidence-only" : "standard"};
  for (const [key,value] of Object.entries(exact)) if (unit[key] !== value) errors.push(`${unit.id}:${key}`);
  if (!Array.isArray(unit.evidence) || !["employees","fit_score","intent","region"].every(k => unit.evidence.some(e => String(e).startsWith(`${k}=`)))) errors.push(`${unit.id}:evidence`);
  if (!String(unit.next_action || "").includes(a.signal)) errors.push(`${unit.id}:personalization`);
}
if (errors.length) { console.error(JSON.stringify(errors)); process.exit(1); }
console.log(JSON.stringify({valid:true, units:units.length, files:files.sort()}));
'''


def monitoring_expected(entity: str, index: int) -> dict[str, object]:
    return {
        "entity": entity,
        "event_code": f"EVENT-{index:02d}",
        "severity": "high" if index % 4 == 0 else "medium" if index % 2 == 0 else "low",
        "source_ids": [f"D{index:02d}-OFFICIAL", f"D{index:02d}-LATE"],
        "follow_up_trigger": f"TRIGGER-{index:02d}",
    }


def build_monitoring() -> None:
    root = FIXTURES / "monitoring"
    documents: list[dict[str, object]] = []
    expected: list[dict[str, object]] = []
    for index in range(1, 13):
        entity = f"Entity-{index:02d}"
        expected.append(monitoring_expected(entity, index))
        severity = expected[-1]["severity"]
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
        """# Fictional competitive-monitoring corpus (_test only)

Search the 84 frozen documents. Each worker owns exact fictional entities and writes one JSON array to
`alerts/<actor>.json`. Return one locally complete alert per owned entity with `entity`, `event_code`,
`severity`, `source_ids`, `uncertainty`, and `follow_up_trigger`. Prefer authoritative late evidence;
do not count rumor, duplicate or superseded material as separate events. Run the declared verifier,
commit and report. There is no summary memo.
""",
    )
    write(root / "verify-monitoring.mjs", MONITOR_VERIFIER)
    write(HIDDEN / "monitoring-expected.json", expected)


MONITOR_VERIFIER = r'''#!/usr/bin/env node
import fs from "node:fs";
const docs = JSON.parse(fs.readFileSync("corpus/documents.json", "utf8"));
const entities = [...new Set(docs.map(d => d.entity))].sort();
const requestedFiles = process.argv.slice(2).filter(x => x.endsWith(".json"));
const requestedArg = process.argv.slice(2).find(x => x.startsWith("entities="));
const files = requestedFiles.length ? requestedFiles : (fs.existsSync("alerts") ? fs.readdirSync("alerts").filter(x => x.endsWith(".json")).map(x => `alerts/${x}`) : []);
const alerts = files.flatMap(file => JSON.parse(fs.readFileSync(file, "utf8")));
const expectedEntities = requestedArg ? requestedArg.slice(9).split(",").filter(Boolean) : entities;
const errors = [];
if (new Set(alerts.map(a => a.entity)).size !== alerts.length) errors.push("duplicate entities");
if (alerts.map(a=>a.entity).sort().join() !== [...expectedEntities].sort().join()) errors.push("entity ownership mismatch");
for (const alert of alerts) {
  const index = Number(alert.entity.slice(-2));
  const expected = {event_code:`EVENT-${String(index).padStart(2,"0")}`, severity:index%4===0?"high":index%2===0?"medium":"low", follow_up_trigger:`TRIGGER-${String(index).padStart(2,"0")}`};
  for (const [key,value] of Object.entries(expected)) if (alert[key] !== value) errors.push(`${alert.entity}:${key}`);
  const sources = [...(alert.source_ids || [])].sort();
  if (sources.join() !== [`D${String(index).padStart(2,"0")}-LATE`,`D${String(index).padStart(2,"0")}-OFFICIAL`].sort().join()) errors.push(`${alert.entity}:sources`);
  if (!String(alert.uncertainty || "").trim()) errors.push(`${alert.entity}:uncertainty`);
}
if (errors.length) { console.error(JSON.stringify(errors)); process.exit(1); }
console.log(JSON.stringify({valid:true, alerts:alerts.length, files:files.sort()}));
'''


def build_support() -> None:
    root = FIXTURES / "support"
    cases: list[dict[str, object]] = []
    expected: list[dict[str, object]] = []
    classes = ["ordinary", "expired", "security", "duplicate", "vulnerable", "identity"]
    for index in range(1, 49):
        case_class = classes[(index - 1) % len(classes)]
        case = {
            "id": f"C{index:03d}",
            "class": case_class,
            "summary": f"Fictional {case_class} support case {index}",
            "signal": f"CASE-SIGNAL-{index:03d}",
        }
        cases.append(case)
        action = {
            "ordinary": "resolve-standard",
            "expired": "decline-credit-escalate",
            "security": "security-escalation",
            "duplicate": "merge-duplicate",
            "vulnerable": "human-care-review",
            "identity": "identity-verification",
        }[case_class]
        expected.append({"id": case["id"], "policy_version": 2, "action": action})
    write(root / "data" / "cases.json", cases)
    write(
        root / "README.md",
        """# Fictional support queue (_test only)

Own only the case IDs named in Work. Write `resolutions/<actor>.json`, one unit per case, with `id`,
`policy_version`, `action`, `customer_safe_draft`, `system_action_plan`, and `next_state`. Nothing is sent
or applied. During the run, the supervisor may provide a material policy update; any Attempt started
before it must reconcile preserved work and use the newer policy before handoff.
""",
    )
    write(root / "verify-support-schema.mjs", SUPPORT_SCHEMA_VERIFIER)
    write(HIDDEN / "support-expected.json", expected)


SUPPORT_SCHEMA_VERIFIER = r'''#!/usr/bin/env node
import fs from "node:fs";
const files = process.argv.slice(2).filter(x => x.endsWith(".json"));
const expectedArg = process.argv.slice(2).find(x => x.startsWith("ids="));
const units = files.flatMap(file => JSON.parse(fs.readFileSync(file, "utf8")));
const expected = expectedArg ? expectedArg.slice(4).split(",").filter(Boolean) : units.map(u=>u.id);
const required = ["id","policy_version","action","customer_safe_draft","system_action_plan","next_state"];
const errors = [];
if (new Set(units.map(u=>u.id)).size !== units.length) errors.push("duplicates");
if (units.map(u=>u.id).sort().join() !== [...expected].sort().join()) errors.push("ownership");
for (const unit of units) for (const key of required) if (unit[key] === undefined || unit[key] === "") errors.push(`${unit.id}:${key}`);
if (errors.length) { console.error(JSON.stringify(errors)); process.exit(1); }
console.log(JSON.stringify({valid:true, units:units.length}));
'''


def manifest() -> None:
    records = []
    for path in sorted(path for path in FIXTURES.rglob("*") if path.is_file()):
        records.append({"path": str(path.relative_to(ROOT)), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()})
    for path in sorted(path for path in HIDDEN.rglob("*") if path.is_file()):
        records.append({"path": str(path.relative_to(ROOT)), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()})
    write(ROOT / "fixture-manifest.json", {"fictional_test_data": True, "files": records})


def main() -> None:
    for path in (FIXTURES, HIDDEN):
        if path.exists():
            shutil.rmtree(path)
    build_sales()
    build_monitoring()
    build_support()
    manifest()


if __name__ == "__main__":
    main()
