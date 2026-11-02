#!/usr/bin/env bash
# Checks every open pull request on this computer, with no GitHub Actions (ADR 0010 in
# restless-cloud): each new head commit runs the same qualification its release would, and its
# result is posted to GitHub as the `local-ci` commit status, so a PR shows pass or fail before merge.
#   Core:  dagger qualify of the exact commit (scripts/release/local/core-release.sh, step 1);
#   Cloud: scripts/release/local/qualify.sh with an empty environment, as cloud-release.sh runs it.
# Only branches of the BlueprintLabIO repositories are checked, never forks: a fork's code would
# run on this computer. One commit at a time; a commit is checked once. Logs go to
# ~/.local/state/restless-release/pr-checks/.
#
#   scripts/release/local/pr-checks.sh          (scheduled by restless-pr-checks.timer)
set -uo pipefail
core="$(cd "$(dirname "$0")/../../.." && pwd)"
# The main Cloud clone, not a work worktree: worktrees get cleaned up.
cloud="${RESTLESS_CLOUD_DIR:-$HOME/code/restless-cloud}"
dagger="${DAGGER_BIN:-$HOME/.local/bin/dagger}"
state="${XDG_STATE_HOME:-$HOME/.local/state}/restless-release/pr-checks"
mkdir -p "$state"
exec 9>"$state/.lock"
flock -n 9 || exit 0
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH" CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-6}"
log() { printf '%s %s\n' "$(date -u +%FT%TZ)" "$*" >>"$state/pr-checks.log"; }

status() { # repo sha state description
  gh api -X POST "repos/$1/statuses/$2" -f state="$3" -f context=local-ci -f description="$4" >/dev/null 2>&1 \
    || log "could not post $3 for $1@${2:0:12}"
}

check() { # repo checkout number sha
  local repo="$1" checkout="$2" number="$3" sha="$4" name tree result
  name="$(basename "$repo")-$sha"
  [ -f "$state/$name.result" ] && return 0
  tree="$state/src-$(basename "$repo")"
  log "checking $repo#$number at ${sha:0:12}"
  status "$repo" "$sha" pending "Qualifying on the owner's computer"
  git -C "$checkout" fetch -q origin "pull/$number/head" || { log "fetch failed for $repo#$number"; return 0; }
  git -C "$checkout" worktree remove --force "$tree" >/dev/null 2>&1 || true
  git -C "$checkout" worktree add -q --detach "$tree" "$sha" || { log "worktree failed for $repo#$number"; return 0; }
  case "$repo" in
    */restless-core)
      (cd "$tree" && timeout 3h "$dagger" --silent call qualify --source=.) >"$state/$name.log" 2>&1 ;;
    */restless-cloud)
      (cd "$tree" && timeout 2h env -i HOME="$HOME" USER="${USER:-}" PATH="$PATH" LANG="${LANG:-C.UTF-8}" \
        TERM=dumb scripts/release/local/qualify.sh) >"$state/$name.log" 2>&1 ;;
  esac
  result=$?
  git -C "$checkout" worktree remove --force "$tree" >/dev/null 2>&1 || true
  if [ "$result" = 0 ]; then
    echo pass >"$state/$name.result"; status "$repo" "$sha" success "Qualified"
  else
    echo "fail $result" >"$state/$name.result"; status "$repo" "$sha" failure "Qualification failed (exit $result); log on the owner's computer"
  fi
  log "$repo#$number ${sha:0:12}: $(cat "$state/$name.result")"
}

for pair in "BlueprintLabIO/restless-core:$core" "BlueprintLabIO/restless-cloud:$cloud"; do
  repo="${pair%%:*}" checkout="${pair#*:}"
  gh pr list -R "$repo" --state open --limit 50 --json number,headRefOid,isCrossRepository \
    -q '.[] | select(.isCrossRepository | not) | "\(.number) \(.headRefOid)"' 2>/dev/null |
  while read -r number sha; do
    [ -n "$sha" ] && check "$repo" "$checkout" "$number" "$sha"
  done
done
