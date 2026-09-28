# Short-lived worktrees for agents and founders. Every task starts from a fresh
# origin/dev, lands on dev, and is then removed. Sourced by scripts/restless-dev.

dev_worktree_main_root() {
  local common
  common="$(git -C "$STACK_REPO_ROOT" rev-parse --path-format=absolute --git-common-dir)"
  dirname "$common"
}

dev_worktree_parent() {
  printf '%s\n' "${RESTLESS_WORKTREE_ROOT:-$(dirname "$(dev_worktree_main_root)")/work}"
}

# Commits reachable from <dir>'s HEAD that no remote branch contains.
dev_worktree_unpushed() {
  git -C "$1" rev-list --count HEAD --not --remotes 2>/dev/null || printf '?\n'
}

dev_worktree_in_use() {
  local dir="$1" link
  for link in /proc/[0-9]*/cwd; do
    case "$(readlink "$link" 2>/dev/null)/" in
      "$dir"/*) return 0 ;;
    esac
  done
  return 1
}

dev_worktree_new() {
  local name="${1:-}"
  if ! [[ "$name" =~ ^[a-z0-9][a-z0-9._-]{0,63}$ ]]; then
    printf 'usage: restless-dev worktree new <name>   (lowercase, digits, . _ -)\n' >&2
    return 2
  fi
  local dir branch="${RESTLESS_WORKTREE_BRANCH_PREFIX:-feat/}${name}"
  dir="$(dev_worktree_parent)/${name}"
  git -C "$STACK_REPO_ROOT" fetch --quiet origin dev
  git -C "$STACK_REPO_ROOT" worktree add -b "$branch" "$dir" origin/dev
  printf '\nWORKTREE %s\nBRANCH   %s (from origin/dev %s)\n' \
    "$dir" "$branch" "$(git -C "$dir" rev-parse --short HEAD)"
  printf 'Rebase on origin/dev before verifying and before landing; remove it once landed:\n'
  printf '  restless-dev worktree prune\n'
}

# One line per worktree: age, uncommitted files, commits no remote has.
dev_worktree_stale() {
  local main dir head branch age dirty unpushed now
  main="$(dev_worktree_main_root)"
  now="$(date +%s)"
  git -C "$STACK_REPO_ROOT" fetch --quiet origin
  printf '%-7s %-6s %-9s %s\n' AGE DIRTY UNPUSHED WORKTREE
  while IFS= read -r line; do
    case "$line" in
      "worktree "*) dir="${line#worktree }" ;;
      "branch "*) branch="${line#branch refs/heads/}" ;;
      "HEAD "*) head="${line#HEAD }"; branch="(detached)" ;;
      "")
        if [ -n "${dir:-}" ] && [ "$dir" != "$main" ] && [ -d "$dir" ]; then
          age=$(( (now - $(git -C "$dir" log -1 --format=%ct 2>/dev/null || echo "$now")) / 86400 ))
          dirty="$(git -C "$dir" status --porcelain 2>/dev/null | wc -l | tr -d ' ')"
          unpushed="$(dev_worktree_unpushed "$dir")"
          printf '%-7s %-6s %-9s %s [%s]\n' "${age}d" "$dirty" "$unpushed" "$dir" "$branch"
        fi
        dir="" branch="" head=""
        ;;
    esac
  done < <(git -C "$STACK_REPO_ROOT" worktree list --porcelain; printf '\n')
}

# Remove worktrees whose every commit is on a remote and that have no
# uncommitted change and no process inside. Anything else is reported, never
# removed: unpushed or uncommitted work belongs to whoever made it.
dev_worktree_prune() {
  local main dir branch removed=0 kept=0
  main="$(dev_worktree_main_root)"
  git -C "$STACK_REPO_ROOT" fetch --quiet origin
  git -C "$STACK_REPO_ROOT" worktree prune
  while IFS= read -r line; do
    case "$line" in
      "worktree "*) dir="${line#worktree }" ;;
      "branch "*) branch="${line#branch refs/heads/}" ;;
      "")
        if [ -n "${dir:-}" ] && [ "$dir" != "$main" ] && [ -d "$dir" ]; then
          local reason=""
          if [ -n "$(git -C "$dir" status --porcelain --untracked-files=normal 2>/dev/null | grep -v '^?? .*node_modules/\?$')" ]; then
            reason="uncommitted changes"
          elif [ "$(dev_worktree_unpushed "$dir")" != "0" ]; then
            reason="$(dev_worktree_unpushed "$dir") commit(s) on no remote"
          elif dev_worktree_in_use "$dir"; then
            reason="a process is running inside it"
          fi
          if [ -n "$reason" ]; then
            printf 'KEEP    %s (%s)\n' "$dir" "$reason"
            kept=$((kept + 1))
          else
            git -C "$STACK_REPO_ROOT" worktree remove --force "$dir"
            if [ -n "${branch:-}" ] &&
               git -C "$STACK_REPO_ROOT" merge-base --is-ancestor "$branch" origin/dev 2>/dev/null; then
              git -C "$STACK_REPO_ROOT" branch -q -D "$branch" 2>/dev/null || true
            fi
            printf 'REMOVED %s\n' "$dir"
            removed=$((removed + 1))
          fi
        fi
        dir="" branch=""
        ;;
    esac
  done < <(git -C "$STACK_REPO_ROOT" worktree list --porcelain; printf '\n')
  printf '\n%s removed, %s kept.\n' "$removed" "$kept"
}
