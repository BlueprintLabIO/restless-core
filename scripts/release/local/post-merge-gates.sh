#!/usr/bin/env bash
# Runs the slow release gates on each new commit of main, in the background, so they never block a
# release: connections smoke, zero-jank and constrained-host resources (each skips a tree it has
# already passed). The result is posted to GitHub as the `post-merge-gates` status on that commit,
# and a failure is logged with the gate's evidence path for a fix. Logs go to
# ~/.local/state/restless-release/post-merge-gates/.
#
#   scripts/release/local/post-merge-gates.sh   (scheduled by restless-post-merge-gates.timer)
set -uo pipefail
core="$(cd "$(dirname "$0")/../../.." && pwd)"
state="${XDG_STATE_HOME:-$HOME/.local/state}/restless-release/post-merge-gates"
mkdir -p "$state"
exec 9>"$state/.lock"
flock -n 9 || exit 0
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH" CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"
log() { printf '%s %s\n' "$(date -u +%FT%TZ)" "$*" >>"$state/gates.log"; }
status() { # sha state description
  gh api -X POST "repos/BlueprintLabIO/restless-core/statuses/$1" -f state="$2" -f context=post-merge-gates \
    -f description="$3" >/dev/null 2>&1 || log "could not post $2 for ${1:0:12}"
}

git -C "$core" fetch -q origin main || exit 0
sha="$(git -C "$core" rev-parse origin/main)"
[ -f "$state/$sha.result" ] && exit 0
tree="$state/src"
git -C "$core" worktree remove --force "$tree" >/dev/null 2>&1 || true
git -C "$core" worktree add -q --detach "$tree" "$sha" || { log "worktree failed for ${sha:0:12}"; exit 0; }
log "gating ${sha:0:12}"
status "$sha" pending "Smoke, jank and resource gates running"
failed=
for gate in connections-smoke jank-gate resource-gate; do
  start=$(date +%s)
  if (cd "$tree" && env -i HOME="$HOME" USER="${USER:-}" PATH="$PATH" LANG="${LANG:-C.UTF-8}" TERM=dumb \
      RESTLESS_CLOUD_DIR="${RESTLESS_CLOUD_DIR:-$HOME/code/restless-cloud}" CARGO_BUILD_JOBS="$CARGO_BUILD_JOBS" \
      timeout 2h "scripts/release/local/$gate.sh") >"$state/$sha-$gate.log" 2>&1; then
    log "${sha:0:12} $gate pass ($(( $(date +%s) - start ))s)"
  else
    log "${sha:0:12} $gate FAIL ($(( $(date +%s) - start ))s), log $state/$sha-$gate.log"
    failed="$failed $gate"
  fi
done
git -C "$core" worktree remove --force "$tree" >/dev/null 2>&1 || true
if [ -z "$failed" ]; then
  echo pass >"$state/$sha.result"; status "$sha" success "Smoke, jank and resource gates passed"
else
  echo "fail$failed" >"$state/$sha.result"; status "$sha" failure "Failed:$failed (log on the owner's computer)"
fi
