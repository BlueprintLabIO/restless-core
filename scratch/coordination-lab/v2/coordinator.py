from __future__ import annotations

import asyncio
import hashlib
import json
import os
import re
import socket
import sqlite3
import subprocess
import time
from pathlib import Path
from typing import Any

from store import connect, emit, initialize, json_rows, transaction, uid


IMAGE = os.environ.get("COORD_IMAGE", "restless-company-image:latest")
GATEWAY_PORT = int(os.environ.get("COORD_GATEWAY_PORT", "7796"))
LAB_ROOT = Path(__file__).resolve().parents[1]


def run(argv: list[str], *, cwd: Path | None = None, check: bool = True) -> subprocess.CompletedProcess[str]:
    process = subprocess.run(argv, cwd=cwd, text=True, capture_output=True)
    if check and process.returncode:
        raise RuntimeError(f"command failed ({process.returncode}): {argv!r}\n{process.stderr[-2000:]}")
    return process


def safe_name(value: str) -> str:
    cleaned = re.sub(r"[^a-zA-Z0-9_.-]+", "-", value).strip("-.")
    if not cleaned:
        raise ValueError("empty safe name")
    return cleaned[:50]


class WorkspaceManager:
    def __init__(self, run_dir: Path, run_id: str):
        self.run_dir = run_dir
        self.run_id = safe_name(run_id)
        self.canonical = run_dir / "canonical"
        self.context = run_dir / "context"
        self.homes = run_dir / "homes"
        self.workspaces = run_dir / "workspaces"
        self.homes.mkdir(exist_ok=True)
        self.workspaces.mkdir(exist_ok=True)

    def _cell_name(self, identity: str) -> str:
        return f"restless-coord-v2-{self.run_id}-{safe_name(identity)}"[:63]

    def _home(self, identity: str) -> Path:
        home = self.homes / safe_name(identity)
        home.mkdir(parents=True, exist_ok=True)
        (home / "models.yml").write_text(
            "# Scratch gateway route only.\n"
            "providers:\n"
            "  anthropic:\n"
            f"    baseUrl: http://host.docker.internal:{GATEWAY_PORT}\n"
            "    apiKey: RESTLESS_MODEL_GATEWAY_TOKEN\n"
            "    transport: pi-native\n"
            "  openrouter:\n"
            f"    baseUrl: http://host.docker.internal:{GATEWAY_PORT}\n"
            "    apiKey: RESTLESS_MODEL_GATEWAY_TOKEN\n"
            "    api: openai-completions\n"
            "    transport: pi-native\n"
            "    models:\n"
            "      - id: stealth/ox-alpha\n"
            "        name: Ox Alpha\n"
            "        reasoning: true\n"
            "        supportsTools: true\n"
            "        input: [text]\n"
            "        cost: {input: 0, output: 0, cacheRead: 0, cacheWrite: 0}\n"
            "        contextWindow: 1048576\n"
            "        maxTokens: 131072\n"
            "      - id: z-ai/glm-5.2:free\n"
            "        name: Z.ai GLM 5.2 (free)\n"
            "        reasoning: true\n"
            "        supportsTools: true\n"
            "        tokenizer: glm5\n"
            "        input: [text]\n"
            "        cost: {input: 0, output: 0, cacheRead: 0, cacheWrite: 0}\n"
            "        contextWindow: 256000\n"
            "        maxTokens: 131072\n"
        )
        return home

    def _ensure_cell(
        self,
        identity: str,
        workspace: Path,
        *,
        read_only: bool,
        home_identity: str | None = None,
    ) -> str:
        cell = self._cell_name(identity)
        inspected = run(["docker", "inspect", cell], check=False)
        if inspected.returncode == 0:
            state = run(["docker", "inspect", "-f", "{{.State.Running}}", cell]).stdout.strip()
            if state != "true":
                run(["docker", "start", cell])
            return cell
        mode = ":ro" if read_only else ""
        home = self._home(home_identity or identity)
        run(
            [
                "docker",
                "run",
                "-d",
                "--name",
                cell,
                "--label",
                "restless.coordination.lab=v2",
                "--label",
                f"restless.coordination.run={self.run_id}",
                "--add-host=host.docker.internal:host-gateway",
                "-v",
                f"{workspace}:/workspace{mode}",
                "-v",
                f"{self.context}:/context:ro",
                "-v",
                f"{home}:/company",
                "-v",
                f"{LAB_ROOT}:/harness:ro",
                "--entrypoint",
                "/bin/sleep",
                IMAGE,
                "infinity",
            ]
        )
        return cell

    def ensure_exec_cell(self) -> str:
        return self._ensure_cell("exec", self.canonical, read_only=True)

    def ensure_coordination_cell(self, actor: str, *, read_only: bool) -> str:
        """Bind the run's accountable coordinator to the canonical candidate.

        Graph-control runs retain the historical read-only Exec posture.  The
        artifact-led experiment deliberately gives its outcome lead the one
        writable canonical checkout so integration and native verification are
        leadership work rather than another graph node.
        """
        return self._ensure_cell(actor, self.canonical, read_only=read_only)

    def ensure_work(self, work: sqlite3.Row, desired_base: str | None = None) -> tuple[str, str, str]:
        workspace = self.workspaces / work["id"]
        lease_base = desired_base
        if work["owner"] == "integration-lead":
            lease_base = run(
                ["git", "-C", str(self.canonical), "rev-parse", "candidate"]
            ).stdout.strip()
        if workspace.exists() and lease_base:
            descends = run(
                ["git", "-C", str(workspace), "merge-base", "--is-ancestor", lease_base, "HEAD"],
                check=False,
            )
            if descends.returncode:
                # The Work's required input advanced along a sibling history.
                # Preserve the old checkout as evidence and start a new input
                # epoch for this revision rather than resetting it in place.
                workspace = self.workspaces / f"{work['id']}-r{work['revision']}"
        if not workspace.exists():
            run(["git", "clone", "--no-local", str(self.canonical), str(workspace)])
            base_ref = lease_base or work["base_ref"]
            resolved = run(["git", "-C", str(self.canonical), "rev-parse", base_ref]).stdout.strip()
            run(["git", "-C", str(workspace), "checkout", "-b", work["branch"], resolved])
            run(["git", "-C", str(workspace), "config", "user.name", f"Restless {work['owner']}"])
            run(["git", "-C", str(workspace), "config", "user.email", f"{work['owner']}@restless.invalid"])
        else:
            resolved = lease_base or run(
                ["git", "-C", str(workspace), "rev-parse", f"{work['base_ref']}^{{commit}}"],
                check=False,
            ).stdout.strip()
            if not resolved:
                resolved = run(["git", "-C", str(workspace), "merge-base", "HEAD", "origin/HEAD"]).stdout.strip()
        cell = self._ensure_cell(
            workspace.name,
            workspace,
            read_only=False,
            home_identity=work["id"],
        )
        return str(workspace), cell, resolved

    def cell_exec(self, cell: str, argv: list[str], *, timeout: int = 300, check: bool = False) -> subprocess.CompletedProcess[str]:
        process = subprocess.run(
            ["docker", "exec", "-u", "company", "-w", "/workspace", cell, *argv],
            text=True,
            capture_output=True,
            timeout=timeout,
        )
        if check and process.returncode:
            raise RuntimeError(f"cell command failed ({process.returncode}): {argv!r}\n{process.stderr[-2000:]}")
        return process

    def ensure_http_server(self, cell: str, port: int) -> None:
        """Ensure native browser verifiers have the static candidate server they expect."""
        probe_argv = [
            "python3",
            "-c",
            (
                "import urllib.request; "
                f"urllib.request.urlopen('http://127.0.0.1:{port}/index.html', timeout=2).read(1)"
            ),
        ]
        if self.cell_exec(cell, probe_argv, timeout=5).returncode == 0:
            return
        started = subprocess.run(
            [
                "docker",
                "exec",
                "-d",
                "-u",
                "company",
                "-w",
                "/workspace",
                cell,
                "python3",
                "-m",
                "http.server",
                str(port),
                "--bind",
                "127.0.0.1",
                "--directory",
                "/workspace",
            ],
            text=True,
            capture_output=True,
        )
        if started.returncode:
            raise RuntimeError(f"could not start candidate server on {port}: {started.stderr[-1000:]}")
        for _ in range(30):
            if self.cell_exec(cell, probe_argv, timeout=5).returncode == 0:
                return
            time.sleep(0.1)
        raise RuntimeError(f"candidate server on {port} did not become ready")

    def observe(self, work: sqlite3.Row) -> dict[str, Any]:
        if not work["cell"]:
            return {"workspace": work["workspace"], "cell": None, "status": "workspace_not_claimed"}
        status = self.cell_exec(work["cell"], ["git", "status", "--porcelain"])
        head = self.cell_exec(work["cell"], ["git", "rev-parse", "HEAD"])
        return {
            "workspace": "/workspace",
            "cell": work["cell"],
            "git_status": status.stdout,
            "head": head.stdout.strip() if head.returncode == 0 else None,
        }

    def stop_actor_process(self, cell: str | None) -> None:
        if not cell:
            return
        subprocess.run(
            [
                "docker",
                "exec",
                "-u",
                "root",
                cell,
                "pkill",
                "-TERM",
                "-f",
                "/usr/local/bin/omp acp",
            ],
            text=True,
            capture_output=True,
        )

    def import_artifact(self, work: sqlite3.Row, commit: str) -> str:
        workspace = Path(work["workspace"])
        ref = f"refs/heads/artifacts/{safe_name(work['id'])}"
        run(["git", "-C", str(self.canonical), "fetch", str(workspace), f"{commit}:{ref}"])
        if work["owner"] == "integration-lead":
            ancestor = run(["git", "-C", str(self.canonical), "merge-base", "--is-ancestor", work["base_ref"], commit], check=False)
            if ancestor.returncode:
                raise RuntimeError("integration result does not descend from its leased candidate base")
            updated = run(
                ["git", "-C", str(self.canonical), "update-ref", "refs/heads/candidate", commit, work["base_ref"]],
                check=False,
            )
            if updated.returncode:
                raise RuntimeError("candidate changed while integration lease was active")
        return ref


class Coordinator:
    def __init__(self, run_dir: Path, run_id: str, notify_queue: asyncio.Queue[dict[str, Any]] | None = None):
        self.run_dir = run_dir
        self.run_id = run_id
        self.db_path = run_dir / "state.db"
        initialize(self.db_path)
        self.conn = connect(self.db_path)
        manifest_path = run_dir / "manifest.json"
        manifest = json.loads(manifest_path.read_text()) if manifest_path.exists() else {}
        self.coordination_actor = manifest.get("coordination_actor", "exec")
        self.workspaces = WorkspaceManager(run_dir, run_id)
        self.notify_queue = notify_queue
        self.trace_path = run_dir / "timeline.jsonl"
        self.server: asyncio.Server | None = None
        self.port = 0

    def close(self) -> None:
        self.conn.close()

    def emit(self, kind: str, payload: dict[str, Any], actor: str | None = None) -> int:
        return emit(self.conn, kind, payload, actor)

    def wake(self, target: str, cause: str, payload: dict[str, Any]) -> int:
        cursor = self.conn.execute(
            "INSERT INTO outbox(target,cause,payload_json,created_at) VALUES(?,?,?,?)",
            (target, cause, json.dumps(payload, sort_keys=True), time.time()),
        )
        event = {"target": target, "cause": cause, **payload}
        self.emit("wake_requested", event)
        if self.notify_queue is not None:
            self.notify_queue.put_nowait(event)
        return int(cursor.lastrowid)

    async def start_server(self) -> None:
        self.server = await asyncio.start_server(self._handle_client, "0.0.0.0", 0)
        sock = next(iter(self.server.sockets or []), None)
        if not sock:
            raise RuntimeError("coordinator socket did not bind")
        self.port = int(sock.getsockname()[1])

    async def stop_server(self) -> None:
        if self.server:
            self.server.close()
            await self.server.wait_closed()

    async def _handle_client(self, reader: asyncio.StreamReader, writer: asyncio.StreamWriter) -> None:
        try:
            line = await reader.readline()
            if not line:
                return
            request = json.loads(line)
            kind = request.get("type")
            if kind == "trace":
                result = self.record_trace(request)
            elif kind == "inspect":
                result = self.snapshot(request["actor"])
            elif kind == "command":
                result = self.command(request)
            else:
                raise ValueError(f"unknown coordinator request type {kind!r}")
            writer.write((json.dumps({"ok": True, "result": result}, sort_keys=True) + "\n").encode())
        except Exception as exc:
            writer.write((json.dumps({"ok": False, "error": str(exc)}) + "\n").encode())
        finally:
            await writer.drain()
            writer.close()
            await writer.wait_closed()

    def record_trace(self, request: dict[str, Any]) -> dict[str, Any]:
        line = {
            "at": request.get("at") or str(int(time.time() * 1000)),
            "actor": request.get("actor"),
            "kind": request.get("kind"),
            "payload": request.get("payload") or {},
            "turn_id": request.get("turn_id"),
        }
        with self.trace_path.open("a") as handle:
            handle.write(json.dumps(line, sort_keys=True) + "\n")
        if line["kind"] == "model_usage" and line["turn_id"]:
            payload = line["payload"]
            self.conn.execute(
                "UPDATE turns SET cost_usd=COALESCE(?,cost_usd), used_tokens=COALESCE(?,used_tokens), cached_input_tokens=COALESCE(?,cached_input_tokens), reasoning_output_tokens=COALESCE(?,reasoning_output_tokens) WHERE id=? AND ended_at IS NULL",
                (
                    payload.get("cost_usd"),
                    payload.get("used_tokens"),
                    payload.get("cached_input_tokens"),
                    payload.get("reasoning_output_tokens"),
                    line["turn_id"],
                ),
            )
        return {"recorded": True}

    def _actor(self, actor: str) -> sqlite3.Row:
        row = self.conn.execute("SELECT * FROM actors WHERE id=? AND active=1", (actor,)).fetchone()
        if not row:
            raise ValueError(f"unknown active actor {actor}")
        return row

    def _attempt_context(self, request: dict[str, Any]) -> tuple[sqlite3.Row, sqlite3.Row] | None:
        actor = request["actor"]
        attempt_id = request.get("attempt") or ""
        if not attempt_id:
            if actor != self.coordination_actor:
                raise ValueError(
                    f"only coordination actor {self.coordination_actor!r} may mutate without a claimed Attempt lease"
                )
            return None
        attempt = self.conn.execute("SELECT * FROM attempts WHERE id=?", (attempt_id,)).fetchone()
        if not attempt or attempt["state"] != "running":
            raise ValueError("Attempt lease is not running")
        work = self.conn.execute("SELECT * FROM work WHERE id=?", (attempt["work_id"],)).fetchone()
        if attempt["actor"] != actor or work["owner"] != actor:
            raise ValueError("Attempt lease actor does not own Work")
        if attempt["revision"] != work["revision"]:
            raise ValueError("stale Attempt revision")
        if request.get("lease_token") != attempt["lease_token"]:
            raise ValueError("invalid Attempt lease token")
        if attempt["lease_expires_at"] < time.time():
            raise ValueError("Attempt lease expired")
        return attempt, work

    def command(self, request: dict[str, Any]) -> dict[str, Any]:
        actor = request["actor"]
        self._actor(actor)
        name = request["name"]
        args = request.get("args") or {}
        key = request.get("idempotency_key")
        if not key or len(key) > 120:
            raise ValueError("mutation requires idempotency_key (1-120 characters)")
        attempt_id = request.get("attempt") or ""
        scope_key = f"{actor}:{attempt_id or '-'}:{key}"
        request_hash = hashlib.sha256(
            json.dumps({"name": name, "args": args}, sort_keys=True, separators=(",", ":")).encode()
        ).hexdigest()
        existing = self.conn.execute("SELECT * FROM commands WHERE scope_key=?", (scope_key,)).fetchone()
        if existing:
            if existing["request_hash"] != request_hash:
                raise ValueError("idempotency key was already used with different input")
            return json.loads(existing["result_json"])

        context = self._attempt_context(request)
        with transaction(self.conn):
            result = self._apply(name, args, actor, context)
            self.conn.execute(
                "INSERT INTO commands(scope_key,actor,attempt_id,idempotency_key,name,request_hash,args_json,result_json,created_at) VALUES(?,?,?,?,?,?,?,?,?)",
                (
                    scope_key,
                    actor,
                    attempt_id or None,
                    key,
                    name,
                    request_hash,
                    json.dumps(args, sort_keys=True),
                    json.dumps(result, sort_keys=True),
                    time.time(),
                ),
            )
            self.emit("command_applied", {"command": name, "scope_key": scope_key}, actor)
        return result

    def _apply(
        self,
        name: str,
        args: dict[str, Any],
        actor: str,
        context: tuple[sqlite3.Row, sqlite3.Row] | None,
    ) -> dict[str, Any]:
        if name == "send":
            target = args["to"]
            self._actor(target)
            message = uid("msg")
            self.conn.execute(
                "INSERT INTO messages(id,sender,recipient,body,refs_json,created_at) VALUES(?,?,?,?,?,?)",
                (message, actor, target, args["body"], json.dumps(args.get("refs", [])), time.time()),
            )
            self.wake(target, "message_appended", {"message": message, "from": actor, "delivery": "next_wake"})
            return {"message": message, "delivery": "queued_for_next_wake"}

        if name == "commission":
            owner = args["owner"]
            self._actor(owner)
            if owner == actor:
                raise ValueError(
                    "commission crosses an actor boundary; do the coordinator's own work directly "
                    "instead of creating self-owned Work"
                )
            for gate in args.get("gates", []):
                argv = gate.get("argv")
                if not isinstance(argv, list) or not argv or any(not isinstance(part, str) or not part for part in argv):
                    raise ValueError("gate argv must contain an executable followed by separate non-empty arguments")
                if any(character.isspace() for character in argv[0]):
                    raise ValueError(
                        "gate argv[0] must be one executable, not a shell command string; use separate arguments or explicit ['sh','-c',command]"
                    )
            requires = list(args.get("requires", []))
            revises = list(args.get("revises", []))
            for other in requires + revises:
                if not self.conn.execute("SELECT 1 FROM work WHERE id=?", (other,)).fetchone():
                    raise ValueError(f"unknown Work {other}")
            if owner == "integration-lead":
                if not requires:
                    raise ValueError("integration Work requires at least one produced Work")
                incomplete = self.conn.execute(
                    f"SELECT id FROM work WHERE id IN ({','.join('?' for _ in requires)}) AND status!='completed'",
                    requires,
                ).fetchall()
                if incomplete:
                    raise ValueError("integration Work requires completed producer Work")
                existing = self.conn.execute(
                    "SELECT id FROM work WHERE owner='integration-lead' AND status IN ('active','blocked') LIMIT 1"
                ).fetchone()
                if existing:
                    raise ValueError(f"integration lease already exists on {existing['id']}")
                base_ref = "candidate"
            elif owner == "artifact-critic":
                if not requires:
                    raise ValueError("artifact critic Work requires at least one completed Work to review")
                incomplete = self.conn.execute(
                    f"SELECT id FROM work WHERE id IN ({','.join('?' for _ in requires)}) AND status!='completed'",
                    requires,
                ).fetchall()
                if incomplete:
                    raise ValueError("artifact critic Work requires completed Work")
                base_ref = "candidate"
            else:
                base_ref = args.get("base_ref") or "candidate"
            work_id = uid("work")
            branch = f"lab/v2/{safe_name(self.run_id)}/{work_id}"
            now = time.time()
            self.conn.execute(
                "INSERT INTO work(id,created_by,owner,outcome,expected_artifact,status,revision,base_ref,branch,created_at,updated_at) VALUES(?,?,?,?,?,'active',1,?,?,?,?)",
                (work_id, actor, owner, args["outcome"], args["expected_artifact"], base_ref, branch, now, now),
            )
            for other in requires:
                self.conn.execute("INSERT INTO work_edges VALUES(?,?,'requires')", (work_id, other))
            for other in revises:
                self.conn.execute("INSERT INTO work_edges VALUES(?,?,'revises')", (work_id, other))
            for position, gate in enumerate(args.get("gates", [])):
                self.conn.execute(
                    "INSERT INTO gates VALUES(?,?,?,?)",
                    (work_id, position, gate["name"], json.dumps(gate["argv"])),
                )
            self.emit(
                "work_commissioned",
                {"work": work_id, "owner": owner, "requires": requires, "revises": revises},
                actor,
            )
            if not requires or not self.conn.execute(
                f"SELECT 1 FROM work WHERE id IN ({','.join('?' for _ in requires)}) AND status!='completed' LIMIT 1",
                requires,
            ).fetchone():
                self.wake(owner, "work_became_ready", {"work": work_id})
            return {"work": work_id, "status": "active", "delivery": "owner_wakes_when_dependencies_are_ready"}

        if name == "redirect":
            work = self.conn.execute("SELECT * FROM work WHERE id=?", (args["work"],)).fetchone()
            if not work:
                raise ValueError("unknown Work")
            action = args["action"]
            new_owner = args.get("new_owner")
            if action == "reassign":
                if not new_owner:
                    raise ValueError("reassign requires new_owner")
                self._actor(new_owner)
            running = self.conn.execute(
                "SELECT * FROM attempts WHERE work_id=? AND state='running'", (work["id"],)
            ).fetchone()
            reason = args.get("feedback") or args["reason"]
            if running:
                self.conn.execute(
                    "UPDATE attempts SET cancel_requested_at=? WHERE id=?",
                    (time.time(), running["id"]),
                )
                self.conn.execute(
                    "UPDATE work SET pending_action=?,pending_owner=?,pending_reason=?,feedback=?,updated_at=? WHERE id=?",
                    (action, new_owner, reason, reason, time.time(), work["id"]),
                )
                self.emit(
                    "work_transition_pending",
                    {"work": work["id"], "attempt": running["id"], "action": action},
                    actor,
                )
                self.wake(work["owner"], "cancellation_requested", {"work": work["id"], "attempt": running["id"]})
                return {"work": work["id"], "action": action, "pending": True, "attempt": running["id"]}
            self._apply_redirect_now(work, action, reason, new_owner)
            return {"work": work["id"], "action": action, "pending": False}

        if name == "report":
            if context is None:
                raise ValueError("report requires Attempt")
            attempt, work = context
            if attempt["cancel_requested_at"] and args["disposition"] != "abandoned":
                raise ValueError("Attempt cancellation is pending; only an abandoned terminal report is accepted")
            disposition = args["disposition"]
            if disposition == "progress":
                self.emit(
                    "attempt_progress",
                    {"attempt": attempt["id"], "work": work["id"], "summary": args["summary"]},
                    actor,
                )
                self.wake(work["created_by"], "attempt_progress", {"attempt": attempt["id"], "work": work["id"]})
                return {"attempt": attempt["id"], "state": "running", "delivery": "creator_next_wake"}
            gate_results: list[dict[str, Any]] = []
            commit: str | None = None
            if disposition == "outcome_met":
                observed_before = self.workspaces.observe(work)
                if observed_before.get("git_status", "").strip():
                    raise ValueError("outcome_met requires a clean committed workspace")
                commit = observed_before.get("head")
                if not commit:
                    raise ValueError("workspace has no observable commit")
                if commit == work["base_ref"]:
                    raise ValueError("outcome_met requires a commit that advances beyond the Work input")
                supplied_commits = {
                    artifact["reference"]
                    for artifact in args.get("artifacts", [])
                    if artifact["kind"] == "commit"
                }
                if supplied_commits and supplied_commits != {commit}:
                    raise ValueError("reported commit artifact must equal the exact observed workspace HEAD")
                for gate in self.conn.execute(
                    "SELECT * FROM gates WHERE work_id=? ORDER BY position", (work["id"],)
                ).fetchall():
                    argv = json.loads(gate["argv_json"])
                    process = self.workspaces.cell_exec(work["cell"], argv)
                    result = {
                        "name": gate["name"],
                        "argv": argv,
                        "exit": process.returncode,
                        "stdout": process.stdout[-3000:],
                        "stderr": process.stderr[-3000:],
                    }
                    gate_results.append(result)
                    if process.returncode:
                        break
                observed_after = self.workspaces.observe(work)
                if (
                    observed_after.get("head") != commit
                    or observed_after.get("git_status", "").strip()
                ):
                    gate_results.append(
                        {
                            "name": "workspace-integrity",
                            "argv": [],
                            "exit": 1,
                            "stdout": "",
                            "stderr": "Declared gates changed HEAD or left the producer workspace dirty",
                        }
                    )
                failed_gates = [result for result in gate_results if result["exit"]]
                if failed_gates:
                    self.emit(
                        "attempt_verification_failed",
                        {
                            "attempt": attempt["id"],
                            "work": work["id"],
                            "candidate_commit": commit,
                            "gates": gate_results,
                        },
                        actor,
                    )
                    return {
                        "attempt": attempt["id"],
                        "state": "running",
                        "work_status": work["status"],
                        "candidate_status": "revision_required",
                        "candidate_commit": commit,
                        "gates": gate_results,
                    }
                imported_ref = self.workspaces.import_artifact(work, commit)
                imported = run(
                    ["git", "-C", str(self.workspaces.canonical), "rev-parse", f"{imported_ref}^{{commit}}"]
                ).stdout.strip()
                if imported != commit:
                    raise RuntimeError("imported artifact ref does not resolve to the verified producer commit")
            state = {
                "outcome_met": "produced",
                "blocked": "blocked",
                "abandoned": "abandoned",
            }[disposition]
            work_status = "completed" if state == "produced" else "abandoned" if state == "abandoned" else "blocked"
            self.conn.execute(
                "UPDATE attempts SET state=?,summary=?,ended_at=? WHERE id=?",
                (state, args["summary"], time.time(), attempt["id"]),
            )
            self.conn.execute(
                "UPDATE work SET status=?,feedback=?,pending_action=NULL,pending_owner=NULL,pending_reason=NULL,updated_at=? WHERE id=?",
                (work_status, args.get("resume_condition"), time.time(), work["id"]),
            )
            refs: list[dict[str, str]] = []
            seen_refs: set[tuple[str, str]] = set()
            for artifact in ([{"kind": "commit", "reference": commit}] if commit else []) + list(
                args.get("artifacts", [])
            ):
                normalized = (artifact["kind"], artifact["reference"])
                if normalized in seen_refs:
                    continue
                seen_refs.add(normalized)
                refs.append({"kind": normalized[0], "reference": normalized[1]})
            for artifact in refs:
                self.conn.execute(
                    "INSERT INTO artifacts VALUES(?,?,?,?,?,?,?)",
                    (
                        uid("artifact"),
                        work["id"],
                        attempt["id"],
                        artifact["kind"],
                        artifact["reference"],
                        1 if artifact["kind"] == "commit" else 0,
                        time.time(),
                    ),
                )
            self.emit(
                "attempt_terminal",
                {"attempt": attempt["id"], "work": work["id"], "state": state, "gates": gate_results},
                actor,
            )
            self.wake(work["created_by"], "attempt_terminal", {"attempt": attempt["id"], "work": work["id"], "state": state})
            if state == "produced":
                self._wake_newly_ready_dependents(work["id"])
            return {"attempt": attempt["id"], "state": state, "work_status": work_status, "gates": gate_results}

        if name == "request_judgement":
            judgement = uid("judgement")
            self.conn.execute(
                "INSERT INTO judgements(id,requested_by,assigned_to,subject,question,resume_condition,state,created_at) VALUES(?,?,?,?,?,?,'open',?)",
                (
                    judgement,
                    actor,
                    args["assigned_to"],
                    args["subject"],
                    args["question"],
                    args["resume_condition"],
                    time.time(),
                ),
            )
            self.emit("judgement_requested", {"judgement": judgement, "assigned_to": args["assigned_to"]}, actor)
            if args["assigned_to"] != "owner":
                self.wake(args["assigned_to"], "judgement_requested", {"judgement": judgement})
            return {"judgement": judgement, "state": "open"}

        if name == "decide":
            subject = args["subject"]
            request_id = args.get("request")
            open_same = self.conn.execute(
                "SELECT * FROM judgements WHERE subject=? AND state='open' ORDER BY created_at LIMIT 1",
                (subject,),
            ).fetchone()
            if open_same and request_id != open_same["id"]:
                raise ValueError("an open judgement on this subject must be resolved explicitly")
            if request_id:
                judgement = self.conn.execute(
                    "SELECT * FROM judgements WHERE id=? AND state='open'", (request_id,)
                ).fetchone()
                if not judgement:
                    raise ValueError("unknown open judgement")
                if judgement["assigned_to"] not in (actor, "owner"):
                    raise ValueError("actor is not assigned this judgement")
                self.conn.execute(
                    "UPDATE judgements SET state='resolved',choice=?,rationale=?,resolved_at=? WHERE id=?",
                    (args["choice"], args["rationale"], time.time(), request_id),
                )
            decision = uid("decision")
            self.conn.execute(
                "INSERT INTO decisions VALUES(?,?,?,?,?,?,?)",
                (
                    decision,
                    actor,
                    subject,
                    args["choice"],
                    args["rationale"],
                    json.dumps(args.get("evidence", [])),
                    time.time(),
                ),
            )
            self.emit("decision_recorded", {"decision": decision, "subject": subject, "choice": args["choice"]}, actor)
            return {"decision": decision}

        if name == "schedule":
            schedule = uid("schedule")
            self.conn.execute(
                "INSERT INTO schedules VALUES(?,?,?,?,NULL)",
                (schedule, actor, args["reason"], float(args["not_before_unix"])),
            )
            self.emit("schedule_created", {"schedule": schedule, "not_before": args["not_before_unix"]}, actor)
            return {"schedule": schedule}

        raise ValueError(f"unknown command {name}")

    def _apply_redirect_now(self, work: sqlite3.Row, action: str, reason: str, new_owner: str | None) -> None:
        if action == "repair":
            self.conn.execute(
                "UPDATE work SET status='active',revision=revision+1,feedback=?,pending_action=NULL,pending_owner=NULL,pending_reason=NULL,updated_at=? WHERE id=?",
                (reason, time.time(), work["id"]),
            )
            target = work["owner"]
        elif action == "reassign":
            self.conn.execute(
                "UPDATE work SET owner=?,status='active',revision=revision+1,feedback=?,pending_action=NULL,pending_owner=NULL,pending_reason=NULL,updated_at=? WHERE id=?",
                (new_owner, reason, time.time(), work["id"]),
            )
            target = new_owner
        else:
            self.conn.execute(
                "UPDATE work SET status='abandoned',feedback=?,pending_action=NULL,pending_owner=NULL,pending_reason=NULL,updated_at=? WHERE id=?",
                (reason, time.time(), work["id"]),
            )
            target = work["owner"]
        self.emit("work_redirected", {"work": work["id"], "action": action, "reason": reason})
        if target and action != "abandon":
            self.wake(target, "work_changed", {"work": work["id"], "action": action})

    def _wake_newly_ready_dependents(self, completed_work: str) -> None:
        rows = self.conn.execute(
            """
            SELECT DISTINCT w.id,w.owner FROM work_edges e JOIN work w ON w.id=e.work_id
            WHERE e.other_work_id=? AND e.kind='requires' AND w.status='active'
              AND NOT EXISTS (
                SELECT 1 FROM work_edges pending JOIN work dependency ON dependency.id=pending.other_work_id
                WHERE pending.work_id=w.id AND pending.kind='requires' AND dependency.status!='completed'
              )
            """,
            (completed_work,),
        ).fetchall()
        for row in rows:
            self.wake(row["owner"], "work_became_ready", {"work": row["id"]})

    def snapshot(self, actor: str) -> dict[str, Any]:
        self._actor(actor)
        work = json_rows(self.conn.execute("SELECT * FROM work ORDER BY created_at").fetchall())
        attempts = json_rows(self.conn.execute("SELECT * FROM attempts ORDER BY started_at").fetchall())
        edges = json_rows(self.conn.execute("SELECT * FROM work_edges ORDER BY work_id,kind").fetchall())
        artifacts = json_rows(self.conn.execute("SELECT * FROM artifacts ORDER BY created_at").fetchall())
        messages = json_rows(
            self.conn.execute("SELECT * FROM messages WHERE recipient=? ORDER BY created_at", (actor,)).fetchall()
        )
        judgements = json_rows(
            self.conn.execute(
                "SELECT * FROM judgements WHERE state='open' AND assigned_to IN (?, 'owner') ORDER BY created_at",
                (actor,),
            ).fetchall()
        )
        decisions = json_rows(self.conn.execute("SELECT * FROM decisions ORDER BY created_at").fetchall())
        actors = json_rows(self.conn.execute("SELECT * FROM actors ORDER BY id").fetchall())
        for collection in (messages, decisions):
            for row in collection:
                for key in list(row):
                    if key.endswith("_json") and row[key]:
                        row[key.removesuffix("_json")] = json.loads(row.pop(key))
        return {
            "actors": actors,
            "work": work,
            "edges": edges,
            "attempts": attempts,
            "artifacts": artifacts,
            "messages_for_actor": messages,
            "open_judgements": judgements,
            "decisions": decisions,
        }

    def claim_ready(self, limit: int, lease_seconds: int = 480) -> list[dict[str, Any]]:
        claimed: list[dict[str, Any]] = []
        candidates = self.conn.execute(
            """
            SELECT w.* FROM work w
            WHERE w.status='active' AND w.pending_action IS NULL
              AND NOT EXISTS (SELECT 1 FROM attempts a WHERE a.work_id=w.id AND a.revision=w.revision)
              AND NOT EXISTS (SELECT 1 FROM attempts a WHERE a.actor=w.owner AND a.state='running')
              AND NOT EXISTS (
                SELECT 1 FROM work_edges e JOIN work dependency ON dependency.id=e.other_work_id
                WHERE e.work_id=w.id AND e.kind='requires' AND dependency.status!='completed'
              )
            ORDER BY w.created_at
            """
        ).fetchall()
        for candidate in candidates:
            if len(claimed) >= limit:
                break
            required_base = self._single_required_commit(candidate["id"])
            workspace, cell, resolved_base = self.workspaces.ensure_work(candidate, required_base)
            with transaction(self.conn):
                fresh = self.conn.execute("SELECT * FROM work WHERE id=?", (candidate["id"],)).fetchone()
                if fresh["status"] != "active" or fresh["revision"] != candidate["revision"]:
                    continue
                if self.conn.execute(
                    "SELECT 1 FROM attempts WHERE actor=? AND state='running'", (fresh["owner"],)
                ).fetchone():
                    continue
                attempt_id = uid("attempt")
                lease = uid("lease")
                now = time.time()
                self.conn.execute(
                    "UPDATE work SET workspace=?,cell=?,base_ref=?,updated_at=? WHERE id=?",
                    (workspace, cell, resolved_base, now, fresh["id"]),
                )
                self.conn.execute(
                    "INSERT INTO attempts(id,work_id,revision,actor,lease_token,lease_expires_at,state,started_at) VALUES(?,?,?,?,?,?,'running',?)",
                    (attempt_id, fresh["id"], fresh["revision"], fresh["owner"], lease, now + lease_seconds, now),
                )
                # The Work snapshot supplied with this lease is the delivery of
                # every currently pending wake for the actor.  The outbox is a
                # durable wake mechanism, not a second work queue.
                self.conn.execute(
                    "UPDATE outbox SET delivered_at=? WHERE target=? AND delivered_at IS NULL",
                    (now, fresh["owner"]),
                )
                self.emit(
                    "attempt_started",
                    {"attempt": attempt_id, "work": fresh["id"], "revision": fresh["revision"]},
                    fresh["owner"],
                )
                claimed.append(
                    {
                        **dict(fresh),
                        "workspace": workspace,
                        "cell": cell,
                        "base_ref": resolved_base,
                        "attempt": attempt_id,
                        "lease_token": lease,
                        "lease_expires_at": now + lease_seconds,
                    }
                )
        return claimed

    def _single_required_commit(self, work_id: str) -> str | None:
        dependencies = self.conn.execute(
            "SELECT other_work_id FROM work_edges WHERE work_id=? AND kind='requires' ORDER BY other_work_id",
            (work_id,),
        ).fetchall()
        if len(dependencies) != 1:
            return None
        rows = self.conn.execute(
            """
            SELECT artifact.reference
            FROM attempts attempt
            JOIN artifacts artifact ON artifact.attempt_id=attempt.id AND artifact.kind='commit'
            WHERE attempt.work_id=? AND attempt.state='produced'
            ORDER BY attempt.ended_at DESC, artifact.created_at DESC
            """,
            (dependencies[0]["other_work_id"],),
        ).fetchall()
        for row in rows:
            resolved = run(
                ["git", "-C", str(self.workspaces.canonical), "rev-parse", f"{row['reference']}^{{commit}}"],
                check=False,
            )
            if resolved.returncode == 0:
                return resolved.stdout.strip()
        return None

    def pending_cancellations(self) -> list[dict[str, Any]]:
        return json_rows(
            self.conn.execute(
                "SELECT id,actor,work_id FROM attempts WHERE state='running' AND cancel_requested_at IS NOT NULL"
            ).fetchall()
        )

    def finalize_cancellation(self, attempt_id: str) -> None:
        with transaction(self.conn):
            attempt = self.conn.execute("SELECT * FROM attempts WHERE id=?", (attempt_id,)).fetchone()
            if not attempt or attempt["state"] != "running":
                return
            work = self.conn.execute("SELECT * FROM work WHERE id=?", (attempt["work_id"],)).fetchone()
            action = work["pending_action"]
            if not action:
                return
            self.conn.execute(
                "UPDATE attempts SET state='abandoned',summary=?,ended_at=? WHERE id=?",
                (f"Lease cancelled for pending {action}", time.time(), attempt_id),
            )
            self._apply_redirect_now(work, action, work["pending_reason"] or action, work["pending_owner"])
            self.emit("attempt_terminal", {"attempt": attempt_id, "work": work["id"], "state": "abandoned"})

    def mark_unknown(self, attempt_id: str, summary: str) -> None:
        with transaction(self.conn):
            attempt = self.conn.execute("SELECT * FROM attempts WHERE id=?", (attempt_id,)).fetchone()
            if not attempt or attempt["state"] != "running":
                return
            work = self.conn.execute("SELECT * FROM work WHERE id=?", (attempt["work_id"],)).fetchone()
            self.conn.execute(
                "UPDATE attempts SET state='unknown',summary=?,ended_at=? WHERE id=?",
                (summary, time.time(), attempt_id),
            )
            self.conn.execute(
                "UPDATE work SET status='blocked',feedback=?,updated_at=? WHERE id=?",
                (summary, time.time(), work["id"]),
            )
            self.emit("attempt_terminal", {"attempt": attempt_id, "work": work["id"], "state": "unknown"})
            self.wake(work["created_by"], "attempt_terminal", {"attempt": attempt_id, "work": work["id"], "state": "unknown"})

    def renew_attempt_lease(
        self,
        attempt_id: str,
        actor: str,
        lease_token: str,
        *,
        lease_seconds: int = 900,
    ) -> bool:
        """Renew the lease only while the supervisor still owns the exact live Attempt."""
        if lease_seconds < 60:
            raise ValueError("Attempt lease renewal must be at least 60 seconds")
        with transaction(self.conn):
            attempt = self.conn.execute("SELECT * FROM attempts WHERE id=?", (attempt_id,)).fetchone()
            if not attempt or attempt["state"] != "running":
                return False
            work = self.conn.execute("SELECT * FROM work WHERE id=?", (attempt["work_id"],)).fetchone()
            if attempt["actor"] != actor or work["owner"] != actor:
                raise ValueError("Attempt lease actor does not own Work")
            if attempt["lease_token"] != lease_token:
                raise ValueError("invalid Attempt lease token")
            if attempt["revision"] != work["revision"] or attempt["cancel_requested_at"]:
                return False
            expires_at = time.time() + lease_seconds
            self.conn.execute(
                "UPDATE attempts SET lease_expires_at=? WHERE id=?",
                (expires_at, attempt_id),
            )
            self.emit(
                "attempt_lease_renewed",
                {"attempt": attempt_id, "work": work["id"], "lease_expires_at": expires_at},
                actor,
            )
            return True

    def reconcile_orphaned_attempts(self) -> list[str]:
        reconciled: list[str] = []
        rows = self.conn.execute(
            "SELECT a.id,a.cancel_requested_at,w.cell FROM attempts a JOIN work w ON w.id=a.work_id WHERE a.state='running'"
        ).fetchall()
        for row in rows:
            self.workspaces.stop_actor_process(row["cell"])
            if row["cancel_requested_at"]:
                self.finalize_cancellation(row["id"])
            else:
                self.mark_unknown(
                    row["id"],
                    "Coordinator restarted without a live controller attachment; orphaned actor process stopped",
                )
            reconciled.append(row["id"])
        if reconciled:
            self.emit("startup_reconciled", {"attempts": reconciled})
        return reconciled

    def attempt(self, attempt_id: str) -> sqlite3.Row | None:
        return self.conn.execute("SELECT * FROM attempts WHERE id=?", (attempt_id,)).fetchone()

    def work(self, work_id: str) -> sqlite3.Row | None:
        return self.conn.execute("SELECT * FROM work WHERE id=?", (work_id,)).fetchone()

    def pending_causes(self, actor: str) -> list[dict[str, Any]]:
        rows = self.conn.execute(
            "SELECT id,cause,payload_json,created_at FROM outbox WHERE target=? AND delivered_at IS NULL ORDER BY id",
            (actor,),
        ).fetchall()
        if rows:
            self.conn.execute(
                "UPDATE outbox SET delivered_at=? WHERE target=? AND delivered_at IS NULL", (time.time(), actor)
            )
        return [
            {
                "id": row["id"],
                "cause": row["cause"],
                "payload": json.loads(row["payload_json"]),
                "created_at": row["created_at"],
            }
            for row in rows
        ]

    def fire_due_schedules(self) -> None:
        with transaction(self.conn):
            for row in self.conn.execute(
                "SELECT * FROM schedules WHERE fired_at IS NULL AND not_before<=?", (time.time(),)
            ).fetchall():
                self.conn.execute("UPDATE schedules SET fired_at=? WHERE id=?", (time.time(), row["id"]))
                self.wake(row["actor"], "schedule_due", {"schedule": row["id"], "reason": row["reason"]})

    def next_schedule_delay(self) -> float | None:
        row = self.conn.execute("SELECT MIN(not_before) FROM schedules WHERE fired_at IS NULL").fetchone()
        if row[0] is None:
            return None
        return max(0.0, float(row[0]) - time.time())

    def cost(self) -> float:
        return float(self.conn.execute("SELECT COALESCE(SUM(cost_usd),0) FROM turns").fetchone()[0])

    def start_turn(self, turn_id: str, actor: str, attempt_id: str | None) -> None:
        self.conn.execute(
            "INSERT INTO turns(id,actor,attempt_id,started_at,cost_usd) VALUES(?,?,?,?,0)",
            (turn_id, actor, attempt_id, time.time()),
        )
        self.emit("turn_started", {"turn": turn_id, "attempt": attempt_id}, actor)

    def finish_turn(self, turn_id: str, result: dict[str, Any]) -> None:
        self.conn.execute(
            "UPDATE turns SET ended_at=?,cost_usd=COALESCE(?,cost_usd),used_tokens=COALESCE(?,used_tokens),output_tokens=?,cached_input_tokens=COALESCE(?,cached_input_tokens),reasoning_output_tokens=COALESCE(?,reasoning_output_tokens),tool_calls=?,end_kind=?,transcript=? WHERE id=?",
            (
                time.time(),
                result.get("cost_usd"),
                result.get("used_tokens"),
                result.get("output_tokens"),
                result.get("cached_input_tokens"),
                result.get("reasoning_output_tokens"),
                len(result.get("tool_calls", [])),
                result.get("stop_reason"),
                (result.get("text") or result.get("error") or "")[-20000:],
                turn_id,
            ),
        )
        self.emit(
            "turn_terminal",
            {
                "turn": turn_id,
                "stop_reason": result.get("stop_reason"),
                "cost_usd": result.get("cost_usd"),
                "error": (result.get("error") or "")[-2000:] or None,
            },
            self.conn.execute("SELECT actor FROM turns WHERE id=?", (turn_id,)).fetchone()[0],
        )

    def run_complete(self) -> bool:
        return bool(
            self.conn.execute(
                "SELECT 1 FROM decisions WHERE subject='run' AND choice='complete' ORDER BY created_at DESC LIMIT 1"
            ).fetchone()
        )

    def summary(self) -> dict[str, Any]:
        return {
            "turns": json_rows(
                self.conn.execute(
                    "SELECT actor,attempt_id,started_at,ended_at,cost_usd,used_tokens,output_tokens,cached_input_tokens,reasoning_output_tokens,tool_calls,end_kind FROM turns ORDER BY started_at"
                ).fetchall()
            ),
            "work": json_rows(self.conn.execute("SELECT * FROM work ORDER BY created_at").fetchall()),
            "attempts": json_rows(self.conn.execute("SELECT * FROM attempts ORDER BY started_at").fetchall()),
            "artifacts": json_rows(self.conn.execute("SELECT * FROM artifacts ORDER BY created_at").fetchall()),
            "decisions": json_rows(self.conn.execute("SELECT * FROM decisions ORDER BY created_at").fetchall()),
            "events_by_kind": dict(self.conn.execute("SELECT kind,COUNT(*) FROM events GROUP BY kind ORDER BY kind")),
            "cost_usd": self.cost(),
            "quick_check": self.conn.execute("PRAGMA quick_check").fetchone()[0],
        }


def request(endpoint: str, payload: dict[str, Any]) -> dict[str, Any]:
    host, port_text = endpoint.rsplit(":", 1)
    with socket.create_connection((host, int(port_text)), timeout=30) as client:
        client.sendall((json.dumps(payload) + "\n").encode())
        data = b""
        while not data.endswith(b"\n"):
            chunk = client.recv(65536)
            if not chunk:
                break
            data += chunk
    response = json.loads(data)
    if not response.get("ok"):
        raise RuntimeError(response.get("error") or "coordinator request failed")
    return response["result"]
