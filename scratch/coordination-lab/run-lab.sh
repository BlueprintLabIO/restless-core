#!/usr/bin/env bash
set -euo pipefail

LAB_DIR="$(cd "$(dirname "$0")" && pwd)"
WORK_DIR="$LAB_DIR/workdir"
IMAGE="${COORD_IMAGE:-restless-company-image:latest}"
CONTAINER="restless-coordination-lab-test"
BROKER_PROFILE="restless-model-broker"
GATEWAY_PROFILE="restless-model-gateway"

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

if ! nc -z 127.0.0.1 7790 2>/dev/null; then
  OMP_PROFILE="$GATEWAY_PROFILE" OMP_AUTH_BROKER_URL="http://127.0.0.1:7789" OMP_AUTH_BROKER_TOKEN="$broker_token" \
    omp auth-gateway serve --bind=0.0.0.0:7790 >"$WORK_DIR/gateway.log" 2>&1 &
  gateway_pid=$!
  for _ in {1..50}; do nc -z 127.0.0.1 7790 2>/dev/null && break; sleep 0.1; done
fi

if ! docker inspect "$CONTAINER" >/dev/null 2>&1; then
  docker run -d --name "$CONTAINER" \
    --add-host=host.docker.internal:host-gateway \
    -v "$WORK_DIR:/lab" \
    -v "$LAB_DIR:/harness:ro" \
    "$IMAGE" sleep infinity >/dev/null
fi
if [[ "$(docker inspect -f '{{.State.Running}}' "$CONTAINER")" != "true" ]]; then
  docker start "$CONTAINER" >/dev/null
fi

export RESTLESS_MODEL_GATEWAY_TOKEN="$gateway_token"
exec python3 "$LAB_DIR/lab.py" "$@"
