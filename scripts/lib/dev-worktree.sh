# Worktrees for agents and founders. Every task starts from a fresh origin/main
# and lands on main. Sourced by scripts/restless-dev.
#
# Task worktrees are pooled: `new` reuses an idle slot (work/slot-N) by
# switching it to a new branch, instead of creating a fresh directory. The
# slot's path never changes, so its Rust incremental cache, target-dev and
# node_modules stay valid and a new task rebuilds only what changed since the
# slot's last task (a fresh directory costs a ~3 minute cold build).
#
# `new` claims a slot; `prune` releases it once the task's commits are on
# origin/main. A claimed slot is never reused while the claim is younger than
# a day, even when it looks clean: its agent may simply be between turns.

DEV_WORKTREE_CLAIM_SECONDS=86400

dev_worktree_main_root() {
  local common
  common="$(git -C "$STACK_REPO_ROOT" rev-parse --path-format=absolute --git-common-dir)"
  dirname "$common"
}

dev_worktree_parent() {
  printf '%s\n' "${RESTLESS_WORKTREE_ROOT:-$(dirname "$(dev_worktree_main_root)")/work}"
}

dev_worktree_pool_size() {
  printf '%s\n' "${RESTLESS_WORKTREE_POOL_SIZE:-4}"
}

# Commits reachable from <dir>'s HEAD that no remote branch contains.
dev_worktree_unpushed() {
  git -C "$1" rev-list --count HEAD --not --remotes 2>/dev/null || printf '?\n'
}

# Another process with its working directory inside <dir>. The caller and its
# ancestors are not counted, so an agent can release the slot it stands in.
dev_worktree_in_use() {
  local dir="$1" link pid own=" " ancestor="$$"
  while [ -n "$ancestor" ] && [ "$ancestor" -gt 1 ] 2>/dev/null; do
    own="${own}${ancestor} "
    ancestor="$(awk '{print $4}' "/proc/${ancestor}/stat" 2>/dev/null || true)"
  done
  for link in /proc/[0-9]*/cwd; do
    pid="${link#/proc/}"
    pid="${pid%/cwd}"
    case "$own" in *" ${pid} "*) continue ;; esac
    case "$(readlink "$link" 2>/dev/null)/" in
      "$dir"/*) return 0 ;;
    esac
  done
  return 1
}

# Why <dir> must be left alone, or nothing when it holds no one's work.
dev_worktree_busy_reason() {
  local dir="$1"
  if [ -n "$(git -C "$dir" status --porcelain --untracked-files=normal 2>/dev/null | grep -v '^?? .*node_modules/\?$')" ]; then
    printf 'uncommitted changes\n'
  elif [ "$(dev_worktree_unpushed "$dir")" != "0" ]; then
    printf '%s commit(s) on no remote\n' "$(dev_worktree_unpushed "$dir")"
  elif dev_worktree_in_use "$dir"; then
    printf 'a process is running inside it\n'
  fi
}

# The claim lives in the worktree's Git metadata, not its working tree, so it
# never shows up as an untracked file. One line: task, claim time, start commit.
dev_worktree_claim_file() {
  printf '%s/restless-claim\n' "$(git -C "$1" rev-parse --path-format=absolute --git-dir)"
}

dev_worktree_read_claim() {
  local file
  file="$(dev_worktree_claim_file "$1")"
  [ -f "$file" ] && head -n 1 "$file"
}

# The claiming task's name, or nothing when unclaimed or the claim expired.
dev_worktree_claim() {
  local name since
  IFS=$'\t' read -r name since _ <<< "$(dev_worktree_read_claim "$1")" || true
  if [ -n "${since:-}" ] && [ $(( $(date +%s) - since )) -lt "$DEV_WORKTREE_CLAIM_SECONDS" ]; then
    printf '%s\n' "$name"
  fi
}

# The claiming task has landed: its branch made commits of its own and all of
# them are now on origin/main. A branch with none may have only just started.
dev_worktree_claim_landed() {
  local dir="$1" base head
  IFS=$'\t' read -r _ _ base <<< "$(dev_worktree_read_claim "$dir")" || true
  head="$(git -C "$dir" rev-parse HEAD 2>/dev/null)" || return 1
  [ -n "${base:-}" ] && [ "$head" != "$base" ] &&
    git -C "$dir" merge-base --is-ancestor "$head" origin/main 2>/dev/null
}

# Cargo keeps one incremental cache per crate variant (build, test harness,
# check, each feature set) and never collects old ones, so a long-lived slot
# grows without bound: one reached 42 GB in a day. Keep each crate's three most
# recently used caches, which are what a warm rebuild reads. All regenerable.
dev_worktree_trim_cache() {
  local dir="$1" incremental crate before after
  before="$(du -sk "$dir" 2>/dev/null | cut -f1)"
  for incremental in "$dir/target/debug/incremental" "$dir/target-dev/debug/incremental"; do
    [ -d "$incremental" ] || continue
    # ls -t is portable (macOS has no find -printf); names are crate_hash.
    for crate in $(ls -1 "$incremental" | sed -E 's/-[^-]+$//' | sort -u); do
      ls -1dt "$incremental/$crate"-*/ 2>/dev/null | tail -n +4 \
        | while IFS= read -r stale; do rm -rf -- "${stale%/}"; done
    done
  done
  after="$(du -sk "$dir" 2>/dev/null | cut -f1)"
  printf '%s\n' "$(( (before - after) / 1024 ))"
}

dev_worktree_slots() {
  local parent
  parent="$(dev_worktree_parent)"
  git -C "$STACK_REPO_ROOT" worktree list --porcelain \
    | sed -n "s#^worktree \(${parent}/slot-[0-9][0-9]*\)\$#\1#p" \
    | sort -t- -k2 -n
}

dev_worktree_new() {
  local name="${1:-}"
  if ! [[ "$name" =~ ^[a-z0-9][a-z0-9._-]{0,63}$ ]]; then
    printf 'usage: restless-dev worktree new <name>   (lowercase, digits, . _ -)\n' >&2
    return 2
  fi
  local dir="" branch="${RESTLESS_WORKTREE_BRANCH_PREFIX:-feat/}${name}" slot previous reused=0
  git -C "$STACK_REPO_ROOT" fetch --quiet origin main
  if git -C "$STACK_REPO_ROOT" show-ref --verify --quiet "refs/heads/${branch}"; then
    printf 'Branch %s already exists; pick another name or finish that task first.\n' "$branch" >&2
    return 1
  fi

  while IFS= read -r slot; do
    [ -n "$slot" ] && [ -d "$slot" ] || continue
    # Free when unclaimed or the claimed task has landed, as prune would say.
    if [ -n "$(dev_worktree_claim "$slot")" ] && ! dev_worktree_claim_landed "$slot"; then
      continue
    fi
    [ -z "$(dev_worktree_busy_reason "$slot")" ] || continue
    dir="$slot"
    break
  done < <(dev_worktree_slots)

  if [ -n "$dir" ]; then
    previous="$(git -C "$dir" symbolic-ref --quiet --short HEAD 2>/dev/null || true)"
    git -C "$dir" switch --quiet -c "$branch" origin/main
    # The previous task's branch has left the slot; drop it once it has landed.
    if [ -n "$previous" ] && git -C "$dir" merge-base --is-ancestor "$previous" origin/main 2>/dev/null; then
      git -C "$dir" branch -q -D "$previous" 2>/dev/null || true
    fi
    reused=1
  else
    local n=1
    while [ -e "$(dev_worktree_parent)/slot-${n}" ]; do n=$((n + 1)); done
    dir="$(dev_worktree_parent)/slot-${n}"
    git -C "$STACK_REPO_ROOT" worktree add --quiet -b "$branch" "$dir" origin/main
  fi
  printf '%s\t%s\t%s\n' "$name" "$(date +%s)" "$(git -C "$dir" rev-parse HEAD)" \
    > "$(dev_worktree_claim_file "$dir")"
  # A warm slot's target holds tens of GB; keep macOS Spotlight out of it.
  mkdir -p "$dir/target" && touch "$dir/target/.metadata_never_index"

  if [ "$reused" = 1 ]; then
    printf '\nWORKTREE %s (reused: its build cache is warm)\n' "$dir"
  else
    printf '\nWORKTREE %s (new slot: its first build is cold)\n' "$dir"
  fi
  printf 'BRANCH   %s (from origin/main %s)\n' "$branch" "$(git -C "$dir" rev-parse --short HEAD)"
  printf 'Rebase on origin/main before verifying and before landing; once landed, release it:\n'
  printf '  restless-dev worktree prune\n'
}

# One line per worktree: age, uncommitted files, commits no remote has, claim.
dev_worktree_stale() {
  local main dir head branch age dirty unpushed now claim
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
          claim="$(dev_worktree_claim "$dir")"
          printf '%-7s %-6s %-9s %s [%s]%s\n' "${age}d" "$dirty" "$unpushed" "$dir" "$branch" \
            "${claim:+ claimed by ${claim}}"
        fi
        dir="" branch="" head=""
        ;;
    esac
  done < <(git -C "$STACK_REPO_ROOT" worktree list --porcelain; printf '\n')
}

# Release pooled slots whose task has landed and keep them warm; remove other
# worktrees, and slots beyond the pool size, once clean, pushed and unused.
# Anything holding someone's work is reported, never touched: unpushed or
# uncommitted work, or a live claim, belongs to whoever made it.
dev_worktree_prune() {
  local main parent dir branch reason claim pool released=0 removed=0 kept=0 slots_kept=0
  main="$(dev_worktree_main_root)"
  parent="$(dev_worktree_parent)"
  pool="$(dev_worktree_pool_size)"
  git -C "$STACK_REPO_ROOT" fetch --quiet origin
  git -C "$STACK_REPO_ROOT" worktree prune
  while IFS= read -r line; do
    case "$line" in
      "worktree "*) dir="${line#worktree }" ;;
      "branch "*) branch="${line#branch refs/heads/}" ;;
      "")
        if [ -n "${dir:-}" ] && [ "$dir" != "$main" ] && [ -d "$dir" ]; then
          reason="$(dev_worktree_busy_reason "$dir")"
          case "$dir" in
            "$parent"/slot-*)
              claim="$(dev_worktree_claim "$dir")"
              if [ -z "$reason" ] && [ -n "$claim" ] && ! dev_worktree_claim_landed "$dir"; then
                reason="claimed by ${claim}; nothing landed yet"
              fi
              ;;
            *) claim="" ;;
          esac
          if [ -n "$reason" ]; then
            printf 'KEEP     %s (%s)\n' "$dir" "$reason"
            kept=$((kept + 1))
            case "$dir" in "$parent"/slot-*) slots_kept=$((slots_kept + 1)) ;; esac
          elif [[ "$dir" == "$parent"/slot-* ]] && [ "$slots_kept" -lt "$pool" ]; then
            slots_kept=$((slots_kept + 1))
            local freed
            freed="$(dev_worktree_trim_cache "$dir")"
            if [ -n "$claim" ]; then
              rm -f "$(dev_worktree_claim_file "$dir")"
              printf 'RELEASED %s (%s landed; kept warm for the next task; %s MB of old caches trimmed)\n' "$dir" "$claim" "$freed"
              released=$((released + 1))
            else
              printf 'IDLE     %s (warm, free for the next task; %s MB of old caches trimmed)\n' "$dir" "$freed"
            fi
          else
            git -C "$STACK_REPO_ROOT" worktree remove --force "$dir"
            if [ -n "${branch:-}" ] &&
               git -C "$STACK_REPO_ROOT" merge-base --is-ancestor "$branch" origin/main 2>/dev/null; then
              git -C "$STACK_REPO_ROOT" branch -q -D "$branch" 2>/dev/null || true
            fi
            printf 'REMOVED  %s\n' "$dir"
            removed=$((removed + 1))
          fi
        fi
        dir="" branch=""
        ;;
    esac
  done < <(git -C "$STACK_REPO_ROOT" worktree list --porcelain; printf '\n')
  printf '\n%s released, %s removed, %s kept.\n' "$released" "$removed" "$kept"
}
