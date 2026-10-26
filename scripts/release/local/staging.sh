#!/usr/bin/env bash
# The local release staging pair (ADR 0010 in restless-cloud): a registry the release pipeline
# pushes to, and a Dagger engine configured to reach it over plain HTTP on a private Docker network.
# Releases then mirror what staging holds into R2 (registry.restless.run). Nothing listens outside
# this computer: the registry publishes to 127.0.0.1 only, for the mirror.
#
#   scripts/release/local/staging.sh up      start (or keep) both, print the env to export
#   scripts/release/local/staging.sh down    stop both; the registry's data volume stays
#   scripts/release/local/staging.sh env     print the env the release scripts use
set -euo pipefail
NETWORK=restless-release
REGISTRY=restless-staging
REGISTRY_IMAGE='registry:3.0.0@sha256:6c5666b861f3505b116bb9aa9b25175e71210414bd010d92035ff64018f9457e'
REGISTRY_PORT=5055
# A fixed address: the engine resolves names through its own DNS, which cannot see Docker names.
SUBNET=172.30.255.0/24
REGISTRY_IP=172.30.255.2
ENGINE=restless-dagger-engine
ENGINE_IMAGE=registry.dagger.io/engine:v0.21.10
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/restless-release"

env_lines() {
  echo "export _EXPERIMENTAL_DAGGER_RUNNER_HOST=docker-container://$ENGINE"
  echo "export RESTLESS_STAGING_REGISTRY=$REGISTRY_IP:5000"
  echo "export RESTLESS_STAGING_URL=http://127.0.0.1:$REGISTRY_PORT"
}

up() {
  docker network inspect "$NETWORK" >/dev/null 2>&1 || docker network create --subnet "$SUBNET" "$NETWORK" >/dev/null
  if ! docker inspect "$REGISTRY" >/dev/null 2>&1; then
    docker volume create restless-staging-data >/dev/null
    docker run -d --name "$REGISTRY" --restart unless-stopped --network "$NETWORK" --ip "$REGISTRY_IP" \
      -p "127.0.0.1:$REGISTRY_PORT:5000" -v restless-staging-data:/var/lib/registry \
      -e REGISTRY_STORAGE_DELETE_ENABLED=true "$REGISTRY_IMAGE" >/dev/null
  else
    docker start "$REGISTRY" >/dev/null
  fi
  mkdir -p "$CONFIG_DIR"
  cat > "$CONFIG_DIR/engine.toml" <<EOF
# Written by scripts/release/local/staging.sh: the staging registry speaks plain HTTP on the
# private release network.
[registry."$REGISTRY_IP:5000"]
  http = true
  insecure = true
EOF
  if ! docker inspect "$ENGINE" >/dev/null 2>&1; then
    docker volume create restless-dagger-engine-state >/dev/null
    docker run -d --name "$ENGINE" --restart unless-stopped --privileged --network "$NETWORK" \
      -v restless-dagger-engine-state:/var/lib/dagger \
      -v "$CONFIG_DIR/engine.toml:/etc/dagger/engine.toml:ro" "$ENGINE_IMAGE" >/dev/null
  else
    docker start "$ENGINE" >/dev/null
  fi
  for _ in $(seq 1 60); do
    curl -fsS "http://127.0.0.1:$REGISTRY_PORT/v2/" >/dev/null 2>&1 && break
    sleep 1
  done
  env_lines
}

case "${1:-up}" in
  up) up ;;
  down) docker stop "$ENGINE" "$REGISTRY" >/dev/null 2>&1 || true ;;
  env) env_lines ;;
  *) echo "usage: $0 up|down|env" >&2; exit 2 ;;
esac
