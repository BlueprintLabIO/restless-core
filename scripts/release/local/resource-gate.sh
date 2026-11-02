#!/usr/bin/env bash
# The constrained-host resource gate (Sprint 109 A0.6) on the exact release candidate. One account
# pool sized like the production plan (8 GB / 4 CPU, installed by Cloud's own VM agent function),
# several `_test` companies and a scripted model: two conversations at once in a 768 MiB Runtime,
# every company woken at once from sleep, a quiet window with zero model calls, and a daemon
# restart. Owner entry (health, principal, Inbox, Work) is probed throughout; any failed entry
# read, unanswered conversation, idle model call or pool OOM kill fails the release
# (scripts/verify-constrained-host.py). Runs after the connections smoke, reusing its restlessd
# and company image. Evidence is kept under ~/.cache/restless-release/resources/<revision>.
#
# Needs passwordless sudo for one transient systemd slice, and Cloud's VM agent script:
# RESTLESS_CLOUD_AGENT, else $RESTLESS_CLOUD_DIR/services/vm-provider/agent/restless-agent.sh.
# Fast lane: a tree that already passed on this computer is not measured again.
set -euo pipefail
root="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$root"
state="${XDG_CACHE_HOME:-$HOME/.cache}/restless-release"
passed="$state/resources-passed"
tree="$(git rev-parse 'HEAD^{tree}')"
if [ -f "$passed/$tree" ]; then
  printf 'resource gate skipped: this tree (%s) passed at %s\n' "${tree:0:12}" "$(cat "$passed/$tree")"
  exit 0
fi
revision="$(git rev-parse HEAD)"
agent="${RESTLESS_CLOUD_AGENT:-${RESTLESS_CLOUD_DIR:-$HOME/code/restless-cloud}/services/vm-provider/agent/restless-agent.sh}"
[ -f "$agent" ] || { echo "resource gate: Cloud's VM agent script is not at $agent; set RESTLESS_CLOUD_AGENT" >&2; exit 1; }
sudo -n true 2>/dev/null || { echo "resource gate: needs passwordless sudo for its transient slice" >&2; exit 1; }
export PATH="$HOME/.cargo/bin:$PATH" CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-8}"
# The connections smoke's build of this checkout: restlessd is already compiled there.
export CARGO_TARGET_DIR="$state/connections-target"
scripts/restless-cargo build -p restlessd
[ -d infra/host-tools/node_modules ] || npm --prefix infra/host-tools ci --no-audit --no-fund >/dev/null
# A private copy: a concurrent build sharing the target may replace the binary mid-run.
copy="$(mktemp -d)"
trap 'rm -rf "$copy"' EXIT
cp "$CARGO_TARGET_DIR/debug/restlessd" "$copy/restlessd"
evidence="$state/resources/$revision-$(date -u +%H%M%S)"
mkdir -p "$evidence"
RESTLESSD_BIN="$copy/restlessd" RESTLESS_CLOUD_AGENT="$agent" \
  python3 scripts/verify-constrained-host.py --image restless-company-image:release-connections \
  --output "$evidence" >"$evidence/run.log" 2>&1 \
  || { echo "resource gate failed; evidence in $evidence (evidence.json, daemon.log, run.log)" >&2; exit 1; }
mkdir -p "$passed" && printf '%s\n' "$revision" >"$passed/$tree"
find "$passed" -type f -printf '%T@ %p\n' | sort -rn | tail -n +41 | cut -d' ' -f2- | xargs -r rm -f
find "$state/resources" -mindepth 1 -maxdepth 1 -type d -printf '%T@ %p\n' | sort -rn | tail -n +11 | cut -d' ' -f2- | xargs -r rm -rf
echo "resource gate passed; evidence in $evidence"
