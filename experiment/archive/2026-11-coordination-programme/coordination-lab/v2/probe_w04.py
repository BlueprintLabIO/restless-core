#!/usr/bin/env python3
"""Prove whether W04 is a distinct manipulation in the v24 actor harness."""

from __future__ import annotations

import json
from pathlib import Path


HERE = Path(__file__).resolve().parent
RUNNER = (HERE / "runner.py").read_text()
MCP = (HERE / "mcp_server.py").read_text()
TURN_HOST = (HERE.parent / "src" / "main.rs").read_text()


checks = {
    "b0_has_zero_staff": "WORKER_MODES = {MODE_GRAPH, MODE_ARTIFACT, MODE_TEAM, MODE_CRITIC}" in RUNNER
    and 'MODE_LEAD: "studio-lead"' in RUNNER,
    "b0_has_process_tool": '"read,bash,edit,write,grep"' in TURN_HOST,
    "nested_models_are_blocked": "forbidden_nested_model" in TURN_HOST
    and 'for command in ("omp", "claude", "codex")' in RUNNER,
    "commission_is_autonomous_work": '"name": "commission"' in MCP
    and "Create one outcome-sized Work responsibility" in MCP,
    "no_bounded_executor_tool": '"name": "dispatch_hand"' not in MCP
    and '"name": "dispatch_operation"' not in MCP
    and '"name": "await_operation"' not in MCP,
}

result = {
    "mechanism": "W04 one brain, many hands",
    "checks": checks,
    "passed": all(checks.values()),
    "finding": (
        "The current B0 already owns ordinary bash/process concurrency. The only cross-actor dispatch "
        "creates autonomous Work, while nested model execution is blocked. There is no distinct bounded "
        "executor capability to manipulate; a shell-background wrapper would be B0 under another name."
    ),
    "disposition": "blocked_unidentifiable" if all(checks.values()) else "probe_failed",
}
print(json.dumps(result, indent=2, sort_keys=True))
raise SystemExit(0 if result["passed"] else 1)
