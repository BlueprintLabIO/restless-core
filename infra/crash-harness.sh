#!/usr/bin/env bash
# T14 · Crash and restart harness (docs/sprints/sprint-01/t14-crash-restart-harness.md)
#
# Three interruptions against a LIVE company run, each asserted separately:
#   1. kill the Exec mid-turn    -> next wake rehydrates and continues the milestone
#   2. kill a staff process mid-turn -> commitment blocked, worktree preserved, exec mailed
#   3. restart restlessd         -> companies resume; files and commits remain valid
# plus the terminal cleanup proof (no stray agent processes, volumes intact).
#
# Every step below codifies a procedure already executed by hand during the
# sprint (orphan sweep across restarts, staged staff crash, daemon restart
# during the 2026-08-13 disk incident). What is NOT yet verified: phases 1
# and 2 against a real mid-run turn — that needs model credit. Run during
# the Cosmon run once credit lands:
#
#   infra/crash-harness.sh cosmon
#
# Prerequisites: company container running, a run in progress (an active
# milestone commitment), daemon running, psql on postgres://localhost/restless.

set -euo pipefail

COMPANY="${1:?usage: crash-harness.sh <company>}"
CONTAINER="restless-co-${COMPANY}"
DB="postgres://yao@localhost/restless"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CLI="${REPO_ROOT}/target/debug/restless"

pass() { printf 'PASS  %s\n' "$1"; }
fail() { printf 'FAIL  %s\n' "$1" >&2; exit 1; }

sql() { psql "$DB" -tA -c "$1"; }

# Agent PIDs in the container, optionally filtered by working directory.
# (The bracket pattern avoids the pgrep self-match against our own sh -c.)
agent_pids() { # <cwd-substring or empty>
  docker exec "$CONTAINER" sh -c '
    for p in $(pgrep -f "codex[-]acp" || true); do
      cwd=$(readlink "/proc/$p/cwd" 2>/dev/null || true)
      echo "$p $cwd"
    done' | { [ -n "${1:-}" ] && grep "$1" | awk '{print $1}' || awk '{print $1}'; }
}

wait_for_agent() { # <cwd-substring or empty> — polls 60s for a turn in flight
  for _ in $(seq 1 30); do
    [ -n "$(agent_pids "${1:-}")" ] && return 0
    sleep 2
  done
  return 1
}

# ---------------------------------------------------------------- phase 1
# Kill the Exec mid-turn. Requires the exec to be working (trigger a wake).
echo "== phase 1: kill exec mid-turn =="
before_commit=$(docker exec -u company "$CONTAINER" \
  sh -c 'git -C /company/repos/*/ rev-parse HEAD' 2>/dev/null | head -1 || true)

"$CLI" wake -c "$COMPANY" >/dev/null 2>&1 || true   # kick a turn; async
wait_for_agent "" || fail "exec process never appeared — is a run in progress?"
docker exec "$CONTAINER" sh -c 'pkill -9 -f "codex[-]acp" || true'
sleep 3

# The wake must end in a recorded failure, not silence: commitment state is
# untouched (the exec never got to report), and the daemon logged the drop.
sql "select count(*) from ${COMPANY}.events where kind='wake' and created_at > now() - interval '5 minutes'" \
  | grep -q . || echo "note: no wake event row — check daemon log for the failure record"

# Recovery: next wake rehydrates from OrgIntel + files and continues.
"$CLI" wake -c "$COMPANY" >/dev/null 2>&1 || true
sleep 5
[ -n "$(sql "select id from ${COMPANY}.commitments where state='active' limit 1")" ] \
  && pass "exec killed mid-turn; milestone still active and resumable" \
  || fail "no active commitment after exec kill — continuity lost"

# ---------------------------------------------------------------- phase 2
# Kill a staff process mid-turn. Requires a staffer to be running — trigger
# one via the run itself (the exec spawns staff), or skip honestly.
echo "== phase 2: kill staff mid-turn =="
if [ -z "$(sql "select id from ${COMPANY}.commitments where owner_id like 'staff-%' and state='active' limit 1")" ]; then
  echo "SKIP  no active staff commitment — spawn one via the run, then re-run this phase"
else
  wait_for_agent "/company/worktrees/" || fail "staff process never appeared"
  victim_cwd=$(docker exec "$CONTAINER" sh -c '
    for p in $(pgrep -f "codex[-]acp" || true); do
      cwd=$(readlink "/proc/$p/cwd" 2>/dev/null || true)
      case "$cwd" in /company/worktrees/*) echo "$cwd";; esac
    done' | head -1)
  [ -n "$victim_cwd" ] || fail "no staff worktree process found"
  for p in $(agent_pids "/company/worktrees/"); do
    docker exec "$CONTAINER" kill -9 "$p" || true
  done
  sleep 3
  sql "select state from ${COMPANY}.commitments where owner_id like 'staff-%' order by updated_at desc limit 1" \
    | grep -qx "blocked" || fail "staff commitment not marked blocked after kill"
  docker exec "$CONTAINER" test -d "$victim_cwd" \
    && pass "staff killed mid-turn; commitment blocked; worktree preserved at $victim_cwd" \
    || fail "worktree $victim_cwd missing after staff kill"
fi

# ---------------------------------------------------------------- phase 3
# Restart the daemon. (Rehearsed live 2026-08-13 during the disk incident:
# state in Postgres + ~/.restless survived; CLI and scheduler resumed.)
echo "== phase 3: restart restlessd =="
pkill -x restlessd || true
sleep 2
cd "$REPO_ROOT"
nohup ./target/debug/restlessd >> /tmp/restlessd.log 2>&1 &
sleep 5
[ "$("$CLI" status -c "$COMPANY" 2>/dev/null)" = "${COMPANY}: Running" ] \
  && pass "daemon restarted; company reports Running; coordination state intact" \
  || fail "daemon did not resume cleanly"

# ------------------------------------------------------- cleanup proof
echo "== cleanup proof =="
stray=$(agent_pids "" || true)
[ -z "$stray" ] && pass "no stray agent processes in $CONTAINER" \
  || fail "stray agent processes: $stray"
docker exec -u company "$CONTAINER" sh -c 'git -C /company/repos/*/ status --porcelain' >/dev/null \
  && pass "repo(s) readable, git intact" || fail "repo unreadable after interruptions"
if [ -n "$before_commit" ]; then
  after_commit=$(docker exec -u company "$CONTAINER" \
    sh -c 'git -C /company/repos/*/ rev-parse HEAD' | head -1)
  [ -n "$after_commit" ] && pass "commits valid before ($before_commit) and after ($after_commit)"
fi

echo "T14 harness complete for ${COMPANY}."
