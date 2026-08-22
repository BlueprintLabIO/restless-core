#!/usr/bin/env python3
from __future__ import annotations

import json
import os
import socket
import sys
from typing import Any


ENDPOINT = os.environ["COORD_ENDPOINT"]
ACTOR = os.environ["COORD_ACTOR"]
ATTEMPT = os.environ.get("COORD_ATTEMPT", "")
LEASE_TOKEN = os.environ.get("COORD_LEASE_TOKEN", "")


def mutation(required: list[str], properties: dict[str, Any]) -> dict[str, Any]:
    return {
        "type": "object",
        "additionalProperties": False,
        "required": ["idempotency_key", *required],
        "properties": {
            "idempotency_key": {
                "type": "string",
                "minLength": 1,
                "maxLength": 120,
                "description": "Caller-chosen stable key. Reuse it only when retrying this exact logical mutation.",
            },
            **properties,
        },
    }


def schemas() -> list[dict[str, Any]]:
    strings = {"type": "array", "items": {"type": "string"}}
    return [
        {
            "name": "inspect_coordination",
            "description": "Read canonical actors, Work graph, Attempts, messages, judgements, decisions, and artifacts. Running work changes only through callbacks; do not poll it.",
            "inputSchema": {"type": "object", "additionalProperties": False, "properties": {}},
        },
        {
            "name": "send",
            "description": "Append semantic communication for the recipient's next wake. It never assigns or completes Work and is not live chat with a running actor.",
            "inputSchema": mutation(
                ["to", "body"],
                {"to": {"type": "string"}, "body": {"type": "string"}, "refs": strings},
            ),
        },
        {
            "name": "commission",
            "description": "Create one outcome-sized Work responsibility. Producer Work ends at a clean commit/report. Integration Work must require completed producer Work. Independent critic Work must require the completed Work it reviews.",
            "inputSchema": mutation(
                ["owner", "outcome", "expected_artifact"],
                {
                    "owner": {
                        "type": "string",
                        "description": "Exact recipient actor ID from the supplied roster. It must not be the caller's own actor ID.",
                    },
                    "outcome": {"type": "string"},
                    "expected_artifact": {"type": "string"},
                    "requires": strings,
                    "revises": strings,
                    "gates": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "additionalProperties": False,
                            "required": ["name", "argv"],
                            "properties": {
                                "name": {"type": "string"},
                                "argv": {
                                    "type": "array",
                                    "minItems": 1,
                                    "items": {"type": "string"},
                                    "description": "Exact executable followed by separate arguments, for example [\"test\",\"-s\",\"path\"]. Shell syntax requires an explicit [\"sh\",\"-c\",\"...\"].",
                                },
                            },
                        },
                    },
                },
            ),
        },
        {
            "name": "redirect",
            "description": "Repair, reassign, or abandon Work. Against a running lease this requests cancellation; the transition applies only after the Attempt stops.",
            "inputSchema": mutation(
                ["work", "action", "reason"],
                {
                    "work": {"type": "string"},
                    "action": {"enum": ["repair", "reassign", "abandon"]},
                    "reason": {"type": "string"},
                    "feedback": {"type": "string"},
                    "new_owner": {"type": "string"},
                },
            ),
        },
        {
            "name": "report",
            "description": "Append an Attempt callback. progress is nonterminal. outcome_met submits the exact clean HEAD and runs declared gates; a failed gate returns revision_required while keeping this Attempt live so you can repair and resubmit. A passing outcome_met, blocked, or abandoned report is terminal.",
            "inputSchema": mutation(
                ["disposition", "summary"],
                {
                    "disposition": {"enum": ["progress", "outcome_met", "blocked", "abandoned"]},
                    "summary": {"type": "string"},
                    "artifacts": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "additionalProperties": False,
                            "required": ["kind", "reference"],
                            "properties": {"kind": {"type": "string"}, "reference": {"type": "string"}},
                        },
                    },
                    "evidence": strings,
                    "resume_condition": {"type": "string"},
                },
            ),
        },
        {
            "name": "request_judgement",
            "description": "Request a bounded irreducible judgement with an observable resume condition. Reversible operating choices belong in decide.",
            "inputSchema": mutation(
                ["assigned_to", "subject", "question", "resume_condition"],
                {
                    "assigned_to": {"type": "string"},
                    "subject": {"type": "string"},
                    "question": {"type": "string"},
                    "resume_condition": {"type": "string"},
                },
            ),
        },
        {
            "name": "decide",
            "description": "Resolve a judgement or record an accountable choice. An open judgement on the same subject must be explicitly resolved.",
            "inputSchema": mutation(
                ["subject", "choice", "rationale"],
                {
                    "request": {"type": "string"},
                    "subject": {"type": "string"},
                    "choice": {"type": "string"},
                    "rationale": {"type": "string"},
                    "evidence": strings,
                },
            ),
        },
        {
            "name": "schedule",
            "description": "Schedule a genuinely time-driven future wake. Never use it to poll delegated Work.",
            "inputSchema": mutation(
                ["reason", "not_before_unix"],
                {"reason": {"type": "string"}, "not_before_unix": {"type": "number"}},
            ),
        },
    ]


def coordinator(payload: dict[str, Any]) -> dict[str, Any]:
    host, port_text = ENDPOINT.rsplit(":", 1)
    with socket.create_connection((host, int(port_text)), timeout=310) as client:
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


def call(name: str, arguments: dict[str, Any]) -> dict[str, Any]:
    if name == "inspect_coordination":
        return coordinator({"type": "inspect", "actor": ACTOR})
    args = dict(arguments)
    idempotency_key = args.pop("idempotency_key")
    return coordinator(
        {
            "type": "command",
            "actor": ACTOR,
            "attempt": ATTEMPT,
            "lease_token": LEASE_TOKEN,
            "idempotency_key": idempotency_key,
            "name": name,
            "args": args,
        }
    )


def respond(request_id: Any, result: Any = None, error: dict[str, Any] | None = None) -> None:
    payload: dict[str, Any] = {"jsonrpc": "2.0", "id": request_id}
    if error is not None:
        payload["error"] = error
    else:
        payload["result"] = result
    print(json.dumps(payload), flush=True)


def main() -> None:
    tools = schemas()
    for line in sys.stdin:
        try:
            request = json.loads(line)
            method = request.get("method")
            if method == "initialize":
                respond(
                    request["id"],
                    {
                        "protocolVersion": request.get("params", {}).get("protocolVersion", "2024-11-05"),
                        "capabilities": {"tools": {}},
                        "serverInfo": {"name": "orgintel-coordination-v2", "version": "0.2.0"},
                    },
                )
            elif method == "notifications/initialized":
                continue
            elif method == "tools/list":
                respond(request["id"], {"tools": tools})
            elif method == "tools/call":
                params = request["params"]
                result = call(params["name"], params.get("arguments", {}))
                respond(
                    request["id"],
                    {"content": [{"type": "text", "text": json.dumps(result, indent=2, sort_keys=True)}]},
                )
            else:
                respond(request["id"], error={"code": -32601, "message": f"method not found: {method}"})
        except Exception as exc:
            respond(request.get("id") if "request" in locals() else None, error={"code": -32603, "message": str(exc)})


if __name__ == "__main__":
    main()
