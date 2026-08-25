#!/usr/bin/env python3
"""Drive EXP-05 through Restless's production daemon and OrgIntel scheduler.

The controller owns only frozen `_test` inputs, arrivals, exact validation and
evidence capture. It never claims Work, launches an actor, decides semantic
completion, or implements a second scheduler.
"""

from __future__ import annotations

import argparse
import asyncio
import hashlib
import json
import math
import os
import re
import shutil
import statistics
import subprocess
import sys
import time
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Callable


ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[3]
CLI = REPO / "target/debug/restless"
CATALOG = json.loads((ROOT / "arm-catalog.json").read_text())
CONTRACT = json.loads((ROOT / "frozen-contract.json").read_text())
EXEC_MODEL = CONTRACT["models"]["exec"]["selector"]
LEAD_MODEL = CONTRACT["models"]["lead"]["selector"]
STAFF_MODEL = CONTRACT["models"]["staff"]["selector"]
PRODUCTION_EFFORT = CONTRACT["models"]["staff"]["effort"]
MODEL_PROVIDER = LEAD_MODEL.split("/", 1)[0]
MODEL_CREDENTIAL_REFERENCE = "infisical:/providers/zai/ZAI_API_KEY"
RUNTIME_ROOT = Path("/company/experiment/EXP-05")
RESULTS = ROOT / "results"
PROGRAM_CEILING_USD = 100.0

# Product actor displays are colleague identities, not prettified machine IDs
# or repeated role labels. Keep them frozen so topology is the only changing
# organisational variable across cells.
ACTOR_DISPLAY_NAMES = {
    "revenue-direction": "Mara Voss",
    "account-north": "Avery Lin",
    "account-south": "Noah Patel",
    "account-east": "Imani Brooks",
    "account-west": "Luca Meyer",
    "account-owner": "Sofia Reyes",
    "customer-direction": "Priya Shah",
    "customer-north": "Daniel Cho",
    "customer-south": "Amara Okafor",
    "customer-east": "Elias Novak",
    "customer-west": "Zoe Martin",
    "case-owner": "Nina Park",
    "intelligence-direction": "Elena Torres",
    "signal-primary": "Theo Bennett",
    "signal-secondary": "Mina Kaur",
    "signal-tertiary": "Jon Bell",
    "signal-quaternary": "Aya Mori",
    "monitoring-owner": "Owen Price",
    "capacity-direction": "Caleb Wong",
    "capacity-alpha": "Maya Singh",
    "capacity-beta": "Felix Grant",
    "capacity-gamma": "Leila Haddad",
    "capacity-delta": "Samir Costa",
    "operations-direction": "Rhea Collins",
    "reconciliation-owner": "Marco Silva",
    "continuity-direction": "Anika Rao",
    "continuity-worker": "Jamie Chen",
    "continuity-control": "Robin Foster",
}


class RunFailure(RuntimeError):
    pass


class SafetyEnvelopeExceeded(RunFailure):
    pass


@dataclass(frozen=True)
class Unit:
    number: int
    name: str
    source: Path
    runtime_source: Path
    output_name: str
    runtime_output: Path
    progress: Path
    offset_seconds: int
    fixed_worker: int | None


@dataclass(frozen=True)
class Arm:
    arm_id: str
    wave: int
    domain: str
    shape: str
    workers: int
    demand: str | None = None
    supervision: str | None = None

    @classmethod
    def load(cls, arm_id: str) -> "Arm":
        try:
            row = CATALOG["arms"][arm_id]
        except KeyError as error:
            raise RunFailure(f"unknown arm {arm_id!r}") from error
        return cls(
            arm_id=arm_id,
            wave=int(row["wave"]),
            domain=str(row["domain"]),
            shape=str(row["shape"]),
            workers=int(row["workers"]),
            demand=row.get("demand"),
            supervision=row.get("supervision"),
        )


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def json_write(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def actor_display(actor_id: str) -> str:
    try:
        return ACTOR_DISPLAY_NAMES[actor_id]
    except KeyError as error:
        raise RunFailure(f"no frozen colleague identity for actor {actor_id!r}") from error


def run_sync(argv: list[str], *, check: bool = True, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    merged = os.environ.copy()
    if env:
        merged.update(env)
    result = subprocess.run(argv, cwd=REPO, text=True, capture_output=True, env=merged)
    if check and result.returncode != 0:
        raise RunFailure(
            f"command failed ({result.returncode}): {' '.join(argv)}\n{result.stderr.strip()}"
        )
    return result


async def run_async(argv: list[str], *, check: bool = True) -> tuple[int, str, str]:
    process = await asyncio.create_subprocess_exec(
        *argv,
        cwd=REPO,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )
    try:
        stdout, stderr = await process.communicate()
    except asyncio.CancelledError:
        # A stopped experiment must not leave its blocking `restless tell`
        # child alive to keep changing the company after evidence capture has
        # ended. The short grace period supervises OS teardown; it is not an
        # agent-work deadline.
        process.terminate()
        try:
            await asyncio.wait_for(process.wait(), timeout=5)
        except TimeoutError:
            process.kill()
            await process.wait()
        raise
    text_out = stdout.decode(errors="replace")
    text_err = stderr.decode(errors="replace")
    if check and process.returncode != 0:
        raise RunFailure(
            f"command failed ({process.returncode}): {' '.join(argv)}\n{text_err.strip()}"
        )
    return process.returncode or 0, text_out, text_err


def cli(*args: str, check: bool = True) -> str:
    return run_sync([str(CLI), *args], check=check).stdout.strip()


def cli_json(*args: str) -> Any:
    argv = [str(CLI), *args]
    for attempt in range(3):
        result = run_sync(argv, check=False)
        if result.returncode == 0:
            return json.loads(result.stdout.strip())
        transient_read_failure = "pool timed out while waiting for an open connection" in result.stderr
        if not transient_read_failure or attempt == 2:
            raise RunFailure(
                f"command failed ({result.returncode}): {' '.join(argv)}\n{result.stderr.strip()}"
            )
        # These are read-only observer commands. Retrying a transient pool
        # acquisition cannot duplicate Work or an effect; the tiny backoff is
        # substrate recovery, never a semantic completion timer.
        time.sleep(0.1 * (attempt + 1))
    raise AssertionError("bounded read retry exhausted without a disposition")


def experiment_spend() -> dict[str, object]:
    rows: list[dict[str, object]] = []
    for company in cli_json("company", "list"):
        if not (
            str(company).startswith("exp05_")
            and str(company).endswith("_test")
            and company != "exp05_model_anchor_test"
        ):
            continue
        result = cli("spend", "-c", str(company), check=False)
        try:
            spend = json.loads(result)
        except json.JSONDecodeError:
            raise RunFailure(f"could not read authoritative spend for {company}")
        rows.append({"company": company, **spend})
    accounted_usd = sum(float(row["accounted_usd"]) for row in rows)
    committed_usd = sum(
        float(row["ceiling_usd"])
        if row.get("status") == "metering_unknown"
        else float(row["accounted_usd"])
        for row in rows
    )
    return {
        "ceiling_usd": PROGRAM_CEILING_USD,
        "accounted_usd": accounted_usd,
        "committed_usd": committed_usd,
        "unknown_reserve_usd": committed_usd - accounted_usd,
        "metering_unknown_companies": [
            row["company"] for row in rows if row.get("status") == "metering_unknown"
        ],
        "companies": rows,
    }


def admit_program_cell(cell_ceiling_usd: float) -> dict[str, object]:
    spend = experiment_spend()
    if float(spend["committed_usd"]) + cell_ceiling_usd > PROGRAM_CEILING_USD:
        raise RunFailure(
            f"EXP-05 programme admission would exceed ${PROGRAM_CEILING_USD:.0f}: "
            f"${float(spend['accounted_usd']):.6f} exactly accounted, "
            f"${float(spend['unknown_reserve_usd']):.6f} reserved for unknown metering, and "
            f"${cell_ceiling_usd:.0f} requested"
        )
    return spend


def container_name(company: str) -> str:
    return f"restless-co-{company}"


def docker_copy(source: Path, company: str, destination: Path) -> None:
    container = container_name(company)
    cli("attach", "-c", company, "mkdir", "-p", str(destination.parent))
    run_sync(["docker", "cp", str(source), f"{container}:{destination}"])
    run_sync(["docker", "exec", container, "chown", "company:company", str(destination)])


def docker_read(company: str, path: Path) -> bytes | None:
    result = run_sync(
        ["docker", "exec", "-u", "company", container_name(company), "cat", str(path)],
        check=False,
    )
    if result.returncode != 0:
        return None
    return result.stdout.encode()


def copy_text(company: str, destination: Path, text: str, staging: Path) -> None:
    local = staging / "runtime-files" / str(destination).lstrip("/")
    local.parent.mkdir(parents=True, exist_ok=True)
    local.write_text(text)
    docker_copy(local, company, destination)


def actor_ids(domain: str, workers: int) -> tuple[str, list[str], str]:
    if domain == "sales":
        return "revenue-direction", ["account-north", "account-south", "account-east", "account-west"][:workers], "Revenue Queue"
    if domain == "support":
        return "customer-direction", ["customer-north", "customer-south", "customer-east", "customer-west"][:workers], "Customer Operations"
    if domain == "monitoring":
        return "intelligence-direction", ["signal-primary", "signal-secondary", "signal-tertiary", "signal-quaternary"][:workers], "Market Intelligence"
    if domain == "capacity":
        return "capacity-direction", ["capacity-alpha", "capacity-beta", "capacity-gamma", "capacity-delta"][:workers], "Capacity Probe"
    if domain == "operations":
        return "capacity-direction", ["capacity-alpha", "capacity-beta", "capacity-gamma", "capacity-delta"][:workers], "Capacity Probe"
    if domain == "continuity":
        return "continuity-direction", ["continuity-worker"], "Continuity Probe"
    raise RunFailure(f"no actor fixture for domain {domain}")


def fixed_worker(shape: str, number: int) -> int | None:
    if shape == "q1":
        return 1
    if shape == "q2":
        return 1 if number % 2 else 2
    if shape == "q4":
        return (number - 1) % 4 + 1
    if shape == "elastic":
        return None
    raise RunFailure(f"unsupported shape {shape}")


def units_for(arm: Arm) -> list[Unit]:
    if arm.domain == "sales":
        assert arm.demand
        return [
            Unit(
                number=batch,
                name=f"sales batch {batch:02d}",
                source=ROOT / f"fixtures/sales/runtime-inputs/{arm.demand}/batch-{batch:02d}.json",
                runtime_source=RUNTIME_ROOT / f"inbox/sales/batch-{batch:02d}.json",
                output_name=f"batch-{batch:02d}.json",
                runtime_output=RUNTIME_ROOT / f"outputs/sales/batch-{batch:02d}.json",
                progress=RUNTIME_ROOT / f"progress/sales/batch-{batch:02d}.produce",
                offset_seconds=0 if arm.demand == "D0" else (batch - 1) * 45,
                fixed_worker=fixed_worker(arm.shape, batch),
            )
            for batch in range(1, 7)
        ]
    if arm.domain == "support":
        return [
            Unit(
                number=batch,
                name=f"support batch {batch:02d}",
                source=ROOT / f"fixtures/support/runtime-inputs/batch-{batch:02d}.json",
                runtime_source=RUNTIME_ROOT / f"inbox/support/batch-{batch:02d}.json",
                output_name=f"batch-{batch:02d}.json",
                runtime_output=RUNTIME_ROOT / f"outputs/support/batch-{batch:02d}.json",
                progress=RUNTIME_ROOT / f"progress/support/batch-{batch:02d}.produce",
                offset_seconds=(batch - 1) * 45,
                fixed_worker=fixed_worker("q2", batch),
            )
            for batch in range(1, 13)
        ]
    if arm.domain == "monitoring":
        return [
            Unit(
                number=territory,
                name=f"monitoring territory {territory:02d}",
                source=ROOT / f"fixtures/monitoring/runtime-inputs/territory-{territory:02d}.json",
                runtime_source=RUNTIME_ROOT / f"inbox/monitoring/territory-{territory:02d}.json",
                output_name=f"territory-{territory:02d}.json",
                runtime_output=RUNTIME_ROOT / f"outputs/monitoring/territory-{territory:02d}.json",
                progress=RUNTIME_ROOT / f"progress/monitoring/territory-{territory:02d}.produce",
                offset_seconds=0,
                fixed_worker=fixed_worker(arm.shape, territory),
            )
            for territory in range(1, 5)
        ]
    if arm.domain == "operations":
        return [
            Unit(
                number=territory,
                name=f"operations territory {territory:02d}",
                source=ROOT / f"fixtures/operations/runtime-inputs/territory-{territory:02d}.json",
                runtime_source=RUNTIME_ROOT / f"inbox/operations/territory-{territory:02d}.json",
                output_name=f"territory-{territory:02d}.json",
                runtime_output=RUNTIME_ROOT / f"outputs/operations/territory-{territory:02d}.json",
                progress=RUNTIME_ROOT / f"progress/operations/territory-{territory:02d}.produce",
                offset_seconds=0,
                fixed_worker=fixed_worker(arm.shape, territory),
            )
            for territory in range(1, 5)
        ]
    raise RunFailure(f"unit plan for {arm.domain} is handled by its dedicated runner")


def company_name(arm_id: str, run_token: str) -> str:
    compact = re.sub(r"[^a-z0-9]+", "_", arm_id.lower()).strip("_")[:30]
    token = re.sub(r"[^a-z0-9]+", "", run_token.lower())[:8] or "run"
    identity = hashlib.sha256(f"{arm_id}\0{run_token}".encode()).hexdigest()[:8]
    return f"exp05_{compact}_{token}_{identity}_test"


def company_mission(arm: Arm) -> str:
    return (
        "Run one frozen EXP-05 fictional _test outcome through ordinary Restless coordination. "
        "Nothing may be sent, published, paid, posted, or applied. Exec delegates productive work "
        "to the named accountable lead and returns to availability. The lead remains a non-producing "
        "supervisor; Staff owns every output. Exact source paths, acceptance gates and arrival facts "
        f"are in {RUNTIME_ROOT}/charter.md. Arm: {arm.arm_id}."
    )


def gate_command(arm: Arm, unit: Unit) -> str:
    validator = RUNTIME_ROOT / "tools/validate-visible.py"
    if arm.domain == "support":
        version = RUNTIME_ROOT / "control/support-policy-version"
        shell = (
            f"python3 {validator} support-v$(cat {version}) {unit.runtime_source} "
            f"{unit.runtime_output} --progress {unit.progress}"
        )
        argv = ["sh", "-c", shell]
    else:
        argv = [
            "python3",
            str(validator),
            arm.domain,
            str(unit.runtime_source),
            str(unit.runtime_output),
            "--progress",
            str(unit.progress),
        ]
    return json.dumps({"name": "visible-unit-contract", "command": argv}, separators=(",", ":"))


def charter_for(
    arm: Arm,
    lead: str,
    workers: list[str],
    units: list[Unit],
    *,
    include_support_event: bool = True,
    inputs_visible_at_start: bool = False,
) -> str:
    domain_contract = {
        "sales": "Return the exact locally complete account states required by the frozen sales README and policy. Nothing is sent.",
        "support": "Return safe locally complete case packages under the canonical current policy version. Nothing is sent or applied.",
        "monitoring": "Return one locally complete alert per owned entity. The product is an alert index, never a memo or model fan-in.",
        "operations": "Reconcile every owned fictional invoice/ledger pair into a locally complete exception state. Nothing is posted, paid or changed.",
    }[arm.domain]
    if arm.shape == "elastic":
        roster = (
            f"Start with {workers[0]}. You may commission exactly one relief worker with id `account-relief`, "
            f"role `account queue operator`, model `{STAFF_MODEL}`, and assign only not-yet-commissioned batches. "
            "Do so only after observing backlog age, response value and live capacity; send one factual rationale to Exec. No threshold decides for you."
        )
    else:
        roster = (
            f"The diagnostic roster is fixed at {len(workers)} worker(s): {', '.join(workers)}. "
            "Do not add, retire, reassign across teams, or change models during this arm."
        )
    rows = []
    for unit in units:
        owner = "lead chooses an available existing/authorised relief worker" if unit.fixed_worker is None else workers[unit.fixed_worker - 1]
        rows.append(
            f"- {unit.name}: input `{unit.runtime_source}`; owner `{owner}`; output `{unit.runtime_output}`; "
            f"progress `{unit.progress}`; gate `{gate_command(arm, unit)}`"
        )
    support = ""
    if arm.domain == "support" and include_support_event:
        support = f"""

## Controlled material event

Policy version is read from `{RUNTIME_ROOT}/control/support-policy-version` at each Attempt. A
controlled event is not provider-authenticated. When policy v2 and the C057 obstacle arrive, inspect
the exact event files. In the causal arm, send Work-linked changed facts to only affected live
workers; this is what makes the production cancellation attributable and scoped. In the terminal
arm, after the delayed event, commission the smallest attributable repair Work needed to bring every
post-effective-time batch to v2—prefer one repair territory per existing worker over twelve case or
batch nodes. Never repair outputs yourself.
"""
    arrival_instruction = (
        "The listed inputs are already present when this charter is commissioned. Commission every "
        "listed unit exactly once."
        if inputs_visible_at_start
        else "Inputs appear only when their frozen arrival occurs. On each controlled arrival message, "
        "commission each newly visible unit exactly once."
    )
    return f"""# EXP-05 accountable outcome charter — {arm.arm_id}

You are `{lead}`, accountable for this one fictional `_test` outcome. Remain a non-producing
supervisor: frame, commission, observe, guide, redirect, repair through Staff, and judge the exact
native result. Do not write, edit, merge, or silently repair any candidate output.

## Outcome

{domain_contract}

{roster}

{arrival_instruction} Use one repo-less Staff Work node per listed unit. Do not mirror individual
accounts, cases, documents, or reasoning in Work. The Work outcome must require the worker to:

1. read only its exact source and current policy;
2. create its progress marker immediately after source orientation and before producing output;
3. write the exact JSON array to its output path;
4. run the visible validator;
5. link that output from its live Attempt as kind `review_target`; and
6. return `outcome_met` only after the gate and artifact are real.

Declare `--attempt-limit 3`, the exact expected artifact, and the supplied gate atomically on Work
creation. Routine unit completion needs no message or central summary. A validator-complete event
will arrive only after every exact unit receipt exists. Then inspect the native outputs read-only,
run the visible gates, sample consequential tails, and make one exact outcome judgement. If accepted,
send one material completion decision to Exec. If not, send concrete Work-linked feedback to the
responsible worker and let attributable revision Work repair it. Never build an assembler or rewrite
Staff output.

## Frozen units

{chr(10).join(rows)}
{support}
## Coordination

Use natural, concise messages carrying only decision-changing facts. Do not poll Staff, narrate
status, ask Exec to relay local facts, or create a batch memo. Elapsed time is only a safety envelope;
Work/Attempt terminal facts, artifacts and validator receipts decide completion.
"""


def create_company_config(path: Path, company: str, arm: Arm, ceiling: int) -> None:
    mission = company_mission(arm).replace('"""', "")
    path.write_text(
        f'name = "{company}"\nmission = """{mission}"""\n'
        f'spend_ceiling_usd = "{ceiling}"\nmodel = "{EXEC_MODEL}"\nmodel_failover = []\n'
    )


def ensure_anchor(staging: Path) -> str:
    anchor = "exp05_model_anchor_test"
    mission = "Host-only GLM-5.3 route anchor for fictional EXP-05 tests."
    config = staging / "anchor-company.toml"
    config.write_text(
        f'name = "{anchor}"\nmission = "{mission}"\n'
        f'spend_ceiling_usd = "1"\nmodel = "{EXEC_MODEL}"\nmodel_failover = []\n'
        f'[credentials]\n"model.inference.{MODEL_PROVIDER}" = "{MODEL_CREDENTIAL_REFERENCE}"\n'
    )
    configured = set(cli_json("company", "list"))
    if anchor not in configured:
        cli("company", "create", "--from-file", str(config))
    else:
        cli("company", "set", "-c", anchor, "mission", mission)
        cli("company", "set", "-c", anchor, "model", EXEC_MODEL)
        cli("company", "set", "-c", anchor, "model_failover", "")
        cli("company", "set", "-c", anchor, "spend_ceiling_usd", "1")
        cli(
            "company", "set", "-c", anchor,
            f"credentials.model.inference.{MODEL_PROVIDER}",
            MODEL_CREDENTIAL_REFERENCE,
        )
        current = cli("company", "show", "-c", anchor)
        if "model.inference.openai-codex" in current:
            cli(
                "company", "unset", "-c", anchor,
                "credentials.model.inference.openai-codex",
            )
    return anchor


def provision(arm: Arm, run_token: str, result_dir: Path) -> tuple[str, str, list[str], list[Unit]]:
    if not CLI.is_file():
        raise RunFailure(f"missing built CLI at {CLI}")
    company = company_name(arm.arm_id, run_token)
    configured = set(cli_json("company", "list"))
    if company in configured:
        raise RunFailure(f"refusing to reuse existing counted company {company}")
    cell_ceiling = 8 if arm.wave == 0 else 20 if arm.wave == 4 else 12
    programme_spend = admit_program_cell(cell_ceiling)
    json_write(result_dir / "control/programme-spend-at-admission.json", programme_spend)
    config = result_dir / "control/company.toml"
    create_company_config(config, company, arm, cell_ceiling)
    cli("company", "create", "--from-file", str(config))
    cli("up", "-c", company)
    lead, workers, team_name = actor_ids(arm.domain, arm.workers)
    cli(
        "people", "create", "-c", company, "--id", lead,
        "--role", f"accountable {arm.domain} outcome lead", "--display", actor_display(lead),
        "--model", LEAD_MODEL, "--reason", "frozen EXP-05 accountable supervision boundary",
    )
    for worker in workers:
        cli(
            "people", "create", "-c", company, "--id", worker,
            "--role", f"{arm.domain} queue operator", "--display", actor_display(worker),
            "--model", STAFF_MODEL, "--reason", "frozen EXP-05 independently closing production capacity",
        )
    cli(
        "teams", "create", "-c", company, "--name", team_name, "--lead", lead,
        "--brief", f"Own {arm.arm_id}; exact charter at {RUNTIME_ROOT}/charter.md; lead never produces.",
    )
    for worker in workers:
        cli(
            "teams", "assign", "-c", company, "--actor", worker, "--team", team_name,
            "--reason", "frozen EXP-05 diagnostic roster",
        )
    units = units_for(arm)
    runtime_dirs = [
        RUNTIME_ROOT / "tools", RUNTIME_ROOT / "control", RUNTIME_ROOT / "inbox",
        RUNTIME_ROOT / "outputs", RUNTIME_ROOT / "progress",
    ]
    cli("attach", "-c", company, "mkdir", "-p", *map(str, runtime_dirs))
    docker_copy(ROOT / "validate-visible.py", company, RUNTIME_ROOT / "tools/validate-visible.py")
    charter = result_dir / "control/charter.md"
    charter.write_text(charter_for(arm, lead, workers, units))
    docker_copy(charter, company, RUNTIME_ROOT / "charter.md")
    if arm.domain == "sales":
        docker_copy(ROOT / "fixtures/sales/POLICY.md", company, RUNTIME_ROOT / "policy/sales.md")
        docker_copy(ROOT / "fixtures/sales/README.md", company, RUNTIME_ROOT / "policy/sales-README.md")
    elif arm.domain == "support":
        docker_copy(ROOT / "fixtures/support/POLICY-v1.md", company, RUNTIME_ROOT / "policy/support-v1.md")
        docker_copy(ROOT / "fixtures/support/README.md", company, RUNTIME_ROOT / "policy/support-README.md")
        copy_text(company, RUNTIME_ROOT / "control/support-policy-version", "1\n", result_dir / "control")
    elif arm.domain == "monitoring":
        docker_copy(ROOT / "fixtures/monitoring/README.md", company, RUNTIME_ROOT / "policy/monitoring-README.md")
    elif arm.domain == "operations":
        docker_copy(ROOT / "fixtures/operations/README.md", company, RUNTIME_ROOT / "policy/operations-README.md")
    return company, lead, workers, units


def reveal(company: str, unit: Unit) -> None:
    docker_copy(unit.source, company, unit.runtime_source)


def send_controlled_event(company: str, lead: str, body: str) -> str:
    return cli(
        "message", "-c", company, "--from", "owner", "--to", lead,
        f"[CONTROLLED EXP-05 _TEST EVENT; transport_authenticated=false] {body}",
    )


def graph(company: str) -> dict[str, Any]:
    return cli_json("work", "graph", "-c", company)


def events(company: str) -> list[dict[str, Any]]:
    return cli_json("events", "-c", company, "--limit", "10000")


async def wait_until(
    description: str,
    predicate: Callable[[], Any],
    deadline: float,
    interval: float = 0.75,
) -> Any:
    while True:
        value = predicate()
        if value:
            return value
        if time.monotonic() >= deadline:
            raise SafetyEnvelopeExceeded(f"safety envelope expired while waiting for {description}")
        await asyncio.sleep(interval)


def first_attempt_time(company: str) -> str | None:
    attempts = graph(company)["attempts"]
    return min((row["started_at"] for row in attempts), default=None)


def actor_first_attempt_time(company: str, actor: str) -> str | None:
    attempts = graph(company)["attempts"]
    return min(
        (row["started_at"] for row in attempts if row.get("actor_id") == actor),
        default=None,
    )


def running_attempt_times(company: str, actors: set[str]) -> dict[str, str] | None:
    running = {
        row["actor_id"]: row["started_at"]
        for row in graph(company)["attempts"]
        if row.get("actor_id") in actors and row.get("state") == "running"
    }
    return running if set(running) == actors else None


def output_snapshot(company: str, unit: Unit, local: Path) -> bool:
    body = docker_read(company, unit.runtime_output)
    if body is None:
        return False
    local.parent.mkdir(parents=True, exist_ok=True)
    local.write_bytes(body)
    return True


def validate_local_unit(arm: Arm, unit: Unit, output: Path, policy: int | None = None) -> tuple[bool, str]:
    domain = arm.domain
    if domain == "support":
        domain = f"support-v{policy or 2}"
    result = run_sync(
        [sys.executable, str(ROOT / "validate-visible.py"), domain, str(unit.source), str(output)],
        check=False,
    )
    return result.returncode == 0, (result.stdout or result.stderr).strip()


def all_attempts_terminal_for_units(company: str, units: list[Unit]) -> bool:
    state = graph(company)
    expected = {str(unit.runtime_output) for unit in units}
    works = [row for row in state["work"] if row.get("expected_artifact") in expected]
    if len(works) < len(units):
        return False
    attempts_by_work = {
        row["work_id"] for row in state["attempts"] if row["state"] != "running"
    }
    if any(row["id"] not in attempts_by_work for row in works):
        return False
    running = {row["work_id"] for row in state["attempts"] if row["state"] == "running"}
    return not any(row["id"] in running for row in works)


def support_trigger_ready(company: str, units: list[Unit]) -> bool:
    state = graph(company)
    started = [row for row in state["attempts"] if row["state"] == "running"]
    if len(started) < 2:
        return False
    return any(docker_read(company, unit.progress) is not None for unit in units[:2])


def deliver_support_event(
    company: str,
    lead: str,
    result_dir: Path,
    effective_at: str,
) -> None:
    docker_copy(ROOT / "fixtures/support/POLICY-v2.md", company, RUNTIME_ROOT / "policy/support-v2.md")
    docker_copy(ROOT / "fixtures/support/events/material-policy-change.json", company, RUNTIME_ROOT / "events/material-policy-change.json")
    docker_copy(ROOT / "fixtures/support/events/worker-obstacle.json", company, RUNTIME_ROOT / "events/worker-obstacle.json")
    copy_text(company, RUNTIME_ROOT / "control/support-policy-version", "2\n", result_dir / "control")
    observation = result_dir / "control/support-event-observation.json"
    json_write(
        observation,
        {
            "effective_at": effective_at,
            "transport_authenticated": False,
            "source": "controlled EXP-05 _test event",
        },
    )
    docker_copy(observation, company, RUNTIME_ROOT / "events/support-event-observation.json")
    send_controlled_event(
        company,
        lead,
        f"The frozen policy event is now delivered. It was effective at {effective_at}, not at delivery. Read {RUNTIME_ROOT}/events/support-event-observation.json, material-policy-change.json, worker-obstacle.json and policy v2. Redirect only affected Work through attributable Work-linked feedback; do no production.",
    )


async def arrival_schedule(
    arm: Arm,
    company: str,
    lead: str,
    units: list[Unit],
    start: float,
    record: list[dict[str, object]],
) -> None:
    for unit in units:
        if unit.offset_seconds == 0:
            continue
        due = start + unit.offset_seconds
        remaining = due - time.monotonic()
        if remaining > 0:
            await asyncio.sleep(remaining)
        reveal(company, unit)
        observed = utc_now()
        send_controlled_event(
            company,
            lead,
            f"{unit.name} arrived at {observed}; exact source {unit.runtime_source}. Commission it once under the frozen charter.",
        )
        record.append({"unit": unit.name, "offset_seconds": unit.offset_seconds, "observed_at": observed})


async def collect_receipts(
    arm: Arm,
    company: str,
    units: list[Unit],
    result_dir: Path,
    deadline: float,
    policy: int | None = None,
) -> list[dict[str, object]]:
    receipts: dict[int, dict[str, object]] = {}
    outputs = result_dir / "outputs"
    while len(receipts) < len(units):
        for unit in units:
            if unit.number in receipts:
                continue
            local = outputs / unit.output_name
            if not output_snapshot(company, unit, local):
                continue
            valid, detail = validate_local_unit(arm, unit, local, policy)
            if valid:
                receipts[unit.number] = {
                    "unit": unit.name,
                    "output": unit.output_name,
                    "accepted_at": utc_now(),
                    "arrival_offset_seconds": unit.offset_seconds,
                    "validator": json.loads(detail),
                    "sha256": hashlib.sha256(local.read_bytes()).hexdigest(),
                }
        if len(receipts) == len(units):
            break
        if time.monotonic() >= deadline:
            raise SafetyEnvelopeExceeded(
                f"safety envelope expired with {len(receipts)}/{len(units)} exact validator receipts"
            )
        await asyncio.sleep(0.75)
    return [receipts[index] for index in sorted(receipts)]


def source_ids(arm: Arm, unit: Unit) -> list[str]:
    rows = json.loads(unit.source.read_text())
    key = "entity" if arm.domain == "monitoring" else "id"
    return sorted({str(row[key]) for row in rows})


def full_verify(arm: Arm, company: str, units: list[Unit], result_dir: Path) -> dict[str, object]:
    state = graph(company)
    files = [result_dir / "outputs" / unit.output_name for unit in units]
    ownership = {unit.output_name: source_ids(arm, unit) for unit in units}
    ownership_path = result_dir / "ownership.json"
    json_write(ownership_path, ownership)
    domain = "support-v2" if arm.domain == "support" else arm.domain
    index = result_dir / "deterministic-index.json"
    result = run_sync(
        [sys.executable, str(ROOT / "verify.py"), domain, *map(str, files), "--ownership", str(ownership_path), "--index", str(index)],
        check=False,
    )
    exact = json.loads(result.stdout) if result.returncode == 0 else {"valid": False, "error": result.stderr.strip()}
    expected_paths = {str(unit.runtime_output): unit for unit in units}
    work_by_path = {row.get("expected_artifact"): row for row in state["work"] if row.get("expected_artifact") in expected_paths}
    attribution_errors: list[str] = []
    for path, unit in expected_paths.items():
        work = work_by_path.get(path)
        if not work:
            attribution_errors.append(f"{path}: no Work owner")
            continue
        attempts = [row for row in state["attempts"] if row["work_id"] == work["id"]]
        attempt_ids = {row["id"] for row in attempts}
        artifacts = [row for row in state["artifacts"] if row.get("uri") == path and row.get("attempt_id") in attempt_ids]
        if not artifacts:
            attribution_errors.append(f"{path}: no attributable artifact")
        elif any(row.get("created_by") != work["owner_id"] for row in artifacts):
            attribution_errors.append(f"{path}: artifact actor differs from Work owner")
    return {"exact": exact, "attribution_valid": not attribution_errors, "attribution_errors": attribution_errors}


def parse_time(value: str) -> float:
    return datetime.fromisoformat(value.replace("Z", "+00:00")).timestamp()


def percentile(values: list[float], q: float) -> float | None:
    if not values:
        return None
    ordered = sorted(values)
    index = (len(ordered) - 1) * q
    lower = math.floor(index)
    upper = math.ceil(index)
    if lower == upper:
        return ordered[lower]
    return ordered[lower] * (upper - index) + ordered[upper] * (index - lower)


def interval_summary(intervals: list[tuple[float, float]]) -> dict[str, float]:
    summed = sum(end - start for start, end in intervals)
    merged: list[list[float]] = []
    for start, end in sorted(intervals):
        if not merged:
            merged = [[start, end]]
        elif start <= merged[-1][1]:
            merged[-1][1] = max(merged[-1][1], end)
        else:
            merged.append([start, end])
    return {
        "summed": summed,
        "union": sum(end - start for start, end in merged),
    }


def model_turn_intervals(
    history: list[dict[str, Any]], actors: set[str]
) -> list[tuple[float, float]]:
    pending: dict[str, list[float]] = {actor: [] for actor in actors}
    intervals: list[tuple[float, float]] = []
    for row in sorted(history, key=lambda event: parse_time(event["created_at"])):
        actor = row.get("actor_id")
        if actor not in actors:
            continue
        if row["kind"] == "model_session_ready":
            pending[actor].append(parse_time(row["created_at"]))
        elif row["kind"] == "turn_usage" and pending[actor]:
            intervals.append((pending[actor].pop(0), parse_time(row["created_at"])))
    return intervals


def sales_value_metrics(
    arm: Arm,
    receipts: list[dict[str, object]],
    start_wall: float,
) -> dict[str, object]:
    if arm.domain != "sales" or not arm.demand:
        accepted = sum(int(row["validator"]["units"]) for row in receipts)
        return {
            "value_adjusted_units": float(accepted),
            "initial_value": None,
            "accepted_value": None,
            "missed_service_value": None,
            "high_consequence_retained_fraction": None,
        }
    accepted_by_batch = {
        int(str(row["unit"]).rsplit(" ", 1)[-1]): parse_time(str(row["accepted_at"]))
        for row in receipts
    }
    curves = json.loads((ROOT / f"fixtures/sales/arrivals/{arm.demand}.json").read_text())
    ratios: list[float] = []
    initial_value = 0.0
    accepted_value = 0.0
    high_ratios: list[float] = []
    for row in curves:
        batch = int(row["batch"])
        delay = max(
            0.0,
            accepted_by_batch[batch]
            - (start_wall + int(row["arrival_offset_seconds"])),
        )
        half_life = row.get("response_half_life_seconds")
        ratio = 1.0 if half_life is None else 2 ** (-delay / float(half_life))
        value = float(row.get("initial_value", 1.0))
        ratios.append(ratio)
        initial_value += value
        accepted_value += value * ratio
        if row["value_curve"] == "high-consequence":
            high_ratios.append(ratio)
    return {
        "value_adjusted_units": sum(ratios),
        "initial_value": initial_value,
        "accepted_value": accepted_value,
        "missed_service_value": initial_value - accepted_value,
        "high_consequence_retained_fraction": (
            statistics.mean(high_ratios) if high_ratios else None
        ),
    }


def metrics(
    company: str,
    receipts: list[dict[str, object]],
    start_wall: float,
    *,
    arm: Arm | None = None,
    leads: set[str] | None = None,
    workers: set[str] | None = None,
) -> dict[str, object]:
    state = graph(company)
    history = events(company)
    latencies = [parse_time(str(row["accepted_at"])) - (start_wall + int(row["arrival_offset_seconds"])) for row in receipts]
    attempts = state["attempts"]
    leads = leads or set()
    workers = workers or {
        str(row["actor_id"]) for row in attempts if row.get("actor_id") != "exec"
    }
    intervals = [
        (parse_time(row["started_at"]), parse_time(row["finished_at"]))
        for row in attempts
        if row.get("finished_at") and row.get("actor_id") in workers
    ]
    worker_time = interval_summary(intervals)
    lead_time = interval_summary(model_turn_intervals(history, leads))
    usage = [row for row in history if row["kind"] == "turn_usage"]
    readiness = [row for row in history if row["kind"] == "model_session_ready"]
    accepted_units = sum(int(row["validator"]["units"]) for row in receipts)
    queue_seconds = max(parse_time(str(row["accepted_at"])) for row in receipts) - start_wall
    outcome_end = max(
        [
            parse_time(str(row["accepted_at"])) for row in receipts
        ]
        + [
            parse_time(row["created_at"])
            for row in usage
            if row.get("actor_id") in leads
        ]
    )
    outcome_seconds = outcome_end - start_wall
    value = sales_value_metrics(arm, receipts, start_wall) if arm else {
        "value_adjusted_units": float(accepted_units),
        "initial_value": None,
        "accepted_value": None,
        "missed_service_value": None,
        "high_consequence_retained_fraction": None,
    }
    spend = cli_json("spend", "-c", company)
    estimated_list_cost = sum(
        float(row["body"]["estimated_list_cost_usd"])
        for row in usage
        if row["body"].get("estimated_list_cost_usd") is not None
    )
    value_adjusted = float(value["value_adjusted_units"])
    return {
        "accepted_units": accepted_units,
        "queue_seconds": queue_seconds,
        "outcome_review_seconds": outcome_seconds,
        "unit_latency_seconds": {
            "p50": percentile(latencies, 0.50),
            "p90": percentile(latencies, 0.90),
            "p99": percentile(latencies, 0.99),
            "max": max(latencies) if latencies else None,
        },
        "accepted_units_per_request_hour": (
            accepted_units * 3600 / queue_seconds if queue_seconds > 0 else None
        ),
        "accepted_units_per_active_worker_hour": (
            accepted_units * 3600 / worker_time["summed"]
            if worker_time["summed"] > 0 else None
        ),
        "value": value,
        "value_adjusted_units_per_request_hour": (
            value_adjusted * 3600 / queue_seconds if queue_seconds > 0 else None
        ),
        "value_adjusted_units_per_active_worker_hour": (
            value_adjusted * 3600 / worker_time["summed"]
            if worker_time["summed"] > 0 else None
        ),
        "worker_active_seconds": worker_time,
        "peak_staff_attempt_concurrency": peak_attempt_concurrency(attempts, workers),
        "peak_staff_model_concurrency": peak_model_session_concurrency(history, workers)[0],
        "unterminated_staff_model_sessions": peak_model_session_concurrency(history, workers)[1],
        "lead_active_seconds": lead_time,
        "lead_active_fraction_of_outcome_window": (
            lead_time["summed"] / outcome_seconds if outcome_seconds > 0 else None
        ),
        "attempts": len(attempts),
        "lead_wakes": sum(
            1
            for row in history
            if row["kind"] == "actor_wake_end" and row.get("actor_id") in leads
        ),
        "usage_events": len(usage),
        "observed_tokens": sum(
            int(row["body"].get("tokens") or 0) for row in usage
        ),
        "configured_efforts": sorted({row["body"].get("configured_effort") for row in usage if row["body"].get("configured_effort")}),
        "models": sorted({row["body"].get("model") for row in usage if row["body"].get("model")}),
        "session_resumptions": sum(bool(row["body"].get("resumed")) for row in readiness),
        "spend": spend,
        "estimated_list_cost_usd": estimated_list_cost,
        "charged_cost_per_accepted_unit_usd": (
            float(spend["accounted_usd"]) / accepted_units if accepted_units else None
        ),
        "estimated_list_cost_per_accepted_unit_usd": (
            estimated_list_cost / accepted_units if accepted_units else None
        ),
        "billing_disposition": "charged metered API cost is authoritative; missing spend metering invalidates the cell",
    }


def support_change_metrics(
    state: dict[str, Any],
    event_record: dict[str, object],
    units: list[Unit],
    result_dir: Path,
) -> dict[str, object]:
    effective = parse_time(str(event_record["effective_at"]))
    delivered = parse_time(str(event_record["delivered_at"]))
    expected_paths = {str(unit.runtime_output) for unit in units}
    work_ids = {
        row["id"]
        for row in state["work"]
        if row.get("expected_artifact") in expected_paths
        or "support" in str(row.get("title", "")).lower()
        or "case" in str(row.get("title", "")).lower()
    }
    attempts = [row for row in state["attempts"] if row["work_id"] in work_ids]
    running_at_effective = [
        row
        for row in attempts
        if parse_time(row["started_at"]) <= effective
        and (
            row.get("finished_at") is None
            or parse_time(row["finished_at"]) >= effective
        )
    ]
    replacement_starts = [
        parse_time(row["started_at"])
        for row in attempts
        if parse_time(row["started_at"]) >= delivered
    ]
    stale_finishes = [
        parse_time(row["finished_at"])
        for row in running_at_effective
        if row.get("finished_at") and parse_time(row["finished_at"]) >= delivered
    ]
    terminal_v1 = result_dir / "terminal-v1"
    v1_outputs = len(list(terminal_v1.glob("batch-*.json"))) if terminal_v1.is_dir() else 0
    attempt_ids = {row["id"] for row in attempts}
    return {
        "running_attempts_at_effective_time": len(running_at_effective),
        "first_replacement_attempt_from_effective_seconds": (
            min(replacement_starts) - effective if replacement_starts else None
        ),
        "first_stale_attempt_end_from_delivery_seconds": (
            min(stale_finishes) - delivered if stale_finishes else None
        ),
        "attempt_feedback_inputs": sum(
            row["attempt_id"] in attempt_ids for row in state["attempt_feedback"]
        ),
        "repair_attempts": max(0, len(attempts) - len(work_ids)),
        "withheld_v1_first_pass_outputs": v1_outputs,
        "final_stale_outputs_accepted": 0,
        "final_policy_version": 2,
    }


def blind_packet(arm: Arm, result_dir: Path) -> Path:
    packet = result_dir / "blind-packet"
    if packet.exists():
        shutil.rmtree(packet)
    packet.mkdir(parents=True)
    shutil.copy(ROOT / "blind-evaluation-rubric.md", packet / "rubric.md")
    shutil.copy(result_dir / "deterministic-index.json", packet / "candidate.json")
    if arm.wave == 4:
        source_units = units_for(arm)
        if arm.domain != "operations":
            source_units = source_units[:1]
        source_rows = [
            row
            for unit in source_units
            for row in json.loads(unit.source.read_text())
        ]
        json_write(packet / "source.json", source_rows)
    elif arm.domain == "sales":
        shutil.copy(ROOT / "fixtures/sales/data/accounts.json", packet / "source.json")
    elif arm.domain == "support":
        shutil.copy(ROOT / "fixtures/support/data/cases.json", packet / "source.json")
    elif arm.domain == "monitoring":
        shutil.copy(ROOT / "fixtures/monitoring/corpus/documents.json", packet / "source.json")
    elif arm.domain == "operations":
        shutil.copy(ROOT / "fixtures/operations/data/invoices.json", packet / "source.json")
    if arm.domain == "sales":
        shutil.copy(ROOT / "fixtures/sales/POLICY.md", packet / "policy.md")
    elif arm.domain == "support":
        shutil.copy(ROOT / "fixtures/support/POLICY-v2.md", packet / "policy.md")
    (packet / "owner-contract.md").write_text(
        "Judge the candidate as a locally closing fictional business outcome. Nothing was sent or applied. "
        "Return one JSON object with numeric scores, consequential_defects, evidence, and decision.\n"
    )
    return packet


def validate_blind_judgement(value: object) -> list[str]:
    if not isinstance(value, dict):
        return ["judgement is not a JSON object"]
    expected_top = {
        "scores", "worst_unit", "high_consequence_breach",
        "consequential_defects", "evidence", "decision",
    }
    errors: list[str] = []
    if set(value) != expected_top:
        errors.append("top-level fields differ from frozen schema")
    score_fields = {
        "usefulness", "grounding", "safe_actionability", "tail_handling",
        "uncertainty_calibration", "native_review_readiness",
    }
    scores = value.get("scores")
    if not isinstance(scores, dict) or set(scores) != score_fields:
        errors.append("score fields differ from frozen schema")
    else:
        for name, score in scores.items():
            if isinstance(score, bool) or not isinstance(score, (int, float)) or not 0 <= score <= 10:
                errors.append(f"{name} score is outside 0..10")
    worst = value.get("worst_unit")
    if not isinstance(worst, dict) or set(worst) != {"id", "score", "defect"}:
        errors.append("worst_unit differs from frozen schema")
    elif (
        isinstance(worst["score"], bool)
        or not isinstance(worst["score"], (int, float))
        or not 0 <= worst["score"] <= 10
    ):
        errors.append("worst_unit score is outside 0..10")
    if not isinstance(value.get("high_consequence_breach"), bool):
        errors.append("high_consequence_breach is not boolean")
    for field in ("consequential_defects", "evidence"):
        if not isinstance(value.get(field), list) or not all(
            isinstance(item, str) for item in value.get(field, [])
        ):
            errors.append(f"{field} is not a string array")
    if value.get("decision") not in {"accept", "repair", "reject"}:
        errors.append("decision is not accept, repair, or reject")
    return errors


def run_blind_evaluator(arm: Arm, result_dir: Path) -> dict[str, object]:
    packet = blind_packet(arm, result_dir)
    prompt = (
        "Read the supplied frozen owner contract, source, policy when present, rubric and candidate. "
        "Do not infer producer identity or topology. Return JSON only."
    )
    args = [
        "omp", "-p", "--profile", "restless-model-broker", "--model", CONTRACT["models"]["blind_evaluator"]["selector"],
        "--thinking", CONTRACT["models"]["blind_evaluator"]["effort"], "--no-tools", "--no-session",
        "--system-prompt", "You are a blinded outcome evaluator. Never reveal private reasoning; return only the requested structured judgement.",
        f"@{packet / 'rubric.md'}", f"@{packet / 'owner-contract.md'}", f"@{packet / 'source.json'}",
    ]
    policy = packet / "policy.md"
    if policy.exists():
        args.append(f"@{policy}")
    args.extend([f"@{packet / 'candidate.json'}", prompt])
    result = run_sync(args, check=False)
    raw = result.stdout.strip()
    parsed: object = {"raw": raw, "stderr": result.stderr.strip()}
    if result.returncode == 0:
        try:
            parsed = json.loads(raw)
        except json.JSONDecodeError:
            match = re.search(r"\{.*\}", raw, re.DOTALL)
            if match:
                try:
                    parsed = json.loads(match.group(0))
                except json.JSONDecodeError:
                    pass
    validation_errors = validate_blind_judgement(parsed)
    return {
        "valid": result.returncode == 0 and not validation_errors,
        "validation_errors": validation_errors,
        "process_exit": result.returncode,
        "model": CONTRACT["models"]["blind_evaluator"],
        "judgement": parsed,
    }


def final_review_pair(
    history: list[dict[str, Any]], lead: str, after_epoch: float
) -> tuple[dict[str, Any], dict[str, Any]] | None:
    """Find material lead-to-Exec closure even if a cosmetic wake-end is absent."""
    ordered = sorted(history, key=lambda row: (parse_time(row["created_at"]), int(row["id"])))
    for index, row in enumerate(ordered):
        if (
            row["kind"] != "wake"
            or row.get("actor_id") != "exec"
            or parse_time(row["created_at"]) < after_epoch
            or row.get("body", {}).get("reason") != f"message from {lead}"
        ):
            continue
        exec_end = next(
            (
                candidate
                for candidate in ordered[index + 1 :]
                if candidate["kind"] == "wake_end"
                and candidate.get("actor_id") == "exec"
            ),
            None,
        )
        if exec_end is None:
            continue
        exec_end_epoch = parse_time(exec_end["created_at"])
        lead_ends = [
            candidate
            for candidate in ordered
            if candidate.get("actor_id") == lead
            and candidate["kind"] in {"actor_wake_end", "turn_usage"}
            and after_epoch <= parse_time(candidate["created_at"]) <= exec_end_epoch
        ]
        if lead_ends:
            return lead_ends[-1], exec_end
    return None


async def finalize_completed_arm(
    *,
    arm: Arm,
    company: str,
    lead: str,
    workers: list[str],
    units: list[Unit],
    result_dir: Path,
    receipts: list[dict[str, object]],
    start_wall: float,
    first_started_at: str,
    tell_started: str,
    tell_result: tuple[int, str, str],
    arrivals: list[dict[str, object]],
    event_record: dict[str, object] | None,
    deadline: float,
    skip_blind: bool,
    send_validator_callback: bool,
    preserved_blind: dict[str, object] | None = None,
) -> dict[str, object]:
    """Finalize already-produced native evidence without replaying production."""
    receipt_payload = {
        "kind": "exact_validator_complete",
        "arm": arm.arm_id,
        "units": sum(int(row["validator"]["units"]) for row in receipts),
        "receipts": receipts,
        "created_at": utc_now(),
    }
    json_write(result_dir / "validator-receipts.json", receipt_payload)
    docker_copy(result_dir / "validator-receipts.json", company, RUNTIME_ROOT / "control/validator-receipts.json")
    validation_complete_at = max(str(row["accepted_at"]) for row in receipts)
    validation_complete_epoch = parse_time(validation_complete_at)
    callback_at: str | None = None
    if send_validator_callback:
        callback_at = utc_now()
        send_controlled_event(
            company,
            lead,
            f"Every exact unit validator receipt now exists at {RUNTIME_ROOT}/control/validator-receipts.json. Perform the frozen read-only native review and make one outcome judgement; do not rewrite or fan in outputs.",
        )
    # Product completion callbacks may naturally wake the lead before this
    # observer copies its redundant receipt bundle. The evidence boundary is
    # exact acceptance, not controller message ordering.
    review_event, exec_review_event = await wait_until(
        "post-acceptance lead-to-Exec judgement",
        lambda: final_review_pair(events(company), lead, validation_complete_epoch),
        deadline,
    )
    exact = full_verify(arm, company, units, result_dir)
    snapshot_events = events(company)
    snapshot_graph = graph(company)
    json_write(result_dir / "events.json", snapshot_events)
    json_write(result_dir / "work-graph.json", snapshot_graph)
    measured = metrics(
        company,
        receipts,
        start_wall,
        arm=arm,
        leads={lead},
        workers=set(workers),
    )
    measured.update({
        "tell_started_at": tell_started,
        "first_staff_attempt_at": first_started_at,
        "exec_to_staff_dispatch_seconds": parse_time(first_started_at) - parse_time(tell_started),
        "exact_validation_completed_at": validation_complete_at,
        "validator_callback_at": callback_at,
        "lead_review_completed_at": review_event["created_at"],
        "lead_review_seconds": parse_time(review_event["created_at"]) - validation_complete_epoch,
        "exec_judgement_completed_at": exec_review_event["created_at"],
        "arrivals": arrivals,
        "support_event": event_record,
    })
    if event_record is not None:
        measured["support_change"] = support_change_metrics(
            snapshot_graph, event_record, units, result_dir
        )
    json_write(result_dir / "metrics.json", measured)
    blind = preserved_blind
    if blind is None and not skip_blind and exact["exact"].get("valid"):
        blind = run_blind_evaluator(arm, result_dir)
    if blind is not None:
        json_write(result_dir / "blind-evaluation.json", blind)
    evaluation_valid = skip_blind or (blind is not None and bool(blind.get("valid")))
    expected_concurrency = 4 if arm.arm_id == "wave0-q4-admission" else 1
    runtime_valid = (
        set(measured["models"]) == {LEAD_MODEL, STAFF_MODEL}
        and measured["configured_efforts"] == [PRODUCTION_EFFORT]
        and measured["peak_staff_model_concurrency"] >= expected_concurrency
        and measured["unterminated_staff_model_sessions"] == 0
    )
    result = {
        "arm": arm.arm_id,
        "company": company,
        "validity": "counted" if exact["exact"].get("valid") and exact["attribution_valid"] and evaluation_valid and runtime_valid else "invalid",
        "exact_evaluation": exact,
        "blind_evaluation": blind,
        "runtime_valid": runtime_valid,
        "metrics": measured,
        "tell_process": {"exit": tell_result[0], "stdout": tell_result[1], "stderr": tell_result[2]},
        "finished_at": utc_now(),
    }
    json_write(result_dir / "run-result.json", result)
    return result


async def run_arm(arm_id: str, run_token: str, wall_seconds: int, skip_blind: bool) -> dict[str, object]:
    arm = Arm.load(arm_id)
    if arm.domain == "company":
        raise RunFailure("Wave 4 uses the dedicated company runner; invoke `run-wave4`")
    result_dir = RESULTS / f"{arm_id}-{run_token}"
    if result_dir.exists():
        raise RunFailure(f"result directory already exists: {result_dir}")
    result_dir.mkdir(parents=True)
    json_write(result_dir / "run-manifest.json", {"arm": arm.__dict__, "run_token": run_token, "started_at": utc_now(), "counted_path": "restlessd + OrgIntel scheduler + scoped ACP", "models": CONTRACT["models"]})
    company, lead, workers, units = provision(arm, run_token, result_dir)
    deadline = time.monotonic() + wall_seconds
    arrivals: list[dict[str, object]] = []
    for unit in units:
        if unit.offset_seconds == 0:
            reveal(company, unit)
            arrivals.append({"unit": unit.name, "offset_seconds": 0, "observed_at": utc_now()})
    tell_started = utc_now()
    owner_text = (
        f"Start counted fictional _test arm {arm.arm_id}. This requires productive execution. "
        f"Delegate exactly once to existing accountable lead `{lead}` under the existing team and exact charter at {RUNTIME_ROOT}/charter.md, then return to portfolio availability. "
        "Do not produce, choose Staff assignments, create a second lead, change models, or wait for delegated work. Nothing may be sent or applied."
    )
    tell_task = asyncio.create_task(run_async([str(CLI), "tell", "-c", company, owner_text]))
    first_started_at = await wait_until("first Staff Attempt", lambda: first_attempt_time(company), deadline)
    start_wall = parse_time(first_started_at)
    start_mono = time.monotonic()
    arrival_task = asyncio.create_task(arrival_schedule(arm, company, lead, units, start_mono, arrivals))

    event_record: dict[str, object] | None = None
    if arm.domain == "support":
        await wait_until("two support Attempts and attributable produce marker", lambda: support_trigger_ready(company, units), deadline, 0.2)
        event_record = {"effective_at": utc_now(), "delivery": arm.supervision}
        if arm.supervision == "causal":
            deliver_support_event(company, lead, result_dir, str(event_record["effective_at"]))
            event_record["delivered_at"] = utc_now()
        else:
            await wait_until(
                "terminal first-pass support outputs",
                lambda: all_attempts_terminal_for_units(company, units)
                and all(
                    output_snapshot(company, unit, result_dir / "terminal-v1" / unit.output_name)
                    and validate_local_unit(arm, unit, result_dir / "terminal-v1" / unit.output_name, 1)[0]
                    for unit in units
                ),
                deadline,
            )
            deliver_support_event(company, lead, result_dir, str(event_record["effective_at"]))
            event_record["delivered_at"] = utc_now()

    await arrival_task
    receipts = await collect_receipts(arm, company, units, result_dir, deadline, 2 if arm.domain == "support" else None)
    tell_result = await await_process("Exec delegation process", tell_task, deadline)
    return await finalize_completed_arm(
        arm=arm,
        company=company,
        lead=lead,
        workers=workers,
        units=units,
        result_dir=result_dir,
        receipts=receipts,
        start_wall=start_wall,
        first_started_at=first_started_at,
        tell_started=tell_started,
        tell_result=tell_result,
        arrivals=arrivals,
        event_record=event_record,
        deadline=deadline,
        skip_blind=skip_blind,
        send_validator_callback=True,
    )


async def finalize_existing_arm(
    arm_id: str, run_token: str, wall_seconds: int, skip_blind: bool
) -> dict[str, object]:
    """Recover a completed cell after its observer stopped during finalization."""
    arm = Arm.load(arm_id)
    if arm.domain == "company":
        raise RunFailure(
            "Wave 4 requires its dedicated runner because it contains several companies"
        )
    result_dir = RESULTS / f"{arm_id}-{run_token}"
    if not result_dir.exists():
        raise RunFailure(f"result directory does not exist: {result_dir}")
    existing_result_path = result_dir / "run-result.json"
    existing_result: dict[str, Any] | None = None
    preserved_blind: dict[str, object] | None = None
    if existing_result_path.exists():
        existing_result = json.loads(existing_result_path.read_text())
        blind = existing_result.get("blind_evaluation")
        blind_retry = (
            existing_result.get("validity") == "invalid"
            and isinstance(blind, dict)
            and blind.get("valid") is False
        )
        telemetry_refresh = (
            existing_result.get("validity") == "invalid"
            and existing_result.get("runtime_valid") is False
            and isinstance(blind, dict)
            and blind.get("valid") is True
            and existing_result.get("exact_evaluation", {}).get("exact", {}).get("valid") is True
            and existing_result.get("exact_evaluation", {}).get("attribution_valid") is True
            and existing_result.get("metrics", {}).get("unterminated_staff_model_sessions", 0) > 0
        )
        if not blind_retry and not telemetry_refresh:
            raise RunFailure(f"run is already finalized: {existing_result_path}")
        if blind_retry:
            retry_path = result_dir / "blind-invalid-run-result.json"
            if retry_path.exists():
                raise RunFailure(
                    f"blind evaluator already failed twice for this cell: {retry_path}"
                )
            shutil.copy(existing_result_path, retry_path)
        else:
            preserved_blind = blind
    elif arm.domain == "support":
        raise RunFailure(
            "support finalization requires its preserved event-bearing run result"
        )
    company = company_name(arm.arm_id, run_token)
    if company not in set(cli_json("company", "list")):
        raise RunFailure(f"product company is unavailable: {company}")
    lead, workers, _ = actor_ids(arm.domain, arm.workers)
    units = units_for(arm)
    deadline = time.monotonic() + wall_seconds
    receipt_path = result_dir / "validator-receipts.json"
    had_receipt_bundle = receipt_path.exists()
    if had_receipt_bundle:
        receipts = json.loads(receipt_path.read_text())["receipts"]
    else:
        receipts = await collect_receipts(arm, company, units, result_dir, deadline)
    if len(receipts) != len(units):
        raise RunFailure(
            f"cannot finalize {arm_id}: {len(receipts)} exact receipts for {len(units)} units"
        )
    first_started_at = first_attempt_time(company)
    if first_started_at is None:
        raise RunFailure(f"cannot finalize {arm_id}: no Staff Attempt exists")
    start_wall = parse_time(first_started_at)
    history = events(company)
    tell_started = (
        existing_result.get("metrics", {}).get("tell_started_at")
        if existing_result is not None
        else None
    ) or min(
        (
            row["created_at"]
            for row in history
            if row["kind"] == "wake" and row.get("actor_id") == "exec"
        ),
        key=parse_time,
        default=json.loads((result_dir / "run-manifest.json").read_text())["started_at"],
    )
    arrivals = (
        existing_result.get("metrics", {}).get("arrivals")
        if existing_result is not None
        else None
    ) or [
        {
            "unit": unit.name,
            "offset_seconds": unit.offset_seconds,
            "observed_at": datetime.fromtimestamp(
                start_wall + unit.offset_seconds, timezone.utc
            ).isoformat(),
            "reconstructed": True,
        }
        for unit in units
    ]
    event_record = (
        existing_result.get("metrics", {}).get("support_event")
        if existing_result is not None and arm.domain == "support"
        else None
    )
    return await finalize_completed_arm(
        arm=arm,
        company=company,
        lead=lead,
        workers=workers,
        units=units,
        result_dir=result_dir,
        receipts=receipts,
        start_wall=start_wall,
        first_started_at=first_started_at,
        tell_started=tell_started,
        tell_result=(0, "recovered completed product evidence; production was not replayed", ""),
        arrivals=arrivals,
        event_record=event_record,
        deadline=deadline,
        skip_blind=skip_blind,
        send_validator_callback=not had_receipt_bundle,
        preserved_blind=preserved_blind,
    )


async def await_process(
    description: str,
    task: asyncio.Task[tuple[int, str, str]],
    deadline: float,
) -> tuple[int, str, str]:
    remaining = deadline - time.monotonic()
    if remaining <= 0:
        raise SafetyEnvelopeExceeded(f"safety envelope expired while waiting for {description}")
    try:
        return await asyncio.wait_for(task, timeout=remaining)
    except TimeoutError as error:
        raise SafetyEnvelopeExceeded(
            f"safety envelope expired while waiting for {description}"
        ) from error


def peak_attempt_concurrency(attempts: list[dict[str, Any]], actors: set[str]) -> int:
    boundaries: list[tuple[float, int]] = []
    for row in attempts:
        if row.get("actor_id") not in actors or not row.get("finished_at"):
            continue
        boundaries.append((parse_time(row["started_at"]), 1))
        boundaries.append((parse_time(row["finished_at"]), -1))
    running = 0
    peak = 0
    # An Attempt finishing at the same instant another starts does not overlap.
    for _, delta in sorted(boundaries, key=lambda item: (item[0], item[1])):
        running += delta
        peak = max(peak, running)
    return peak


def peak_model_session_concurrency(
    history: list[dict[str, Any]], actors: set[str]
) -> tuple[int, int]:
    """Measure actual ACP overlap, not Attempts waiting before model admission.

    A normal turn closes on its usage snapshot. An intentionally interrupted
    turn may emit no ACP usage notification even though the supervised process
    and host-owned metering drain both terminate; its Attempt process end is
    therefore the exact local close boundary.
    """
    open_sessions: dict[str, list[float]] = {actor: [] for actor in actors}
    boundaries: list[tuple[float, int]] = []
    for row in sorted(history, key=lambda event: (parse_time(event["created_at"]), int(event["id"]))):
        actor = row.get("actor_id")
        if actor not in actors:
            continue
        when = parse_time(row["created_at"])
        if row["kind"] == "model_session_ready":
            open_sessions[actor].append(when)
        elif row["kind"] in {"turn_usage", "attempt_process_ended"} and open_sessions[actor]:
            started = open_sessions[actor].pop(0)
            boundaries.append((started, 1))
            boundaries.append((when, -1))
    running = 0
    peak = 0
    for _, delta in sorted(boundaries, key=lambda item: (item[0], item[1])):
        running += delta
        peak = max(peak, running)
    return peak, sum(len(starts) for starts in open_sessions.values())


async def run_wave4(run_token: str, wall_seconds: int, skip_blind: bool) -> dict[str, object]:
    """Run the frozen four-department company through one production scheduler."""
    arm = Arm.load("company-q1x4-r1")
    result_dir = RESULTS / f"{arm.arm_id}-{run_token}"
    if result_dir.exists():
        raise RunFailure(f"result directory already exists: {result_dir}")
    result_dir.mkdir(parents=True)
    company = company_name(arm.arm_id, run_token)
    programme_spend = admit_program_cell(20)
    json_write(result_dir / "control/programme-spend-at-admission.json", programme_spend)
    config = result_dir / "control/company.toml"
    create_company_config(config, company, arm, 20)
    cli("company", "create", "--from-file", str(config))
    cli("up", "-c", company)

    sales_arm = Arm("company-sales", 4, "sales", "q1", 1, demand="D0")
    support_arm = Arm("company-support", 4, "support", "q1", 1)
    monitoring_arm = Arm("company-monitoring", 4, "monitoring", "q1", 1)
    operations_arm = Arm("company-operations", 4, "operations", "q1", 1)
    plans: dict[str, dict[str, Any]] = {
        "sales": {
            "arm": sales_arm,
            "lead": "revenue-direction",
            "worker": "account-owner",
            "team": "Revenue Outcome",
            "units": units_for(sales_arm)[:1],
        },
        "support": {
            "arm": support_arm,
            "lead": "customer-direction",
            "worker": "case-owner",
            "team": "Customer Outcome",
            "units": units_for(support_arm)[:1],
        },
        "monitoring": {
            "arm": monitoring_arm,
            "lead": "intelligence-direction",
            "worker": "monitoring-owner",
            "team": "Intelligence Outcome",
            "units": units_for(monitoring_arm)[:1],
        },
        "operations": {
            "arm": operations_arm,
            "lead": "operations-direction",
            "worker": "reconciliation-owner",
            "team": "Operations Outcome",
            "units": units_for(operations_arm),
        },
    }
    json_write(
        result_dir / "run-manifest.json",
        {
            "arm": arm.__dict__,
            "run_token": run_token,
            "started_at": utc_now(),
            "counted_path": "restlessd + OrgIntel scheduler + scoped ACP",
            "models": CONTRACT["models"],
            "frozen_slices": {
                domain: [unit.name for unit in plan["units"]]
                for domain, plan in plans.items()
            },
        },
    )
    cli(
        "attach", "-c", company, "mkdir", "-p",
        str(RUNTIME_ROOT / "tools"), str(RUNTIME_ROOT / "control"),
        str(RUNTIME_ROOT / "charters"), str(RUNTIME_ROOT / "inbox"),
        str(RUNTIME_ROOT / "outputs"), str(RUNTIME_ROOT / "progress"),
        str(RUNTIME_ROOT / "policy"),
    )
    docker_copy(ROOT / "validate-visible.py", company, RUNTIME_ROOT / "tools/validate-visible.py")
    docker_copy(ROOT / "fixtures/sales/POLICY.md", company, RUNTIME_ROOT / "policy/sales.md")
    docker_copy(ROOT / "fixtures/sales/README.md", company, RUNTIME_ROOT / "policy/sales-README.md")
    docker_copy(ROOT / "fixtures/support/POLICY-v2.md", company, RUNTIME_ROOT / "policy/support-v2.md")
    docker_copy(ROOT / "fixtures/support/README.md", company, RUNTIME_ROOT / "policy/support-README.md")
    docker_copy(ROOT / "fixtures/monitoring/README.md", company, RUNTIME_ROOT / "policy/monitoring-README.md")
    docker_copy(ROOT / "fixtures/operations/README.md", company, RUNTIME_ROOT / "policy/operations-README.md")
    copy_text(company, RUNTIME_ROOT / "control/support-policy-version", "2\n", result_dir / "control")

    for domain, plan in plans.items():
        lead = plan["lead"]
        worker = plan["worker"]
        team = plan["team"]
        subarm = plan["arm"]
        units = plan["units"]
        charter_path = RUNTIME_ROOT / f"charters/{domain}.md"
        cli(
            "people", "create", "-c", company, "--id", lead,
            "--role", f"accountable {domain} outcome lead",
            "--display", actor_display(lead), "--model", LEAD_MODEL,
            "--reason", "frozen EXP-05 company-level accountability boundary",
        )
        cli(
            "people", "create", "-c", company, "--id", worker,
            "--role", f"{domain} end-to-end producer",
            "--display", actor_display(worker), "--model", STAFF_MODEL,
            "--reason", "frozen EXP-05 locally closing production owner",
        )
        cli(
            "teams", "create", "-c", company, "--name", team, "--lead", lead,
            "--brief", f"Own the exact {domain} slice at {charter_path}; lead never produces.",
        )
        cli(
            "teams", "assign", "-c", company, "--actor", worker, "--team", team,
            "--reason", "frozen EXP-05 one-worker department",
        )
        charter = result_dir / f"control/{domain}-charter.md"
        charter.write_text(
            charter_for(
                subarm,
                lead,
                [worker],
                units,
                include_support_event=False,
                inputs_visible_at_start=True,
            )
        )
        docker_copy(charter, company, charter_path)

    initial_domains = ("sales", "support", "monitoring")
    for domain in initial_domains:
        for unit in plans[domain]["units"]:
            reveal(company, unit)

    deadline = time.monotonic() + wall_seconds
    initial_tell_started = utc_now()
    initial_tell_task = asyncio.create_task(
        run_async(
            [
                str(CLI), "tell", "-c", company,
                "Start the three independent fictional _test outcomes now: sales, support and monitoring. "
                f"Delegate each exactly once to its existing accountable lead using {RUNTIME_ROOT}/charters/sales.md, "
                f"{RUNTIME_ROOT}/charters/support.md and {RUNTIME_ROOT}/charters/monitoring.md, then return to portfolio availability. "
                "Do no production, choose no Staff assignments, create no summary outcome and change no roster or model. Nothing may be sent or applied.",
            ]
        )
    )

    async def collect_and_review(domain: str) -> dict[str, object]:
        plan = plans[domain]
        domain_dir = result_dir / domain
        receipts = await collect_receipts(
            plan["arm"], company, plan["units"], domain_dir, deadline,
            2 if domain == "support" else None,
        )
        payload = {
            "kind": "exact_validator_complete",
            "domain": domain,
            "units": sum(int(row["validator"]["units"]) for row in receipts),
            "receipts": receipts,
            "created_at": utc_now(),
        }
        receipt_path = domain_dir / "validator-receipts.json"
        json_write(receipt_path, payload)
        runtime_receipt = RUNTIME_ROOT / f"control/{domain}-validator-receipts.json"
        docker_copy(receipt_path, company, runtime_receipt)
        callback_at = utc_now()
        send_controlled_event(
            company,
            plan["lead"],
            f"Every exact {domain} receipt now exists at {runtime_receipt}. Review only your native outcome and make one judgement; do not rewrite, summarize another department, or relay status.",
        )
        callback_epoch = parse_time(callback_at)
        review_event = await wait_until(
            f"{domain} accountable-lead review wake",
            lambda: next(
                (
                    row
                    for row in reversed(events(company))
                    if row["kind"] == "actor_wake_end"
                    and row.get("actor_id") == plan["lead"]
                    and parse_time(row["created_at"]) >= callback_epoch
                ),
                None,
            ),
            deadline,
        )
        return {
            "receipts": receipts,
            "callback_at": callback_at,
            "review_completed_at": review_event["created_at"],
            "review_seconds": parse_time(review_event["created_at"]) - callback_epoch,
        }

    collection_tasks = {
        domain: asyncio.create_task(collect_and_review(domain))
        for domain in initial_domains
    }
    initial_tell = await await_process("initial three-department Exec dispatch", initial_tell_task, deadline)
    initial_tell_completed_at = utc_now()
    if initial_tell[0] != 0:
        raise RunFailure(f"initial company dispatch failed: {initial_tell[2].strip()}")

    initial_workers = {plans[domain]["worker"] for domain in initial_domains}
    running_initial = await wait_until(
        "three concurrently running department Staff Attempts",
        lambda: running_attempt_times(company, initial_workers),
        deadline,
        0.2,
    )
    first_attempts: dict[str, str] = {
        domain: running_initial[plans[domain]["worker"]] for domain in initial_domains
    }

    for unit in plans["operations"]["units"]:
        reveal(company, unit)
    fourth_request_started = utc_now()
    operations_collection = asyncio.create_task(collect_and_review("operations"))
    fourth_tell_task = asyncio.create_task(
        run_async(
            [
                str(CLI), "tell", "-c", company,
                "A fourth independent fictional _test owner outcome has arrived: reconcile the 32 invoice/ledger pairs under "
                f"{RUNTIME_ROOT}/charters/operations.md. Delegate it exactly once to existing accountable lead `operations-direction` "
                "and return to portfolio availability. Do no production, disturb no existing department, create no company summary and change no roster or model. Nothing may be posted, paid or applied.",
            ]
        )
    )
    first_attempts["operations"] = await wait_until(
        "fourth-department Staff Attempt",
        lambda: actor_first_attempt_time(company, plans["operations"]["worker"]),
        deadline,
    )
    fourth_tell = await await_process("fourth-request Exec dispatch", fourth_tell_task, deadline)
    fourth_tell_completed_at = utc_now()
    if fourth_tell[0] != 0:
        raise RunFailure(f"fourth company dispatch failed: {fourth_tell[2].strip()}")

    collections: dict[str, dict[str, object]] = {}
    for domain, task in collection_tasks.items():
        collections[domain] = await task
    collections["operations"] = await operations_collection

    latest_review_epoch = max(
        parse_time(value["review_completed_at"]) for value in collections.values()
    )
    company_exec_review = await wait_until(
        "company Exec post-department-judgement wake",
        lambda: next(
            (
                row
                for row in reversed(events(company))
                if row["kind"] == "wake_end"
                and row.get("actor_id") == "exec"
                and parse_time(row["created_at"]) >= latest_review_epoch
            ),
            None,
        ),
        deadline,
    )

    exact: dict[str, object] = {}
    blind: dict[str, object] = {}
    all_receipts: list[dict[str, object]] = []
    for domain, plan in plans.items():
        domain_dir = result_dir / domain
        exact[domain] = full_verify(plan["arm"], company, plan["units"], domain_dir)
        all_receipts.extend(collections[domain]["receipts"])
        if not skip_blind and exact[domain]["exact"].get("valid"):
            blind[domain] = run_blind_evaluator(plan["arm"], domain_dir)

    state = graph(company)
    history = events(company)
    expected_owners = {
        str(unit.runtime_output): plan["worker"]
        for plan in plans.values()
        for unit in plan["units"]
    }
    leakage_errors = [
        f"{row.get('expected_artifact')}: owned by {row.get('owner_id')}, expected {expected_owners[row.get('expected_artifact')]}"
        for row in state["work"]
        if row.get("expected_artifact") in expected_owners
        and row.get("owner_id") != expected_owners[row.get("expected_artifact")]
    ]
    start_wall = min(parse_time(value) for value in first_attempts.values())
    measured = metrics(
        company,
        all_receipts,
        start_wall,
        leads={plan["lead"] for plan in plans.values()},
        workers={plan["worker"] for plan in plans.values()},
    )
    measured.update(
        {
            "initial_tell_started_at": initial_tell_started,
            "initial_tell_completed_at": initial_tell_completed_at,
            "fourth_request_started_at": fourth_request_started,
            "fourth_tell_completed_at": fourth_tell_completed_at,
            "first_staff_attempts": first_attempts,
            "fourth_dispatch_seconds": parse_time(first_attempts["operations"])
            - parse_time(fourth_request_started),
            "exec_returned_before_fourth_request": parse_time(initial_tell_completed_at)
            <= parse_time(fourth_request_started),
            "exec_cli_unavailable_seconds": (
                parse_time(initial_tell_completed_at) - parse_time(initial_tell_started)
                + parse_time(fourth_tell_completed_at) - parse_time(fourth_request_started)
            ),
            "peak_staff_attempt_concurrency": peak_attempt_concurrency(
                state["attempts"], {plan["worker"] for plan in plans.values()}
            ),
            "peak_staff_model_concurrency": peak_model_session_concurrency(
                history, {plan["worker"] for plan in plans.values()}
            )[0],
            "unterminated_staff_model_sessions": peak_model_session_concurrency(
                history, {plan["worker"] for plan in plans.values()}
            )[1],
            "cross_department_leakage": leakage_errors,
            "lead_to_lead_message_count": None,
            "lead_to_lead_message_disposition": "not exposed by the bounded Work graph/event projection; do not infer zero",
            "department_callbacks": {
                domain: {
                    "callback_at": value["callback_at"],
                    "review_completed_at": value["review_completed_at"],
                    "review_seconds": value["review_seconds"],
                }
                for domain, value in collections.items()
            },
            "exec_judgement_completed_at": company_exec_review["created_at"],
        }
    )
    json_write(result_dir / "events.json", history)
    json_write(result_dir / "work-graph.json", state)
    json_write(result_dir / "metrics.json", measured)
    if blind:
        json_write(result_dir / "blind-evaluations.json", blind)
    exact_valid = all(
        value["exact"].get("valid") and value["attribution_valid"]
        for value in exact.values()
    )
    expected_models = {LEAD_MODEL, STAFF_MODEL}
    runtime_valid = (
        measured["exec_returned_before_fourth_request"]
        and measured["peak_staff_model_concurrency"] >= 3
        and measured["unterminated_staff_model_sessions"] == 0
        and not leakage_errors
        and set(measured["models"]) == expected_models
        and measured["configured_efforts"] == [PRODUCTION_EFFORT]
    )
    blind_valid = skip_blind or (
        set(blind) == set(plans)
        and all(bool(value.get("valid")) for value in blind.values())
    )
    result = {
        "arm": arm.arm_id,
        "company": company,
        "validity": "counted" if exact_valid and runtime_valid and blind_valid else "invalid",
        "exact_evaluations": exact,
        "blind_evaluations": blind or None,
        "runtime_valid": runtime_valid,
        "blind_valid": blind_valid,
        "metrics": measured,
        "tell_processes": {
            "initial": {"exit": initial_tell[0], "stdout": initial_tell[1], "stderr": initial_tell[2]},
            "fourth": {"exit": fourth_tell[0], "stdout": fourth_tell[1], "stderr": fourth_tell[2]},
        },
        "finished_at": utc_now(),
    }
    json_write(result_dir / "run-result.json", result)
    return result


def finalize_existing_wave4(run_token: str) -> dict[str, object]:
    """Retry only malformed Wave-4 blind judgements; never replay production."""
    result_dir = RESULTS / f"company-q1x4-r1-{run_token}"
    result_path = result_dir / "run-result.json"
    if not result_path.is_file():
        raise RunFailure(f"no completed Wave-4 evidence at {result_path}")
    result = json.loads(result_path.read_text())
    blind = result.get("blind_evaluations")
    if not isinstance(blind, dict):
        raise RunFailure("Wave-4 result has no preserved blind evaluations")
    arms = {
        "sales": Arm("company-sales", 4, "sales", "q1", 1, demand="D0"),
        "support": Arm("company-support", 4, "support", "q1", 1),
        "monitoring": Arm("company-monitoring", 4, "monitoring", "q1", 1),
        "operations": Arm("company-operations", 4, "operations", "q1", 1),
    }
    invalid_domains = [
        domain
        for domain in arms
        if not isinstance(blind.get(domain), dict) or not blind[domain].get("valid")
    ]
    if not invalid_domains:
        raise RunFailure("Wave-4 blind evaluations are already valid")
    for domain in invalid_domains:
        retry_path = result_dir / domain / "blind-invalid-evaluation.json"
        if retry_path.exists():
            raise RunFailure(
                f"blind evaluator already failed twice for Wave-4 domain {domain}"
            )
        json_write(retry_path, blind.get(domain))
        blind[domain] = run_blind_evaluator(arms[domain], result_dir / domain)
    json_write(result_dir / "blind-evaluations.json", blind)
    result["blind_evaluations"] = blind
    result["blind_valid"] = set(blind) == set(arms) and all(
        bool(value.get("valid")) for value in blind.values()
    )
    exact = result.get("exact_evaluations", {})
    exact_valid = set(exact) == set(arms) and all(
        value.get("exact", {}).get("valid") and value.get("attribution_valid")
        for value in exact.values()
    )
    result["validity"] = (
        "counted"
        if exact_valid and result.get("runtime_valid") and result["blind_valid"]
        else "invalid"
    )
    result["blind_retry_finished_at"] = utc_now()
    json_write(result_path, result)
    return result


async def run_continuity_gate(run_token: str, wall_seconds: int) -> dict[str, object]:
    """G1 + G3: process-cold continuity and exact causal interruption."""
    arm = Arm("wave0-continuity", 0, "continuity", "q2", 2)
    result_dir = RESULTS / f"wave0-continuity-{run_token}"
    if result_dir.exists():
        raise RunFailure(f"result directory already exists: {result_dir}")
    result_dir.mkdir(parents=True)
    company = company_name(arm.arm_id, run_token)
    programme_spend = admit_program_cell(8)
    json_write(result_dir / "control/programme-spend-at-admission.json", programme_spend)
    config = result_dir / "control/company.toml"
    create_company_config(config, company, arm, 8)
    cli("company", "create", "--from-file", str(config))
    cli("up", "-c", company)
    lead = "continuity-direction"
    workers = ["continuity-worker", "continuity-control"]
    cli(
        "people", "create", "-c", company, "--id", lead,
        "--role", "accountable continuity lead", "--display", actor_display(lead),
        "--model", LEAD_MODEL, "--reason", "EXP-05 G1/G3 non-producing supervisor",
    )
    for worker in workers:
        cli(
            "people", "create", "-c", company, "--id", worker,
            "--role", "bounded continuity producer", "--display", actor_display(worker),
            "--model", STAFF_MODEL, "--reason", "EXP-05 G1/G3 exact Staff process",
        )
    cli(
        "teams", "create", "-c", company, "--name", "Continuity Probe", "--lead", lead,
        "--brief", f"Own G1/G3; exact charter at {RUNTIME_ROOT}/charter.md; lead never produces.",
    )
    for worker in workers:
        cli(
            "teams", "assign", "-c", company, "--actor", worker, "--team", "Continuity Probe",
            "--reason", "frozen G1/G3 roster",
        )
    cli("attach", "-c", company, "mkdir", "-p", str(RUNTIME_ROOT / "tools"), str(RUNTIME_ROOT / "control"), str(RUNTIME_ROOT / "outputs"), str(RUNTIME_ROOT / "progress"), str(RUNTIME_ROOT / "inbox"))
    docker_copy(ROOT / "validate-visible.py", company, RUNTIME_ROOT / "tools/validate-visible.py")
    docker_copy(ROOT / "fixtures/operations/runtime-inputs/territory-01.json", company, RUNTIME_ROOT / "inbox/control-operations.json")
    marker_source = result_dir / "control/marker-source.json"
    json_write(marker_source, {"marker": "EXP05-CONTINUITY-V1"})
    docker_copy(marker_source, company, RUNTIME_ROOT / "inbox/marker-source.json")
    marker_validator = result_dir / "control/validate-marker.py"
    marker_validator.write_text(
        "import json,sys\n"
        "a=json.load(open(sys.argv[1])); b=json.load(open(sys.argv[2]))\n"
        "assert b == a and set(b) == {'marker'}\n"
        "print(json.dumps({'valid':True,'marker':b['marker']},sort_keys=True))\n"
    )
    docker_copy(marker_validator, company, RUNTIME_ROOT / "tools/validate-marker.py")
    target_output = RUNTIME_ROOT / "outputs/continuity-target.json"
    target_progress = RUNTIME_ROOT / "progress/continuity-target.produce"
    control_output = RUNTIME_ROOT / "outputs/continuity-control.json"
    control_progress = RUNTIME_ROOT / "progress/continuity-control.produce"
    target_gate = json.dumps(
        {
            "name": "marker-contract",
            "command": [
                "python3", str(RUNTIME_ROOT / "tools/validate-marker.py"),
                str(RUNTIME_ROOT / "inbox/marker-source.json"), str(target_output),
            ],
        },
        separators=(",", ":"),
    )
    control_gate = json.dumps(
        {
            "name": "unaffected-control-contract",
            "command": [
                "python3", str(RUNTIME_ROOT / "tools/validate-visible.py"), "operations",
                str(RUNTIME_ROOT / "inbox/control-operations.json"), str(control_output),
                "--progress", str(control_progress),
            ],
        },
        separators=(",", ":"),
    )
    charter = result_dir / "control/charter.md"
    charter.write_text(
        f"""# EXP-05 G1/G3 product-path charter

Remain a non-producing supervisor. Commission exactly two repo-less Work nodes, then observe and
repair through Staff:

- `continuity-worker` reads `{RUNTIME_ROOT}/inbox/marker-source.json`, creates
  `{target_progress}` immediately after orientation, writes exactly the one-key JSON object to
  `{target_output}`, validates it, links it as `review_target`, and returns outcome_met. Expected
  artifact `{target_output}`; attempt limit 3; atomic gate `{target_gate}`.
- `continuity-control` independently reconciles `{RUNTIME_ROOT}/inbox/control-operations.json` into
  `{control_output}`, with progress marker `{control_progress}`, exact ordinary operations schema,
  visible validation, and an attributable `review_target`. Expected artifact `{control_output}`;
  attempt limit 2; atomic gate `{control_gate}`.

Do no production. If the target process is interrupted by changed Work feedback, inspect its exact
recovery evidence and resume the same Work because the source changed; do not recreate it or touch
the unaffected control. When the exact validator-complete event arrives, review both artifacts
read-only and send one material G1/G3 judgement to Exec. Do not poll or narrate status.
"""
    )
    docker_copy(charter, company, RUNTIME_ROOT / "charter.md")
    deadline = time.monotonic() + wall_seconds
    tell_started = utc_now()
    tell_task = asyncio.create_task(
        run_async(
            [
                str(CLI), "tell", "-c", company,
                f"Run the bounded fictional G1/G3 conformance at {RUNTIME_ROOT}/charter.md. Delegate exactly to existing lead `{lead}` and return to availability. Do no production and change no roster or model.",
            ]
        )
    )

    def target_work() -> dict[str, Any] | None:
        return next((row for row in graph(company)["work"] if row.get("expected_artifact") == str(target_output)), None)

    work = await wait_until("continuity target Work", target_work, deadline)
    target_id = work["id"]

    def first_target_ready() -> dict[str, Any] | None:
        ready = [
            row for row in events(company)
            if row["kind"] == "model_session_ready" and row.get("actor_id") == "continuity-worker"
        ]
        return min(ready, key=lambda row: row["id"]) if ready else None

    first_ready = await wait_until("first Staff target session readiness", first_target_ready, deadline, 0.2)
    feedback_at = utc_now()
    json_write(marker_source, {"marker": "EXP05-CONTINUITY-V2"})
    docker_copy(marker_source, company, RUNTIME_ROOT / "inbox/marker-source.json")
    cli(
        "message", "-c", company, "--from", "owner", "--to", "continuity-worker",
        "--work", target_id,
        "[CONTROLLED EXP-05 _TEST CHANGE; transport_authenticated=false] The exact marker source changed to V2 after session readiness. Re-read the source and do not accept any V1 output.",
    )

    def second_target_ready() -> dict[str, Any] | None:
        ready = sorted(
            [
                row for row in events(company)
                if row["kind"] == "model_session_ready" and row.get("actor_id") == "continuity-worker"
            ],
            key=lambda row: row["id"],
        )
        return ready[1] if len(ready) >= 2 else None

    second_ready = await wait_until("resumed Staff target session", second_target_ready, deadline, 0.2)

    async def collect_probe_outputs() -> tuple[Path, Path]:
        target_local = result_dir / "outputs/continuity-target.json"
        control_local = result_dir / "outputs/continuity-control.json"
        await wait_until(
            "V2 target artifact",
            lambda: output_snapshot(company, Unit(1, "target", marker_source, Path(), target_output.name, target_output, target_progress, 0, 1), target_local)
            and run_sync([sys.executable, str(marker_validator), str(marker_source), str(target_local)], check=False).returncode == 0,
            deadline,
        )
        await wait_until(
            "unaffected control artifact",
            lambda: output_snapshot(company, Unit(2, "control", ROOT / "fixtures/operations/runtime-inputs/territory-01.json", Path(), control_output.name, control_output, control_progress, 0, 2), control_local)
            and run_sync([sys.executable, str(ROOT / "validate-visible.py"), "operations", str(ROOT / "fixtures/operations/runtime-inputs/territory-01.json"), str(control_local)], check=False).returncode == 0,
            deadline,
        )
        return target_local, control_local

    await collect_probe_outputs()
    receipt = result_dir / "validator-receipts.json"
    json_write(receipt, {"valid": True, "target": "V2", "unaffected_control": True, "created_at": utc_now()})
    docker_copy(receipt, company, RUNTIME_ROOT / "control/validator-receipts.json")
    callback_at = utc_now()
    send_controlled_event(
        company, lead,
        f"G1/G3 exact receipts are complete at {RUNTIME_ROOT}/control/validator-receipts.json. Review both native artifacts and make one judgement.",
    )
    callback_epoch = parse_time(callback_at)
    await wait_until(
        "lead post-gate review",
        lambda: any(
            row["kind"] == "actor_wake_end" and row.get("actor_id") == lead and parse_time(row["created_at"]) >= callback_epoch
            for row in events(company)
        ),
        deadline,
    )
    await wait_until(
        "Exec post-judgement wake",
        lambda: any(
            row["kind"] == "wake_end"
            and row.get("actor_id") == "exec"
            and parse_time(row["created_at"]) >= callback_epoch
            for row in events(company)
        ),
        deadline,
    )
    tell_result = await tell_task
    state = graph(company)
    history = events(company)
    target_attempts = [row for row in state["attempts"] if row["work_id"] == target_id]
    control_work = next(row for row in state["work"] if row.get("expected_artifact") == str(control_output))
    control_attempts = [row for row in state["attempts"] if row["work_id"] == control_work["id"]]
    lead_readiness = sorted(
        [
            row
            for row in history
            if row["kind"] == "model_session_ready" and row.get("actor_id") == lead
        ],
        key=lambda row: row["id"],
    )
    target_ended = [
        row for row in history
        if row["kind"] == "attempt_process_ended" and row["body"].get("work_id") == target_id
    ]
    feedback_epoch = parse_time(feedback_at)
    first_end_after_feedback = min(
        (parse_time(row["created_at"]) for row in target_ended if parse_time(row["created_at"]) >= feedback_epoch),
        default=None,
    )
    checks = {
        "lead_process_cold_hot_session": len(lead_readiness) >= 2 and any(row["body"].get("resumed") for row in lead_readiness[1:]),
        "worker_process_cold_hot_session": bool(second_ready["body"].get("resumed")),
        "exact_models": first_ready["body"].get("model") == STAFF_MODEL and second_ready["body"].get("model") == STAFF_MODEL and all(row["body"].get("model") == LEAD_MODEL for row in lead_readiness),
        "production_effort": first_ready["body"].get("configured_effort") == PRODUCTION_EFFORT and second_ready["body"].get("configured_effort") == PRODUCTION_EFFORT and all(row["body"].get("configured_effort") == PRODUCTION_EFFORT for row in lead_readiness),
        "target_relaunched": len(target_attempts) >= 2,
        "first_target_not_accepted": target_attempts[0]["state"] != "produced",
        "second_target_produced": target_attempts[-1]["state"] == "produced",
        "unaffected_control_single_attempt": len(control_attempts) == 1 and control_attempts[0]["state"] == "produced",
        "workspace_progress_preserved": docker_read(company, target_progress) is not None,
        "usage_attributable": sum(1 for row in history if row["kind"] == "turn_usage" and row.get("actor_id") in {lead, *workers}) >= 4,
    }
    result = {
        "gate": "G1+G3",
        "company": company,
        "valid": all(checks.values()),
        "checks": checks,
        "feedback_at": feedback_at,
        "cancellation_observed_seconds": None if first_end_after_feedback is None else first_end_after_feedback - feedback_epoch,
        "first_target_readiness": first_ready,
        "second_target_readiness": second_ready,
        "lead_readiness": lead_readiness,
        "target_attempts": target_attempts,
        "control_attempts": control_attempts,
        "tell_started_at": tell_started,
        "tell_process": {"exit": tell_result[0], "stdout": tell_result[1], "stderr": tell_result[2]},
        "finished_at": utc_now(),
    }
    json_write(result_dir / "events.json", history)
    json_write(result_dir / "work-graph.json", state)
    json_write(result_dir / "run-result.json", result)
    return result


def dry_run(arm_id: str) -> dict[str, object]:
    arm = Arm.load(arm_id)
    if arm.domain == "company":
        slices = {
            "sales": (Arm("company-sales", 4, "sales", "q1", 1, demand="D0"), 1),
            "support": (Arm("company-support", 4, "support", "q1", 1), 1),
            "monitoring": (Arm("company-monitoring", 4, "monitoring", "q1", 1), 1),
            "operations": (Arm("company-operations", 4, "operations", "q1", 1), 4),
        }
        errors: list[str] = []
        hashes: dict[str, str] = {}
        counts: dict[str, int] = {}
        for domain, (subarm, count) in slices.items():
            units = units_for(subarm)[:count]
            charter = charter_for(
                subarm,
                f"{domain}-direction",
                [f"{domain}-owner"],
                units,
                include_support_event=False,
                inputs_visible_at_start=True,
            )
            hashes[domain] = hashlib.sha256(charter.encode()).hexdigest()
            unit_key = "entity" if domain == "monitoring" else "id"
            counts[domain] = len(
                {
                    row[unit_key]
                    for unit in units
                    for row in json.loads(unit.source.read_text())
                }
            )
            for unit in units:
                if not unit.source.is_file():
                    errors.append(f"missing {unit.source}")
                try:
                    json.loads(gate_command(subarm, unit))
                except Exception as error:  # noqa: BLE001 - report all malformed slices
                    errors.append(f"{unit.name}: {error}")
        return {
            "arm": arm_id,
            "dedicated_runner": True,
            "valid": not errors,
            "errors": errors,
            "population": counts,
            "charter_sha256": hashes,
        }
    lead, workers, _ = actor_ids(arm.domain, arm.workers)
    units = units_for(arm)
    charter = charter_for(arm, lead, workers, units)
    errors: list[str] = []
    if "non-producing" not in charter or "review_target" not in charter:
        errors.append("charter lost supervisor or ReviewTarget boundary")
    for unit in units:
        if not unit.source.is_file():
            errors.append(f"missing {unit.source}")
        try:
            parsed = json.loads(gate_command(arm, unit))
            if parsed["name"] != "visible-unit-contract":
                errors.append(f"bad gate for {unit.name}")
        except Exception as error:  # noqa: BLE001 - dry-run must report every malformed fixture
            errors.append(f"{unit.name}: {error}")
    return {"arm": arm_id, "valid": not errors, "errors": errors, "units": len(units), "charter_sha256": hashlib.sha256(charter.encode()).hexdigest()}


def preflight() -> dict[str, object]:
    checks: dict[str, object] = {}
    checks["cli"] = str(CLI) if CLI.is_file() else None
    checks["fixture_self_test"] = json.loads(
        run_sync([sys.executable, str(ROOT / "self-test.py")]).stdout
    )
    checks["arms"] = [dry_run(arm_id) for arm_id in CATALOG["base_order"]]
    usage = run_sync(["omp", "--profile", "restless-model-broker", "usage"], check=False)
    checks["omp_usage_exit"] = usage.returncode
    checks["provider_route_observed"] = MODEL_PROVIDER.lower() in usage.stdout.lower()
    catalog = run_sync(
        ["omp", "--profile", "restless-model-broker", "models", MODEL_PROVIDER, "--json"],
        check=False,
    )
    catalog_models: list[dict[str, object]] = []
    if catalog.returncode == 0:
        try:
            catalog_models = json.loads(catalog.stdout).get("models", [])
        except json.JSONDecodeError:
            pass
    catalog_selectors = {
        str(row.get("selector")) for row in catalog_models if row.get("selector")
    }
    required_selectors = {LEAD_MODEL, STAFF_MODEL}
    checks["provider_catalog_selectors"] = sorted(catalog_selectors)
    checks["exact_frozen_selectors_advertised"] = required_selectors <= catalog_selectors
    continuity_paths = sorted(
        RESULTS.glob("wave0-continuity-*/run-result.json"),
        key=lambda path: path.stat().st_mtime,
    )
    exact_execution = False
    if continuity_paths:
        continuity = json.loads(continuity_paths[-1].read_text())
        exact_execution = bool(
            continuity.get("valid")
            and continuity.get("checks", {}).get("exact_models")
            and continuity.get("checks", {}).get("production_effort")
            and continuity.get("checks", {}).get("usage_attributable")
        )
    checks["exact_execution_probe"] = "passed" if exact_execution else "not_run"
    checks["valid"] = (
        bool(checks["cli"])
        and checks["fixture_self_test"]["valid"]
        and checks["provider_route_observed"]
        and (checks["exact_frozen_selectors_advertised"] or exact_execution)
        and all(row.get("valid", True) for row in checks["arms"])
    )
    return checks


def record_preflight() -> dict[str, object]:
    result = preflight()
    result["recorded_at"] = utc_now()
    json_write(RESULTS / "wave0-preflight.json", result)
    return result


def record_deterministic_gates() -> dict[str, object]:
    fixture_result = json.loads(
        run_sync([sys.executable, str(ROOT / "self-test.py")]).stdout
    )
    result = {
        "gate": "G4-deterministic-local-closure",
        "recorded_at": utc_now(),
        "valid": fixture_result["valid"],
        "fixture_manifest_sha256": hashlib.sha256(
            (ROOT / "fixture-manifest.json").read_bytes()
        ).hexdigest(),
        "frozen_contract_sha256": hashlib.sha256(
            (ROOT / "frozen-contract.json").read_bytes()
        ).hexdigest(),
        "result": fixture_result,
        "scope": "deterministic fixture/evaluator conformance only; no model or product-path claim",
    }
    json_write(RESULTS / "wave0-deterministic-gates.json", result)
    return result


def run_live_command(
    coroutine: Any,
    result_dir: Path,
    *,
    company: str,
    command: str,
    arm: str | None = None,
) -> dict[str, object]:
    try:
        return asyncio.run(coroutine)
    except (Exception, KeyboardInterrupt) as error:  # noqa: BLE001 - every invalid run must leave evidence
        result_dir.mkdir(parents=True, exist_ok=True)
        failure: dict[str, object] = {
            "validity": "invalid",
            "command": command,
            "arm": arm,
            "company": company,
            "error_type": type(error).__name__,
            "error": str(error),
            "recorded_at": utc_now(),
        }
        try:
            if company in set(cli_json("company", "list")):
                failure["spend"] = cli_json("spend", "-c", company)
                json_write(result_dir / "work-graph-at-failure.json", graph(company))
                json_write(result_dir / "events-at-failure.json", events(company))
        except Exception as evidence_error:  # noqa: BLE001 - preserve the primary failure
            failure["evidence_capture_error"] = str(evidence_error)
        json_write(result_dir / "run-failure.json", failure)
        print(json.dumps(failure, indent=2, sort_keys=True))
        raise SystemExit(1) from error


def main() -> None:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("catalog")
    sub.add_parser("preflight")
    sub.add_parser("record-preflight")
    sub.add_parser("record-deterministic-gates")
    dry = sub.add_parser("dry-run")
    dry.add_argument("arm")
    anchor = sub.add_parser("ensure-anchor")
    anchor.add_argument("--staging", type=Path, default=RESULTS / "wave0-control")
    run = sub.add_parser("run-arm")
    run.add_argument("arm")
    run.add_argument("--run-token", required=True)
    run.add_argument("--wall-seconds", type=int, default=3600)
    run.add_argument("--skip-blind", action="store_true")
    finalize = sub.add_parser("finalize-arm")
    finalize.add_argument("arm")
    finalize.add_argument("--run-token", required=True)
    finalize.add_argument("--wall-seconds", type=int, default=1800)
    finalize.add_argument("--skip-blind", action="store_true")
    continuity = sub.add_parser("run-continuity-gate")
    continuity.add_argument("--run-token", required=True)
    continuity.add_argument("--wall-seconds", type=int, default=1800)
    wave4 = sub.add_parser("run-wave4")
    wave4.add_argument("--run-token", required=True)
    wave4.add_argument("--wall-seconds", type=int, default=3600)
    wave4.add_argument("--skip-blind", action="store_true")
    finalize_wave4 = sub.add_parser("finalize-wave4")
    finalize_wave4.add_argument("--run-token", required=True)
    args = parser.parse_args()
    if args.command == "catalog":
        print(json.dumps(CATALOG, indent=2, sort_keys=True))
    elif args.command == "preflight":
        print(json.dumps(preflight(), indent=2, sort_keys=True))
    elif args.command == "record-preflight":
        print(json.dumps(record_preflight(), indent=2, sort_keys=True))
    elif args.command == "record-deterministic-gates":
        print(json.dumps(record_deterministic_gates(), indent=2, sort_keys=True))
    elif args.command == "dry-run":
        print(json.dumps(dry_run(args.arm), indent=2, sort_keys=True))
    elif args.command == "ensure-anchor":
        args.staging.mkdir(parents=True, exist_ok=True)
        print(json.dumps({"anchor": ensure_anchor(args.staging)}, sort_keys=True))
    elif args.command == "run-arm":
        arm = Arm.load(args.arm)
        result_dir = RESULTS / f"{args.arm}-{args.run_token}"
        result = run_live_command(
            run_arm(args.arm, args.run_token, args.wall_seconds, args.skip_blind),
            result_dir,
            company=company_name(arm.arm_id, args.run_token),
            command=f"run-arm {args.arm}",
            arm=args.arm,
        )
        print(json.dumps(result, indent=2, sort_keys=True))
    elif args.command == "finalize-arm":
        arm = Arm.load(args.arm)
        result_dir = RESULTS / f"{args.arm}-{args.run_token}"
        result = run_live_command(
            finalize_existing_arm(
                args.arm, args.run_token, args.wall_seconds, args.skip_blind
            ),
            result_dir,
            company=company_name(arm.arm_id, args.run_token),
            command=f"finalize-arm {args.arm}",
            arm=args.arm,
        )
        print(json.dumps(result, indent=2, sort_keys=True))
    elif args.command == "run-continuity-gate":
        result_dir = RESULTS / f"wave0-continuity-{args.run_token}"
        result = run_live_command(
            run_continuity_gate(args.run_token, args.wall_seconds),
            result_dir,
            company=company_name("wave0-continuity", args.run_token),
            command="run-continuity-gate",
            arm="wave0-continuity",
        )
        print(json.dumps(result, indent=2, sort_keys=True))
    elif args.command == "run-wave4":
        result_dir = RESULTS / f"company-q1x4-r1-{args.run_token}"
        result = run_live_command(
            run_wave4(args.run_token, args.wall_seconds, args.skip_blind),
            result_dir,
            company=company_name("company-q1x4-r1", args.run_token),
            command="run-wave4",
            arm="company-q1x4-r1",
        )
        print(json.dumps(result, indent=2, sort_keys=True))
    elif args.command == "finalize-wave4":
        result = finalize_existing_wave4(args.run_token)
        print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
