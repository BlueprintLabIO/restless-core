#!/usr/bin/env bash
set -euo pipefail

# [dns] in buildkitd.toml configures RUN executors, not the daemon's Go
# registry/OAuth resolver. Repair only this release builder, preserving its
# process, network, state volume, and the runner's host resolver.
builder="${1:?usage: prepare-builder-dns.sh BUILDER_NAME}"
[[ "$builder" =~ ^[a-zA-Z0-9][a-zA-Z0-9_.-]*$ ]] || { echo 'Invalid release builder name' >&2; exit 1; }
container="buildx_buildkit_${builder}0"
[[ "$(docker buildx inspect "$builder" | awk '$1 == "Driver:" { print $2 }')" == docker-container ]] || { echo 'Release builder must use docker-container' >&2; exit 1; }
[[ "$(docker inspect "$container" --format '{{.State.Running}}')" == true ]] || { echo 'Release BuildKit container is not running' >&2; exit 1; }
container_id="$(docker inspect "$container" --format '{{.Id}}')"
[[ "$container_id" =~ ^[0-9a-f]{64}$ ]] || { echo 'Invalid release BuildKit container identity' >&2; exit 1; }
resolver_path="$(docker inspect "$container_id" --format '{{.ResolvConfPath}}')"
[[ "$resolver_path" == */"$container_id"/resolv.conf ]] || { echo 'Release builder resolver is not Docker-managed' >&2; exit 1; }
[[ "$(docker inspect "$container_id" --format '{{range .Mounts}}{{if eq .Destination "/var/lib/buildkit"}}{{.Name}}{{end}}{{end}}')" == "${container}_state" ]] || { echo 'Release builder state volume does not match' >&2; exit 1; }

docker exec --user 0 "$container_id" sh -ec '
  printf "%s\n" "nameserver 1.1.1.1" "nameserver 8.8.8.8" "options timeout:2 attempts:2" > /etc/resolv.conf
'
# Go checks resolv.conf at most once every five seconds. Allow a previously
# running daemon to observe this exact container-scoped update before export.
sleep 6
# Resolve a public GHCR image through BuildKit itself: this exercises the Go
# daemon registry resolver, normal certificate validation and token exchange.
# outline reads image metadata only; it exports no layers and publishes nothing.
probe_context="$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/restless-release-dns.XXXXXX")"
trap 'rm -rf -- "$probe_context"' EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
printf '%s\n' 'FROM ghcr.io/oras-project/oras:v1.2.2' > "$probe_context/Dockerfile"
if BUILDKIT_NO_CLIENT_TOKEN=true timeout --kill-after=5s 45s \
  docker buildx build --builder "$builder" --pull --call=outline \
  --progress=plain "$probe_context" > "$probe_context/probe.log" 2>&1; then
  echo 'Release builder registry DNS/TLS/token metadata: PASS; existing cache retained'
else
  status=$?
  echo "Release builder GHCR metadata preflight failed (exit=$status)" >&2
  # Never expose token responses, credentials or arbitrary registry bodies.
  # A Go DNS failure has an allowlisted host/address and useful root cause.
  grep -Eo 'lookup ghcr\.io on [0-9a-fA-F:.]+: (read udp [0-9a-fA-F:.>-]+: )?(read: )?(i/o timeout|no such host|connection refused|network is unreachable)' \
    "$probe_context/probe.log" | head -1 >&2 || true
  exit 1
fi
