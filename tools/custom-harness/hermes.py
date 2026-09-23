#!/usr/bin/env python3
"""Scope the native Hermes ACP runtime to the host's Restless launch contract."""
import inspect
import sys
from pathlib import Path

package, prompt_file = map(Path, sys.argv[1:3])
context = prompt_file.read_text()
if not context.strip():
    raise RuntimeError("Restless system instructions are required")
sys.path.insert(0, str(package))

from acp_adapter import entry

entry._load_env()
import run_agent
from toolsets import create_custom_toolset

# Use native tool registration with an exact list: the broad browser bundle also exposes
# Hermes's separate credential vault, which does not belong in the Restless actor contract.
create_custom_toolset("restless", "Restless runtime tools", tools=[
    "terminal", "process_manage", "read_file", "write_file", "patch", "search_files",
    "web_search", "web_extract", "browser_exec",
])

NativeAgent = run_agent.AIAgent
required = {"skip_context_files", "load_soul_identity", "skip_memory", "skip_background_review", "enabled_toolsets"}
if not required.issubset(inspect.signature(NativeAgent).parameters):
    raise RuntimeError("This Hermes version does not expose the required host launch controls")


class RestlessAgent(NativeAgent):
    def __init__(self, *args, **kwargs):
        # Native provider resolution/OAuth stays in Hermes's SessionManager. Tool and identity policy is Core's.
        kwargs.update(
            enabled_toolsets=["restless"],
            disabled_toolsets=["delegate", "memory", "todo", "skills", "session_search"],
            skip_context_files=True,
            load_soul_identity=False,
            skip_memory=True,
            skip_background_review=True,
            fallback_model=None,
            session_db=None,
        )
        super().__init__(*args, **kwargs)

    def _build_system_prompt(self, system_message=None):
        # Called again after native compaction/model rebuilds; the standing host identity must survive both.
        self._cached_system_prompt_static = context
        return context


# SessionManager imports this factory for initial sessions and model changes. This replacement is
# confined to this process; installed Hermes files and its independent credential profile stay intact.
run_agent.AIAgent = RestlessAgent
entry.main([])
