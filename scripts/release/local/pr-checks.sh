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
# Two workers run side by side (below). Each claims one unchecked commit at a time and lists the
# open PRs again after every check, so a short check never waits behind a long one.
slot=
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
      crates/*|Cargo.toml|Cargo.lock|rust-toolchain*) groups+=(rust engine release) ;;
      # The engine tests read the generated web contract and the identity corpora.
      web/src/lib/model/generated/*|web/src/lib/model/fixtures/*) groups+=(web issuer engine) ;;
      docs/dogfood/company-identity/*) groups+=(engine) ;;
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

# The groups of a comma list that this tree's qualify declares (its QUALIFY_GROUPS line); all of
# them for a tree with no such line, which qualifies everything below anyway.
known_groups() { # tree groups
  local declared group kept=()
  declared="$(grep -m1 '^const QUALIFY_GROUPS' "$1/.dagger/src/index.ts")" || { echo "$2"; return 0; }
  for group in ${2//,/ }; do
    case "$declared" in *"'$group'"*) kept+=("$group") ;; esac
  done
  [ ${#kept[@]} -gt 0 ] && (IFS=,; echo "${kept[*]}")
  return 0
}

# Runs the tree's own Core qualify. Each worker slot has its own Cargo target volumes: Cargo trusts
# mtimes, so a slot sharing a target with another commit tests that commit's code. A branch from
# before --cache-slot has only the one shared volume, so those branches take turns.
qualify_core() { # tree [qualify arguments]
  local tree="$1"; shift
  cd "$tree" || return 1
  if grep -q "cacheSlot: string" .dagger/src/index.ts; then
    timeout 3h "$dagger" --silent call qualify --source=. --cache-slot="pr-$slot" "$@"
  else
    timeout 3h flock "$state/.shared-target.lock" "$dagger" --silent call qualify --source=. "$@"
  fi
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
  echo "$slot" >"$state/$name.claim/slot"
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
      # A branch from before a group existed cannot run it; it runs the groups its qualify knows.
      [ -z "$groups" ] || [ "$groups" = all ] || groups="$(known_groups "$tree" "$groups")"
      if [ -z "$groups" ]; then
        echo "no qualified input changed (docs only)" >"$state/$name.log"
      elif [ "$groups" = all ] || ! grep -q "only: string = ''" "$tree/.dagger/src/index.ts"; then
        # The PR's own qualify runs; one from before scoped groups runs everything.
        (qualify_core "$tree") >"$state/$name.log" 2>&1
      else
        log "$repo#$number qualifying $groups"
        (qualify_core "$tree" --only="$groups") >"$state/$name.log" 2>&1
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
  rm -rf "$state/$name.claim"
  log "$repo#$number ${sha:0:12}: $(cat "$state/$name.result")"
}

# Open PR heads as "repo checkout number sha", priority PRs first.
candidates() {
  local pair repo checkout
  for pair in "BlueprintLabIO/restless-core:$core" "BlueprintLabIO/restless-cloud:$cloud"; do
    repo="${pair%%:*}" checkout="${pair#*:}"
    gh pr list -R "$repo" --state open --limit 50 --json number,headRefOid,isCrossRepository \
      -q '.[] | select(.isCrossRepository | not) | "\(.number) \(.headRefOid)"' 2>/dev/null |
      sed "s|^|$repo $checkout |"
  done |
  # PR numbers listed in $state/priority (one per line) are checked first, e.g. a release's blockers.
  awk -v pf="$state/priority" 'BEGIN { while ((getline n < pf) > 0) p[n] = 1 } { print (($3 in p) ? 0 : 1), $0 }' |
  sort -s -k1,1n | cut -d' ' -f2-
}

worker() {
  slot="$1"
  exec 9>"$state/.slot-$slot.lock"
  flock -n 9 || return 0
  # Claims this slot left behind when a run was killed.
  grep -lx "$slot" "$state"/*.claim/slot 2>/dev/null | xargs -r -n1 dirname | xargs -r rm -rf
  local repo checkout number sha found
  while :; do
    found=
    while read -r repo checkout number sha; do
      [ -n "$sha" ] || continue
      [ -f "$state/$(basename "$repo")-$sha.result" ] && continue
      [ -d "$state/$(basename "$repo")-$sha.claim" ] && continue
      found=1; check "$repo" "$checkout" "$number" "$sha"; break
    done < <(candidates)
    [ -n "$found" ] || break
  done
}

worker 1 &
worker 2 &
wait
