#!/usr/bin/env bash
# Core's nightly smokes on this computer, with no GitHub (ADR 0010 in restless-cloud): what
# connections-smoke.yml and multiplayer-smoke.yml ran on the self-hosted runner.
#   1. the connections smoke with the skills smoke, on a fresh build of the commit;
#   2. the multiplayer smoke (owner and two members), against a disposable `_test` PostgreSQL.
# The commit is this checkout's HEAD (or $RESTLESS_NIGHTLY_REF), tested in a detached worktree so
# the working copy is never touched. Evidence and a summary go to
# ~/.local/state/restless-release/nightly/<date>-<sha>/; the exit status says whether both passed.
#
#   scripts/release/local/nightly.sh            (scheduled by restless-core-nightly.timer)
set -uo pipefail
root="$(cd "$(dirname "$0")/../../.." && pwd)"
ref="${RESTLESS_NIGHTLY_REF:-HEAD}"
revision="$(git -C "$root" rev-parse "$ref^{commit}")" || exit 1
state="${XDG_STATE_HOME:-$HOME/.local/state}/restless-release"
out="$state/nightly/$(date -u +%F)-${revision:0:12}"
src="$state/nightly-src"
image="restless-company-image:nightly"
database="restless-nightly-postgres_test"
mkdir -p "$out"
exec > >(tee -a "$out/nightly.log") 2>&1
log() { printf '\n== %s  %s\n' "$(date -u +%H:%M:%S)" "$*"; }
cleanup() {
  docker rm -f "$database" >/dev/null 2>&1 || true
  rm -f "$out/database-url"
  git -C "$root" worktree remove --force "$src" >/dev/null 2>&1 || true
}
trap cleanup EXIT
export PATH="$HOME/.cargo/bin:$PATH" CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-6}"
export CARGO_TARGET_DIR="$state/nightly-target"

log "Core $revision"
git -C "$root" worktree remove --force "$src" >/dev/null 2>&1 || true
git -C "$root" worktree add --detach "$src" "$revision" >/dev/null || exit 1
cd "$src"

connections=fail multiplayer=fail
log "build"
if scripts/restless-cargo build -p restlessd -p restless \
  && npm --prefix infra/host-tools ci --no-audit --no-fund >/dev/null \
  && IMAGE="$image" REVISION="$revision" bash scripts/release/with-company-builder.sh bash -c '
    source scripts/lib/dev-release.sh
    dev_build_company_image "$PWD" "$IMAGE" "$REVISION"'; then
  log "connections smoke (with skills)"
  RESTLESSD_BIN="$CARGO_TARGET_DIR/debug/restlessd" RESTLESS_CLI_BIN="$CARGO_TARGET_DIR/debug/restless" \
    RESTLESS_COMPANY_IMAGE="$image" python3 scripts/connections-smoke --target core --with-skills \
    --output "$out/connections" && connections=pass

  log "multiplayer smoke"
  password="$(head -c 24 /dev/urandom | base64 | tr -dc 'A-Za-z0-9')"
  docker rm -f "$database" >/dev/null 2>&1 || true
  if docker run -d --name "$database" -e POSTGRES_PASSWORD="$password" -p 127.0.0.1::5432 postgres:16 >/dev/null; then
    port="$(docker port "$database" 5432/tcp | head -1 | cut -d: -f2)"
    for _ in $(seq 1 30); do docker exec "$database" pg_isready -U postgres >/dev/null 2>&1 && break; sleep 2; done
    (umask 077; printf 'postgres://postgres:%s@127.0.0.1:%s/postgres\n' "$password" "$port" >"$out/database-url")
    pinned="$(docker image inspect --format '{{index .RepoDigests 0}}' "$image" 2>/dev/null)"
    [ -n "$pinned" ] || pinned="$image@$(docker image inspect --format '{{.Id}}' "$image")"
    npm --prefix web ci --ignore-scripts >/dev/null && npx --prefix web playwright install chromium >/dev/null \
      && npm --prefix services/native-sheets ci --no-audit --no-fund >/dev/null \
      && RESTLESS_TEST_DATABASE_URL_FILE="$out/database-url" RESTLESS_IDENTITY_TEST_OUTPUT="$out/multiplayer" \
        RESTLESS_COMPANY_IMAGE="$pinned" scripts/multiplayer-smoke && multiplayer=pass
  fi
fi
docker image rm "$image" >/dev/null 2>&1 || true
printf '{"revision":"%s","connections":"%s","multiplayer":"%s","finished":"%s"}\n' \
  "$revision" "$connections" "$multiplayer" "$(date -u +%FT%TZ)" | tee "$out/summary.json"
[ "$connections" = pass ] && [ "$multiplayer" = pass ]
