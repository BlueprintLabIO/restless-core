#!/usr/bin/env bash
# List useful tools actually present on this company computer. This stays a
# local observation so agents can inspect their workstation without a daemon
# endpoint or a separate inventory service. It resolves paths only: discovery
# must not invoke an agent, open a GUI, or block on an arbitrary --version.
set -euo pipefail

json=false
case "${1:-}" in
  "") ;;
  --json) json=true ;;
  -h|--help)
    cat <<'USAGE'
Usage: company-tools [--json]

Lists named tools installed in the company Runtime, with their resolved path
and purpose. It does not execute the listed tools. Use --json for the same
observed inventory in machine-readable form.
USAGE
    exit 0
    ;;
  *)
    echo "Usage: company-tools [--json]" >&2
    exit 2
    ;;
esac

tools=(
  restless restless-runtime-bridge
  codex omp claude-agent-acp
  node npm pnpm bun
  python3 git curl jq rg
  resend mcp-remote playwright godot chromium socat
  company-supervisorctl supervisorctl
  wmctrl xdotool scrot
  restless-scenario restless-web-review
  focus-company-chromium start-company-chromium start-company-godot
  start-company-terminal start-company-desktop-panel wait-for-company-display
)

purpose_for() {
  local tool="$1"
  case "$tool" in
    restless) echo "company coordination CLI" ;;
    restless-runtime-bridge) echo "Runtime Bridge process" ;;
    codex|omp|claude-agent-acp) echo "agent harness" ;;
    node|npm|pnpm|bun) echo "JavaScript runtime or package tool" ;;
    python3) echo "Python runtime" ;;
    git) echo "source control" ;;
    curl|jq|rg) echo "network, JSON, or text utility" ;;
    resend) echo "Resend email CLI" ;;
    mcp-remote) echo "remote MCP client" ;;
    playwright) echo "browser automation library CLI" ;;
    godot) echo "game editor and exporter" ;;
    chromium) echo "company browser" ;;
    socat) echo "private TCP bridge utility" ;;
    company-supervisorctl|supervisorctl) echo "company service control" ;;
    wmctrl|xdotool|scrot) echo "desktop observation or input utility" ;;
    restless-scenario) echo "scenario review helper" ;;
    restless-web-review) echo "web review helper" ;;
    focus-company-chromium) echo "focus existing company browser" ;;
    start-company-chromium) echo "start company browser" ;;
    start-company-godot) echo "start company game editor" ;;
    start-company-terminal) echo "start company terminal" ;;
    start-company-desktop-panel) echo "start company desktop panel" ;;
    wait-for-company-display) echo "wait for shared display" ;;
  esac
}

if "$json"; then
  printf '['
fi

first=true
for tool in "${tools[@]}"; do
  path="$(command -v "$tool" 2>/dev/null || true)"
  [ -n "$path" ] || continue
  purpose="$(purpose_for "$tool")"
  if "$json"; then
    "$first" || printf ','
    first=false
    jq -cn --arg name "$tool" --arg path "$path" --arg purpose "$purpose" \
      '{name: $name, path: $path, purpose: $purpose}'
  else
    printf '%-28s %s (%s)\n' "$tool" "$path" "$purpose"
  fi
done

if "$json"; then
  printf ']\n'
fi
