#!/usr/bin/env python3
"""Scratch Actor Host adapter for the official, ChatGPT-authenticated Codex CLI.

The adapter deliberately owns transport only. OrgIntel remains authoritative for
coordination state, while Codex owns its model conversation and the Runtime owns
the ordinary workspace. JSONL events are projected into the same factual trace
stream used by the ACP comparison driver.
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import socket
import subprocess
from pathlib import Path
from typing import Any


def required(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        raise RuntimeError(f"missing {name}")
    return value


def emit(kind: str, payload: dict[str, Any]) -> None:
    endpoint = required("COORD_EVENT_ENDPOINT")
    host, port_text = endpoint.rsplit(":", 1)
    event = {
        "type": "trace",
        "at": str(int(__import__("time").time() * 1000)),
        "actor": required("COORD_ACTOR"),
        "kind": kind,
        "payload": payload,
        "turn_id": required("COORD_TURN_ID"),
    }
    try:
        with socket.create_connection((host, int(port_text)), timeout=10) as client:
            client.sendall((json.dumps(event) + "\n").encode())
            response = b""
            while not response.endswith(b"\n"):
                chunk = client.recv(65536)
                if not chunk:
                    break
                response += chunk
    except OSError:
        # Trace delivery is diagnostic. The runner still records the terminal
        # result, and OrgIntel callbacks use their own MCP request path.
        return


def toml_string(value: str) -> str:
    # JSON string syntax is valid TOML basic-string syntax for these values.
    return json.dumps(value)


def session_file() -> Path:
    actor = required("COORD_ACTOR")
    work = os.environ.get("COORD_WORK", "")
    workspace = Path(required("COORD_HOST_WORKDIR")).resolve()
    key = (
        actor
        if os.environ.get("COORD_ACTOR_KIND") == "exec"
        else f"{actor}:{work or 'unclaimed'}:{workspace}"
    )
    digest = hashlib.sha256(key.encode()).hexdigest()[:20]
    directory = Path(required("COORD_TURN_DIR")) / "codex-sessions"
    directory.mkdir(parents=True, exist_ok=True)
    return directory / f"{digest}.json"


def validate_runtime_identity() -> Path:
    """Fail before model launch if this turn is attached to the wrong Git workspace."""
    workdir = Path(required("COORD_HOST_WORKDIR")).resolve()
    if not workdir.is_dir():
        raise RuntimeError(f"actor workspace does not exist: {workdir}")
    root = subprocess.run(
        ["git", "-C", str(workdir), "rev-parse", "--show-toplevel"],
        text=True,
        capture_output=True,
        check=True,
    ).stdout.strip()
    if Path(root).resolve() != workdir:
        raise RuntimeError(f"actor workspace root mismatch: expected {workdir}, observed {root}")
    branch = subprocess.run(
        ["git", "-C", str(workdir), "branch", "--show-current"],
        text=True,
        capture_output=True,
        check=True,
    ).stdout.strip()
    expected_branch = os.environ.get("COORD_EXPECTED_BRANCH")
    if expected_branch is None:
        raise RuntimeError("missing COORD_EXPECTED_BRANCH")
    if branch != expected_branch:
        raise RuntimeError(f"actor branch mismatch: expected {expected_branch!r}, observed {branch!r}")
    work = os.environ.get("COORD_WORK", "")
    if work and not workdir.name.startswith(work):
        raise RuntimeError(f"Work workspace mismatch: {workdir.name!r} is not bound to {work!r}")
    emit(
        "runtime_identity_verified",
        {"workspace": str(workdir), "branch": branch, "work": work or None},
    )
    return workdir


def mcp_config() -> list[str]:
    env = {
        "COORD_ENDPOINT": required("COORD_HOST_ENDPOINT"),
        "COORD_ACTOR": required("COORD_ACTOR"),
        "COORD_ATTEMPT": os.environ.get("COORD_ATTEMPT", ""),
        "COORD_LEASE_TOKEN": os.environ.get("COORD_LEASE_TOKEN", ""),
        "COORD_RUN_ID": required("COORD_RUN_ID"),
    }
    inline_env = ",".join(f"{key}={toml_string(value)}" for key, value in env.items())
    return [
        "-c",
        f"mcp_servers.orgintel.command={toml_string(required('COORD_PYTHON'))}",
        "-c",
        f"mcp_servers.orgintel.args=[{toml_string(required('COORD_HOST_MCP_SERVER'))}]",
        "-c",
        f"mcp_servers.orgintel.env={{{inline_env}}}",
        "-c",
        "mcp_servers.orgintel.startup_timeout_sec=10",
        "-c",
        "mcp_servers.orgintel.tool_timeout_sec=310",
        "-c",
        "mcp_servers.orgintel.default_tools_approval_mode=\"approve\"",
    ]


def command() -> tuple[list[str], Path, bool]:
    model = required("COORD_MODEL")
    workdir = Path(required("COORD_HOST_WORKDIR"))
    system = Path(required("COORD_HOST_SYSTEM_PATH")).read_text()
    project_state = os.environ.get("COORD_HOST_PROJECT_STATE_PATH", "")
    if project_state:
        system = system.replace("/company/project-state.md", project_state)
    state_path = session_file()
    yolo = os.environ.get("COORD_CODEX_YOLO") == "1"
    common = [
        "--ignore-user-config",
        "--ignore-rules",
        "--skip-git-repo-check",
        "--json",
        "-m",
        model,
        "-c",
        f"model_reasoning_effort={toml_string(os.environ.get('COORD_REASONING_EFFORT', 'low'))}",
        "-c",
        "approval_policy=\"on-request\"",
        "-c",
        "approvals_reviewer=\"auto_review\"",
        "-c",
        f"developer_instructions={toml_string(system)}",
        *mcp_config(),
    ]
    if yolo:
        common.append("--dangerously-bypass-approvals-and-sandbox")
    if state_path.exists():
        thread_id = json.loads(state_path.read_text())["thread_id"]
        return ["codex", "exec", "resume", *common, thread_id, "-"], state_path, True

    sandbox = "read-only" if os.environ.get("COORD_READ_ONLY") == "1" else "workspace-write"
    args = ["codex", "exec", *common, "-C", str(workdir)]
    if not yolo:
        args.extend(["--sandbox", sandbox])
    git_dir = Path(required("COORD_CANONICAL_GIT_DIR"))
    if not yolo and sandbox == "workspace-write" and git_dir.exists() and git_dir not in workdir.parents:
        args.extend(["--add-dir", str(git_dir)])
    extra_write_dir = os.environ.get("COORD_EXTRA_WRITE_DIR", "")
    if not yolo and sandbox == "workspace-write" and extra_write_dir:
        args.extend(["--add-dir", extra_write_dir])
    args.append("-")
    return args, state_path, False


def project_event(event: dict[str, Any], transcript: dict[str, Any]) -> None:
    event_type = event.get("type")
    if event_type == "thread.started":
        transcript["thread_id"] = event.get("thread_id")
        emit("model_session", {"thread_id": event.get("thread_id")})
        return
    if event_type == "item.started":
        item = event.get("item") or {}
        item_type = item.get("type")
        if item_type in {"command_execution", "mcp_tool_call", "file_change"}:
            title = item.get("command") or item.get("tool") or item.get("path") or item_type
            transcript["tool_calls"].append(str(title))
            emit("tool_started", {"id": item.get("id"), "title": title, "tool_kind": item_type})
        return
    if event_type == "item.completed":
        item = event.get("item") or {}
        item_type = item.get("type")
        if item_type == "agent_message":
            text = str(item.get("text") or "")
            transcript["text"] += text
            emit("agent_text", {"text": text})
        elif item_type in {"command_execution", "mcp_tool_call", "file_change"}:
            emit(
                "tool_terminal",
                {
                    "id": item.get("id"),
                    "status": item.get("status") or "completed",
                    "title": item.get("command") or item.get("tool") or item.get("path"),
                },
            )
        return
    if event_type == "turn.completed":
        usage = event.get("usage") or {}
        input_tokens = int(usage.get("input_tokens") or 0)
        output_tokens = int(usage.get("output_tokens") or 0)
        transcript.update(
            {
                "used_tokens": input_tokens + output_tokens,
                "output_tokens": output_tokens,
                "cached_input_tokens": int(usage.get("cached_input_tokens") or 0),
                "reasoning_output_tokens": int(usage.get("reasoning_output_tokens") or 0),
                "stop_reason": "endturn",
            }
        )
        emit(
            "model_usage",
            {
                "used_tokens": input_tokens + output_tokens,
                "input_tokens": input_tokens,
                "output_tokens": output_tokens,
                "cached_input_tokens": transcript["cached_input_tokens"],
                "reasoning_output_tokens": transcript["reasoning_output_tokens"],
                "cost_usd": None,
            },
        )
    elif event_type in {"turn.failed", "error"}:
        error = event.get("error")
        message = event.get("message") or (error.get("message") if isinstance(error, dict) else error)
        transcript["error"] = str(message or json.dumps(event))
        transcript["stop_reason"] = "failed"


def main() -> None:
    prompt = Path(required("COORD_PROMPT_PATH")).read_text()
    workdir = validate_runtime_identity()
    args, state_path, resumed = command()
    transcript: dict[str, Any] = {
        "text": "",
        "tool_calls": [],
        "used_tokens": None,
        "output_tokens": None,
        "cached_input_tokens": None,
        "reasoning_output_tokens": None,
        "cost_usd": None,
        "stop_reason": None,
        "resumed": resumed,
    }
    process = subprocess.Popen(
        args,
        cwd=workdir,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
    )
    assert process.stdin is not None
    assert process.stdout is not None
    process.stdin.write(prompt)
    process.stdin.close()
    for line in process.stdout:
        stripped = line.strip()
        if not stripped:
            continue
        try:
            project_event(json.loads(stripped), transcript)
        except (json.JSONDecodeError, TypeError, ValueError) as exc:
            emit("transport_event_unparsed", {"line": stripped[-1000:], "error": str(exc)})
    stderr = process.stderr.read() if process.stderr is not None else ""
    returncode = process.wait()
    if transcript.get("thread_id"):
        temporary = state_path.with_suffix(".tmp")
        temporary.write_text(json.dumps({"thread_id": transcript["thread_id"]}))
        temporary.replace(state_path)
    if returncode and not transcript.get("error"):
        transcript["error"] = re.sub(r"\s+", " ", stderr).strip()[-4000:]
        transcript["stop_reason"] = "failed"
    if transcript["stop_reason"] is None:
        transcript["stop_reason"] = "unknown"
        transcript["error"] = transcript.get("error") or "Codex process ended without a terminal event"
    transcript["model"] = required("COORD_MODEL")
    transcript["exit_code"] = returncode
    transcript["stderr_tail"] = stderr[-2000:]
    print(json.dumps(transcript, sort_keys=True))


if __name__ == "__main__":
    main()
