#!/usr/bin/env bash
set -euo pipefail

V2_DIR="$(cd "$(dirname "$0")" && pwd)"
LAB_DIR="$(cd "$V2_DIR/.." && pwd)"
WORK_DIR="$V2_DIR/workdir"
BROKER_PROFILE="restless-model-broker"
GATEWAY_PROFILE="restless-model-gateway-v21"
GATEWAY_PORT="${COORD_GATEWAY_PORT:-7796}"

mkdir -p "$WORK_DIR"

broker_token="$(OMP_PROFILE="$BROKER_PROFILE" omp auth-broker token)"
gateway_token="$(OMP_PROFILE="$GATEWAY_PROFILE" omp auth-gateway token)"

cleanup() {
  if [[ -n "${gateway_pid:-}" ]]; then kill "$gateway_pid" 2>/dev/null || true; fi
  if [[ -n "${broker_pid:-}" ]]; then kill "$broker_pid" 2>/dev/null || true; fi
}
trap cleanup EXIT

if ! nc -z 127.0.0.1 7789 2>/dev/null; then
  OMP_PROFILE="$BROKER_PROFILE" omp auth-broker serve --bind=127.0.0.1:7789 >"$WORK_DIR/broker.log" 2>&1 &
  broker_pid=$!
  for _ in {1..50}; do nc -z 127.0.0.1 7789 2>/dev/null && break; sleep 0.1; done
fi

if nc -z 127.0.0.1 "$GATEWAY_PORT" 2>/dev/null; then
  echo "Dedicated coordination gateway port $GATEWAY_PORT is already in use" >&2
  exit 1
else
  OMP_PROFILE="$GATEWAY_PROFILE" OMP_AUTH_BROKER_URL="http://127.0.0.1:7789" OMP_AUTH_BROKER_TOKEN="$broker_token" \
    omp auth-gateway serve --bind="0.0.0.0:$GATEWAY_PORT" >"$WORK_DIR/gateway-v21.log" 2>&1 &
  gateway_pid=$!
  for _ in {1..50}; do nc -z 127.0.0.1 "$GATEWAY_PORT" 2>/dev/null && break; sleep 0.1; done
fi

export RESTLESS_MODEL_GATEWAY_TOKEN="$gateway_token"
export COORD_GATEWAY_PORT="$GATEWAY_PORT"
python3 "$V2_DIR/runner.py" "$@"
