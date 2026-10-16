#!/usr/bin/env bash
set -euo pipefail

# Native smoke builds share the runner's Docker daemon, but must not change its
# default builder or depend on WSL's host-only DNS forwarder inside build steps.
[[ $# -gt 0 ]] || { echo 'usage: with-company-builder.sh COMMAND [ARGUMENTS...]' >&2; exit 2; }
scratch="$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/restless-company-build.XXXXXX")"
builder="restless-company-${scratch##*.}"
builder="${builder,,}"
cleanup() {
  docker buildx rm "$builder" >/dev/null 2>&1 || true
  rm -rf -- "$scratch"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
cat > "$scratch/buildkitd.toml" <<'EOF'
[dns]
  nameservers = ["1.1.1.1", "8.8.8.8"]
[worker.oci]
  max-parallelism = 2
  gc = true
  reservedSpace = "512MB"
  maxUsedSpace = "8GB"
  minFreeSpace = "30GB"
[history]
  maxAge = 3600
  maxEntries = 5
EOF
docker buildx create --name "$builder" --driver docker-container \
  --driver-opt default-load=true --buildkitd-config "$scratch/buildkitd.toml" >/dev/null
docker buildx inspect "$builder" --bootstrap >/dev/null
bash "$(dirname "$0")/prepare-builder-dns.sh" "$builder"
export BUILDX_BUILDER="$builder"
"$@"
