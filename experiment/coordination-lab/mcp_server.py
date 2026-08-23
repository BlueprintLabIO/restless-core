#!/usr/bin/env python3
from __future__ import annotations

import json
import os
import socket
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

from store import connect, emit, snapshot, transaction, uid, wake


DB = os.environ["COORD_DB"]
ACTOR = os.environ["COORD_ACTOR"]
ATTEMPT = os.environ.get("COORD_ATTEMPT", "")
RUN_ID = os.environ["COORD_RUN_ID"]
REPO = os.environ["COORD_REPO"]
NOTIFY_HOST = os.environ.get("COORD_NOTIFY_HOST", "host.docker.internal")
NOTIFY_PORT = int(os.environ.get("COORD_NOTIFY_PORT", "0"))


def notify(payload: dict[str, Any]) -> None:
    if not NOTIFY_PORT:
        return
    try:
        with socket.create_connection((NOTIFY_HOST, NOTIFY_PORT), timeout=1) as sock:
            sock.sendall((json.dumps(payload) + "\n").encode())
    except OSError:
        # The durable outbox remains authoritative; startup reconciliation can recover it.
        pass


def schemas() -> list[dict[str, Any]]:
    obj = {"type": "object", "additionalProperties": False}
    string_list = {"type": "array", "items": {"type": "string"}}
    return [
        {"name": "inspect_coordination", "description": "Read current actors, Work graph, Attempts, messages, judgements and artifacts.", "inputSchema": {**obj, "properties": {}}},
        {"name": "send", "description": "Send semantic communication. It never assigns or completes Work.", "inputSchema": {**obj, "required": ["to", "body"], "properties": {"to": {"type": "string"}, "body": {"type": "string"}, "refs": string_list}}},
        {"name": "commission", "description": "Create one outcome-sized Work responsibility and its graph/workspace atomically. Use an existing durable actor.", "inputSchema": {**obj, "required": ["owner", "outcome", "expected_artifact"], "properties": {
            "owner": {"type": "string"}, "outcome": {"type": "string"},
            "expected_artifact": {"type": "string"}, "base_ref": {"type": "string"},
            "requires": string_list, "revises": string_list,
            "gates": {"type": "array", "items": {"type": "object", "required": ["name", "argv"], "properties": {"name": {"type": "string"}, "argv": string_list}}},
        }}},
        {"name": "redirect", "description": "Repair, reassign, or abandon existing Work. Repair increments its revision and preserves the workspace.", "inputSchema": {**obj, "required": ["work", "action", "reason"], "properties": {"work": {"type": "string"}, "action": {"enum": ["repair", "reassign", "abandon"]}, "reason": {"type": "string"}, "feedback": {"type": "string"}, "new_owner": {"type": "string"}}}},
        {"name": "report", "description": "Append an asynchronous Attempt callback. progress is nonterminal; outcome_met runs gates; blocked/abandoned close the Attempt.", "inputSchema": {**obj, "required": ["disposition", "summary"], "properties": {
            "disposition": {"enum": ["progress", "outcome_met", "blocked", "abandoned"]},
            "summary": {"type": "string"},
            "artifacts": {"type": "array", "items": {"type": "object", "required": ["kind", "reference"], "properties": {"kind": {"type": "string"}, "reference": {"type": "string"}}}},
            "evidence": string_list, "resume_condition": {"type": "string"},
        }}},
        {"name": "request_judgement", "description": "Request a bounded internal or owner judgement with an observable resume condition.", "inputSchema": {**obj, "required": ["assigned_to", "subject", "question", "resume_condition"], "properties": {"assigned_to": {"type": "string"}, "subject": {"type": "string"}, "question": {"type": "string"}, "resume_condition": {"type": "string"}}}},
        {"name": "decide", "description": "Resolve a judgement or record an accountable organisational choice. Use subject=run and choice=complete only when the best artifact is prepared for review.", "inputSchema": {**obj, "required": ["subject", "choice", "rationale"], "properties": {"request": {"type": "string"}, "subject": {"type": "string"}, "choice": {"type": "string"}, "rationale": {"type": "string"}, "evidence": {"type": "array", "items": {"type": "string"}}}}},
        {"name": "schedule", "description": "Schedule a genuinely time-driven future wake. Never use this to poll delegated Work.", "inputSchema": {**obj, "required": ["reason", "not_before_unix"], "properties": {"reason": {"type": "string"}, "not_before_unix": {"type": "number"}}}},
    ]


def command_once(name: str, args: dict[str, Any], fn) -> dict[str, Any]:
    command_id = f"mcp-{uid('cmd')}"
    conn = connect(DB)
    try:
        result = fn(conn)
        conn.execute(
            "INSERT INTO commands(id,actor,name,args_json,result_json,created_at) VALUES(?,?,?,?,?,?)",
            (command_id, ACTOR, name, json.dumps(args, sort_keys=True), json.dumps(result, sort_keys=True), time.time()),
        )
        emit(conn, "command_applied", {"command": name, "command_id": command_id}, ACTOR)
        return result
    finally:
        conn.close()


def handle(name: str, args: dict[str, Any]) -> dict[str, Any]:
    if name == "inspect_coordination":
        conn = connect(DB)
        try:
            return snapshot(conn, ACTOR)
        finally:
            conn.close()

    if name == "send":
        def apply(conn):
            to = args["to"]
            if not conn.execute("SELECT 1 FROM actors WHERE id=? AND active=1", (to,)).fetchone():
                raise ValueError(f"unknown active actor {to}")
            mid = uid("msg")
            with transaction(conn):
                conn.execute(
                    "INSERT INTO messages(id,sender,recipient,body,refs_json,created_at) VALUES(?,?,?,?,?,?)",
                    (mid, ACTOR, to, args["body"], json.dumps(args.get("refs", [])), time.time()),
                )
                wake(conn, to, "message_appended", {"message": mid, "from": ACTOR})
            notify({"target": to, "cause": "message_appended", "id": mid})
            return {"message": mid}
        return command_once(name, args, apply)

    if name == "commission":
        def apply(conn):
            owner = args["owner"]
            if not conn.execute("SELECT 1 FROM actors WHERE id=? AND active=1", (owner,)).fetchone():
                raise ValueError(f"unknown active actor {owner}")
            work_id = uid("work")
            base_ref = args.get("base_ref") or "514b7b3"
            branch = f"lab/{RUN_ID}/{work_id}"
            worktree = f"/lab/{RUN_ID}/worktrees/{work_id}"
            requires = args.get("requires", [])
            revises = args.get("revises", [])
            for other in requires + revises:
                if not conn.execute("SELECT 1 FROM work WHERE id=?", (other,)).fetchone():
                    raise ValueError(f"unknown Work {other}")
            Path(worktree).parent.mkdir(parents=True, exist_ok=True)
            proc = subprocess.run(
                ["git", "-c", f"safe.directory={REPO}", "-C", REPO, "worktree", "add", "-b", branch, worktree, base_ref],
                text=True, capture_output=True,
            )
            if proc.returncode:
                raise RuntimeError(f"worktree creation failed: {proc.stderr[-1000:]}")
            try:
                now = time.time()
                with transaction(conn):
                    conn.execute(
                        "INSERT INTO work(id,created_by,owner,outcome,expected_artifact,status,revision,base_ref,branch,worktree,created_at,updated_at) VALUES(?,?,?,?,?,'active',1,?,?,?,?,?)",
                        (work_id, ACTOR, owner, args["outcome"], args["expected_artifact"], base_ref, branch, worktree, now, now),
                    )
                    for other in requires:
                        conn.execute("INSERT INTO work_edges VALUES(?,?,'requires')", (work_id, other))
                    for other in revises:
                        conn.execute("INSERT INTO work_edges VALUES(?,?,'revises')", (work_id, other))
                    for position, gate in enumerate(args.get("gates", [])):
                        conn.execute("INSERT INTO gates VALUES(?,?,?,?)", (work_id, position, gate["name"], json.dumps(gate["argv"])))
                    emit(conn, "work_commissioned", {"work": work_id, "owner": owner, "requires": requires, "revises": revises}, ACTOR)
                    wake(conn, owner, "work_became_ready", {"work": work_id})
            except Exception:
                subprocess.run(["git", "-c", f"safe.directory={REPO}", "-C", REPO, "worktree", "remove", "--force", worktree], capture_output=True)
                raise
            notify({"target": owner, "cause": "work_became_ready", "id": work_id})
            return {"work": work_id, "worktree": worktree, "branch": branch}
        return command_once(name, args, apply)

    if name == "redirect":
        def apply(conn):
            row = conn.execute("SELECT * FROM work WHERE id=?", (args["work"],)).fetchone()
            if not row:
                raise ValueError("unknown Work")
            action = args["action"]
            with transaction(conn):
                if action == "repair":
                    conn.execute("UPDATE work SET status='active', revision=revision+1, feedback=?, updated_at=? WHERE id=?", (args.get("feedback") or args["reason"], time.time(), row["id"]))
                elif action == "reassign":
                    owner = args.get("new_owner")
                    if not owner or not conn.execute("SELECT 1 FROM actors WHERE id=? AND active=1", (owner,)).fetchone():
                        raise ValueError("reassign needs an existing active new_owner")
                    conn.execute("UPDATE work SET owner=?, status='active', feedback=?, updated_at=? WHERE id=?", (owner, args.get("feedback") or args["reason"], time.time(), row["id"]))
                else:
                    conn.execute("UPDATE work SET status='abandoned', feedback=?, updated_at=? WHERE id=?", (args["reason"], time.time(), row["id"]))
                emit(conn, "work_redirected", {"work": row["id"], "action": action, "reason": args["reason"]}, ACTOR)
                target = args.get("new_owner") or row["owner"]
                wake(conn, target, "work_changed", {"work": row["id"], "action": action})
            notify({"target": target, "cause": "work_changed", "id": row["id"]})
            return {"work": row["id"], "action": action}
        return command_once(name, args, apply)

    if name == "report":
        if not ATTEMPT:
            raise ValueError("report is only available inside a claimed Attempt")
        def apply(conn):
            attempt = conn.execute("SELECT * FROM attempts WHERE id=?", (ATTEMPT,)).fetchone()
            if not attempt or attempt["state"] != "running":
                raise ValueError("Attempt is not running")
            work = conn.execute("SELECT * FROM work WHERE id=?", (attempt["work_id"],)).fetchone()
            disposition = args["disposition"]
            if disposition == "progress":
                with transaction(conn):
                    emit(conn, "attempt_progress", {"attempt": ATTEMPT, "work": work["id"], "summary": args["summary"]}, ACTOR)
                    wake(conn, work["created_by"], "attempt_progress", {"attempt": ATTEMPT, "work": work["id"]})
                notify({"target": work["created_by"], "cause": "attempt_progress", "id": ATTEMPT})
                return {"attempt": ATTEMPT, "state": "running", "note": "progress is nonterminal; keep working or make a terminal report"}

            gate_results = []
            if disposition == "outcome_met":
                status = subprocess.run(["git", "-c", f"safe.directory={work['worktree']}", "-C", work["worktree"], "status", "--porcelain"], text=True, capture_output=True)
                if status.stdout.strip():
                    raise ValueError("outcome_met requires a clean committed worktree; commit the artifact first")
                for gate in conn.execute("SELECT * FROM gates WHERE work_id=? ORDER BY position", (work["id"],)).fetchall():
                    argv = json.loads(gate["argv_json"])
                    proc = subprocess.run(argv, cwd=work["worktree"], text=True, capture_output=True, timeout=300)
                    gate_results.append({"name": gate["name"], "argv": argv, "exit": proc.returncode, "stdout": proc.stdout[-3000:], "stderr": proc.stderr[-3000:]})
                    if proc.returncode:
                        disposition = "failed_gate"
                        break
            state = {"outcome_met": "produced", "blocked": "blocked", "abandoned": "abandoned", "failed_gate": "failed"}[disposition]
            work_status = "completed" if state == "produced" else "abandoned" if state == "abandoned" else "blocked"
            with transaction(conn):
                conn.execute("UPDATE attempts SET state=?, summary=?, ended_at=? WHERE id=?", (state, args["summary"], time.time(), ATTEMPT))
                conn.execute("UPDATE work SET status=?, feedback=?, updated_at=? WHERE id=?", (work_status, args.get("resume_condition"), time.time(), work["id"]))
                refs = list(args.get("artifacts", []))
                head = subprocess.run(["git", "-c", f"safe.directory={work['worktree']}", "-C", work["worktree"], "rev-parse", "HEAD"], text=True, capture_output=True)
                if head.returncode == 0:
                    refs.append({"kind": "commit", "reference": head.stdout.strip()})
                for artifact in refs:
                    conn.execute("INSERT INTO artifacts VALUES(?,?,?,?,?,?,?)", (uid("artifact"), work["id"], ATTEMPT, artifact["kind"], artifact["reference"], 0, time.time()))
                emit(conn, "attempt_terminal", {"attempt": ATTEMPT, "work": work["id"], "state": state, "gates": gate_results, "evidence": args.get("evidence", [])}, ACTOR)
                wake(conn, work["created_by"], "attempt_terminal", {"attempt": ATTEMPT, "work": work["id"], "state": state})
            notify({"target": work["created_by"], "cause": "attempt_terminal", "id": ATTEMPT})
            return {"attempt": ATTEMPT, "state": state, "work_status": work_status, "gates": gate_results}
        return command_once(name, args, apply)

    if name == "request_judgement":
        def apply(conn):
            jid = uid("judgement")
            with transaction(conn):
                conn.execute("INSERT INTO judgements(id,requested_by,assigned_to,subject,question,resume_condition,state,created_at) VALUES(?,?,?,?,?,?,'open',?)", (jid, ACTOR, args["assigned_to"], args["subject"], args["question"], args["resume_condition"], time.time()))
                emit(conn, "judgement_requested", {"judgement": jid, "assigned_to": args["assigned_to"]}, ACTOR)
                if args["assigned_to"] != "owner":
                    wake(conn, args["assigned_to"], "judgement_requested", {"judgement": jid})
            if args["assigned_to"] != "owner":
                notify({"target": args["assigned_to"], "cause": "judgement_requested", "id": jid})
            return {"judgement": jid, "state": "open"}
        return command_once(name, args, apply)

    if name == "decide":
        def apply(conn):
            did = uid("decision")
            request = args.get("request")
            with transaction(conn):
                if request:
                    judgement = conn.execute("SELECT * FROM judgements WHERE id=? AND state='open'", (request,)).fetchone()
                    if not judgement:
                        raise ValueError("unknown open judgement")
                    if judgement["assigned_to"] not in (ACTOR, "owner"):
                        raise ValueError("actor is not assigned this judgement")
                    conn.execute("UPDATE judgements SET state='resolved',choice=?,rationale=?,resolved_at=? WHERE id=?", (args["choice"], args["rationale"], time.time(), request))
                    wake(conn, judgement["requested_by"], "judgement_resolved", {"judgement": request, "choice": args["choice"]})
                conn.execute("INSERT INTO decisions VALUES(?,?,?,?,?,?,?)", (did, ACTOR, args["subject"], args["choice"], args["rationale"], json.dumps(args.get("evidence", [])), time.time()))
                emit(conn, "decision_recorded", {"decision": did, "subject": args["subject"], "choice": args["choice"]}, ACTOR)
            if request:
                notify({"target": judgement["requested_by"], "cause": "judgement_resolved", "id": request})
            return {"decision": did}
        return command_once(name, args, apply)

    if name == "schedule":
        def apply(conn):
            sid = uid("schedule")
            conn.execute("INSERT INTO schedules VALUES(?,?,?,?,NULL)", (sid, ACTOR, args["reason"], float(args["not_before_unix"])))
            emit(conn, "schedule_created", {"schedule": sid, "not_before": args["not_before_unix"], "reason": args["reason"]}, ACTOR)
            return {"schedule": sid}
        return command_once(name, args, apply)

    raise ValueError(f"unknown tool {name}")


def respond(request_id: Any, result: Any = None, error: dict[str, Any] | None = None) -> None:
    payload = {"jsonrpc": "2.0", "id": request_id}
    if error:
        payload["error"] = error
    else:
        payload["result"] = result
    sys.stdout.write(json.dumps(payload) + "\n")
    sys.stdout.flush()


def main() -> None:
    for line in sys.stdin:
        try:
            request = json.loads(line)
            method = request.get("method")
            if method == "initialize":
                respond(request["id"], {"protocolVersion": "2025-03-26", "capabilities": {"tools": {}}, "serverInfo": {"name": "coordination-lab", "version": "0.1"}})
            elif method == "tools/list":
                respond(request["id"], {"tools": schemas()})
            elif method == "tools/call":
                params = request.get("params", {})
                try:
                    result = handle(params["name"], params.get("arguments", {}))
                    respond(request["id"], {"content": [{"type": "text", "text": json.dumps(result, indent=2, sort_keys=True)}], "isError": False})
                except Exception as exc:
                    respond(request["id"], {"content": [{"type": "text", "text": str(exc)}], "isError": True})
            elif "id" in request:
                respond(request["id"], error={"code": -32601, "message": f"method not found: {method}"})
        except Exception as exc:
            if isinstance(locals().get("request"), dict) and "id" in request:
                respond(request["id"], error={"code": -32603, "message": str(exc)})


if __name__ == "__main__":
    main()
