#!/usr/bin/env bash
# The connections smoke on the exact release candidate, as the GitHub release workflow ran it before
# ADR 0010 (restless-cloud): build restlessd, restless and the company image from this checkout,
# then drive the daemon, CLI, company image and host tools end to end.
#
# Fast lane: the smoke never touches the cockpit or the docs, so when nothing else changed since a
# run that passed on this computer it would test identical bytes again; it says so and skips.
set -euo pipefail
root="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$root"
image=restless-company-image:release-connections
state="${XDG_CACHE_HOME:-$HOME/.cache}/restless-release"
passed="$state/connections-passed"
inputs="$(git ls-tree HEAD | awk -F'\t' '$2 != "web" && $2 != "docs" && $2 != "experiment"' | sha256sum | cut -c1-64)"
if [ -f "$passed/$inputs" ]; then
  printf 'connections smoke skipped: its inputs (%s) passed at %s\n' "${inputs:0:12}" "$(cat "$passed/$inputs")"
  exit 0
fi
evidence="$(mktemp -d)"
trap 'rm -rf "$evidence"' EXIT
export PATH="$HOME/.cargo/bin:$PATH" CARGO_INCREMENTAL=0
export CARGO_TARGET_DIR="$state/connections-target"
# docs/BUILD_STORAGE.md: clean a persistent target above 30 GiB.
if [ -d "$CARGO_TARGET_DIR" ] && (( $(du -s --block-size=1G "$CARGO_TARGET_DIR" | cut -f1) > 30 )); then
  cargo clean --target-dir "$CARGO_TARGET_DIR"
fi
scripts/restless-cargo build -p restlessd -p restless
npm --prefix infra/host-tools ci --no-audit --no-fund >/dev/null
previous="$(docker image inspect --format '{{.Id}}' "$image" 2>/dev/null || true)"
revision="$(git rev-parse HEAD)"
IMAGE="$image" REVISION="$revision" bash scripts/release/with-company-builder.sh bash -c '
  source scripts/lib/dev-release.sh
  dev_build_company_image "$PWD" "$IMAGE" "$REVISION"
'
# Remove only the exact image this build replaced; a container still using it keeps it.
if [ -n "$previous" ] && [ "$previous" != "$(docker image inspect --format '{{.Id}}' "$image")" ]; then
  docker image rm "$previous" >/dev/null 2>&1 || true
fi
RESTLESSD_BIN="$CARGO_TARGET_DIR/debug/restlessd" RESTLESS_CLI_BIN="$CARGO_TARGET_DIR/debug/restless" \
  RESTLESS_COMPANY_IMAGE="$image" python3 scripts/connections-smoke --target core --output "$evidence"
mkdir -p "$passed" && printf '%s\n' "$revision" >"$passed/$inputs"
find "$passed" -type f -printf '%T@ %p\n' | sort -rn | tail -n +41 | cut -d' ' -f2- | xargs -r rm -f
