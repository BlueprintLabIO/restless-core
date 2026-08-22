#!/usr/bin/env python3
from __future__ import annotations

import argparse
import asyncio
import hashlib
import json
import os
import re
import signal
import shutil
import shlex
import sqlite3
import subprocess
import sys
import time
import tomllib
import urllib.request
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
LAB_ROOT = HERE.parent

from coordinator import Coordinator, request, run, safe_name  # noqa: E402
from store import connect, emit, initialize, uid  # noqa: E402


WORK_ROOT = HERE / "workdir"
SEED = "514b7b3d0a65e093af608b08ca142344412181f4"
LEAD_MODEL = os.environ.get("COORD_LEAD_MODEL", "anthropic/claude-sonnet-4-5")
DEFAULT_WORKER_POOL = tuple(
    model.strip()
    for model in os.environ.get(
        "COORD_FREE_WORKER_POOL",
        "cohere/north-mini-code:free,poolside/laguna-s-2.1:free",
    ).split(",")
    if model.strip()
)
TURN_COMMAND = shlex.split(
    os.environ.get("COORD_TURN_COMMAND", str(LAB_ROOT / "target" / "release" / "coordination-lab-turn"))
)
SOURCE_CONTAINER = "restless-co-cosmon"
SOURCE_REPO = "/company/repos/cosmon-game"
SOURCE_CONFIG = Path.home() / ".restless" / "companies" / "cosmon.toml"
RESERVATION_USD = 1.5
CEILING_USD = 6.0
MAX_STAFF = int(os.environ.get("COORD_MAX_STAFF", "3"))
DRAIN_GRACE_SECONDS = int(os.environ.get("COORD_DRAIN_GRACE_SECONDS", "120"))

MODE_GRAPH = "graph_control"
MODE_ARTIFACT = "artifact_led"
MODE_SINGLE = "single_agent"
MODES = (MODE_GRAPH, MODE_ARTIFACT, MODE_SINGLE)

ACTORS = {
    "exec": (
        "Studio Exec",
        "Own the milestone, delegation, integration strategy, and final operating decision. Inspect only enough to frame and review substantial production; do not implement the game yourself.",
    ),
    "gameplay-systems": (
        "Gameplay systems engineer",
        "Own combat, creature switching, bonding, progression, boss mechanics, and executable gameplay verification when commissioned.",
    ),
    "world-content": (
        "World and content builder",
        "Own handcrafted biome construction, traversal, trainers, quests, habitats, and story content when commissioned.",
    ),
    "experience-presentation": (
        "Experience and presentation engineer",
        "Own interface, visual readability, creature personality presentation, spacecraft/home experience, and review-state preparation when commissioned.",
    ),
    "artifact-critic": (
        "Independent playable-build critic",
        "Review only the runnable artifact and success contract. Do not read producer reasoning. Seek concrete defects, regressions, missing requirements, and false claims.",
    ),
    "integration-lead": (
        "Integration lead",
        "Own the one writable integration workspace. Consume produced commit artifacts through explicit requires edges, converge them, run combined gates, and report one candidate commit.",
    ),
    "studio-lead": (
        "Game product lead",
        "Own the playable milestone, shared product understanding, canonical candidate, integration, quality judgement, and adaptive delegation.",
    ),
    "single-agent": (
        "Single-agent studio baseline",
        "Own product judgement, implementation, verification, and the canonical candidate without team coordination machinery.",
    ),
}


def actors_for_mode(mode: str) -> dict[str, tuple[str, str]]:
    specialists = {
        actor: ACTORS[actor]
        for actor in ("gameplay-systems", "world-content", "experience-presentation", "artifact-critic")
    }
    if mode == MODE_GRAPH:
        return {"exec": ACTORS["exec"], **specialists, "integration-lead": ACTORS["integration-lead"]}
    if mode == MODE_ARTIFACT:
        return {"studio-lead": ACTORS["studio-lead"], **specialists}
    if mode == MODE_SINGLE:
        return {"single-agent": ACTORS["single-agent"]}
    raise ValueError(f"unknown experiment mode {mode!r}")


def coordination_actor_for_mode(mode: str) -> str:
    return {
        MODE_GRAPH: "exec",
        MODE_ARTIFACT: "studio-lead",
        MODE_SINGLE: "single-agent",
    }[mode]


def mission() -> str:
    with SOURCE_CONFIG.open("rb") as source:
        return tomllib.load(source)["mission"]


def prove_free_worker_pool(models: list[str]) -> list[dict[str, Any]]:
    """Live-prove that every worker model is still zero-price and tool-capable."""
    if not models:
        raise RuntimeError("team modes require at least one explicitly pinned free worker model")
    with urllib.request.urlopen("https://openrouter.ai/api/v1/models", timeout=30) as response:
        body = json.load(response)
    catalogue = {row.get("id"): row for row in body.get("data", []) if isinstance(row, dict)}
    checked_at = datetime.now(timezone.utc).isoformat()
    proofs: list[dict[str, Any]] = []
    for model in models:
        row = catalogue.get(model)
        if not row:
            raise RuntimeError(f"worker model {model!r} is absent from the live OpenRouter catalogue")
        pricing = row.get("pricing") or {}
        prompt_price = str(pricing.get("prompt", ""))
        completion_price = str(pricing.get("completion", ""))
        parameters = sorted(item for item in row.get("supported_parameters", []) if isinstance(item, str))
        modalities = (row.get("architecture") or {}).get("input_modalities", [])
        if prompt_price != "0" or completion_price != "0":
            raise RuntimeError(
                f"worker model {model!r} is not free (prompt={prompt_price}, completion={completion_price})"
            )
        if "tools" not in parameters or "text" not in modalities:
            raise RuntimeError(f"worker model {model!r} does not advertise text input and tool support")
        proofs.append(
            {
                "id": model,
                "name": row.get("name") or model,
                "checked_at": checked_at,
                "context_length": int(row.get("context_length") or 0),
                "prompt_price": prompt_price,
                "completion_price": completion_price,
                "supported_parameters": parameters,
            }
        )
    return proofs


def prove_worker_runtime(cell: str, models: list[str]) -> list[dict[str, Any]]:
    """Prove each free worker through the exact container + gateway path.

    The public OpenRouter catalogue proves price and advertised capabilities,
    but it does not prove that the Company Runtime has a current model registry
    or that the credential broker can actually authorize inference.  This
    probe deliberately crosses both boundaries before paid lead work starts.
    """
    if not os.environ.get("RESTLESS_MODEL_GATEWAY_TOKEN"):
        raise RuntimeError("RESTLESS_MODEL_GATEWAY_TOKEN is required for worker runtime proof")
    common = [
        "docker",
        "exec",
        "-u",
        "company",
        "-w",
        "/workspace",
        "-e",
        "PI_CODING_AGENT_DIR=/company",
        "-e",
        "COORD_GATEWAY_PORT",
        "-e",
        "RESTLESS_MODEL_GATEWAY_TOKEN",
        cell,
        "/usr/local/bin/omp",
    ]
    refreshed = subprocess.run(
        [*common, "models", "refresh", "--json"],
        text=True,
        capture_output=True,
        timeout=90,
    )
    if refreshed.returncode:
        raise RuntimeError(
            "Company Runtime model refresh failed: "
            + (refreshed.stderr or refreshed.stdout)[-2_000:]
        )
    try:
        catalogue = json.loads(refreshed.stdout)
    except json.JSONDecodeError as exc:
        raise RuntimeError(f"Company Runtime model refresh returned invalid JSON: {exc}") from exc
    selectors = {
        row.get("selector")
        for row in catalogue.get("models", [])
        if isinstance(row, dict)
    }
    missing = [model for model in models if f"openrouter/{model}" not in selectors]
    if missing:
        raise RuntimeError(f"Company Runtime model catalogue is missing worker models: {missing}")

    proofs: list[dict[str, Any]] = []
    probe_thinking = os.environ.get("COORD_WORKER_PROBE_THINKING", "").strip()
    probe_max_seconds = int(os.environ.get("COORD_WORKER_PROBE_MAX_SECONDS", "90"))
    if probe_max_seconds < 10 or probe_max_seconds > 600:
        raise RuntimeError("COORD_WORKER_PROBE_MAX_SECONDS must be between 10 and 600")
    for model in models:
        started = time.monotonic()
        runtime_selector = f"openrouter/{model}"
        thinking_args = ["--thinking", probe_thinking] if probe_thinking else []
        probe = subprocess.run(
            [
                *common,
                "-p",
                "--model",
                runtime_selector,
                "--system-prompt",
                "You are a runtime readiness probe. Follow the exact reply instruction and do nothing else.",
                "--config",
                "/harness/omp-runtime.yml",
                "--extension",
                "/harness/v2/openrouter-live-free-models.ts",
                "--no-extensions",
                "--no-rules",
                "--no-tools",
                *thinking_args,
                "--max-time",
                f"{probe_max_seconds}s",
                "Reply with exactly WORKER_RUNTIME_READY.",
            ],
            text=True,
            capture_output=True,
            timeout=probe_max_seconds + 30,
        )
        elapsed = round(time.monotonic() - started, 3)
        combined = (probe.stdout + "\n" + probe.stderr).strip()
        if probe.returncode or "WORKER_RUNTIME_READY" not in probe.stdout:
            raise RuntimeError(
                f"worker runtime inference failed for {model!r} "
                f"(exit={probe.returncode}, elapsed={elapsed}s): {combined[-2_000:]}"
            )
        proofs.append(
            {
                "model": model,
                "runtime_selector": runtime_selector,
                "company_runtime_catalogue": True,
                "gateway_inference": True,
                "reply": "WORKER_RUNTIME_READY",
                "elapsed_seconds": elapsed,
                "thinking": probe_thinking or "runtime-default",
                "max_seconds": probe_max_seconds,
            }
        )
    return proofs


def initial_project_state(scenario: str) -> str:
    return f"""# Current product state

## Outcome

Produce the next coherent, playable Cosmon milestone from seed `{SEED}` under the owner directive.

## Current candidate

Seed `{SEED}`. The repository README reports a working exploration, encounter, Resonance Bond,
battle, roster, and evolution foundation. Treat those claims as inputs to verify, not truth by fiat.

## Current milestone

Unchosen. Inspect the native candidate, select the smallest integrated milestone that materially
advances the directive, and record it here before or alongside delegation.

## Product decisions and quality bar

- Preserve the working core loop and existing executable checks.
- Prefer one coherent playable increment over several disconnected features.
- Judge contributions in the running product, not from producer narration.

## Open gaps and risks

- Determine from native inspection and executable evidence.

## Active responsibilities

- None yet.

## Next proof

Run the current candidate, inspect its evidence, choose the milestone, and delegate only responsibilities
that genuinely benefit from another actor.

<!-- Owner directive hash: {hashlib.sha256(scenario.encode()).hexdigest()} -->
"""


def docker_cells(run_id: str) -> list[str]:
    result = run(
        [
            "docker",
            "ps",
            "-a",
            "--filter",
            "label=restless.coordination.lab=v2",
            "--filter",
            f"label=restless.coordination.run={safe_name(run_id)}",
            "--format",
            "{{.Names}}",
        ]
    )
    cells = [line.strip() for line in result.stdout.splitlines() if line.strip()]
    prefix = f"restless-coord-v2-{safe_name(run_id)}-"
    if any(not cell.startswith(prefix) for cell in cells):
        raise RuntimeError(f"refusing unexpected cleanup targets: {cells}")
    return cells


def cleanup_cells(run_id: str) -> list[str]:
    cells = docker_cells(run_id)
    for cell in cells:
        run(["docker", "rm", "-f", cell])
    return cells


def safe_reset(path: Path) -> None:
    root = WORK_ROOT.resolve()
    resolved = path.resolve()
    if resolved.parent != root or not resolved.name:
        raise RuntimeError(f"refusing unsafe reset target {resolved}")
    if path.exists():
        shutil.rmtree(path)


def runtime_unknown_summary(result: dict[str, Any]) -> str:
    """Project the bounded Runtime observation needed to recover an Attempt."""
    observation: dict[str, Any] = {
        "runtime_outcome": result.get("stop_reason") or "unknown",
        "model": result.get("model") or "unknown",
    }
    if result.get("error"):
        observation["error_excerpt"] = " ".join(str(result["error"]).split())[:1_200]
    return "Actor process ended without a terminal report: " + json.dumps(observation, sort_keys=True)


def prepare(
    run_id: str,
    *,
    mode: str = MODE_GRAPH,
    lead_model: str = LEAD_MODEL,
    worker_pool: list[str] | None = None,
    spend_ceiling_usd: float = CEILING_USD,
    wall_clock_seconds: int = 1800,
    drain_grace_seconds: int = DRAIN_GRACE_SECONDS,
    scenario_text: str | None = None,
) -> Path:
    if mode not in MODES:
        raise ValueError(f"mode must be one of {MODES}")
    selected_workers = list(DEFAULT_WORKER_POOL if worker_pool is None else worker_pool)
    if mode != MODE_SINGLE and not selected_workers:
        raise ValueError("team modes require a non-empty free worker model pool")
    if spend_ceiling_usd <= 0:
        raise ValueError("spend ceiling must be positive")
    if wall_clock_seconds < 60:
        raise ValueError("wall-clock envelope must be at least 60 seconds")
    if not 15 <= drain_grace_seconds <= 300:
        raise ValueError("drain grace must be between 15 and 300 seconds")
    actors = actors_for_mode(mode)
    coordination_actor = coordination_actor_for_mode(mode)
    cleanup_cells(run_id)
    run_dir = WORK_ROOT / run_id
    safe_reset(run_dir)
    run_dir.mkdir(parents=True)
    seed_source = run_dir / "seed-source"
    canonical = run_dir / "canonical"
    run(["docker", "cp", f"{SOURCE_CONTAINER}:{SOURCE_REPO}", str(seed_source)])
    run(["git", "clone", "--no-local", str(seed_source), str(canonical)])
    if mode in (MODE_ARTIFACT, MODE_SINGLE):
        run(["git", "-C", str(canonical), "checkout", "-B", "candidate", SEED])
    else:
        run(["git", "-C", str(canonical), "checkout", "--detach", SEED])
    run(["git", "-C", str(canonical), "clean", "-fd"])
    observed = run(["git", "-C", str(canonical), "rev-parse", "HEAD"]).stdout.strip()
    if observed != SEED:
        raise RuntimeError(f"seed mismatch: expected {SEED}, got {observed}")
    run(["git", "-C", str(canonical), "update-ref", "refs/heads/candidate", SEED])
    shutil.rmtree(seed_source)

    context = run_dir / "context"
    (context / "system").mkdir(parents=True)
    (context / "prompts").mkdir()
    runtime_bin = context / "runtime-bin"
    runtime_bin.mkdir()
    for command in ("node", "npm", "npx", "bun"):
        (runtime_bin / command).symlink_to(f"/usr/local/bin/{command}")
    for command in ("omp", "claude", "codex"):
        blocked = runtime_bin / command
        blocked.write_text(
            "#!/bin/sh\n"
            "echo 'Nested model execution is disabled; use the current actor or commission Work.' >&2\n"
            "exit 126\n"
        )
        blocked.chmod(0o755)

    scenario = scenario_text if scenario_text is not None else mission()
    if not scenario.strip():
        raise ValueError("scenario must not be empty")
    (context / "scenario.md").write_text(scenario)
    if mode == MODE_ARTIFACT:
        lead_home = run_dir / "homes" / coordination_actor
        lead_home.mkdir(parents=True)
        (lead_home / "project-state.md").write_text(initial_project_state(scenario))
    roster = "\n".join(
        f"- `{actor}` — {role}: {brief}"
        for actor, (role, brief) in actors.items()
        if actor != coordination_actor
    )
    if mode == MODE_GRAPH:
        coordination_system = f"""# Coordination lab v2 — graph-control Exec

You are the persistent company Exec. Your role is coordination, judgement, and integration strategy; your project workspace is read-only. Substantial production must be commissioned as exact Work.

Each wake contains the durable owner directive, the README from the exact current candidate, the wake causes, and canonical coordination state. Treat that as the current company evidence. Do not inspect implementation files yourself. If the evidence is insufficient, commission bounded reconnaissance to the appropriate Staff actor. Your scarce wake is for choosing and assigning the next useful outcomes.

When current evidence already names concrete missing product outcomes, commission the smallest playable production increment directly. Do not replace production with a broad audit, reconnaissance report, explanation document, or review of the unchanged candidate. Independent critique follows completed producer or integration Work through an explicit `requires` edge.

Available roster:
{roster}

Commands require a caller-chosen `idempotency_key`. Choose a short semantic key and reuse it only to retry the exact mutation.

Producer Work ends at a clean commit plus terminal report. Producers never merge or publish. When produced outcomes should be combined, commission exactly one `integration-lead` Work with explicit `requires` edges to all producer Work. Only the integration lead receives an integration lease.

Treat actor wakes as asynchronous. Once useful Work is commissioned, end this turn. Do not inspect or poll running Attempts, send status checks to an actor already running, or infer progress from elapsed time. You will be woken by progress, terminal, cancellation, schedule, and dependency events.

Actor accountability is independent of model or provider selection. A transient provider failure does not change who owns the Work: repair the same durable Work under the same accountable actor. The Runtime Bridge rotates that actor's recorded free-model pool at the Attempt boundary. Reassign Work only when a different capability or accountable owner is genuinely required.

An Attempt summary containing a Runtime outcome, model, and error excerpt is direct observed evidence. Use it to choose recovery; do not ask Staff to reproduce a transport or provider error. `send` delivery is `next_wake`: it adds context to a future actor wake and is not an immediate diagnostic RPC.

Messages never reopen blocked Work and are not a second assignment path. When an existing blocked Work should continue, call `redirect(action=repair)` in the same wake; its feedback and queued messages become recovery input to the next Attempt. Ending after `send` alone leaves that Work blocked.

There is no mid-run owner help. Make reversible operating decisions yourself. Request owner judgement only for irreducible identity, legal attestation, taste, or consequential approval. Never contradict an unresolved judgement request with a separate decision.

Use outcome-sized Work, deterministic argv gates where useful, and evidence from produced callbacks. `outcome_met` is a claim; gates and independent review determine whether it holds. Call `decide(subject=run, choice=complete)` only after one integrated candidate is prepared and independently reviewed.
"""
    elif mode == MODE_ARTIFACT:
        coordination_system = f"""# Coordination lab v21 — artifact-led Game Product Lead

You are the accountable lead for one playable Cosmon milestone. Own shared product understanding,
delegation, integration, native verification, quality judgement, and the canonical `candidate` branch.
Your current directory is that writable canonical candidate. You may inspect, run, integrate, and make
small coherence repairs directly. Delegate substantial separable craft to the smallest useful roster.

Available roster:
{roster}

The running product is primary evidence. At each event wake, inspect the supplied candidate probe and
then use your workspace whenever deeper judgement is needed. Producer reports are claims until you
cherry-pick their exact commits, run the combined candidate, and judge the resulting experience.

Maintain `/company/project-state.md` as the team's concise shared situation model. Before ending every
wake, update its milestone, current candidate, product decisions, open gaps, active responsibilities,
and next proof. Preserve useful prior decisions; remove stale narrative. This file is ordinary team
memory, not a second task database and not part of the product repository.

Use `commission` only when a durable responsibility crosses an actor boundary. Do not manufacture Work
for your own inspection, integration, testing, small repairs, planning, or project-state updates. Work
nodes record ownership and durable handoffs; they are not the plan or the product.

The free specialists are useful but slower and more context-limited than you. Metabolise the broad
owner directive yourself, then give each specialist an atomic contribution it can finish in one turn:
normally one primary module or similarly bounded surface, one exact extension seam, and one executable
proof. Never delegate the whole vertical milestone, broad repository discovery, or cross-cutting
integration. Prefer two independent small contributions that you can integrate and judge over one
large Work item that asks a worker to rediscover the product.

Producer Work ends at a clean commit and terminal report. Their commit is imported into canonical
`refs/heads/artifacts/<work-id>` and is also named in coordination evidence. Integrate it yourself with
ordinary Git, resolve conflicts against the evolving whole, and verify the combined candidate. Keep the
canonical checkout clean at wake boundaries.

Treat wakes as asynchronous events. After delegating every currently useful responsibility, update
project state and quiesce. Do not poll running workers. A callback wakes you. Repair the same Work after
a provider/runtime failure; actor accountability must not change merely to select another model.

Once at least two useful delegated contributions have been integrated, commission `artifact-critic`
against all relevant completed producer Work so its workspace starts from the current candidate. Give
the critic the observable milestone and require a concrete review artifact. Judge and repair from that
review; do not accept it mechanically.

There is no mid-run owner help. Make reversible product and engineering decisions yourself. Call
`decide(subject=run, choice=complete)` only after one clean, advanced, executable candidate has combined
at least two delegated contributions, passed its checks, and received independent native-artifact review.
"""
    else:
        coordination_system = """# Coordination lab v21 — strong single-agent baseline

You own this Cosmon milestone end to end. Work directly in the writable canonical `candidate` branch:
inspect the current product, choose the smallest coherent playable increment, implement it, run the
existing and new executable checks, and leave one clean meaningful commit. You have no team and should
not create Work or simulate delegation. Prefer a working integrated product increment over planning or
management prose. There is no mid-run owner help. Record `decide(subject=run, choice=complete)` only
when the exact candidate is advanced, clean, and executable; otherwise leave a truthful continuation.
"""
    (context / "system" / f"{coordination_actor}.md").write_text(coordination_system)
    shared = """You are durable Staff working inside one persistent Work workspace. Your current working directory is the only project workspace available and is bound to your claimed actor, Work revision, Attempt, and lease. Do not seek the company integration checkout or another Work workspace.

Commands require a caller-chosen `idempotency_key`; reuse it only for an exact retry. Work until the outcome is genuinely met or blocked. Producer Work ends at a clean meaningful commit and terminal `report`; never merge to main, candidate, or another branch. A progress report is nonterminal. An `outcome_met` report whose declared gate fails returns `revision_required` and keeps this same Attempt live: repair the exact failure and resubmit with a new idempotency key. Before ending, always reach a passing `outcome_met` or call terminal `blocked|abandoned`.

"""
    for actor, (role, brief) in actors.items():
        if actor == coordination_actor:
            continue
        extra = ""
        if actor == "integration-lead":
            extra = (
                "You are the sole integration writer. Consume the exact required commit artifacts listed in the prompt, "
                "cherry-pick or merge them inside your current working directory, resolve conflicts, run combined gates, and report one clean candidate commit.\n\n"
            )
        team_context = (
            "Your accountable product lead maintains the milestone and current product judgement in the project-state context supplied with each Attempt. Fit your contribution into that evolving whole; do not create a parallel product plan.\n\n"
            if mode == MODE_ARTIFACT
            else ""
        )
        (context / "system" / f"{actor}.md").write_text(
            f"# {role}\n\n{shared}{extra}{team_context}{brief}\n"
        )

    initialize(run_dir / "state.db")
    conn = connect(run_dir / "state.db")
    for actor, (role, brief) in actors.items():
        conn.execute("INSERT INTO actors(id,role,brief) VALUES(?,?,?)", (actor, role, brief))
    emit(
        conn,
        "run_prepared",
        {
            "run": run_id,
            "seed": observed,
            "mode": mode,
            "lead_model": lead_model,
            "worker_pool": selected_workers,
            "harness": "v21",
        },
    )
    conn.close()
    (run_dir / "manifest.json").write_text(
        json.dumps(
            {
                "run": run_id,
                "seed": observed,
                "mode": mode,
                "coordination_actor": coordination_actor,
                "lead_model": lead_model,
                "worker_model_pool": selected_workers,
                "wall_clock_seconds": wall_clock_seconds,
                "drain_grace_seconds": drain_grace_seconds,
                "scenario_sha256": hashlib.sha256(scenario.encode()).hexdigest(),
                "spend_ceiling_usd": spend_ceiling_usd,
                "turn_reservation_usd": RESERVATION_USD,
                "max_staff_concurrency": MAX_STAFF,
                "actors": {actor: {"role": role, "brief": brief} for actor, (role, brief) in actors.items()},
            },
            indent=2,
            sort_keys=True,
        )
    )
    return run_dir


@dataclass
class ActiveTurn:
    actor: str
    task: asyncio.Task[dict[str, Any]]
    attempt: str | None


class LabRun:
    def __init__(self, run_id: str):
        self.run_id = run_id
        self.run_dir = WORK_ROOT / run_id
        manifest_path = self.run_dir / "manifest.json"
        if not manifest_path.exists():
            raise RuntimeError(f"run {run_id!r} is not prepared")
        self.manifest = json.loads(manifest_path.read_text())
        self.mode = self.manifest.get("mode", MODE_GRAPH)
        self.lead_actor = self.manifest.get("coordination_actor", "exec")
        self.lead_model = self.manifest.get("lead_model", LEAD_MODEL)
        self.worker_pool = list(self.manifest.get("worker_model_pool", []))
        self.spend_ceiling = float(self.manifest.get("spend_ceiling_usd", CEILING_USD))
        self.turn_reservation = float(self.manifest.get("turn_reservation_usd", RESERVATION_USD))
        self.max_staff = int(self.manifest.get("max_staff_concurrency", MAX_STAFF))
        self.queue: asyncio.Queue[dict[str, Any]] = asyncio.Queue()
        self.coordinator = Coordinator(self.run_dir, run_id, self.queue)
        self.coordinator.reconcile_orphaned_attempts()
        self.started = time.monotonic()
        self.deadline = self.started + float(self.manifest.get("wall_clock_seconds", 3600))
        self.drain_grace_seconds = float(
            self.manifest.get("drain_grace_seconds", DRAIN_GRACE_SECONDS)
        )
        self.tasks: dict[str, ActiveTurn] = {}
        self.idle_wakes = 0
        self.envelope_warned = False
        self.stopping = False

    def remaining(self) -> float:
        return max(0.0, self.deadline - time.monotonic())

    def reserved_cost(self, additional: int = 0) -> float:
        return self.coordinator.cost() + (len(self.tasks) + additional) * self.turn_reservation

    def can_launch(self) -> bool:
        return self.reserved_cost(1) <= self.spend_ceiling

    def select_model(self, actor: str, attempt: str | None) -> str:
        actor_key = actor.upper().replace("-", "_")
        model_key = "COORD_MODEL_" + actor_key
        pool_key = "COORD_MODEL_POOL_" + actor_key
        explicit_model = os.environ.get(model_key)
        explicit_pool = [model.strip() for model in os.environ.get(pool_key, "").split(",") if model.strip()]
        if explicit_model:
            return explicit_model
        if actor == self.lead_actor:
            return self.lead_model
        model_pool = explicit_pool or self.worker_pool
        if not model_pool:
            raise RuntimeError(f"no model pool configured for worker {actor!r}")
        revision = 1
        if attempt:
            row = self.coordinator.conn.execute("SELECT revision FROM attempts WHERE id=?", (attempt,)).fetchone()
            if not row:
                raise RuntimeError(f"Attempt {attempt} is absent from canonical coordination state")
            revision = int(row["revision"])
        actor_offset = int(hashlib.sha256(actor.encode()).hexdigest()[:8], 16)
        return model_pool[(actor_offset + revision - 1) % len(model_pool)]

    def prompt_path(self, actor: str, prompt: str, turn_id: str) -> Path:
        path = self.run_dir / "context" / "prompts" / f"{actor}-{turn_id}.md"
        path.write_text(prompt)
        return path

    async def run_turn(
        self,
        actor: str,
        prompt: str,
        *,
        cell: str,
        attempt: str | None = None,
        lease_token: str = "",
        max_time: str = "8m",
        read_only: bool = False,
    ) -> dict[str, Any]:
        turn_id = f"{int(time.time() * 1000)}-{uid('turn')}"
        prompt_path = self.prompt_path(actor, prompt, turn_id)
        self.coordinator.start_turn(turn_id, actor, attempt)
        container_endpoint = f"host.docker.internal:{self.coordinator.port}"
        event_endpoint = f"127.0.0.1:{self.coordinator.port}"
        if attempt:
            workspace_row = self.coordinator.conn.execute(
                "SELECT w.id,w.workspace,w.branch FROM attempts a JOIN work w ON w.id=a.work_id WHERE a.id=?",
                (attempt,),
            ).fetchone()
            if not workspace_row or not workspace_row["workspace"]:
                raise RuntimeError(f"Attempt {attempt} has no host workspace")
            host_workdir = workspace_row["workspace"]
            work_id = workspace_row["id"]
            expected_branch = workspace_row["branch"]
        else:
            host_workdir = str(self.run_dir / "canonical")
            work_id = ""
            expected_branch = "candidate" if self.mode in (MODE_ARTIFACT, MODE_SINGLE) else ""
        selected_model = self.select_model(actor, attempt)
        runtime_model = (
            selected_model
            if actor == self.lead_actor
            or selected_model.startswith("openrouter/")
            or selected_model.startswith("gpt-")
            else f"openrouter/{selected_model}"
        )
        self.coordinator.emit(
            "model_selected",
            {
                "turn": turn_id,
                "attempt": attempt,
                "model": selected_model,
                "runtime_selector": runtime_model,
            },
            actor,
        )
        env = os.environ.copy()
        env.update(
            {
                "COORD_ACTOR": actor,
                "COORD_MODEL": runtime_model,
                "COORD_PROMPT_PATH": str(prompt_path),
                "COORD_SYSTEM_PATH": f"/context/system/{actor}.md",
                "COORD_HOST_SYSTEM_PATH": str(self.run_dir / "context" / "system" / f"{actor}.md"),
                "COORD_WORKDIR": "/workspace",
                "COORD_HOST_WORKDIR": host_workdir,
                "COORD_ATTEMPT": attempt or "",
                "COORD_WORK": work_id,
                "COORD_EXPECTED_BRANCH": expected_branch,
                "COORD_LEASE_TOKEN": lease_token,
                "COORD_RUN_ID": self.run_id,
                "COORD_CONTAINER": cell,
                "COORD_ENDPOINT": container_endpoint,
                "COORD_EVENT_ENDPOINT": event_endpoint,
                "COORD_HOST_ENDPOINT": event_endpoint,
                "COORD_TURN_ID": turn_id,
                "COORD_AGENT_HOME": "/company",
                "COORD_READ_ONLY": "1" if read_only else "0",
                "COORD_ACTOR_KIND": "exec" if actor == self.lead_actor else "staff",
                "COORD_RUNTIME_BIN": "/context/runtime-bin",
                "COORD_MCP_SERVER": "/harness/v2/mcp_server.py",
                "COORD_HOST_MCP_SERVER": str(HERE / "mcp_server.py"),
                "COORD_PYTHON": sys.executable,
                "COORD_TURN_DIR": str(self.run_dir / "context" / "turns"),
                "COORD_CANONICAL_GIT_DIR": str(self.run_dir / "canonical" / ".git"),
                "COORD_HOST_PROJECT_STATE_PATH": str(self.project_state_path())
                if self.mode == MODE_ARTIFACT and actor == self.lead_actor
                else "",
                "COORD_EXTRA_WRITE_DIR": str(self.project_state_path().parent)
                if self.mode == MODE_ARTIFACT and actor == self.lead_actor
                else "",
                "COORD_REASONING_EFFORT": os.environ.get(
                    "COORD_LEAD_REASONING" if actor == self.lead_actor else "COORD_WORKER_REASONING",
                    "medium" if actor == self.lead_actor else "low",
                ),
                "COORD_MAX_TIME": max_time,
            }
        )
        process = await asyncio.create_subprocess_exec(
            *TURN_COMMAND,
            env=env,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            start_new_session=True,
        )

        async def stop_process_tree() -> None:
            """Stop the exact scratch cell and its local docker-exec client."""
            stopped = await asyncio.create_subprocess_exec(
                "docker",
                "stop",
                "-t",
                "2",
                cell,
                stdout=asyncio.subprocess.DEVNULL,
                stderr=asyncio.subprocess.DEVNULL,
            )
            try:
                await asyncio.wait_for(stopped.wait(), timeout=8)
            except asyncio.TimeoutError:
                stopped.kill()
                await stopped.wait()
            if process.returncode is None:
                try:
                    os.killpg(process.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
                try:
                    await asyncio.wait_for(process.wait(), timeout=5)
                except asyncio.TimeoutError:
                    try:
                        os.killpg(process.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    await process.wait()
            restarted = await asyncio.create_subprocess_exec(
                "docker",
                "start",
                cell,
                stdout=asyncio.subprocess.DEVNULL,
                stderr=asyncio.subprocess.DEVNULL,
            )
            await restarted.wait()

        try:
            stdout, stderr = await asyncio.wait_for(
                process.communicate(),
                timeout=max(1.0, self.remaining() + self.drain_grace_seconds),
            )
        except asyncio.CancelledError:
            current_task = asyncio.current_task()
            if current_task is not None:
                while current_task.cancelling():
                    current_task.uncancel()
            try:
                await stop_process_tree()
            except BaseException as exc:
                self.coordinator.emit(
                    "turn_stop_failed",
                    {"turn": turn_id, "cell": cell, "error": str(exc)},
                    actor,
                )
            result = {
                "text": "",
                "tool_calls": [],
                "cost_usd": None,
                "used_tokens": None,
                "output_tokens": None,
                "stop_reason": "controller_cancelled",
            }
        except asyncio.TimeoutError:
            try:
                await stop_process_tree()
            except BaseException as exc:
                self.coordinator.emit(
                    "turn_stop_failed",
                    {"turn": turn_id, "cell": cell, "error": str(exc)},
                    actor,
                )
            result = {
                "text": "",
                "tool_calls": [],
                "cost_usd": None,
                "used_tokens": None,
                "output_tokens": None,
                "stop_reason": "global_timeout",
            }
        else:
            if process.returncode:
                result = {
                    "text": "",
                    "tool_calls": [],
                    "cost_usd": None,
                    "used_tokens": None,
                    "output_tokens": None,
                    "stop_reason": "failed",
                    "error": stderr.decode(errors="replace")[-4000:],
                }
            else:
                lines = [line for line in stdout.decode(errors="replace").splitlines() if line.strip()]
                result = json.loads(lines[-1]) if lines else {"text": "", "tool_calls": [], "stop_reason": "empty"}
        result.setdefault("model", runtime_model)
        self.coordinator.finish_turn(turn_id, result)
        return result

    def project_state_path(self) -> Path:
        return self.run_dir / "homes" / self.lead_actor / "project-state.md"

    def project_state(self) -> str:
        path = self.project_state_path()
        return path.read_text() if path.exists() else "Project state is absent."

    def run_candidate_check(
        self,
        cell: str,
        canonical: Path,
        check_file: str,
        candidate: str | None = None,
    ) -> tuple[Any, str]:
        """Run one native proof in an isolated commit export.

        Proofs are allowed to capture screenshots and create other operating
        files. They must not mutate the canonical candidate checkout while
        producing evidence about it.
        """
        candidate = candidate or run(
            ["git", "-C", str(canonical), "rev-parse", "candidate"]
        ).stdout.strip()
        isolated_script = """
set -eu
check_file=$1
candidate=$2
review_dir=$(mktemp -d /tmp/restless-candidate-check.XXXXXX)
cleanup() { rm -rf "$review_dir"; }
trap cleanup EXIT INT TERM
git archive "$candidate" | tar -x -C "$review_dir"
cd "$review_dir"
node "$check_file"
"""
        process = self.coordinator.workspaces.cell_exec(
            cell,
            ["sh", "-c", isolated_script, "candidate-check", check_file, candidate],
            timeout=180,
            check=False,
        )
        combined = f"{process.stdout}\n{process.stderr}"
        if process.returncode == 0 or not re.search(r"ERR_CONNECTION_REFUSED|ECONNREFUSED", combined, re.I):
            return process, "self_owned"
        source_result = run(
            ["git", "-C", str(canonical), "show", f"{candidate}:{check_file}"],
            check=False,
        )
        source = source_result.stdout if source_result.returncode == 0 else ""
        ports = sorted(set(re.findall(r"127\.0\.0\.1:(\d+)", source)))
        if not ports:
            return process, "self_owned"
        fixture_script = """
set -eu
check_file=$1
candidate=$2
shift 2
review_dir=$(mktemp -d /tmp/restless-candidate-check.XXXXXX)
pids=''
cleanup() {
  for pid in $pids; do kill "$pid" 2>/dev/null || true; done
  for pid in $pids; do wait "$pid" 2>/dev/null || true; done
  rm -rf "$review_dir"
}
trap cleanup EXIT INT TERM
git archive "$candidate" | tar -x -C "$review_dir"
cd "$review_dir"
for port in "$@"; do
  python3 -m http.server "$port" >/tmp/restless-candidate-server-"$port".log 2>&1 &
  pids="$pids $!"
done
sleep 0.4
node "$check_file"
"""
        self.coordinator.emit(
            "candidate_check_fixture_retry",
            {"file": check_file, "ports": ports},
        )
        return (
            self.coordinator.workspaces.cell_exec(
                cell,
                [
                    "sh",
                    "-c",
                    fixture_script,
                    "candidate-check",
                    check_file,
                    candidate,
                    *ports,
                ],
                timeout=180,
                check=False,
            ),
            "ephemeral_static_fallback",
        )

    def candidate_evidence(self, cell: str, *, run_checks: bool = True) -> dict[str, Any]:
        canonical = self.run_dir / "canonical"
        candidate = run(["git", "-C", str(self.run_dir / "canonical"), "rev-parse", "candidate"]).stdout.strip()
        head = run(["git", "-C", str(canonical), "rev-parse", "HEAD"]).stdout.strip()
        status = run(["git", "-C", str(canonical), "status", "--porcelain"]).stdout
        changed = run(["git", "-C", str(canonical), "diff", "--stat", SEED, candidate], check=False).stdout
        readme_result = run(
            ["git", "-C", str(self.run_dir / "canonical"), "show", f"{candidate}:README.md"],
            check=False,
        )
        readme = readme_result.stdout if readme_result.returncode == 0 else "README.md is absent from the current candidate."
        if len(readme) > 14_000:
            readme = readme[:14_000] + "\n[README truncated]"
        checks: list[dict[str, Any]] = []
        if run_checks:
            # Executable checks own their fixtures, including any HTTP server.
            # Starting an implicit shared server here races self-contained
            # checks and can turn a healthy candidate into EADDRINUSE noise.
            check_files = run(
                ["git", "-C", str(canonical), "ls-files", "verify-*.mjs"], check=False
            ).stdout.splitlines()
            for check_file in check_files:
                process, fixture_mode = self.run_candidate_check(
                    cell, canonical, check_file, candidate
                )
                combined_output = f"{process.stdout}\n{process.stderr}"
                observed_errors = [
                    int(match)
                    for match in re.findall(r"errors observed:\s*(\d+)", combined_output, flags=re.IGNORECASE)
                ]
                failure_markers = len(re.findall(r"(?m)^\[FAIL\]", combined_output))
                passed = (
                    process.returncode == 0
                    and failure_markers == 0
                    and all(count == 0 for count in observed_errors)
                )
                checks.append(
                    {
                        "file": check_file,
                        "fixture_mode": fixture_mode,
                        "exit": process.returncode,
                        "passed": passed,
                        "failure_markers": failure_markers,
                        "errors_observed": observed_errors,
                        "stdout": process.stdout[-2_500:],
                        "stderr": process.stderr[-2_500:],
                    }
                )
        post_head = run(["git", "-C", str(canonical), "rev-parse", "HEAD"]).stdout.strip()
        post_status = run(["git", "-C", str(canonical), "status", "--porcelain"]).stdout
        workspace_integrity = head == post_head and status == post_status
        evidence = {
            "candidate_commit": candidate,
            "checkout_head": head,
            "checkout_clean": not status.strip() and not post_status.strip() and workspace_integrity,
            "git_status": post_status,
            "precheck_git_status": status,
            "postcheck_head": post_head,
            "postcheck_git_status": post_status,
            "workspace_integrity_passed": workspace_integrity,
            "changed_from_seed": changed,
            "readme": readme,
            "checks": checks,
        }
        evidence_dir = self.run_dir / "context" / "candidate-evidence"
        evidence_dir.mkdir(exist_ok=True)
        evidence_key = hashlib.sha256(
            f"{candidate}\n{head}\n{status}\n{post_head}\n{post_status}".encode()
        ).hexdigest()[:16]
        (evidence_dir / f"{evidence_key}.json").write_text(json.dumps(evidence, indent=2, sort_keys=True))
        return evidence

    def lead_projection(self) -> dict[str, Any]:
        snapshot = self.coordinator.snapshot(self.lead_actor)
        latest_attempts: dict[str, dict[str, Any]] = {}
        for attempt in snapshot["attempts"]:
            latest_attempts[attempt["work_id"]] = {
                "id": attempt["id"],
                "revision": attempt["revision"],
                "actor": attempt["actor"],
                "state": attempt["state"],
                "summary": attempt.get("summary"),
            }
        return {
            "responsibilities": [
                {
                    "id": work["id"],
                    "owner": work["owner"],
                    "outcome": work["outcome"],
                    "expected_artifact": work["expected_artifact"],
                    "status": work["status"],
                    "revision": work["revision"],
                    "feedback": work.get("feedback"),
                    "latest_attempt": latest_attempts.get(work["id"]),
                }
                for work in snapshot["work"]
            ],
            "durable_handoffs": snapshot["edges"],
            "artifacts": snapshot["artifacts"],
            "messages": snapshot["messages_for_actor"],
            "open_judgements": snapshot["open_judgements"],
            "decisions": snapshot["decisions"],
        }

    def coordination_prompt(self, causes: list[dict[str, Any]], cell: str) -> str:
        candidate_evidence = self.candidate_evidence(cell, run_checks=self.mode != MODE_GRAPH)
        if self.mode == MODE_GRAPH:
            return (
            "# Event-driven Exec wake\n\n"
            "## Durable owner directive\n\n"
            + (self.run_dir / "context" / "scenario.md").read_text()
            + "\n\n## Exact current candidate evidence\n\n"
            + f"Candidate commit: `{candidate_evidence['candidate_commit']}`\n\n"
            + "```markdown\n"
            + candidate_evidence["readme"]
            + "\n```\n\n"
            + "Do not re-inspect implementation during this wake. Reconcile the directive against the candidate evidence and Work graph. If further discovery is needed, commission it as Staff Work.\n\n"
            f"Wake causes:\n```json\n{json.dumps(causes, indent=2)}\n```\n\n"
            f"Canonical coordination state:\n```json\n{json.dumps(self.coordinator.snapshot(self.lead_actor), indent=2)}\n```\n\n"
            "React only to these events and canonical evidence. Do not poll running Attempts. Commission, redirect, decide, or end the turn. If produced commits should be combined, commission one integration-lead Work with explicit requires edges."
            )
        if self.mode == MODE_SINGLE:
            return f"""# Strong single-agent outcome wake

## Owner directive

{(self.run_dir / 'context' / 'scenario.md').read_text()}

## Exact native candidate evidence

```json
{json.dumps(candidate_evidence, indent=2)}
```

Wake causes:
```json
{json.dumps(causes, indent=2)}
```

Continue directly from the exact candidate. Inspect and run whatever you need, produce the smallest
coherent playable advance, verify it, commit it cleanly, and truthfully close or leave continuation.
"""
        return f"""# Artifact-triggered Game Product Lead wake

## Owner directive

{(self.run_dir / 'context' / 'scenario.md').read_text()}

## Persistent shared project state

```markdown
{self.project_state()}
```

## Exact native candidate evidence

```json
{json.dumps(candidate_evidence, indent=2)}
```

## New event causes

```json
{json.dumps(causes, indent=2)}
```

## Sparse responsibility and artifact map

```json
{json.dumps(self.lead_projection(), indent=2)}
```

Lead from the evolving product, not the map. Inspect or run the candidate where judgement requires it;
integrate exact useful commits; repair incoherence; commission only genuine cross-actor responsibility;
and update `/company/project-state.md` before quiescing.
"""

    async def coordination_turn(self, causes: list[dict[str, Any]], cell: str) -> dict[str, Any]:
        before = self.project_state() if self.mode == MODE_ARTIFACT else ""
        result = await self.run_turn(
            self.lead_actor,
            self.coordination_prompt(causes, cell),
            cell=cell,
            read_only=self.mode == MODE_GRAPH,
        )
        if self.mode == MODE_ARTIFACT:
            after = self.project_state()
            self.coordinator.emit(
                "project_state_observed",
                {
                    "changed": before != after,
                    "sha256": hashlib.sha256(after.encode()).hexdigest(),
                    "bytes": len(after.encode()),
                },
                self.lead_actor,
            )
        return result

    def staff_prompt(self, item: dict[str, Any]) -> str:
        snapshot = self.coordinator.snapshot(item["owner"])
        required_ids = [
            edge["other_work_id"]
            for edge in snapshot["edges"]
            if edge["work_id"] == item["id"] and edge["kind"] == "requires"
        ]
        latest_produced: dict[str, str] = {}
        for attempt in snapshot["attempts"]:
            if attempt["state"] == "produced":
                latest_produced[attempt["work_id"]] = attempt["id"]
        produced_attempts = set(latest_produced.values())
        inputs = [
            artifact
            for artifact in snapshot["artifacts"]
            if artifact["work_id"] in required_ids and artifact["attempt_id"] in produced_attempts
        ]
        gates = [
            {"name": row["name"], "argv": json.loads(row["argv_json"])}
            for row in self.coordinator.conn.execute(
                "SELECT name,argv_json FROM gates WHERE work_id=? ORDER BY position", (item["id"],)
            ).fetchall()
        ]
        observed_raw = self.coordinator.workspaces.observe(self.coordinator.work(item["id"]))
        observed = {
            "git_status": observed_raw.get("git_status", ""),
            "head": observed_raw.get("head"),
        }
        prior_attempts = [
            {
                "revision": attempt["revision"],
                "actor": attempt["actor"],
                "state": attempt["state"],
                "summary": attempt.get("summary"),
            }
            for attempt in snapshot["attempts"]
            if attempt["work_id"] == item["id"] and attempt["id"] != item["attempt"]
        ]
        diff_result = run(
            ["git", "-C", item["workspace"], "diff", "--no-ext-diff", "--unified=3"],
            check=False,
        )
        workspace_diff = diff_result.stdout
        diff_bytes = workspace_diff.encode()
        if len(diff_bytes) > 16_000:
            workspace_diff = (
                diff_bytes[:16_000].decode(errors="replace")
                + f"\n[diff truncated; sha256={hashlib.sha256(diff_bytes).hexdigest()}; bytes={len(diff_bytes)}]"
            )
        shared_product_context = ""
        if self.mode == MODE_ARTIFACT:
            shared_product_context = f"""
Current product state maintained by the accountable lead:
```markdown
{self.project_state()}
```

The accountable lead has deliberately distilled the broad owner brief into this current situation and
your exact Work. Do not reopen the whole product mission or perform broad repository discovery. Start
from the named extension seam and inspect only the files needed for this bounded contribution. The Work
outcome remains your responsibility; do not replace it with a new project plan.
"""
        return f"""# Claimed Work execution lease

Work: {item['id']}
Revision: {item['revision']}
Attempt: {item['attempt']}
Lease expires: {item['lease_expires_at']}
Outcome: {item['outcome']}
Expected artifact: {item['expected_artifact']}
Workspace: your current working directory
Branch: {item['branch']}
Feedback: {item.get('feedback') or 'none'}

{shared_product_context}

Observed persistent workspace at lease claim:
```json
{json.dumps(observed, indent=2)}
```

Prior Attempts for this same durable Work:
```json
{json.dumps(prior_attempts, indent=2)}
```

Exact uncommitted recovery diff (empty means clean):
```diff
{workspace_diff}
```

Start from this recovery evidence. Continue valid existing work and resolve the unfinished delta before broad repository discovery; do not reapply commits or recreate artifacts already present.

Required produced artifacts:
```json
{json.dumps(inputs, indent=2)}
```

Declared gates:
```json
{json.dumps(gates, indent=2)}
```

The exact Work outcome above is your bounded responsibility; read only the project files it needs.
Work to a clean commit and terminal report. Producers must not mutate the canonical candidate. In the
artifact-led mode, the accountable lead consumes your exact commit directly; in graph-control mode,
integration-lead consumes the required commit references above.
"""

    async def staff_turn(self, item: dict[str, Any]) -> dict[str, Any]:
        result = await self.run_turn(
            item["owner"],
            self.staff_prompt(item),
            cell=item["cell"],
            attempt=item["attempt"],
            lease_token=item["lease_token"],
        )
        attempt = self.coordinator.attempt(item["attempt"])
        if self.stopping or self.coordinator.cost() >= self.spend_ceiling or self.remaining() <= 1:
            if attempt and attempt["state"] == "running":
                self.coordinator.mark_unknown(item["attempt"], "global run envelope ended before terminal report")
            return result
        if attempt and attempt["state"] == "running" and attempt["cancel_requested_at"]:
            self.coordinator.finalize_cancellation(item["attempt"])
            return result
        if attempt and attempt["state"] == "running":
            self.coordinator.mark_unknown(item["attempt"], runtime_unknown_summary(result))
        return result

    async def drain_active_tasks(self) -> None:
        """Stop dispatch and let active actor turns checkpoint naturally before cancellation."""
        self.stopping = True
        if not self.tasks:
            return
        self.coordinator.emit(
            "run_draining",
            {
                "active_actors": sorted(self.tasks),
                "grace_seconds": self.drain_grace_seconds,
            },
        )
        terminal_tasks = [active.task for active in self.tasks.values()]
        _, pending = await asyncio.wait(terminal_tasks, timeout=self.drain_grace_seconds)
        for task in pending:
            task.cancel()
        await asyncio.gather(*terminal_tasks, return_exceptions=True)

    async def execute(self) -> dict[str, Any]:
        codex_pool = self.mode != MODE_SINGLE and all(
            model.startswith("gpt-") for model in self.worker_pool
        )
        if self.mode != MODE_SINGLE and not codex_pool:
            proofs = await asyncio.to_thread(prove_free_worker_pool, self.worker_pool)
            (self.run_dir / "free-worker-proof.json").write_text(json.dumps(proofs, indent=2, sort_keys=True))
            self.coordinator.emit("free_worker_pool_proved", {"models": proofs})
        await self.coordinator.start_server()
        coordination_cell = self.coordinator.workspaces.ensure_coordination_cell(
            self.lead_actor, read_only=self.mode == MODE_GRAPH
        )
        if self.mode != MODE_SINGLE and not codex_pool:
            runtime_proofs = await asyncio.to_thread(
                prove_worker_runtime, coordination_cell, self.worker_pool
            )
            (self.run_dir / "worker-runtime-proof.json").write_text(
                json.dumps(runtime_proofs, indent=2, sort_keys=True)
            )
            self.coordinator.emit("worker_runtime_proved", {"models": runtime_proofs})
        prior_turns = self.coordinator.conn.execute("SELECT COUNT(*) FROM turns").fetchone()[0]
        causes = self.coordinator.pending_causes(self.lead_actor) if prior_turns else []
        if not causes:
            causes = [{
                "cause": "coordinator_resumed" if prior_turns else "owner_directive",
                "payload": {"run": self.run_id, "seed": SEED, "mode": self.mode},
            }]
        task = asyncio.create_task(self.coordination_turn(causes, coordination_cell))
        self.tasks[self.lead_actor] = ActiveTurn(self.lead_actor, task, None)

        while self.remaining() > 1 and self.coordinator.cost() < self.spend_ceiling:
            self.coordinator.fire_due_schedules()
            for cancellation in self.coordinator.pending_cancellations():
                active = self.tasks.get(cancellation["actor"])
                if active and active.attempt == cancellation["id"] and not active.task.done():
                    active.task.cancel()

            staff_active = sum(1 for actor in self.tasks if actor != self.lead_actor)
            available = min(
                self.max_staff - staff_active,
                int(max(0.0, (self.spend_ceiling - self.reserved_cost()) // self.turn_reservation)),
            )
            if available > 0:
                for item in self.coordinator.claim_ready(available, lease_seconds=900):
                    task = asyncio.create_task(self.staff_turn(item))
                    self.tasks[item["owner"]] = ActiveTurn(item["owner"], task, item["attempt"])

            if self.lead_actor not in self.tasks and self.can_launch():
                causes = self.coordinator.pending_causes(self.lead_actor)
                if causes:
                    task = asyncio.create_task(self.coordination_turn(causes, coordination_cell))
                    self.tasks[self.lead_actor] = ActiveTurn(self.lead_actor, task, None)

            if self.coordinator.run_complete() and not self.tasks:
                break

            if (self.reserved_cost() >= self.spend_ceiling * 0.9 or self.remaining() <= 360) and not self.envelope_warned:
                self.coordinator.wake(
                    self.lead_actor,
                    "envelope_warning",
                    {"cost_usd": self.coordinator.cost(), "reserved_cost_usd": self.reserved_cost(), "remaining_seconds": self.remaining()},
                )
                self.envelope_warned = True

            if not self.tasks:
                ready = self.coordinator.conn.execute(
                    "SELECT 1 FROM work WHERE status='active' AND pending_action IS NULL LIMIT 1"
                ).fetchone()
                # This bounded lab launches Staff only for ready Work. A Staff
                # message remains durable context for that actor's next Work
                # wake, but is not itself runnable and must not hold the run
                # open indefinitely. Production OrgIntel supports free-form
                # conversation wakes independently of this comparison runner.
                pending = self.coordinator.conn.execute(
                    "SELECT 1 FROM outbox WHERE target=? AND delivered_at IS NULL LIMIT 1",
                    (self.lead_actor,),
                ).fetchone()
                if not ready and not pending:
                    if self.idle_wakes >= 1:
                        break
                    self.idle_wakes += 1
                    self.coordinator.wake(self.lead_actor, "organisation_idle", {"open_work": False})
                    continue

            waiters = [active.task for active in self.tasks.values()]
            notification = asyncio.create_task(self.queue.get())
            waiters.append(notification)
            delay = self.coordinator.next_schedule_delay()
            timer = asyncio.create_task(asyncio.sleep(min(delay if delay is not None else self.remaining(), self.remaining())))
            waiters.append(timer)
            done, _ = await asyncio.wait(waiters, return_when=asyncio.FIRST_COMPLETED)
            if notification not in done:
                notification.cancel()
            if timer not in done:
                timer.cancel()
            for actor, active in list(self.tasks.items()):
                if active.task.done():
                    try:
                        await active.task
                    except Exception as exc:
                        self.coordinator.emit("actor_task_failed", {"error": str(exc)}, actor)
                        if active.attempt:
                            self.coordinator.mark_unknown(active.attempt, f"actor task failed: {exc}")
                            self.coordinator.wake(
                                self.lead_actor,
                                "actor_task_failed",
                                {"actor": actor, "error": str(exc)},
                            )
                    self.tasks.pop(actor, None)

        await self.drain_active_tasks()
        for active in self.tasks.values():
            if active.attempt:
                self.coordinator.mark_unknown(active.attempt, "global run envelope ended")
        self.coordinator.emit(
            "run_terminal",
            {
                "cost_usd": self.coordinator.cost(),
                "decision_complete": self.coordinator.run_complete(),
                "elapsed_seconds": time.monotonic() - self.started,
            },
        )
        summary = self.coordinator.summary()
        summary["run"] = self.run_id
        summary["mode"] = self.mode
        summary["lead_actor"] = self.lead_actor
        summary["lead_model"] = self.lead_model
        summary["worker_model_pool"] = self.worker_pool
        summary["candidate_evidence"] = self.candidate_evidence(
            coordination_cell, run_checks=self.mode != MODE_GRAPH
        )
        if self.mode == MODE_ARTIFACT:
            summary["project_state"] = self.project_state()
        (self.run_dir / "summary.json").write_text(json.dumps(summary, indent=2, sort_keys=True))
        await self.coordinator.stop_server()
        self.coordinator.close()
        return summary


def command_payload(
    actor: str,
    name: str,
    key: str,
    args: dict[str, Any],
    *,
    attempt: str = "",
    lease_token: str = "",
) -> dict[str, Any]:
    return {
        "type": "command",
        "actor": actor,
        "attempt": attempt,
        "lease_token": lease_token,
        "idempotency_key": key,
        "name": name,
        "args": args,
    }


async def fault_test(run_id: str) -> dict[str, Any]:
    run_dir = prepare(run_id)
    queue: asyncio.Queue[dict[str, Any]] = asyncio.Queue()
    coordinator = Coordinator(run_dir, run_id, queue)
    await coordinator.start_server()
    endpoint = f"127.0.0.1:{coordinator.port}"
    checks: list[dict[str, Any]] = []

    def check(name: str, condition: bool, detail: Any = None) -> None:
        checks.append({"name": name, "pass": bool(condition), "detail": detail})
        if not condition:
            raise AssertionError(f"{name}: {detail}")

    projected_failure = runtime_unknown_summary(
        {
            "stop_reason": "error",
            "model": "provider/model:free",
            "error": "429: upstream provider shared pool is rate limited",
        }
    )
    check(
        "unknown Attempt retains bounded Runtime recovery evidence",
        '"runtime_outcome": "error"' in projected_failure
        and '"model": "provider/model:free"' in projected_failure
        and "429" in projected_failure
        and len(projected_failure) < 1_500,
        projected_failure,
    )

    async def call(payload: dict[str, Any]) -> dict[str, Any]:
        return await asyncio.to_thread(request, endpoint, payload)

    producer_args = {
        "owner": "gameplay-systems",
        "outcome": "Produce a clean seed-derived probe commit",
        "expected_artifact": "clean commit",
        "gates": [{"name": "head", "argv": ["git", "rev-parse", "HEAD"]}],
    }
    first = await call(command_payload("exec", "commission", "fault-producer", producer_args))
    duplicate = await call(command_payload("exec", "commission", "fault-producer", producer_args))
    check("duplicate command returns original result", first == duplicate, {"first": first, "duplicate": duplicate})
    check(
        "duplicate command creates one Work",
        coordinator.conn.execute("SELECT COUNT(*) FROM work WHERE id=?", (first["work"],)).fetchone()[0] == 1,
    )
    try:
        await call(
            command_payload(
                "exec",
                "commission",
                "fault-producer",
                {**producer_args, "outcome": "different input must fail"},
            )
        )
        changed_key_rejected = False
    except RuntimeError:
        changed_key_rejected = True
    check("same idempotency key with different input is rejected", changed_key_rejected)

    claimed = coordinator.claim_ready(1, lease_seconds=900)
    check("producer Work claims exactly one Attempt", len(claimed) == 1, claimed)
    item = claimed[0]
    mounts = json.loads(run(["docker", "inspect", item["cell"]]).stdout)[0]["Mounts"]
    destinations = {mount["Destination"]: mount["RW"] for mount in mounts}
    check("Work cell mounts only scoped project workspace", destinations.get("/workspace") is True, destinations)
    check("Work cell has a writable persistent OS home", destinations.get("/company") is True, destinations)
    check("Work cell has no canonical or database mount", "/lab" not in destinations and "/canonical" not in destinations, destinations)
    isolation = run(
        [
            "docker",
            "exec",
            "-u",
            "company",
            item["cell"],
            "sh",
            "-c",
            "test -w /workspace && test -w /company && test ! -w /context && test ! -e /lab",
        ],
        check=False,
    )
    check("Work cell project write scope is isolated", isolation.returncode == 0, isolation.stderr)

    mcp_probe = await asyncio.to_thread(
        subprocess.run,
        [
            "docker",
            "exec",
            "-i",
            "-u",
            "company",
            "-e",
            f"COORD_ENDPOINT=host.docker.internal:{coordinator.port}",
            "-e",
            "COORD_ACTOR=gameplay-systems",
            "-e",
            f"COORD_ATTEMPT={item['attempt']}",
            "-e",
            f"COORD_LEASE_TOKEN={item['lease_token']}",
            item["cell"],
            "python3",
            "/harness/v2/mcp_server.py",
        ],
        input=(
            json.dumps({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2024-11-05"}})
            + "\n"
            + json.dumps(
                {"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "inspect_coordination", "arguments": {}}}
            )
            + "\n"
        ),
        text=True,
        capture_output=True,
        timeout=30,
    )
    mcp_lines = [json.loads(line) for line in mcp_probe.stdout.splitlines() if line.strip()]
    check(
        "thin MCP traverses Work cell to single writer",
        mcp_probe.returncode == 0 and len(mcp_lines) == 2 and "result" in mcp_lines[-1],
        mcp_probe.stderr,
    )

    producer_workspace = Path(item["workspace"])
    (producer_workspace / "fault-producer.txt").write_text("advanced producer output\n")
    run(["git", "-C", str(producer_workspace), "add", "fault-producer.txt"])
    run(["git", "-C", str(producer_workspace), "commit", "-m", "fault: advance producer output"])
    report_args = {"disposition": "outcome_met", "summary": "Advanced producer commit is a valid probe", "evidence": ["head gate"]}
    reported = await call(
        command_payload(
            "gameplay-systems",
            "report",
            "fault-report",
            report_args,
            attempt=item["attempt"],
            lease_token=item["lease_token"],
        )
    )
    repeated_report = await call(
        command_payload(
            "gameplay-systems",
            "report",
            "fault-report",
            report_args,
            attempt=item["attempt"],
            lease_token=item["lease_token"],
        )
    )
    check("duplicate terminal callback is harmless", reported == repeated_report and reported["state"] == "produced")

    two_phase = await call(
        command_payload(
            "exec",
            "commission",
            "fault-two-phase-work",
            {
                "owner": "world-content",
                "outcome": "Repair a failed candidate gate inside one live Attempt",
                "expected_artifact": "one verified clean commit",
                "gates": [{"name": "ready", "argv": ["test", "-s", "gate-ready.txt"]}],
            },
        )
    )
    two_phase_item = next(
        item for item in coordinator.claim_ready(1, lease_seconds=900) if item["id"] == two_phase["work"]
    )
    two_phase_workspace = Path(two_phase_item["workspace"])
    (two_phase_workspace / "candidate-before-gate.txt").write_text("preserved candidate\n")
    run(["git", "-C", str(two_phase_workspace), "add", "candidate-before-gate.txt"])
    run(["git", "-C", str(two_phase_workspace), "commit", "-m", "fault: candidate before gate repair"])
    rejected_commit = run(["git", "-C", str(two_phase_workspace), "rev-parse", "HEAD"]).stdout.strip()
    revision_required = await call(
        command_payload(
            "world-content",
            "report",
            "fault-two-phase-submit-1",
            {
                "disposition": "outcome_met",
                "summary": "first candidate requires a gate repair",
                "artifacts": [{"kind": "commit", "reference": rejected_commit}],
            },
            attempt=two_phase_item["attempt"],
            lease_token=two_phase_item["lease_token"],
        )
    )
    check(
        "failed candidate gate keeps the same Attempt live",
        revision_required.get("candidate_status") == "revision_required"
        and revision_required["state"] == "running"
        and coordinator.attempt(two_phase_item["attempt"])["state"] == "running"
        and coordinator.work(two_phase["work"])["status"] == "active",
        revision_required,
    )
    check(
        "failed candidate gate records no accepted artifact",
        coordinator.conn.execute(
            "SELECT COUNT(*) FROM artifacts WHERE work_id=?", (two_phase["work"],)
        ).fetchone()[0]
        == 0
        and run(
            [
                "git",
                "-C",
                str(coordinator.workspaces.canonical),
                "show-ref",
                "--verify",
                f"refs/heads/artifacts/{two_phase['work']}",
            ],
            check=False,
        ).returncode
        != 0,
    )
    (two_phase_workspace / "gate-ready.txt").write_text("ready\n")
    run(["git", "-C", str(two_phase_workspace), "add", "gate-ready.txt"])
    run(["git", "-C", str(two_phase_workspace), "commit", "-m", "fault: repair candidate gate"])
    accepted_commit = run(["git", "-C", str(two_phase_workspace), "rev-parse", "HEAD"]).stdout.strip()
    accepted = await call(
        command_payload(
            "world-content",
            "report",
            "fault-two-phase-submit-2",
            {
                "disposition": "outcome_met",
                "summary": "repaired candidate passes in the original Attempt",
                "artifacts": [{"kind": "commit", "reference": accepted_commit}],
            },
            attempt=two_phase_item["attempt"],
            lease_token=two_phase_item["lease_token"],
        )
    )
    accepted_rows = coordinator.conn.execute(
        "SELECT reference,observed FROM artifacts WHERE work_id=? AND kind='commit'",
        (two_phase["work"],),
    ).fetchall()
    check(
        "repaired candidate terminalises the original Attempt exactly once",
        accepted["state"] == "produced"
        and coordinator.attempt(two_phase_item["attempt"])["state"] == "produced"
        and [(row["reference"], row["observed"]) for row in accepted_rows] == [(accepted_commit, 1)],
        {"result": accepted, "artifacts": [dict(row) for row in accepted_rows]},
    )

    try:
        await call(
            command_payload(
                "exec",
                "commission",
                "fault-shell-shaped-gate",
                {
                    "owner": "gameplay-systems",
                    "outcome": "Reject an ambiguous shell-shaped gate",
                    "expected_artifact": "none",
                    "gates": [{"name": "bad", "argv": ["test -f file && test -s file"]}],
                },
            )
        )
        shell_gate_rejected = False
    except RuntimeError:
        shell_gate_rejected = True
    check("shell-shaped single-string gate is rejected at commission", shell_gate_rejected)

    try:
        await call(
            command_payload(
                "exec",
                "commission",
                "fault-unbound-critic",
                {
                    "owner": "artifact-critic",
                    "outcome": "Review no produced outcome",
                    "expected_artifact": "must be rejected",
                },
            )
        )
        unbound_critic_rejected = False
    except RuntimeError:
        unbound_critic_rejected = True
    check("critic Work without a completed dependency is rejected", unbound_critic_rejected)

    no_op_work = await call(
        command_payload(
            "exec",
            "commission",
            "fault-no-op-work",
            {
                "owner": "gameplay-systems",
                "outcome": "Exercise unchanged-base completion rejection",
                "expected_artifact": "new commit",
            },
        )
    )
    no_op_item = next(item for item in coordinator.claim_ready(1, lease_seconds=900) if item["id"] == no_op_work["work"])
    try:
        await call(
            command_payload(
                "gameplay-systems",
                "report",
                "fault-no-op-report",
                {"disposition": "outcome_met", "summary": "unchanged base must fail"},
                attempt=no_op_item["attempt"],
                lease_token=no_op_item["lease_token"],
            )
        )
        no_op_rejected = False
    except RuntimeError:
        no_op_rejected = True
    check("unchanged base cannot satisfy new Work", no_op_rejected)
    await call(
        command_payload(
            "gameplay-systems",
            "report",
            "fault-no-op-blocked",
            {"disposition": "blocked", "summary": "no-op correctly rejected"},
            attempt=no_op_item["attempt"],
            lease_token=no_op_item["lease_token"],
        )
    )

    stale_work = await call(
        command_payload(
            "exec",
            "commission",
            "fault-stale-work",
            {
                "owner": "world-content",
                "outcome": "Exercise stale callback rejection",
                "expected_artifact": "clean commit",
            },
        )
    )
    stale_item = next(item for item in coordinator.claim_ready(1, lease_seconds=900) if item["id"] == stale_work["work"])
    pending = await call(
        command_payload(
            "exec",
            "redirect",
            "fault-repair-running",
            {"work": stale_work["work"], "action": "repair", "reason": "fault-injected cancellation"},
        )
    )
    check("redirect against running Attempt is pending", pending.get("pending") is True, pending)
    coordinator.finalize_cancellation(stale_item["attempt"])
    repaired = coordinator.claim_ready(1, lease_seconds=900)[0]
    check("repair preserves Work and increments revision", repaired["id"] == stale_item["id"] and repaired["revision"] == 2)
    try:
        await call(
            command_payload(
                "world-content",
                "report",
                "fault-stale-report",
                {"disposition": "blocked", "summary": "stale"},
                attempt=stale_item["attempt"],
                lease_token=stale_item["lease_token"],
            )
        )
        stale_rejected = False
    except RuntimeError:
        stale_rejected = True
    check("stale callback after repair is rejected", stale_rejected)

    for number in range(2):
        await call(
            command_payload(
                "exec",
                "commission",
                f"fault-one-lease-{number}",
                {
                    "owner": "experience-presentation",
                    "outcome": f"Actor lease probe {number}",
                    "expected_artifact": "clean commit",
                },
            )
        )
    same_actor_claims = [item for item in coordinator.claim_ready(3, lease_seconds=900) if item["owner"] == "experience-presentation"]
    check("one actor cannot hold two running leases", len(same_actor_claims) == 1, same_actor_claims)

    try:
        await call(
            command_payload(
                "exec",
                "commission",
                "fault-incomplete-integration",
                {
                    "owner": "integration-lead",
                    "outcome": "Integrate incomplete Work",
                    "expected_artifact": "must be rejected",
                    "requires": [stale_work["work"]],
                },
            )
        )
        incomplete_rejected = False
    except RuntimeError:
        incomplete_rejected = True
    check("integration cannot require incomplete Work", incomplete_rejected)

    integration = await call(
        command_payload(
            "exec",
            "commission",
            "fault-integration",
            {
                "owner": "integration-lead",
                "outcome": "Integrate produced probe",
                "expected_artifact": "one candidate commit",
                "requires": [first["work"]],
            },
        )
    )
    try:
        await call(
            command_payload(
                "exec",
                "commission",
                "fault-integration-two",
                {
                    "owner": "integration-lead",
                    "outcome": "Competing integration",
                    "expected_artifact": "must be rejected",
                    "requires": [first["work"]],
                },
            )
        )
        contention_rejected = False
    except RuntimeError:
        contention_rejected = True
    check("second active integration lease is rejected", contention_rejected, integration)

    integration_item = next(
        item for item in coordinator.claim_ready(10, lease_seconds=900) if item["id"] == integration["work"]
    )
    candidate_before = run(["git", "-C", str(coordinator.workspaces.canonical), "rev-parse", "candidate"]).stdout.strip()
    check("integration lease binds candidate at claim", integration_item["base_ref"] == candidate_before)
    integration_workspace = Path(integration_item["workspace"])
    (integration_workspace / "fault-integration-1.txt").write_text("first candidate advance\n")
    run(["git", "-C", str(integration_workspace), "add", "fault-integration-1.txt"])
    run(["git", "-C", str(integration_workspace), "commit", "-m", "fault: first candidate advance"])
    first_advance = run(["git", "-C", str(integration_workspace), "rev-parse", "HEAD"]).stdout.strip()
    await call(
        command_payload(
            "integration-lead",
            "report",
            "fault-integration-report-1",
            {"disposition": "outcome_met", "summary": "first integration advance"},
            attempt=integration_item["attempt"],
            lease_token=integration_item["lease_token"],
        )
    )
    promoted = run(["git", "-C", str(coordinator.workspaces.canonical), "rev-parse", "candidate"]).stdout.strip()
    check("integration report promotes candidate with compare-and-swap", promoted == first_advance)

    await call(
        command_payload(
            "exec",
            "redirect",
            "fault-integration-repair",
            {"work": integration["work"], "action": "repair", "reason": "exercise advanced-candidate retry"},
        )
    )
    repaired_integration = next(
        item for item in coordinator.claim_ready(10, lease_seconds=900) if item["id"] == integration["work"]
    )
    check(
        "repaired integration lease rebinds advanced candidate",
        repaired_integration["base_ref"] == first_advance,
        repaired_integration,
    )
    (integration_workspace / "fault-integration-2.txt").write_text("second candidate advance\n")
    run(["git", "-C", str(integration_workspace), "add", "fault-integration-2.txt"])
    run(["git", "-C", str(integration_workspace), "commit", "-m", "fault: second candidate advance"])
    second_advance = run(["git", "-C", str(integration_workspace), "rev-parse", "HEAD"]).stdout.strip()
    await call(
        command_payload(
            "integration-lead",
            "report",
            "fault-integration-report-2",
            {"disposition": "outcome_met", "summary": "second integration advance"},
            attempt=repaired_integration["attempt"],
            lease_token=repaired_integration["lease_token"],
        )
    )
    promoted_again = run(["git", "-C", str(coordinator.workspaces.canonical), "rev-parse", "candidate"]).stdout.strip()
    check("repaired integration advances candidate without stale-base rejection", promoted_again == second_advance)

    review = await call(
        command_payload(
            "exec",
            "commission",
            "fault-review",
            {
                "owner": "artifact-critic",
                "outcome": "Review the latest integrated candidate",
                "expected_artifact": "independent review",
                "requires": [integration["work"]],
            },
        )
    )
    review_item = next(item for item in coordinator.claim_ready(10, lease_seconds=900) if item["id"] == review["work"])
    check("single-dependency Work leases latest produced commit", review_item["base_ref"] == second_advance)
    review_workspace = Path(review_item["workspace"])
    (review_workspace / "fault-review-note.txt").write_text("review of second candidate\n")
    run(["git", "-C", str(review_workspace), "add", "fault-review-note.txt"])
    run(["git", "-C", str(review_workspace), "commit", "-m", "fault: review second candidate"])
    await call(
        command_payload(
            "artifact-critic",
            "report",
            "fault-review-report-1",
            {"disposition": "blocked", "summary": "candidate needs another integration revision"},
            attempt=review_item["attempt"],
            lease_token=review_item["lease_token"],
        )
    )

    await call(
        command_payload(
            "exec",
            "redirect",
            "fault-integration-repair-2",
            {"work": integration["work"], "action": "repair", "reason": "advance dependency after review"},
        )
    )
    integration_third = next(
        item for item in coordinator.claim_ready(10, lease_seconds=900) if item["id"] == integration["work"]
    )
    (integration_workspace / "fault-integration-3.txt").write_text("third candidate advance\n")
    run(["git", "-C", str(integration_workspace), "add", "fault-integration-3.txt"])
    run(["git", "-C", str(integration_workspace), "commit", "-m", "fault: third candidate advance"])
    third_advance = run(["git", "-C", str(integration_workspace), "rev-parse", "HEAD"]).stdout.strip()
    await call(
        command_payload(
            "integration-lead",
            "report",
            "fault-integration-report-3",
            {"disposition": "outcome_met", "summary": "third integration advance"},
            attempt=integration_third["attempt"],
            lease_token=integration_third["lease_token"],
        )
    )

    await call(
        command_payload(
            "exec",
            "redirect",
            "fault-review-repair",
            {"work": review["work"], "action": "repair", "reason": "review the advanced dependency"},
        )
    )
    review_retry = next(item for item in coordinator.claim_ready(10, lease_seconds=900) if item["id"] == review["work"])
    check("downstream retry rebinds advanced dependency", review_retry["base_ref"] == third_advance)
    check("divergent downstream retry gets a new input-epoch workspace", review_retry["workspace"] != str(review_workspace))
    retry_head = run(["git", "-C", review_retry["workspace"], "rev-parse", "HEAD"]).stdout.strip()
    check("new input-epoch workspace starts at advanced dependency", retry_head == third_advance)
    await call(
        command_payload(
            "artifact-critic",
            "report",
            "fault-review-report-2",
            {"disposition": "blocked", "summary": "input epoch verified"},
            attempt=review_retry["attempt"],
            lease_token=review_retry["lease_token"],
        )
    )

    concurrent = [
        call(
            command_payload(
                "exec",
                "send",
                f"fault-concurrent-{number}",
                {"to": "exec", "body": f"concurrent message {number}", "refs": []},
            )
        )
        for number in range(24)
    ]
    await asyncio.gather(*concurrent)
    traces = [
        call_trace(endpoint, number)
        for number in range(80)
    ]
    await asyncio.gather(*traces)
    check("single-writer database survives concurrent clients", coordinator.conn.execute("PRAGMA quick_check").fetchone()[0] == "ok")
    valid_trace = all(json.loads(line) for line in coordinator.trace_path.read_text().splitlines())
    check("single trace writer emits valid JSONL", valid_trace)

    await coordinator.stop_server()
    coordinator.close()
    restarted = Coordinator(run_dir, run_id, queue)
    check(
        "durable outbox survives coordinator restart",
        restarted.conn.execute("SELECT COUNT(*) FROM outbox WHERE delivered_at IS NULL").fetchone()[0] > 0,
    )
    check("database remains healthy after restart", restarted.conn.execute("PRAGMA quick_check").fetchone()[0] == "ok")
    orphaned = restarted.reconcile_orphaned_attempts()
    check("restart reconciles orphaned running Attempts", bool(orphaned), orphaned)
    check(
        "restart leaves no running Attempt without a controller",
        restarted.conn.execute("SELECT COUNT(*) FROM attempts WHERE state='running'").fetchone()[0] == 0,
    )
    result = {"run": run_id, "checks": checks, "passed": len(checks), "quick_check": "ok"}
    (run_dir / "fault-results.json").write_text(json.dumps(result, indent=2, sort_keys=True))
    restarted.close()
    return result


async def positive_callback_probe(run_id: str) -> dict[str, Any]:
    run_dir = prepare(run_id)
    lab = LabRun(run_id)
    await lab.coordinator.start_server()
    try:
        commissioned = lab.coordinator.command(
            command_payload(
                "exec",
                "commission",
                "positive-callback-work",
                {
                    "owner": "experience-presentation",
                    "outcome": (
                        "Create docs/v13-positive-callback.md containing the exact line "
                        "RESTLESS_POSITIVE_CALLBACK_V13. Commit the file and call terminal "
                        "report(outcome_met) in this same actor process. Do not inspect unrelated files."
                    ),
                    "expected_artifact": "docs/v13-positive-callback.md",
                    "gates": [
                        {
                            "name": "artifact-nonempty",
                            "argv": ["test", "-s", "docs/v13-positive-callback.md"],
                        }
                    ],
                },
            )
        )
        claimed = lab.coordinator.claim_ready(1, lease_seconds=900)
        if len(claimed) != 1 or claimed[0]["id"] != commissioned["work"]:
            raise RuntimeError(f"positive probe did not claim its one Work: {claimed}")
        result = await lab.staff_turn(claimed[0])
        summary = lab.coordinator.summary()
        summary["actor_result"] = result
        summary["run"] = run_id
        lab.coordinator.emit(
            "run_terminal",
            {
                "cost_usd": lab.coordinator.cost(),
                "decision_complete": False,
                "focused_probe": "positive_callback",
            },
        )
        (run_dir / "summary.json").write_text(json.dumps(summary, indent=2, sort_keys=True))
        return summary
    finally:
        await lab.coordinator.stop_server()
        lab.coordinator.close()


async def artifact_architecture_test(run_id: str) -> dict[str, Any]:
    """Deterministically prove the v21 leadership/context/integration seams."""
    run_dir = prepare(
        run_id,
        mode=MODE_ARTIFACT,
        lead_model="test/sonnet-lead",
        worker_pool=["test/free-a", "test/free-b"],
        spend_ceiling_usd=1.0,
    )
    lab = LabRun(run_id)
    coordinator = lab.coordinator
    checks: list[dict[str, Any]] = []

    def check(name: str, condition: bool, detail: Any = None) -> None:
        checks.append({"name": name, "pass": bool(condition), "detail": detail})
        if not condition:
            raise AssertionError(f"{name}: {detail}")

    check("artifact-led run appoints the product lead as coordinator", coordinator.coordination_actor == "studio-lead")
    project_state = lab.project_state_path()
    check("persistent project state exists outside the product repo", project_state.exists())
    marker = "Deterministic architecture probe: integrate two exact worker commits."
    project_state.write_text(lab.project_state() + f"\n- {marker}\n")

    lead_cell = coordinator.workspaces.ensure_coordination_cell("studio-lead", read_only=False)
    mounts = json.loads(run(["docker", "inspect", lead_cell]).stdout)[0]["Mounts"]
    destinations = {mount["Destination"]: mount["RW"] for mount in mounts}
    check("product lead owns one writable canonical candidate", destinations.get("/workspace") is True, destinations)
    check("product state lives in the lead's persistent home", destinations.get("/company") is True, destinations)

    self_commission_rejected = False
    try:
        coordinator.command(
            command_payload(
                "studio-lead",
                "commission",
                "architecture-self-commission",
                {
                    "owner": "studio-lead",
                    "outcome": "Invalid self-owned coordination node",
                    "expected_artifact": "none",
                },
            )
        )
    except ValueError:
        self_commission_rejected = True
    check("coordinator cannot accidentally commission itself", self_commission_rejected)

    commissioned: list[str] = []
    for number, actor in enumerate(("gameplay-systems", "world-content"), start=1):
        result = coordinator.command(
            command_payload(
                "studio-lead",
                "commission",
                f"architecture-producer-{number}",
                {
                    "owner": actor,
                    "outcome": f"Produce deterministic contribution {number}",
                    "expected_artifact": f"architecture-contribution-{number}.txt in a clean commit",
                    "gates": [
                        {
                            "name": f"contribution-{number}",
                            "argv": ["test", "-s", f"architecture-contribution-{number}.txt"],
                        }
                    ],
                },
            )
        )
        commissioned.append(result["work"])

    claimed = coordinator.claim_ready(2, lease_seconds=900)
    check("two durable cross-actor responsibilities can run in parallel", len(claimed) == 2, claimed)
    for number, item in enumerate(claimed, start=1):
        prompt = lab.staff_prompt(item)
        check(
            f"worker {number} receives distilled project state without the full design brief",
            marker in prompt
            and "Current product state maintained by the accountable lead" in prompt
            and "FULL DESIGN BRIEF FOLLOWS" not in prompt,
        )
        workspace = Path(item["workspace"])
        output = workspace / f"architecture-contribution-{number}.txt"
        output.write_text(f"contribution {number}\n")
        run(["git", "-C", str(workspace), "add", output.name])
        run(["git", "-C", str(workspace), "commit", "-m", f"architecture: contribution {number}"])
        coordinator.command(
            command_payload(
                item["owner"],
                "report",
                f"architecture-report-{number}",
                {"disposition": "outcome_met", "summary": f"Produced contribution {number}"},
                attempt=item["attempt"],
                lease_token=item["lease_token"],
            )
        )

    causes = coordinator.pending_causes("studio-lead")
    terminal_causes = [cause for cause in causes if cause["cause"] == "attempt_terminal"]
    check("artifact completion wakes the accountable lead", len(terminal_causes) == 2, causes)

    artifacts = coordinator.snapshot("studio-lead")["artifacts"]
    commits = [artifact["reference"] for artifact in artifacts if artifact["kind"] == "commit"]
    check("worker handoffs carry exact imported commits", len(commits) == 2, artifacts)
    canonical = run_dir / "canonical"
    for commit in commits:
        run(["git", "-C", str(canonical), "cherry-pick", commit])
    integrated = run(["git", "-C", str(canonical), "rev-parse", "candidate"]).stdout.strip()
    check("lead integration advances one canonical candidate", integrated != SEED, integrated)
    check(
        "lead integrates without manufacturing integration Work",
        coordinator.conn.execute("SELECT COUNT(*) FROM work WHERE owner='studio-lead'").fetchone()[0] == 0,
    )

    review = coordinator.command(
        command_payload(
            "studio-lead",
            "commission",
            "architecture-critic",
            {
                "owner": "artifact-critic",
                "outcome": "Independently review the integrated deterministic candidate",
                "expected_artifact": "concrete review note",
                "requires": commissioned,
            },
        )
    )
    review_item = next(item for item in coordinator.claim_ready(4, lease_seconds=900) if item["id"] == review["work"])
    check("critic starts from the lead-integrated native candidate", review_item["base_ref"] == integrated, review_item)
    coordinator.command(
        command_payload(
            "artifact-critic",
            "report",
            "architecture-critic-blocked",
            {"disposition": "blocked", "summary": "deterministic review lease verified"},
            attempt=review_item["attempt"],
            lease_token=review_item["lease_token"],
        )
    )

    fixture_probe = canonical / "fixture-ownership-probe.mjs"
    fixture_probe.write_text(
        "const response = await fetch('http://127.0.0.1:8199/index.html');\n"
        "if (!response.ok) process.exit(1);\n"
        "console.log('FIXTURE_OWNERSHIP_READY');\n"
    )
    run(["git", "-C", str(canonical), "add", fixture_probe.name])
    run(
        [
            "git",
            "-C",
            str(canonical),
            "commit",
            "-m",
            "Add deterministic fixture ownership probe",
        ]
    )
    integrated = run(["git", "-C", str(canonical), "rev-parse", "candidate"]).stdout.strip()
    fixture_result, fixture_mode = lab.run_candidate_check(
        lead_cell, canonical, fixture_probe.name, integrated
    )
    check(
        "legacy client-only proof receives one isolated ephemeral fixture",
        fixture_result.returncode == 0
        and fixture_mode == "ephemeral_static_fallback"
        and "FIXTURE_OWNERSHIP_READY" in fixture_result.stdout,
        {"mode": fixture_mode, "stdout": fixture_result.stdout, "stderr": fixture_result.stderr},
    )

    evidence = lab.candidate_evidence(lead_cell, run_checks=False)
    check("native candidate evidence names the integrated commit", evidence["candidate_commit"] == integrated, evidence)
    check("canonical candidate is clean after lead integration", evidence["checkout_clean"] is True, evidence)
    check("graph remains a sparse responsibility map", len(coordinator.snapshot("studio-lead")["work"]) == 3)
    check("coordination database remains healthy", coordinator.conn.execute("PRAGMA quick_check").fetchone()[0] == "ok")

    identity_env = os.environ.copy()
    identity_env.update(
        {
            "COORD_EVENT_ENDPOINT": "127.0.0.1:9",
            "COORD_ACTOR": "studio-lead",
            "COORD_TURN_ID": "architecture-identity-probe",
            "COORD_HOST_WORKDIR": str(canonical),
            "COORD_EXPECTED_BRANCH": "candidate",
            "COORD_WORK": "",
        }
    )
    identity_code = (
        f"import sys; sys.path.insert(0,{str(HERE)!r}); "
        "import codex_turn; codex_turn.validate_runtime_identity()"
    )
    identity_ok = subprocess.run(
        [sys.executable, "-c", identity_code],
        env=identity_env,
        text=True,
        capture_output=True,
    )
    identity_env["COORD_EXPECTED_BRANCH"] = "wrong-branch"
    identity_bad = subprocess.run(
        [sys.executable, "-c", identity_code],
        env=identity_env,
        text=True,
        capture_output=True,
    )
    check(
        "Actor Host verifies exact workspace and branch before model launch",
        identity_ok.returncode == 0
        and identity_bad.returncode != 0
        and "actor branch mismatch" in identity_bad.stderr,
        {"good": identity_ok.stderr, "bad": identity_bad.stderr[-500:]},
    )

    drain_marker: list[str] = []

    async def finish_inside_grace() -> dict[str, Any]:
        await asyncio.sleep(0.05)
        drain_marker.append("checkpointed")
        return {"stop_reason": "endturn"}

    drain_task = asyncio.create_task(finish_inside_grace())
    lab.tasks = {"drain-probe": ActiveTurn("drain-probe", drain_task, None)}
    lab.drain_grace_seconds = 0.2
    await lab.drain_active_tasks()
    check(
        "outer envelope drains a productive actor turn before cancellation",
        drain_task.done() and not drain_task.cancelled() and drain_marker == ["checkpointed"],
        drain_marker,
    )
    lab.tasks = {}

    result = {
        "run": run_id,
        "mode": MODE_ARTIFACT,
        "checks": checks,
        "passed": len(checks),
        "candidate": integrated,
        "quick_check": "ok",
    }
    (run_dir / "architecture-results.json").write_text(json.dumps(result, indent=2, sort_keys=True))
    coordinator.close()
    return result


async def positive_callback_repair_probe(run_id: str) -> dict[str, Any]:
    lab = LabRun(run_id)
    await lab.coordinator.start_server()
    try:
        row = lab.coordinator.conn.execute(
            "SELECT * FROM work WHERE expected_artifact='docs/v13-positive-callback.md' ORDER BY created_at DESC LIMIT 1"
        ).fetchone()
        if not row or row["status"] != "blocked":
            raise RuntimeError("positive callback Work is not durably blocked for repair")
        repaired = lab.coordinator.command(
            command_payload(
                "exec",
                "redirect",
                "positive-callback-provider-repair",
                {
                    "work": row["id"],
                    "action": "repair",
                    "reason": "Retry the same durable Work after a transient free-provider 429",
                },
            )
        )
        claimed = lab.coordinator.claim_ready(1, lease_seconds=900)
        if len(claimed) != 1 or claimed[0]["id"] != repaired["work"]:
            raise RuntimeError(f"positive repair did not claim its one Work: {claimed}")
        result = await lab.staff_turn(claimed[0])
        summary = lab.coordinator.summary()
        summary["actor_result"] = result
        summary["run"] = run_id
        summary["focused_probe"] = "positive_callback_provider_repair"
        lab.coordinator.emit(
            "run_terminal",
            {
                "cost_usd": lab.coordinator.cost(),
                "decision_complete": False,
                "focused_probe": "positive_callback_provider_repair",
            },
        )
        (lab.run_dir / "summary.json").write_text(json.dumps(summary, indent=2, sort_keys=True))
        return summary
    finally:
        await lab.coordinator.stop_server()
        lab.coordinator.close()


async def call_trace(endpoint: str, number: int) -> dict[str, Any]:
    return await asyncio.to_thread(
        request,
        endpoint,
        {
            "type": "trace",
            "at": str(int(time.time() * 1000)),
            "actor": "fault",
            "turn_id": "fault-turn",
            "kind": "fault_trace",
            "payload": {"number": number},
        },
    )


def prepare_experiment(
    experiment_id: str,
    *,
    lead_model: str,
    worker_pool: list[str],
    spend_ceiling_usd: float,
    wall_clock_seconds: int,
) -> dict[str, Any]:
    experiment = safe_name(experiment_id)
    arms = {
        "single_agent": f"{experiment}-single",
        "graph_control": f"{experiment}-graph",
        "artifact_led": f"{experiment}-artifact",
    }
    for mode, run_id in (
        (MODE_SINGLE, arms["single_agent"]),
        (MODE_GRAPH, arms["graph_control"]),
        (MODE_ARTIFACT, arms["artifact_led"]),
    ):
        prepare(
            run_id,
            mode=mode,
            lead_model=lead_model,
            worker_pool=worker_pool,
            spend_ceiling_usd=spend_ceiling_usd,
            wall_clock_seconds=wall_clock_seconds,
        )
    manifest = {
        "experiment": experiment,
        "seed": SEED,
        "lead_model": lead_model,
        "worker_model_pool": worker_pool,
        "spend_ceiling_usd_per_arm": spend_ceiling_usd,
        "wall_clock_seconds_per_arm": wall_clock_seconds,
        "arms": arms,
        "hypothesis": (
            "A persistent Sonnet outcome lead with shared project state and direct native-artifact "
            "integration produces a better accepted candidate than graph-control coordination, "
            "and adds value beyond the strongest single member."
        ),
    }
    path = WORK_ROOT / f"{experiment}-experiment.json"
    path.write_text(json.dumps(manifest, indent=2, sort_keys=True))
    return {**manifest, "manifest": str(path)}


async def run_experiment(experiment_id: str) -> dict[str, Any]:
    experiment = safe_name(experiment_id)
    path = WORK_ROOT / f"{experiment}-experiment.json"
    if not path.exists():
        raise RuntimeError(f"experiment {experiment!r} is not prepared")
    manifest = json.loads(path.read_text())
    results: dict[str, Any] = {}
    for arm in ("single_agent", "graph_control", "artifact_led"):
        run_id = manifest["arms"][arm]
        try:
            results[arm] = await LabRun(run_id).execute()
        except Exception as exc:
            results[arm] = {"run": run_id, "mode": arm, "error": str(exc)}
    outcome = {"experiment": experiment, "manifest": manifest, "results": results}
    result_path = WORK_ROOT / f"{experiment}-experiment-results.json"
    result_path.write_text(json.dumps(outcome, indent=2, sort_keys=True))
    return {**outcome, "result_path": str(result_path)}


def worker_runtime_probe(run_id: str) -> dict[str, Any]:
    """Run the two live worker preconditions without spending a lead wake."""
    lab = LabRun(run_id)
    if lab.mode == MODE_SINGLE:
        lab.coordinator.close()
        raise RuntimeError("worker runtime probe requires a team-mode run")
    try:
        catalogue = prove_free_worker_pool(lab.worker_pool)
        cell = lab.coordinator.workspaces.ensure_coordination_cell(
            lab.lead_actor, read_only=lab.mode == MODE_GRAPH
        )
        runtime = prove_worker_runtime(cell, lab.worker_pool)
        result = {
            "run": run_id,
            "worker_model_pool": lab.worker_pool,
            "openrouter_catalogue": catalogue,
            "company_runtime": runtime,
        }
        (lab.run_dir / "worker-runtime-proof.json").write_text(
            json.dumps(result, indent=2, sort_keys=True)
        )
        return result
    finally:
        lab.coordinator.close()


def worker_capability_probe(run_id: str) -> dict[str, Any]:
    """Prove tool use and exact artifact production in an isolated scratch path."""
    lab = LabRun(run_id)
    if lab.mode == MODE_SINGLE:
        lab.coordinator.close()
        raise RuntimeError("worker capability probe requires a team-mode run")
    try:
        catalogue = prove_free_worker_pool(lab.worker_pool)
        cell = lab.coordinator.workspaces.ensure_coordination_cell(
            lab.lead_actor, read_only=False
        )
        common = [
            "docker",
            "exec",
            "-u",
            "company",
            "-w",
            "/workspace",
            "-e",
            "PI_CODING_AGENT_DIR=/company",
            "-e",
            "COORD_GATEWAY_PORT",
            "-e",
            "RESTLESS_MODEL_GATEWAY_TOKEN",
            cell,
            "/usr/local/bin/omp",
        ]
        probe_thinking = os.environ.get("COORD_WORKER_PROBE_THINKING", "").strip()
        probe_max_seconds = int(os.environ.get("COORD_WORKER_PROBE_MAX_SECONDS", "180"))
        thinking_args = ["--thinking", probe_thinking] if probe_thinking else []
        results: list[dict[str, Any]] = []
        for model in lab.worker_pool:
            nonce = uid("admission")
            relative_dir = Path(".coordination-admission") / f"{safe_name(model)}-{nonce}"
            host_dir = lab.run_dir / "canonical" / relative_dir
            host_dir.mkdir(parents=True, exist_ok=False)
            artifact = host_dir / "artifact.txt"
            expected = (
                f"model={model}\n"
                "capability=tool-edit-proof\n"
                f"nonce={nonce}\n"
            )
            container_artifact = f"/workspace/{relative_dir}/artifact.txt"
            prompt = (
                f"Use the write tool to create {container_artifact} with EXACTLY the following UTF-8 content, "
                "including its final newline:\n\n"
                f"{expected}\n"
                "Then use the read tool to read the file back. If and only if it matches exactly, "
                "reply with exactly CAPABILITY_PROBE_COMPLETE. Do not merely describe the action."
            )
            started = time.monotonic()
            probe = subprocess.run(
                [
                    *common,
                    "-p",
                    "--model",
                    f"openrouter/{model}",
                    "--system-prompt",
                    "You are a bounded capability probe. Perform only the requested file operation and verification.",
                    "--config",
                    "/harness/omp-runtime.yml",
                    "--extension",
                    "/harness/v2/openrouter-live-free-models.ts",
                    "--no-extensions",
                    "--no-rules",
                    "--no-skills",
                    "--tools",
                    "read,write",
                    "--auto-approve",
                    *thinking_args,
                    "--max-time",
                    f"{probe_max_seconds}s",
                    prompt,
                ],
                text=True,
                capture_output=True,
                timeout=probe_max_seconds + 30,
            )
            elapsed = round(time.monotonic() - started, 3)
            observed = artifact.read_text() if artifact.exists() else None
            valid = (
                probe.returncode == 0
                and "CAPABILITY_PROBE_COMPLETE" in probe.stdout
                and observed == expected
            )
            results.append(
                {
                    "model": model,
                    "runtime_selector": f"openrouter/{model}",
                    "thinking": probe_thinking or "runtime-default",
                    "max_seconds": probe_max_seconds,
                    "elapsed_seconds": elapsed,
                    "exit_code": probe.returncode,
                    "completion_marker": "CAPABILITY_PROBE_COMPLETE" in probe.stdout,
                    "artifact_path": str(artifact),
                    "artifact_exists": artifact.exists(),
                    "artifact_exact": observed == expected,
                    "artifact_sha256": hashlib.sha256(observed.encode()).hexdigest() if observed is not None else None,
                    "valid": valid,
                    "stdout_tail": probe.stdout[-1_000:],
                    "stderr_tail": probe.stderr[-1_000:],
                }
            )
        proof_suffix = safe_name(probe_thinking or "runtime-default")
        proof_path = lab.run_dir / f"worker-capability-proof-{proof_suffix}-{int(time.time())}.json"
        result = {
            "run": run_id,
            "worker_model_pool": lab.worker_pool,
            "openrouter_catalogue": catalogue,
            "capabilities": results,
            "proof_path": str(proof_path),
        }
        proof_path.write_text(json.dumps(result, indent=2, sort_keys=True))
        return result
    finally:
        lab.coordinator.close()


def main() -> None:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    prepare_parser = sub.add_parser("prepare")
    prepare_parser.add_argument("run_id")
    prepare_parser.add_argument("--mode", choices=MODES, default=MODE_GRAPH)
    prepare_parser.add_argument("--lead-model", default=LEAD_MODEL)
    prepare_parser.add_argument("--worker-pool", default=",".join(DEFAULT_WORKER_POOL))
    prepare_parser.add_argument("--spend-ceiling", type=float, default=CEILING_USD)
    prepare_parser.add_argument("--wall-clock-seconds", type=int, default=1800)
    prepare_parser.add_argument("--drain-grace-seconds", type=int, default=DRAIN_GRACE_SECONDS)
    prepare_parser.add_argument("--scenario-file")
    run_parser = sub.add_parser("run")
    run_parser.add_argument("run_id")
    fault_parser = sub.add_parser("fault-test")
    fault_parser.add_argument("run_id", nargs="?", default="faults")
    positive_parser = sub.add_parser("positive-probe")
    positive_parser.add_argument("run_id", nargs="?", default="positive")
    positive_repair_parser = sub.add_parser("positive-repair-probe")
    positive_repair_parser.add_argument("run_id", nargs="?", default="positive")
    architecture_parser = sub.add_parser("architecture-test")
    architecture_parser.add_argument("run_id", nargs="?", default="v21-architecture")
    experiment_prepare_parser = sub.add_parser("experiment-prepare")
    experiment_prepare_parser.add_argument("experiment_id", nargs="?", default="v21-smoke")
    experiment_prepare_parser.add_argument("--lead-model", default=LEAD_MODEL)
    experiment_prepare_parser.add_argument("--worker-pool", default=",".join(DEFAULT_WORKER_POOL))
    experiment_prepare_parser.add_argument("--spend-ceiling", type=float, default=CEILING_USD)
    experiment_prepare_parser.add_argument("--wall-clock-seconds", type=int, default=1800)
    experiment_run_parser = sub.add_parser("experiment-run")
    experiment_run_parser.add_argument("experiment_id", nargs="?", default="v21-smoke")
    worker_probe_parser = sub.add_parser("worker-runtime-probe")
    worker_probe_parser.add_argument("run_id")
    worker_capability_parser = sub.add_parser("worker-capability-probe")
    worker_capability_parser.add_argument("run_id")
    cleanup_parser = sub.add_parser("cleanup")
    cleanup_parser.add_argument("run_id")
    args = parser.parse_args()
    WORK_ROOT.mkdir(parents=True, exist_ok=True)
    if args.command == "prepare":
        worker_pool = [model.strip() for model in args.worker_pool.split(",") if model.strip()]
        scenario_text = Path(args.scenario_file).read_text() if args.scenario_file else None
        print(
            prepare(
                args.run_id,
                mode=args.mode,
                lead_model=args.lead_model,
                worker_pool=worker_pool,
                spend_ceiling_usd=args.spend_ceiling,
                wall_clock_seconds=args.wall_clock_seconds,
                drain_grace_seconds=args.drain_grace_seconds,
                scenario_text=scenario_text,
            )
        )
    elif args.command == "run":
        print(json.dumps(asyncio.run(LabRun(args.run_id).execute()), indent=2))
    elif args.command == "fault-test":
        print(json.dumps(asyncio.run(fault_test(args.run_id)), indent=2))
    elif args.command == "positive-probe":
        print(json.dumps(asyncio.run(positive_callback_probe(args.run_id)), indent=2))
    elif args.command == "positive-repair-probe":
        print(json.dumps(asyncio.run(positive_callback_repair_probe(args.run_id)), indent=2))
    elif args.command == "architecture-test":
        print(json.dumps(asyncio.run(artifact_architecture_test(args.run_id)), indent=2))
    elif args.command == "experiment-prepare":
        worker_pool = [model.strip() for model in args.worker_pool.split(",") if model.strip()]
        print(
            json.dumps(
                prepare_experiment(
                    args.experiment_id,
                    lead_model=args.lead_model,
                    worker_pool=worker_pool,
                    spend_ceiling_usd=args.spend_ceiling,
                    wall_clock_seconds=args.wall_clock_seconds,
                ),
                indent=2,
            )
        )
    elif args.command == "experiment-run":
        print(json.dumps(asyncio.run(run_experiment(args.experiment_id)), indent=2))
    elif args.command == "worker-runtime-probe":
        print(json.dumps(worker_runtime_probe(args.run_id), indent=2))
    elif args.command == "worker-capability-probe":
        print(json.dumps(worker_capability_probe(args.run_id), indent=2))
    elif args.command == "cleanup":
        print(json.dumps({"removed": cleanup_cells(args.run_id)}, indent=2))


if __name__ == "__main__":
    main()
