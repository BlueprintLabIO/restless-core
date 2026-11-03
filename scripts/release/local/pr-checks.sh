#!/usr/bin/env bash
# Checks every open pull request on this computer, with no GitHub Actions (ADR 0010 in
# restless-cloud): each new head commit runs the same qualification its release would, and its
# result is posted to GitHub as the `local-ci` commit status, so a PR shows pass or fail before merge.
#   Core:  dagger qualify of the exact commit (scripts/release/local/core-release.sh, step 1);
#   Cloud: scripts/release/local/qualify.sh with an empty environment, as cloud-release.sh runs it.
# Only branches of the BlueprintLabIO repositories are checked, never forks: a fork's code would
# run on this computer. Two commits at a time (two worker slots); a commit is checked once. A Core
# PR runs only the qualify groups its changed paths can affect, and a docs-only change passes
# without qualifying; a release still qualifies everything. Logs go to
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
# Two workers: each timer tick takes a free slot, then keeps claiming unchecked commits until none
# are left, so a new PR starts within a minute even while a long check runs in the other slot.
slot=
for candidate in 1 2; do
  exec 9>"$state/.slot-$candidate.lock"
  flock -n 9 && { slot=$candidate; break; }
done
[ -n "$slot" ] || exit 0
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH" CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-6}"
log() { printf '%s %s\n' "$(date -u +%FT%TZ)" "[$slot] $*" >>"$state/pr-checks.log"; }

# Core qualify groups a set of changed paths can affect. Prints nothing when no qualified input
# changed (docs only), and "all" when a path is unknown or touches the checks themselves.
core_groups() {
  local groups=() path
  while read -r path; do
    [ -n "$path" ] || continue
    case "$path" in
      .dagger/*|scripts/release/*|infra/*|contracts/*|release/*|LICENSE|docs/COMPANY_OPERATING_RULES.md) echo all; return ;;
      crates/*|Cargo.toml|Cargo.lock|rust-toolchain*) groups+=(rust release) ;;
      web/*) groups+=(web issuer) ;;
      services/identity/*) groups+=(issuer web) ;;
      services/native-documents-collaboration/*) groups+=(documents) ;;
      services/native-sheets/*) groups+=(sheets release) ;;
      docs/*|*.md|.github/*|.gitignore) ;;
      *) echo all; return ;;
    esac
  done
  [ ${#groups[@]} -gt 0 ] && printf '%s\n' "${groups[@]}" | sort -u | paste -sd, -
  return 0
}

status() { # repo sha state description
  gh api -X POST "repos/$1/statuses/$2" -f state="$3" -f context=local-ci -f description="$4" >/dev/null 2>&1 \
    || log "could not post $3 for $1@${2:0:12}"
}

check() { # repo checkout number sha
  local repo="$1" checkout="$2" number="$3" sha="$4" name tree result
  name="$(basename "$repo")-$sha"
  [ -f "$state/$name.result" ] && return 0
  mkdir "$state/$name.claim" 2>/dev/null || return 0   # the other slot has it
  tree="$state/src-$(basename "$repo")-$slot"
  log "checking $repo#$number at ${sha:0:12}"
  status "$repo" "$sha" pending "Qualifying on the owner's computer"
  git -C "$checkout" fetch -q origin "pull/$number/head" || { log "fetch failed for $repo#$number"; return 0; }
  git -C "$checkout" worktree remove --force "$tree" >/dev/null 2>&1 || true
  git -C "$checkout" worktree add -q --detach "$tree" "$sha" || { log "worktree failed for $repo#$number"; return 0; }
  case "$repo" in
    */restless-core)
      git -C "$checkout" fetch -q origin main
      local groups
      groups="$(git -C "$tree" diff --name-only "$(git -C "$tree" merge-base HEAD origin/main)" HEAD | core_groups)"
      if [ -z "$groups" ]; then
        echo "no qualified input changed (docs only)" >"$state/$name.log"
      elif [ "$groups" = all ]; then
        (cd "$tree" && timeout 3h "$dagger" --silent call qualify --source=.) >"$state/$name.log" 2>&1
      else
        log "$repo#$number qualifying $groups"
        (cd "$tree" && timeout 3h "$dagger" --silent call qualify --source=. --only="$groups") >"$state/$name.log" 2>&1
      fi ;;
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
  rmdir "$state/$name.claim" 2>/dev/null || true
  log "$repo#$number ${sha:0:12}: $(cat "$state/$name.result")"
}

for pair in "BlueprintLabIO/restless-core:$core" "BlueprintLabIO/restless-cloud:$cloud"; do
  repo="${pair%%:*}" checkout="${pair#*:}"
  gh pr list -R "$repo" --state open --limit 50 --json number,headRefOid,isCrossRepository \
    -q '.[] | select(.isCrossRepository | not) | "\(.number) \(.headRefOid)"' 2>/dev/null |
  # PR numbers listed in $state/priority (one per line) are checked first, e.g. a release's blockers.
  awk -v pf="$state/priority" 'BEGIN { while ((getline n < pf) > 0) p[n] = 1 } { print (($1 in p) ? 0 : 1), $0 }' |
  sort -s -k1,1n | cut -d' ' -f2- |
  while read -r number sha; do
    [ -n "$sha" ] && check "$repo" "$checkout" "$number" "$sha"
  done
done
