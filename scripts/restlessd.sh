#!/usr/bin/env bash
# Start restlessd with its provider credentials.
#
# The daemon reads no .env — deliberately, because a secret it loads from a file
# is a secret with a second home. So the environment is assembled here instead,
# from ~/.restless/env, which keeps keys out of your shell history and out of
# the ps-visible command line.
#
#   scripts/restlessd.sh              # foreground, logs to the terminal
#   scripts/restlessd.sh --background # detached, logs to ~/.restless/restlessd.log
#
# Rebuilds first: `cargo test` does not rebuild the binary, and a stale
# target/debug/restlessd is how a route that exists in the source 404s at runtime.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
env_file="${RESTLESS_HOME:-$HOME/.restless}/env"
log="${RESTLESS_HOME:-$HOME/.restless}/restlessd.log"

if [[ ! -f "$env_file" ]]; then
	echo "no $env_file — create it with, e.g.:" >&2
	echo "    MOONSHOT_API_KEY=sk-..." >&2
	echo "    MOONSHOT_BASE_URL=https://api.kimi.com/coding/v1   # For-Coding keys only" >&2
	exit 1
fi

perms="$(stat -f %Lp "$env_file" 2>/dev/null || stat -c %a "$env_file")"
if [[ "$perms" != "600" ]]; then
	echo "warning: $env_file is mode $perms; it holds a provider key. chmod 600 it." >&2
fi

set -a
# shellcheck disable=SC1090
source "$env_file"
set +a

# The daemon runs OMP's credential broker on the host and spawns it as `omp`,
# by name, so it has to be on PATH — and bun's global bin usually is not. A
# missing binary is a fatal daemon start whose error ("create OMP auth-broker
# bearer / No such file or directory") names neither omp nor PATH, so this is
# resolved here rather than left to be rediscovered.
if [[ -z "${RESTLESS_OMP_BIN:-}" ]] && ! command -v omp >/dev/null 2>&1; then
	if [[ -x "$HOME/.bun/bin/omp" ]]; then
		export PATH="$HOME/.bun/bin:$PATH"
	else
		echo "no \`omp\` on PATH — the daemon cannot start without it. Install with:" >&2
		echo "    bun install -g @oh-my-pi/pi-coding-agent@17.2.15   # needs bun >= 1.3.14" >&2
		echo "or point RESTLESS_OMP_BIN at an existing one." >&2
		exit 1
	fi
fi

cd "$root"
cargo build -p restlessd

pkill -f 'target/debug/restlessd' 2>/dev/null || true
sleep 1

if [[ "${1:-}" == "--background" ]]; then
	nohup ./target/debug/restlessd >"$log" 2>&1 &
	sleep 3
	if curl -fsS -m 3 http://127.0.0.1:7788/v1/health >/dev/null 2>&1; then
		echo "restlessd up — owner gateway on :7788, docs at http://127.0.0.1:7788/v1/docs"
		echo "everything else there needs the owner cookie: restless owner-token --rotate"
		echo "logs: $log"
	else
		echo "restlessd did not come up; last lines of $log:" >&2
		tail -20 "$log" >&2
		exit 1
	fi
else
	exec ./target/debug/restlessd
fi
