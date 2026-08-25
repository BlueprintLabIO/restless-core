#!/usr/bin/env bash
set -euo pipefail

work_dir="$(cd "$(dirname "$0")" && pwd)"
gateway_port="${COORD_GATEWAY_PORT:-7796}"
broker_profile="restless-model-broker"
gateway_profile="restless-model-gateway-v21"

broker_token="$(OMP_PROFILE="$broker_profile" omp auth-broker token)"
gateway_token="$(OMP_PROFILE="$gateway_profile" omp auth-gateway token)"

cleanup() {
  if [[ -n "${gateway_pid:-}" ]]; then kill "$gateway_pid" 2>/dev/null || true; fi
  if [[ -n "${broker_pid:-}" ]]; then kill "$broker_pid" 2>/dev/null || true; fi
}
trap cleanup EXIT

if ! nc -z 127.0.0.1 7789 2>/dev/null; then
  OMP_PROFILE="$broker_profile" omp auth-broker serve --bind=127.0.0.1:7789 >"$work_dir/broker.log" 2>&1 &
  broker_pid=$!
  for _ in {1..50}; do nc -z 127.0.0.1 7789 2>/dev/null && break; sleep 0.1; done
fi

if nc -z 127.0.0.1 "$gateway_port" 2>/dev/null; then
  echo "EXP-04 gateway port $gateway_port is already in use" >&2
  exit 1
fi

OMP_PROFILE="$gateway_profile" OMP_AUTH_BROKER_URL="http://127.0.0.1:7789" OMP_AUTH_BROKER_TOKEN="$broker_token" \
  omp auth-gateway serve --bind="0.0.0.0:$gateway_port" >"$work_dir/gateway.log" 2>&1 &
gateway_pid=$!
for _ in {1..50}; do nc -z 127.0.0.1 "$gateway_port" 2>/dev/null && break; sleep 0.1; done

export RESTLESS_MODEL_GATEWAY_TOKEN="$gateway_token"
export COORD_GATEWAY_PORT="$gateway_port"
python3 "$work_dir/exp04.py" "$@"
