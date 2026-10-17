#!/usr/bin/env bash
# Runs at the end of every self-hosted job, pass or fail, so builds clean up after
# themselves rather than filling the runner's disk. The runner is a shared machine
# that also hosts companies and other projects: it keeps every image a container
# uses (running or stopped) and the newest few builds of each Restless image, and
# never touches containers, volumes, networks or other projects' tagged images.
# Removal is without --force, so Docker itself protects an image a container started
# using meanwhile. Dagger and the company builder bound their own caches.
set -uo pipefail

keep="${RESTLESS_RUNNER_KEEP_IMAGES:-3}"
owned='^(restless-company-image|restless-native-documents|restless-cloud-local-[a-z0-9-]+|ghcr\.io/blueprintlabio/restless-[a-z0-9-]+)$'

command -v docker >/dev/null 2>&1 || exit 0
free() { df --output=avail -B1 "${1:-/}" 2>/dev/null | tail -1 | tr -d ' '; }
before=$(free)

# An incomplete inventory removes nothing.
containers=$(docker ps -aq) || exit 0
used=""
if [ -n "$containers" ]; then
  # Container IDs come from Docker; word splitting is intended.
  # shellcheck disable=SC2086
  used=$(docker inspect --format '{{.Image}}' $containers) || exit 0
fi
inventory=$(docker image ls --no-trunc --format '{{.ID}} {{.Repository}} {{.Tag}}') || exit 0

declare -A protected=() seen=() counts=()
while read -r id; do [ -z "$id" ] || protected["$id"]=1; done <<< "$used"

removed=0
# Docker lists newest first.
while read -r id repository tag; do
  [[ "$repository" =~ $owned ]] || continue
  if [ -z "${seen[$repository/$id]:-}" ]; then
    seen[$repository/$id]=1
    counts[$repository]=$(( ${counts[$repository]:-0} + 1 ))
  fi
  (( counts[$repository] > keep )) || continue
  [ -z "${protected[$id]:-}" ] || continue
  # Untag by name where there is one, so an ID shared with another tag is left alone.
  target="$id"; [ "$tag" = "<none>" ] || target="$repository:$tag"
  docker image rm "$target" >/dev/null 2>&1 && removed=$((removed + 1))
done <<< "$inventory"

# Layers no tag points to any more, and old default-builder cache.
docker image prune --force >/dev/null 2>&1 || true
docker builder prune --force --filter until=72h >/dev/null 2>&1 || true

after=$(free)
gib() { awk -v b="${1:-0}" 'BEGIN { printf "%.1f", b / 1073741824 }'; }
message="Runner disk: removed $removed old Restless images; free $(gib "$before") GiB -> $(gib "$after") GiB"
echo "$message"
[ -z "${GITHUB_STEP_SUMMARY:-}" ] || printf '%s\n' "$message" >> "$GITHUB_STEP_SUMMARY"
exit 0
