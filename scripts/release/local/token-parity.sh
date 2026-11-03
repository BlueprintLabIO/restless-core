#!/usr/bin/env bash
# Token parity on a commit of main (non-blocking): runs scripts/bench/token-parity on the exact
# tree, appends the result to a history file and posts it as the `token-parity` status. The status
# is always success when the benchmark ran (the ratios are in its description and the history), so
# it tracks the ratio without ever blocking a release; it is failure only when the benchmark could
# not run. A tree already measured is not measured again.
#
#   scripts/release/local/token-parity.sh <sha>   (from post-merge-gates.sh, in the gated tree)
set -uo pipefail
root="$(cd "$(dirname "$0")/../../.." && pwd)"
sha="${1:-$(git -C "$root" rev-parse HEAD)}"
state="${XDG_STATE_HOME:-$HOME/.local/state}/restless-release/token-parity"
mkdir -p "$state"
tree="$(git -C "$root" rev-parse 'HEAD^{tree}')"
status() { # state description
  gh api -X POST "repos/BlueprintLabIO/restless-core/statuses/$sha" -f state="$1" -f context=token-parity \
    -f description="$2" >/dev/null 2>&1 || true
}
if [ -f "$state/$tree.json" ]; then
  echo "token parity: tree ${tree:0:12} already measured"; exit 0
fi
status pending "Token parity running"
out="$state/$tree.json"
if RESTLESS_DEV_TARGET_DIR="${RESTLESS_DEV_TARGET_DIR:-$HOME/.cache/restless-release/token-parity-target}" BENCH_COMPANY="tokgate_test" BENCH_MOCK_PORT="${BENCH_MOCK_PORT:-18978}" \
    "$root/scripts/bench/token-parity/bench.sh" "$root" "$root/scripts/bench/token-parity/restless.json" "$out"; then
  summary="$(node -e '
    const r = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8"));
    console.log(r.tasks.map((t) => `${t.task.split("_")[0]} ${t.ratio.input_weighted}x`).join(", "));
  ' "$out")"
  node -e '
    const r = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8"));
    console.log(JSON.stringify({ sha: process.argv[2], at: r.at, ratios: Object.fromEntries(r.tasks.map((t) => [t.task, t.ratio])) }));
  ' "$out" "$sha" >>"$state/history.jsonl"
  status success "Restless/direct weighted input: $summary"
  echo "token parity: $summary"
else
  status failure "Token parity could not run (log on the owner's computer)"
  exit 1
fi
