#!/usr/bin/env bash
# The zero-jank gate (Sprint 109 C7) on the exact release candidate: every primary surface and
# Inbox card of a real isolated company, as the owner and a member, at 1440 and 390 wide, empty,
# populated, with slow reads and with failing reads, cold, warm and arrived at by navigation.
# It fails on any layout shift, unstyled frame or named edge off the grid that
# services/identity/test/jank-allowlist.json does not list (services/identity/test/
# multiplayer-jank-journey.mjs). Runs after the connections smoke, reusing its Core build and
# company image, against a disposable `_test` PostgreSQL. Evidence (results, screenshots,
# filmstrips of failing loads) is kept under ~/.cache/restless-release/jank/<revision>.
#
# Fast lane: a tree that already passed on this computer is not measured again.
# Concurrent runs (two release candidates at once) each get their own PostgreSQL container, Core
# port block, Documents image and restlessd copy, so one run never removes another's.
set -euo pipefail
root="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$root"
state="${XDG_CACHE_HOME:-$HOME/.cache}/restless-release"
passed="$state/jank-passed"
tree="$(git rev-parse 'HEAD^{tree}')"
if [ -f "$passed/$tree" ]; then
  printf 'zero-jank gate skipped: this tree (%s) passed at %s\n' "${tree:0:12}" "$(cat "$passed/$tree")"
  exit 0
fi
revision="$(git rev-parse HEAD)"
image=restless-company-image:release-connections
database="restless-release-jank-$$-$(head -c 4 /dev/urandom | od -An -tx1 | tr -d ' 
')-postgres_test"
evidence="$state/jank/$revision-$(date -u +%H%M%S)"
mkdir -p "$state/jank"
secret="$(mktemp -d)"
cleanup() {
  docker rm -f "$database" >/dev/null 2>&1 || true
  rm -rf "$secret"
}
trap cleanup EXIT
export PATH="$HOME/.cargo/bin:$PATH" CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-8}"
# The connections smoke's build of this checkout: restlessd is already compiled there.
export CARGO_TARGET_DIR="$state/connections-target"
if ! docker image inspect "$image" >/dev/null 2>&1; then
  IMAGE="$image" REVISION="$revision" bash scripts/release/with-company-builder.sh bash -c '
    source scripts/lib/dev-release.sh
    dev_build_company_image "$PWD" "$IMAGE" "$REVISION"'
fi
pinned="$(docker image inspect --format '{{index .RepoDigests 0}}' "$image" 2>/dev/null || true)"
[ -n "$pinned" ] || pinned="$image@$(docker image inspect --format '{{.Id}}' "$image")"

password="$(head -c 24 /dev/urandom | base64 | tr -dc 'A-Za-z0-9')"
# Remove databases left by runs that were killed before their trap ran; never a live run's.
docker ps -a --filter label=restless.jank-gate.pid --format '{{.Names}} {{.Label "restless.jank-gate.pid"}}'   | while read -r name pid; do kill -0 "$pid" 2>/dev/null || docker rm -f "$name" >/dev/null 2>&1 || true; done
docker run -d --name "$database" --label restless.jank-gate.pid=$$ -e POSTGRES_PASSWORD="$password" -p 127.0.0.1::5432 postgres:16 >/dev/null
port="$(docker port "$database" 5432/tcp | head -1 | cut -d: -f2)"
for _ in $(seq 1 30); do docker exec "$database" pg_isready -U postgres >/dev/null 2>&1 && break; sleep 2; done
(umask 077; printf 'postgres://postgres:%s@127.0.0.1:%s/postgres\n' "$password" "$port" >"$secret/database-url")

npm --prefix web ci --ignore-scripts --no-audit --no-fund >/dev/null
npx --prefix web playwright install chromium >/dev/null
RESTLESS_TEST_DATABASE_URL_FILE="$secret/database-url" RESTLESS_IDENTITY_TEST_OUTPUT="$evidence" \
  RESTLESS_COMPANY_IMAGE="$pinned" scripts/multiplayer-smoke --jank \
  || { echo "zero-jank gate failed; evidence in $evidence/jank (results.json, fail-*.png/json, film-*)" >&2; exit 1; }
mkdir -p "$passed" && printf '%s\n' "$revision" >"$passed/$tree"
find "$passed" -type f -printf '%T@ %p\n' | sort -rn | tail -n +41 | cut -d' ' -f2- | xargs -r rm -f
# Keep the last ten runs' evidence.
find "$state/jank" -mindepth 1 -maxdepth 1 -type d -printf '%T@ %p\n' | sort -rn | tail -n +11 | cut -d' ' -f2- | xargs -r rm -rf
echo "zero-jank gate passed; evidence in $evidence/jank"
