#!/usr/bin/env python3
from __future__ import annotations

import argparse
import asyncio
import json
import os
import shutil
import socket
import sqlite3
import subprocess
import sys
import time
import tomllib
from pathlib import Path
from typing import Any

from store import connect, emit, initialize, json_rows, snapshot, transaction, uid, wake


HERE = Path(__file__).resolve().parent
WORK_ROOT = HERE / "workdir"
SEED_COMMIT = "514b7b3"
MODEL = "anthropic/claude-sonnet-4-5"
CONTAINER = "restless-coordination-lab-test"
TURN_BIN = HERE / "target" / "release" / "coordination-lab-turn"
SOURCE_CONFIG = Path.home() / ".restless" / "companies" / "cosmon.toml"
SOURCE_CONTAINER = "restless-co-cosmon"

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
}


def run_command(argv: list[str], *, cwd: Path | None = None, check: bool = True, capture: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(argv, cwd=cwd, check=check, text=True, capture_output=capture)


def safe_reset(path: Path) -> None:
    root = WORK_ROOT.resolve()
    resolved = path.resolve()
    if path.is_symlink() or resolved.parent != root or not path.name.startswith(("v0", "v1", "preflight")):
        raise RuntimeError(f"refusing unsafe scratch reset: {path}")
    if path.exists():
        shutil.rmtree(path)


def mission() -> str:
    with SOURCE_CONFIG.open("rb") as source:
        return tomllib.load(source)["mission"]


def container_path(path: Path) -> str:
    relative = path.resolve().relative_to(WORK_ROOT.resolve())
    return f"/lab/{relative.as_posix()}"


def docker(*args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return run_command(["docker", *args], check=check)


def prepare(run_id: str) -> Path:
    run_dir = WORK_ROOT / run_id
    safe_reset(run_dir)
    run_dir.mkdir(parents=True)
    seed_source = run_dir / "seed-source"
    repo = run_dir / "repo"
    docker("cp", f"{SOURCE_CONTAINER}:/company/repos/cosmon-game", str(seed_source))
    run_command(["git", "clone", "--no-local", str(seed_source), str(repo)])
    run_command(["git", "-C", str(repo), "checkout", "--detach", SEED_COMMIT])
    run_command(["git", "-C", str(repo), "clean", "-fd"])
    observed = run_command(["git", "-C", str(repo), "rev-parse", "HEAD"]).stdout.strip()
    if observed != SEED_COMMIT and not observed.startswith(SEED_COMMIT):
        raise RuntimeError(f"seed mismatch: expected {SEED_COMMIT}, got {observed}")
    shutil.rmtree(seed_source)

    db = run_dir / "state.db"
    initialize(str(db))
    conn = connect(str(db))
    for actor, (role, brief) in ACTORS.items():
        conn.execute("INSERT INTO actors(id,role,brief) VALUES(?,?,?)", (actor, role, brief))
    emit(conn, "run_prepared", {"run": run_id, "seed": observed, "model": MODEL})
    conn.close()

    (run_dir / "worktrees").mkdir()
    (run_dir / "prompts").mkdir()
    (run_dir / "system").mkdir()
    (run_dir / "agent-home").mkdir()
    runtime_bin = run_dir / "runtime-bin"
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
    (run_dir / "scenario.md").write_text(mission())
    roster = "\n".join(f"- `{actor}` — {role}: {brief}" for actor, (role, brief) in ACTORS.items() if actor != "exec")
    exec_system = f"""# Restless coordination experiment — Exec

You are the persistent Exec of Cosmon. The owner supplied a large commercial vertical-slice mandate. Your job is to keep the company moving toward the best runnable artifact, not to narrate management.

Substantial domain production must be commissioned as exact Work through the coordination tools. You may inspect the repository enough to frame Work, assess evidence, and choose integration strategy, but you must not edit game source yourself. Messages never assign or complete Work. Staff kickoff comes only from ready Work. Delegated waiting is event-driven: do not schedule polling.

Do not launch `omp`, `claude`, `codex`, another model, or a private subagent from the shell. Every model turn must remain visible to this harness's shared envelope. Commission registered Staff instead.

Available roster:
{roster}

Use outcome-sized Work with expected artifacts, repository workspaces, dependencies, and deterministic argv gates where possible. Do not estimate the entire mandate's duration. Respond to evidence: continue, repair, reassign, abandon, or commission a dependent integration/review outcome. `outcome_met` is a worker claim; checks and independent review determine whether it holds.

There is no mid-run owner help. Record an owner judgement only if genuinely irreducible; otherwise make a reversible decision. When the best integrated playable artifact is prepared for final review, call `decide` with subject `run`, choice `complete`, rationale, and exact evidence. Do not claim completion in prose without the decision.
"""
    (run_dir / "system" / "exec.md").write_text(exec_system)
    shared = """You are one durable Staff actor in a controlled coordination experiment. Work only inside the claimed Work and exact worktree in your prompt. Use ordinary Linux/Git tools for production. Use coordination tools for messages, callbacks, judgement, and Work changes. Commit meaningful artifacts before `report(outcome_met)`. A progress callback is nonterminal; after one, keep working or send a truthful terminal report. Never create private subagents or perform substantial work outside a claimed Attempt. Do not launch `omp`, `claude`, `codex`, another model, or a private subagent from the shell; every model turn must remain visible to the shared envelope.\n\n"""
    for actor, (role, brief) in ACTORS.items():
        if actor == "exec":
            continue
        (run_dir / "system" / f"{actor}.md").write_text(f"# {role}\n\n{shared}{brief}\n")
    for actor in ACTORS:
        home = run_dir / "agent-home" / actor
        home.mkdir(parents=True)
        (home / "models.yml").write_text(
            "# Scratch route; bearer is supplied only in process env.\n"
            "providers:\n"
            "  anthropic:\n"
            "    baseUrl: http://host.docker.internal:7790\n"
            "    apiKey: RESTLESS_MODEL_GATEWAY_TOKEN\n"
            "    transport: pi-native\n"
        )
    manifest = {
        "run": run_id,
        "seed": observed,
        "model": MODEL,
        "wall_clock_seconds": 3600,
        "spend_ceiling_usd": 15.0,
        "max_staff_concurrency": 3,
        "owner_input": "end_review_only",
        "actors": {actor: {"role": role, "brief": brief} for actor, (role, brief) in ACTORS.items()},
    }
    (run_dir / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True))
    return run_dir


class LabRun:
    def __init__(self, run_id: str):
        self.run_id = run_id
        self.run_dir = WORK_ROOT / run_id
        self.db_path = self.run_dir / "state.db"
        self.events_path = self.run_dir / "timeline.jsonl"
        self.started = time.monotonic()
        self.deadline = self.started + 3600
        self.ceiling = 15.0
        self.notify_queue: asyncio.Queue[dict[str, Any]] = asyncio.Queue()
        self.server: asyncio.Server | None = None
        self.port = 0
        self.tasks: dict[str, asyncio.Task] = {}
        self.warning_sent = False
        self.idle_exec_wakes = 0

    def conn(self) -> sqlite3.Connection:
        return connect(str(self.db_path))

    def cost(self) -> float:
        conn = self.conn()
        value = conn.execute("SELECT COALESCE(SUM(cost_usd),0) FROM turns").fetchone()[0]
        conn.close()
        return float(value)

    def remaining_seconds(self) -> float:
        return max(1.0, self.deadline - time.monotonic())

    async def notification(self, reader: asyncio.StreamReader, writer: asyncio.StreamWriter) -> None:
        try:
            line = await reader.readline()
            if line:
                await self.notify_queue.put(json.loads(line))
        finally:
            writer.close()
            await writer.wait_closed()

    async def start_server(self) -> None:
        self.server = await asyncio.start_server(self.notification, "0.0.0.0", 0)
        sock = next(iter(self.server.sockets or []), None)
        if not sock:
            raise RuntimeError("notification socket did not bind")
        self.port = int(sock.getsockname()[1])

    def write_prompt(self, actor: str, prompt: str) -> tuple[Path, str]:
        turn = f"{int(time.time() * 1000)}-{uid('turn')}"
        path = self.run_dir / "prompts" / f"{actor}-{turn}.md"
        path.write_text(prompt)
        return path, turn

    async def run_turn(self, actor: str, prompt: str, *, attempt: str = "", workdir: str | None = None) -> dict[str, Any]:
        prompt_path, turn_id = self.write_prompt(actor, prompt)
        container_workdir = workdir or f"/lab/{self.run_id}/repo"
        system = f"/lab/{self.run_id}/system/{actor}.md"
        env = os.environ.copy()
        env.update({
            "COORD_ACTOR": actor,
            "COORD_MODEL": MODEL,
            "COORD_PROMPT_PATH": str(prompt_path),
            "COORD_SYSTEM_PATH": system,
            "COORD_WORKDIR": container_workdir,
            "COORD_DB_CONTAINER": f"/lab/{self.run_id}/state.db",
            "COORD_ATTEMPT": attempt,
            "COORD_RUN_ID": self.run_id,
            "COORD_CONTAINER": CONTAINER,
            "COORD_NOTIFY_PORT": str(self.port),
            "COORD_EVENTS_PATH": str(self.events_path),
        })
        started_at = time.time()
        conn = self.conn()
        conn.execute("INSERT INTO turns(id,actor,attempt_id,started_at) VALUES(?,?,?,?)", (turn_id, actor, attempt or None, started_at))
        emit(conn, "turn_started", {"turn": turn_id, "attempt": attempt or None}, actor)
        conn.close()
        process = await asyncio.create_subprocess_exec(
            str(TURN_BIN),
            env=env,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        try:
            stdout, stderr = await asyncio.wait_for(process.communicate(), timeout=self.remaining_seconds())
        except asyncio.TimeoutError:
            process.terminate()
            try:
                await asyncio.wait_for(process.wait(), timeout=10)
            except asyncio.TimeoutError:
                process.kill()
            result = {"text": "", "tool_calls": [], "cost_usd": 0.0, "used_tokens": 0, "output_tokens": 0, "stop_reason": "global_timeout", "error": "global run deadline reached"}
        else:
            if process.returncode:
                result = {"text": "", "tool_calls": [], "cost_usd": 0.0, "used_tokens": 0, "output_tokens": 0, "stop_reason": "failed", "error": stderr.decode(errors="replace")[-4000:]}
            else:
                lines = [line for line in stdout.decode(errors="replace").splitlines() if line.strip()]
                result = json.loads(lines[-1]) if lines else {"text": "", "tool_calls": [], "stop_reason": "empty"}
        used = int(result.get("used_tokens") or 0)
        output = int(result.get("output_tokens") or 0)
        observed_cost = result.get("cost_usd")
        estimated = observed_cost is None
        cost = float(observed_cost) if observed_cost is not None else used * 3e-6 + output * 15e-6
        result["accounted_cost_usd"] = cost
        result["cost_estimated"] = estimated
        conn = self.conn()
        conn.execute(
            "UPDATE turns SET ended_at=?,cost_usd=?,used_tokens=?,output_tokens=?,tool_calls=?,end_kind=?,transcript=? WHERE id=?",
            (time.time(), cost, used, output, len(result.get("tool_calls", [])), result.get("stop_reason"), result.get("text", "")[-20000:], turn_id),
        )
        terminal_payload = {
            "turn": turn_id,
            "stop_reason": result.get("stop_reason"),
            "cost_usd": cost,
            "cost_estimated": estimated,
        }
        if result.get("error"):
            terminal_payload["error"] = str(result["error"])[-4000:]
            print(f"{actor} turn failed: {terminal_payload['error']}", file=sys.stderr, flush=True)
        emit(conn, "turn_terminal", terminal_payload, actor)
        conn.close()
        return result

    def state_prompt(self, actor: str, causes: list[dict[str, Any]]) -> str:
        conn = self.conn()
        state = snapshot(conn, actor)
        conn.close()
        return (
            "# Event-driven wake\n\n"
            f"Wake causes:\n```json\n{json.dumps(causes, indent=2)}\n```\n\n"
            f"Canonical coordination state:\n```json\n{json.dumps(state, indent=2)}\n```\n\n"
            "Inspect the evidence and take the smallest useful coordination action. Do not poll delegated Work. If production is still required, commission or repair exact Work. If an integrated playable candidate exists, commission independent review before deciding the run is complete."
        )

    def claim_ready(self, limit: int) -> list[dict[str, Any]]:
        if limit <= 0:
            return []
        conn = self.conn()
        claimed: list[dict[str, Any]] = []
        with transaction(conn):
            candidates = conn.execute(
                """
                SELECT w.* FROM work w
                WHERE w.status='active'
                  AND NOT EXISTS (SELECT 1 FROM attempts a WHERE a.work_id=w.id AND a.revision=w.revision)
                  AND NOT EXISTS (
                    SELECT 1 FROM work_edges e JOIN work dep ON dep.id=e.other_work_id
                    WHERE e.work_id=w.id AND e.kind='requires' AND dep.status!='completed'
                  )
                ORDER BY w.created_at
                """
            ).fetchall()
            for work in candidates:
                if len(claimed) >= limit or work["owner"] in self.tasks:
                    continue
                attempt = uid("attempt")
                conn.execute("INSERT INTO attempts(id,work_id,revision,state,started_at) VALUES(?,?,?,'running',?)", (attempt, work["id"], work["revision"], time.time()))
                emit(conn, "attempt_started", {"attempt": attempt, "work": work["id"], "revision": work["revision"]}, work["owner"])
                conn.execute("UPDATE outbox SET delivered_at=? WHERE target=? AND delivered_at IS NULL", (time.time(), work["owner"]))
                item = dict(work)
                item["attempt"] = attempt
                claimed.append(item)
        conn.close()
        return claimed

    def staff_prompt(self, item: dict[str, Any]) -> str:
        conn = self.conn()
        inputs = json_rows(
            conn.execute(
                """
                SELECT dep.id,dep.outcome,dep.branch,a.kind,a.reference
                FROM work_edges e JOIN work dep ON dep.id=e.other_work_id
                LEFT JOIN artifacts a ON a.work_id=dep.id
                WHERE e.work_id=? AND e.kind='requires'
                ORDER BY dep.created_at,a.created_at
                """,
                (item["id"],),
            ).fetchall()
        )
        messages = json_rows(conn.execute("SELECT sender,body,refs_json,created_at FROM messages WHERE recipient=? ORDER BY created_at", (item["owner"],)).fetchall())
        gates = json_rows(conn.execute("SELECT position,name,argv_json FROM gates WHERE work_id=? ORDER BY position", (item["id"],)).fetchall())
        conn.close()
        return f"""# Claimed Work Attempt

Work: {item['id']}
Revision: {item['revision']}
Attempt: {item['attempt']}
Outcome: {item['outcome']}
Expected artifact: {item['expected_artifact']}
Exact worktree: {item['worktree']}
Branch: {item['branch']}
Feedback: {item.get('feedback') or 'none'}

Required upstream artifacts:
```json
{json.dumps(inputs, indent=2)}
```

Declared gates:
```json
{json.dumps(gates, indent=2)}
```

Relevant messages:
```json
{json.dumps(messages, indent=2)}
```

The complete product brief is available read-only for reference at `/lab/{self.run_id}/scenario.md`; read only the sections required by this outcome. Work until this outcome is committed and genuinely met or genuinely blocked. Send a terminal `report` before ending.
"""

    async def staff_turn(self, item: dict[str, Any]) -> dict[str, Any]:
        result = await self.run_turn(item["owner"], self.staff_prompt(item), attempt=item["attempt"], workdir=item["worktree"])
        conn = self.conn()
        attempt = conn.execute("SELECT * FROM attempts WHERE id=?", (item["attempt"],)).fetchone()
        if attempt and attempt["state"] == "running":
            with transaction(conn):
                conn.execute("UPDATE attempts SET state='unknown',summary=?,ended_at=? WHERE id=?", ("ACP process ended without a terminal report", time.time(), item["attempt"]))
                conn.execute("UPDATE work SET status='blocked',feedback=?,updated_at=? WHERE id=?", ("Attempt ended without terminal report", time.time(), item["id"]))
                emit(conn, "attempt_terminal", {"attempt": item["attempt"], "work": item["id"], "state": "unknown"}, item["owner"])
                wake(conn, item["created_by"], "attempt_terminal", {"attempt": item["attempt"], "work": item["id"], "state": "unknown"})
        conn.close()
        return result

    def pending_causes(self, actor: str) -> list[dict[str, Any]]:
        conn = self.conn()
        rows = conn.execute("SELECT id,cause,payload_json,created_at FROM outbox WHERE target=? AND delivered_at IS NULL ORDER BY id", (actor,)).fetchall()
        if rows:
            conn.execute("UPDATE outbox SET delivered_at=? WHERE target=? AND delivered_at IS NULL", (time.time(), actor))
        conn.close()
        return [{"id": row["id"], "cause": row["cause"], "payload": json.loads(row["payload_json"]), "created_at": row["created_at"]} for row in rows]

    def fire_due_schedules(self) -> None:
        conn = self.conn()
        rows = conn.execute("SELECT * FROM schedules WHERE fired_at IS NULL AND not_before<=?", (time.time(),)).fetchall()
        with transaction(conn):
            for row in rows:
                conn.execute("UPDATE schedules SET fired_at=? WHERE id=?", (time.time(), row["id"]))
                wake(conn, row["actor"], "schedule_due", {"schedule": row["id"], "reason": row["reason"]})
        conn.close()

    def next_schedule_delay(self) -> float | None:
        conn = self.conn()
        row = conn.execute("SELECT MIN(not_before) FROM schedules WHERE fired_at IS NULL").fetchone()
        conn.close()
        if row[0] is None:
            return None
        return max(0.0, float(row[0]) - time.time())

    def run_complete(self) -> bool:
        conn = self.conn()
        row = conn.execute("SELECT 1 FROM decisions WHERE subject='run' AND choice='complete' ORDER BY created_at DESC LIMIT 1").fetchone()
        conn.close()
        return bool(row)

    def has_open_work(self) -> bool:
        conn = self.conn()
        row = conn.execute("SELECT 1 FROM work WHERE status IN ('active','blocked') LIMIT 1").fetchone()
        conn.close()
        return bool(row)

    async def execute(self) -> None:
        await self.start_server()
        initial = (
            "# Owner directive\n\n"
            + (self.run_dir / "scenario.md").read_text()
            + "\n\n# Starting artifact\n\nThe exact repository seed is commit `514b7b3`. It already contains a playable Sunleaf Basin, 12 species, six evolved forms, a team of four, hybrid battle, elemental statuses, Resonance Bonding, and three existing browser verification suites. Preserve it. Build the best integrated next milestone toward the full directive. You have one hour and a shared $15 model-equivalent ceiling. There will be no mid-run owner answer.\n"
        )
        # Actor turns are execution leases, not scheduler locks. In particular,
        # a commission wake must be able to start its owner while Exec is still
        # finishing the coordinating turn that created it.
        self.tasks["exec"] = asyncio.create_task(self.run_turn("exec", initial))
        while self.remaining_seconds() > 1 and self.cost() < self.ceiling:
            self.fire_due_schedules()
            staff_count = sum(1 for actor in self.tasks if actor != "exec")
            for item in self.claim_ready(3 - staff_count):
                self.tasks[item["owner"]] = asyncio.create_task(self.staff_turn(item))

            exec_causes = self.pending_causes("exec") if "exec" not in self.tasks else []
            if exec_causes:
                self.tasks["exec"] = asyncio.create_task(self.run_turn("exec", self.state_prompt("exec", exec_causes)))

            if (self.cost() >= self.ceiling * 0.9 or self.remaining_seconds() <= 360) and not self.warning_sent:
                conn = self.conn()
                wake(conn, "exec", "envelope_warning", {"cost_usd": self.cost(), "remaining_seconds": self.remaining_seconds()})
                conn.close()
                self.warning_sent = True
                continue

            if self.run_complete() and not self.tasks:
                break

            if not self.tasks:
                conn = self.conn()
                pending = conn.execute("SELECT COUNT(*) FROM outbox WHERE delivered_at IS NULL").fetchone()[0]
                ready = conn.execute("SELECT COUNT(*) FROM work WHERE status='active'").fetchone()[0]
                conn.close()
                if not pending and not ready:
                    if self.idle_exec_wakes >= 1:
                        break
                    self.idle_exec_wakes += 1
                    conn = self.conn()
                    wake(conn, "exec", "organisation_idle", {"open_work": self.has_open_work()})
                    conn.close()
                    continue

            waiters: list[asyncio.Task] = list(self.tasks.values())
            notify_task = asyncio.create_task(self.notify_queue.get())
            waiters.append(notify_task)
            delay = self.next_schedule_delay()
            timer_task = asyncio.create_task(asyncio.sleep(min(delay if delay is not None else self.remaining_seconds(), self.remaining_seconds())))
            waiters.append(timer_task)
            done, _ = await asyncio.wait(waiters, return_when=asyncio.FIRST_COMPLETED)
            if notify_task not in done:
                notify_task.cancel()
            if timer_task not in done:
                timer_task.cancel()
            for actor, task in list(self.tasks.items()):
                if task.done():
                    try:
                        await task
                    except Exception as exc:
                        conn = self.conn()
                        emit(conn, "actor_task_failed", {"error": str(exc)}, actor)
                        if actor != "exec":
                            wake(conn, "exec", "actor_task_failed", {"actor": actor, "error": str(exc)})
                        conn.close()
                    self.tasks.pop(actor, None)

        for task in self.tasks.values():
            task.cancel()
        if self.tasks:
            await asyncio.gather(*self.tasks.values(), return_exceptions=True)
        if self.server:
            self.server.close()
            await self.server.wait_closed()
        conn = self.conn()
        emit(conn, "run_terminal", {"cost_usd": self.cost(), "elapsed_seconds": time.monotonic() - self.started, "decision_complete": self.run_complete()})
        conn.close()


def preflight(run_id: str) -> None:
    run_dir = prepare(run_id)
    db = run_dir / "state.db"
    conn = connect(str(db))
    # Atomic commission analogue: an invalid owner cannot leave Work behind.
    before = conn.execute("SELECT COUNT(*) FROM work").fetchone()[0]
    try:
        with transaction(conn):
            conn.execute("INSERT INTO work(id,created_by,owner,outcome,expected_artifact,status,base_ref,branch,worktree,created_at,updated_at) VALUES('bad','exec','missing','x','x','active','x','x','x',?,?)", (time.time(), time.time()))
    except sqlite3.IntegrityError:
        pass
    assert conn.execute("SELECT COUNT(*) FROM work").fetchone()[0] == before
    # Duplicate callbacks/commands are representable as one command id.
    conn.execute("INSERT INTO commands VALUES('stable','exec','send','{}','{}',?)", (time.time(),))
    try:
        conn.execute("INSERT INTO commands VALUES('stable','exec','send','{}','{}',?)", (time.time(),))
    except sqlite3.IntegrityError:
        pass
    assert conn.execute("SELECT COUNT(*) FROM commands WHERE id='stable'").fetchone()[0] == 1
    # One running Attempt per Work is enforced independently of the model.
    now = time.time()
    conn.execute("INSERT INTO work(id,created_by,owner,outcome,expected_artifact,status,base_ref,branch,worktree,created_at,updated_at) VALUES('probe','exec','gameplay-systems','probe','commit','active','514b7b3','probe','/tmp/probe',?,?)", (now, now))
    conn.execute("INSERT INTO attempts(id,work_id,revision,state,started_at) VALUES('a1','probe',1,'running',?)", (now,))
    try:
        conn.execute("INSERT INTO attempts(id,work_id,revision,state,started_at) VALUES('a2','probe',1,'running',?)", (now,))
    except sqlite3.IntegrityError:
        pass
    assert conn.execute("SELECT COUNT(*) FROM attempts WHERE work_id='probe' AND state='running'").fetchone()[0] == 1
    wake(conn, "exec", "preflight", {"probe": True})
    assert conn.execute("SELECT COUNT(*) FROM outbox WHERE target='exec' AND delivered_at IS NULL").fetchone()[0] == 1
    conn.close()
    print(json.dumps({"preflight": "pass", "seed": SEED_COMMIT, "db": str(db)}, indent=2))


def summarize(run_id: str) -> dict[str, Any]:
    run_dir = WORK_ROOT / run_id
    conn = connect(str(run_dir / "state.db"))
    summary = {
        "run": run_id,
        "turns": json_rows(conn.execute("SELECT actor,attempt_id,started_at,ended_at,cost_usd,used_tokens,output_tokens,tool_calls,end_kind FROM turns ORDER BY started_at").fetchall()),
        "work": json_rows(conn.execute("SELECT id,created_by,owner,outcome,status,revision,branch,worktree,feedback FROM work ORDER BY created_at").fetchall()),
        "attempts": json_rows(conn.execute("SELECT id,work_id,revision,state,summary,started_at,ended_at FROM attempts ORDER BY started_at").fetchall()),
        "artifacts": json_rows(conn.execute("SELECT work_id,attempt_id,kind,reference,observed FROM artifacts ORDER BY created_at").fetchall()),
        "decisions": json_rows(conn.execute("SELECT decided_by,subject,choice,rationale,evidence_json,created_at FROM decisions ORDER BY created_at").fetchall()),
        "open_judgements": json_rows(conn.execute("SELECT * FROM judgements WHERE state='open'").fetchall()),
        "events_by_kind": dict(conn.execute("SELECT kind,COUNT(*) FROM events GROUP BY kind ORDER BY kind").fetchall()),
        "total_cost_usd": conn.execute("SELECT COALESCE(SUM(cost_usd),0) FROM turns").fetchone()[0],
    }
    conn.close()
    (run_dir / "summary.json").write_text(json.dumps(summary, indent=2, sort_keys=True))
    return summary


def main() -> None:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    p_prepare = sub.add_parser("prepare")
    p_prepare.add_argument("run_id", choices=["v0", "v1"])
    p_run = sub.add_parser("run")
    p_run.add_argument("run_id", choices=["v0", "v1"])
    p_preflight = sub.add_parser("preflight")
    p_preflight.add_argument("run_id", nargs="?", default="preflight")
    p_summary = sub.add_parser("summary")
    p_summary.add_argument("run_id", choices=["v0", "v1"])
    args = parser.parse_args()
    WORK_ROOT.mkdir(exist_ok=True)
    if args.command == "prepare":
        print(prepare(args.run_id))
    elif args.command == "preflight":
        preflight(args.run_id)
    elif args.command == "run":
        asyncio.run(LabRun(args.run_id).execute())
        print(json.dumps(summarize(args.run_id), indent=2))
    elif args.command == "summary":
        print(json.dumps(summarize(args.run_id), indent=2))


if __name__ == "__main__":
    main()
