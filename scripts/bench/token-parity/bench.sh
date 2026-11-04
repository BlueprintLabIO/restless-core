#!/usr/bin/env bash
# Token parity, end to end: start the counting mock model, bring up one checkout's local dev stack
# on it with a fresh disposable _test company, run the three tasks and the baseline (run.mjs), then
# tear down what it started (company destroyed, daemon and Vite stopped, mock stopped). The
# checkout's dev database and vault containers are restless-dev's own and stay, as they do for it.
#
#   scripts/bench/token-parity/bench.sh [checkout] [scenario.json] [out.json]
#
# checkout defaults to this repository; scenario to restless.json beside this script. To measure
# an older commit, check it out in a worktree and pass restless-before.json. The mock and the runner
# always come from this script's own checkout, so both sides are counted the same way.
# Needs: docker, node, the checkout's web/node_modules (installed if missing). The first run builds
# the daemon and the company image (--reconcile), which takes a while.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../../.." && pwd)"
checkout="$(cd "${1:-$repo}" && pwd)"
scenario="$(readlink -f "${2:-$here/restless.json}")"
out="${3:-$(mktemp -d)/token-parity.json}"
# A fresh company each run, so no earlier session history is carried into the measurement.
company="${BENCH_COMPANY:-tokparity_$(date +%s)_$$_test}"
port="${BENCH_MOCK_PORT:-18977}"
work="$(mktemp -d "${TMPDIR:-/tmp}/token-parity.XXXXXX")"
printf 'checkout %s\nscenario %s\ncompany  %s\nlogs     %s\n' "$checkout" "$scenario" "$company" "$work"

[ -d "$checkout/web/node_modules" ] || npm --prefix "$checkout/web" ci --no-audit --no-fund >"$work/npm.log" 2>&1

node "$repo/tools/mock-model/server.mjs" --port "$port" --scenario "$scenario" --log "$work/calls.jsonl" \
  >"$work/mock.log" 2>&1 &
mock_pid=$!
stack_pid=""
home=""
cleanup() {
  set +e
  if [ -n "$home" ]; then
    # The same dev profile restless-dev used, while its daemon is still up. BENCH_ALSO_DESTROY
    # names earlier bench companies in this profile whose teardown did not finish.
    for doomed in "$company" ${BENCH_ALSO_DESTROY:-}; do
      [[ "$doomed" == *_test ]] || continue
      RESTLESS_PROFILE=dev RESTLESS_RESOURCE_NAMESPACE="$namespace" RESTLESS_PORT_OFFSET="$offset" \
        RESTLESS_HOME="$home" RESTLESS_OWNER_URL="$owner_url" \
        "${RESTLESS_DEV_TARGET_DIR:-$checkout/target-dev}/debug/restless" down -c "$doomed" --destroy >>"$work/teardown.log" 2>&1 \
        || printf 'could not destroy %s; see %s/teardown.log\n' "$doomed" "$work" >&2
    done
  fi
  if [ -n "$stack_pid" ]; then
    kill -INT -- "-$stack_pid" 2>/dev/null
    wait "$stack_pid" 2>/dev/null
  fi
  kill "$mock_pid" 2>/dev/null
  wait "$mock_pid" 2>/dev/null
}
trap cleanup EXIT

# The stack's own launcher, on the mock: any key works there and none is real.
BENCH_MOCK_KEY=bench-mock-not-a-secret \
RESTLESS_ANTHROPIC_BASE_URL="http://127.0.0.1:${port}/v1" \
RESTLESS_DEV_MODEL="${BENCH_MODEL:-anthropic/claude-sonnet-5-5}" \
RESTLESS_DEV_CREDENTIAL_REFERENCE=env:BENCH_MOCK_KEY \
  setsid bash "$checkout/scripts/restless-dev" "$company" --reconcile >"$work/stack.log" 2>&1 &
stack_pid=$!

deadline=$((SECONDS + ${BENCH_STACK_TIMEOUT:-5400}))
until grep -q 'Press Ctrl-C to stop' "$work/stack.log"; do
  if ! kill -0 "$stack_pid" 2>/dev/null || [ "$SECONDS" -ge "$deadline" ]; then
    printf 'the stack did not come up; see %s/stack.log\n' "$work" >&2
    tail -30 "$work/stack.log" >&2
    exit 1
  fi
  sleep 10
done
home="$(awk '$1 == "ROOT" {print $2; exit}' "$work/stack.log")"
offset="$(awk '$1 == "OFFSET" {print $2; exit}' "$work/stack.log")"
namespace="$(awk '$1 == "NAMESPACE" {print $2; exit}' "$work/stack.log")"
owner_url="http://127.0.0.1:$((7788 + offset))"
web_url="http://127.0.0.1:$((5173 + offset))"
cli() {
  RESTLESS_PROFILE=dev RESTLESS_RESOURCE_NAMESPACE="$namespace" RESTLESS_PORT_OFFSET="$offset" \
    RESTLESS_HOME="$home" RESTLESS_OWNER_URL="$owner_url" \
    "${RESTLESS_DEV_TARGET_DIR:-$checkout/target-dev}/debug/restless" "$@"
}
# BENCH_ROUTE=subscription: the owner's companies run on a Claude sign-in (`omp-oauth:anthropic`),
# served through OMP's host gateway on the pi-native wire rather than the Messages relay. The
# stack's broker gets a scripted sign-in (seed-sign-in.mjs) and the company is switched to it.
if [ "${BENCH_ROUTE:-api}" = subscription ]; then
  profile="$HOME/.omp/profiles/restless-model-broker$([ "$offset" = 0 ] || printf -- '-%s' "$offset")"
  broker="http://127.0.0.1:$((7789 + offset))"
  node "$here/seed-sign-in.mjs" --broker "$broker" --token-file "$profile/auth-broker.token"
  cli company set -c "$company" credentials.model.inference omp-oauth:anthropic
  node "$here/seed-sign-in.mjs" --broker "$broker" --token-file "$profile/auth-broker.token" --wait 300
fi
for setting in ${BENCH_COMPANY_SETTINGS:-}; do # key=value, e.g. everyday_model=anthropic/claude-sonnet-5-5
  cli company set -c "$company" "${setting%%=*}" "${setting#*=}"
done
# Let first-start turns (if any) finish before measuring.
sleep "${BENCH_WARMUP_SECONDS:-60}"

node "$here/run.mjs" --origin "$web_url" --company "$company" --mock "http://127.0.0.1:${port}" \
  --route "${BENCH_ROUTE:-api}" --log "$work/calls.jsonl" --out "$out" ${BENCH_RUN_ARGS:-}
printf '\nresult %s\n' "$out"
