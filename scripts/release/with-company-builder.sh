#!/usr/bin/env bash
set -euo pipefail

# Native smoke builds share the runner's Docker daemon, but must not change its
# default builder or depend on WSL's host-only DNS forwarder inside build steps.
#
# The builder persists between jobs (23 October): a builder made and removed per job took its
# cache with it, so every release and every push smoke rebuilt the company image from apt up,
# about five minutes each. Both runners on the PC share it; BuildKit runs their builds side by
# side, and its own garbage collection keeps the cache within maxUsedSpace.
[[ $# -gt 0 ]] || { echo 'usage: with-company-builder.sh COMMAND [ARGUMENTS...]' >&2; exit 2; }
builder="${RESTLESS_COMPANY_BUILDER:-restless-company-cache}"
state="${XDG_CACHE_HOME:-$HOME/.cache}/restless-runner"
mkdir -p "$state"
config="$state/company-buildkitd.toml"
cat > "$config.next" <<'EOF'
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

ready() { docker buildx inspect "$builder" --bootstrap >/dev/null 2>&1; }
create() {
  docker buildx rm "$builder" >/dev/null 2>&1 || true
  docker buildx create --name "$builder" --driver docker-container \
    --driver-opt default-load=true --buildkitd-config "$config" >/dev/null
  docker buildx inspect "$builder" --bootstrap >/dev/null
}
(
  # One job at a time decides whether the builder needs making; the build itself is shared.
  flock 9
  if ! cmp -s "$config.next" "$config"; then
    mv "$config.next" "$config"
    create
  elif ! ready; then
    create
  fi
  rm -f "$config.next"
) 9>"$state/company-builder.lock"

bash "$(dirname "$0")/prepare-builder-dns.sh" "$builder"
export BUILDX_BUILDER="$builder"
"$@"
