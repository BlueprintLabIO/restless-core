#!/usr/bin/env bash
# Install or upgrade the Restless appliance from one published Core release.
#
#   curl -fsSL https://raw.githubusercontent.com/BlueprintLabIO/restless-core/main/scripts/install-core.sh | bash
#
# Or, for one exact release: ... | bash -s -- [<revision>] [--environment <file>] [--force]
#
# <revision> is the 40-character source commit of a release published by the
# "Immutable Core release" workflow; without one, the newest such release on
# main. A fresh install on a host with no PostgreSQL and no
# ~/.restless/orgintel.toml runs its own PostgreSQL in a container. The script:
#   1. fetches that release's bundle from the registry and checks every byte
#      against its sha256 digest;
#   2. verifies the bundle's signed release manifest with Sigstore, accepting
#      only the trusted main release workflow as signer;
#   3. pulls the three images the manifest pins by digest;
#   4. copies the account-plane image's binaries, Cockpit and host tools out
#      of the image; and
#   5. hands them to `restless appliance install` (or `upgrade` when an
#      appliance is already installed), which drains work, activates the
#      release under systemd --user and rolls back if it does not become ready.
#
# Registry access: packages that are not public need `docker login ghcr.io`
# and GHCR_TOKEN (a token with read:packages) in the environment.
set -euo pipefail

GITHUB_REPOSITORY="BlueprintLabIO/restless-core"
# The bundled database: the official image's multi-arch index, pinned.
POSTGRES_IMAGE='postgres:17-alpine@sha256:b0f9560a2de083e2cc7382e75f808c7381a32852a7ec49117deedb300e552b24'
POSTGRES_CONTAINER='restless-stable-postgres'
POSTGRES_PORT=7797
REGISTRY="ghcr.io"
REPOSITORY="blueprintlabio/restless-core-release"
# The same pinned verifier the release workflow signs and verifies with.
COSIGN_IMAGE='ghcr.io/sigstore/cosign/cosign:v2.4.1@sha256:b03690aa52bfe94054187142fba24dc54137650682810633901767d8a3e15b31'
SIGNER='https://github.com/BlueprintLabIO/restless-core/.github/workflows/immutable-core-release.yml@refs/heads/main'
ISSUER='https://token.actions.githubusercontent.com'
BUNDLE_MEDIA_TYPE='application/vnd.restless.core.release-bundle.v1+tar+gzip'

fail() { printf 'install-core: %s\n' "$*" >&2; exit 1; }
step() { printf '\n==> %s\n' "$*" >&2; }

usage() {
  sed -n '2,5p' "${BASH_SOURCE[0]:-/dev/null}" 2>/dev/null >&2 || true
  printf 'usage: install-core.sh [<revision>] [--environment <file>] [--force]\n' >&2
  exit 2
}

require_tools() {
  [ "$(uname -s)" = Linux ] || fail "released Core binaries are Linux builds; on macOS use a source install"
  local tool
  for tool in curl docker jq tar sha256sum systemctl; do
    command -v "$tool" >/dev/null 2>&1 || fail "$tool is required"
  done
  docker info >/dev/null 2>&1 || fail "Docker is not reachable by this user"
}

host_platform() {
  case "$(uname -m)" in
    x86_64 | amd64) printf 'amd64\n' ;;
    aarch64 | arm64) printf 'arm64\n' ;;
    *) fail "unsupported CPU architecture $(uname -m)" ;;
  esac
}

# The newest successful release on main: the source commit its workflow run built.
latest_revision() {
  curl -fsS "https://api.github.com/repos/${GITHUB_REPOSITORY}/actions/workflows/immutable-core-release.yml/runs?branch=main&status=success&per_page=1" \
    | jq -er '.workflow_runs[0].head_sha' \
    || fail "could not find the latest release; pass a revision"
}

# A fresh install on a host with no database of its own gets one: PostgreSQL in a
# container on 127.0.0.1:7797, restarted with Docker, its data in a named volume
# and its password only in ~/.restless (mode 0600). An existing orgintel.toml,
# RESTLESS_PLANE_DATABASE_URL or a server on localhost:5432 (the appliance's
# default) is kept as it is.
ensure_database() {
  local state="${HOME}/.restless" password attempt
  if [ -n "${RESTLESS_PLANE_DATABASE_URL:-}" ] || [ -f "$state/orgintel.toml" ]; then return 0; fi
  if (exec 3<>/dev/tcp/127.0.0.1/5432) 2>/dev/null; then
    printf 'using the PostgreSQL on localhost:5432\n' >&2
    return 0
  fi
  mkdir -p "$state" && chmod 700 "$state"
  if ! docker container inspect "$POSTGRES_CONTAINER" >/dev/null 2>&1; then
    password="$(od -An -tx1 -N32 /dev/urandom | tr -d ' \n')"
    (umask 077 && printf 'POSTGRES_USER=restless\nPOSTGRES_DB=restless\nPOSTGRES_PASSWORD=%s\n' "$password" >"$state/postgres.env")
    docker run --detach --name "$POSTGRES_CONTAINER" --restart unless-stopped \
      --label io.restless.profile=stable \
      --publish "127.0.0.1:${POSTGRES_PORT}:5432" \
      --env-file "$state/postgres.env" \
      --volume "${POSTGRES_CONTAINER}-data:/var/lib/postgresql/data" \
      "$POSTGRES_IMAGE" >/dev/null
  else
    docker start "$POSTGRES_CONTAINER" >/dev/null
    password="$(sed -n 's/^POSTGRES_PASSWORD=//p' "$state/postgres.env" 2>/dev/null || true)"
    [ -n "$password" ] || fail "${POSTGRES_CONTAINER} exists but ~/.restless/postgres.env does not; remove the container or write orgintel.toml"
  fi
  for ((attempt = 0; attempt < 120; attempt += 1)); do
    if docker exec "$POSTGRES_CONTAINER" pg_isready -h 127.0.0.1 -U restless -d restless >/dev/null 2>&1; then
      (umask 077 && printf 'database_url = "postgres://restless:%s@127.0.0.1:%s/restless"\n' "$password" "$POSTGRES_PORT" >"$state/orgintel.toml")
      printf 'PostgreSQL is ready (container %s)\n' "$POSTGRES_CONTAINER" >&2
      return 0
    fi
    sleep 0.5
  done
  fail "PostgreSQL did not become ready; inspect: docker logs ${POSTGRES_CONTAINER}"
}

# Keep the user service running after logout. Usually allowed for one's own
# user; otherwise say the one command that does it.
ensure_linger() {
  [ "$(loginctl show-user "$USER" --property=Linger --value 2>/dev/null)" = yes ] && return 0
  loginctl enable-linger "$USER" 2>/dev/null && return 0
  printf 'note: Restless stops when you log out until you run: sudo loginctl enable-linger %s\n' "$USER" >&2
}

# A bearer token for pulling from the registry: anonymous for public packages,
# GHCR_TOKEN otherwise.
registry_token() {
  local auth=()
  if [ -n "${GHCR_TOKEN:-}" ]; then auth=(-u "${GHCR_USER:-token}:${GHCR_TOKEN}"); fi
  curl -fsS "${auth[@]}" "https://${REGISTRY}/token?service=${REGISTRY}&scope=repository:${REPOSITORY}:pull" \
    | jq -er '.token'
}

# Fetch the bundle for <revision>-<platform>, verifying the manifest and the
# layer against their content digests. Prints the bundle digest.
fetch_bundle() {
  local revision="$1" platform="$2" work="$3" token manifest_digest layer
  token="$(registry_token)" || fail "could not get registry access for ${REPOSITORY}; set GHCR_TOKEN if the package is private"
  curl -fsS -H "Authorization: Bearer ${token}" \
    -H 'Accept: application/vnd.oci.image.manifest.v1+json' \
    -o "$work/bundle-manifest.json" \
    "https://${REGISTRY}/v2/${REPOSITORY}/manifests/${revision}-${platform}" \
    || fail "no published Core release for ${revision} on linux/${platform}"
  manifest_digest="sha256:$(sha256sum "$work/bundle-manifest.json" | cut -d' ' -f1)"
  layer="$(jq -er --arg type "$BUNDLE_MEDIA_TYPE" '[.layers[] | select(.mediaType == $type)] | if length == 1 then .[0].digest else error("expected one bundle layer") end' "$work/bundle-manifest.json")" \
    || fail "the release bundle manifest is not a Core release bundle"
  [[ "$layer" =~ ^sha256:[0-9a-f]{64}$ ]] || fail "the bundle layer digest is malformed"
  curl -fsSL -H "Authorization: Bearer ${token}" -o "$work/bundle.tar.gz" \
    "https://${REGISTRY}/v2/${REPOSITORY}/blobs/${layer}" || fail "could not download the release bundle"
  [ "sha256:$(sha256sum "$work/bundle.tar.gz" | cut -d' ' -f1)" = "$layer" ] \
    || fail "the downloaded bundle does not match its digest ${layer}"
  mkdir -p "$work/bundle"
  tar -xzf "$work/bundle.tar.gz" --no-same-owner --no-same-permissions -C "$work/bundle"
  printf '%s@%s\n' "${REGISTRY}/${REPOSITORY}" "$manifest_digest"
}

# Verify the signed release manifest and print its path inside the bundle.
verify_release() {
  local revision="$1" platform="$2" bundle="$3" index manifest signature expected
  index="$bundle/core-release-bundle.json"
  [ -f "$index" ] || fail "the bundle has no core-release-bundle.json"
  manifest="$(jq -er '.release_manifest.path' "$index")"
  signature="$(jq -er '.release_manifest.signature.path' "$index")"
  expected="$(jq -er '.release_manifest.sha256' "$index")"
  case "$manifest$signature" in *..* | /*) fail "the bundle index names an unsafe path" ;; esac
  [ "sha256:$(sha256sum "$bundle/$manifest" | cut -d' ' -f1)" = "$expected" ] \
    || fail "the release manifest does not match the bundle index"
  docker run --rm -v "$bundle:/work:ro" "$COSIGN_IMAGE" verify-blob \
    --bundle "/work/$signature" --certificate-identity "$SIGNER" --certificate-oidc-issuer "$ISSUER" \
    "/work/$manifest" >&2 \
    || fail "the release manifest signature did not verify against ${SIGNER}"
  jq -e --arg revision "$revision" --arg platform "linux/$platform" \
    '.source_revision == $revision and ((.platforms // ["linux/amd64","linux/arm64"]) | index($platform))' \
    "$bundle/$manifest" >/dev/null \
    || fail "the signed manifest is not release ${revision} for linux/${platform}"
  printf '%s\n' "$bundle/$manifest"
}

# Copy the appliance's files out of the account-plane image.
stage_account_plane() {
  local image="$1" stage="$2" tools="$3" container omp bun
  container="restless-install-$$"
  docker create --name "$container" "$image" >/dev/null
  STAGING_CONTAINER="$container"
  mkdir -p "$stage/bin" "$tools.partial/bin"
  docker cp "$container:/usr/local/bin/restlessd" "$stage/bin/restlessd"
  docker cp "$container:/usr/local/bin/restless" "$stage/bin/restless" 2>/dev/null \
    || fail "this release's account-plane image has no restless CLI; install a newer release"
  docker cp "$container:/opt/restless/cockpit" "$stage/cockpit"
  docker cp "$container:/opt/restless/native-sheets" "$tools.partial/native-sheets"
  docker cp -L "$container:/usr/local/bin/node" "$tools.partial/bin/node"
  # The model broker: Bun plus the globally installed OMP package.
  read -r omp bun < <(docker run --rm --entrypoint /bin/sh "$image" -c \
    'printf "%s %s\n" "$(readlink -f /usr/local/bin/omp)" "$(readlink -f "$(command -v bun)")"')
  [ -n "$omp" ] && [ -n "$bun" ] || fail "the account-plane image has no model broker"
  case "$omp" in /usr/local/bun/*) ;; *) fail "unexpected model broker location $omp" ;; esac
  docker cp "$container:/usr/local/bun" "$tools.partial/bun"
  docker cp "$container:$bun" "$tools.partial/bin/bun"
  local entry="$tools/bun/${omp#/usr/local/bun/}"
  if [ "$(head -c 4 "$tools.partial/bun/${omp#/usr/local/bun/}" | od -An -c | tr -d ' ')" = '177ELF' ]; then
    printf '#!/bin/sh\nexec "%s" "$@"\n' "$entry" >"$tools.partial/bin/omp"
  else
    printf '#!/bin/sh\nexec "%s" "%s" "$@"\n' "$tools/bin/bun" "$entry" >"$tools.partial/bin/omp"
  fi
  chmod 0755 "$tools.partial/bin/omp" "$tools.partial/bin/bun" "$tools.partial/bin/node"
  docker rm -f "$container" >/dev/null
  STAGING_CONTAINER=""
  rm -rf "$tools"
  mv "$tools.partial" "$tools"
}

STAGING_CONTAINER=""
cleanup() {
  [ -z "$STAGING_CONTAINER" ] || docker rm -f "$STAGING_CONTAINER" >/dev/null 2>&1 || true
  [ -z "${WORK:-}" ] || rm -rf "$WORK"
}

write_release_environment() {
  local revision="$1" manifest="$2" tools="$3" output="$4"
  {
    printf 'RESTLESS_SOURCE_COMMIT=%s\n' "$revision"
    printf 'RESTLESS_COMPANY_IMAGE=%s\n' "$(jq -er '.images.company_runtime' "$manifest")"
    printf 'RESTLESS_NATIVE_DOCUMENTS_IMAGE=%s\n' "$(jq -er '.images.native_documents' "$manifest")"
    printf 'RESTLESS_OMP_BIN=%s\n' "$tools/bin/omp"
    printf 'RESTLESS_BUN_BIN=%s\n' "$tools/bin/bun"
    printf 'RESTLESS_SHEETS_WORKER=%s\n' "$tools/native-sheets/src/worker.mjs"
    printf 'RESTLESS_NODE_BIN=%s\n' "$tools/bin/node"
  } >"$output"
}

main() {
  local revision="" appliance_args=()
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --environment) [ "$#" -ge 2 ] || usage; appliance_args+=(--environment "$(realpath "$2")"); shift ;;
      --force) appliance_args+=(--force) ;;
      -h | --help) usage ;;
      -*) usage ;;
      *) [ -z "$revision" ] || usage; revision="$1" ;;
    esac
    shift
  done
  require_tools
  if [ -z "$revision" ]; then
    step "Finding the latest release"
    revision="$(latest_revision)"
    printf 'latest is %s\n' "$revision" >&2
  fi
  [[ "$revision" =~ ^[0-9a-f]{40}$ ]] || usage
  local platform work bundle manifest tools image
  platform="$(host_platform)"
  work="$(mktemp -d)"
  WORK="$work"
  trap cleanup EXIT

  step "Fetching Core ${revision:0:12} for linux/${platform}"
  bundle="$(fetch_bundle "$revision" "$platform" "$work")"
  printf 'bundle %s (digest verified)\n' "$bundle" >&2

  step "Verifying the release signature"
  manifest="$(verify_release "$revision" "$platform" "$work/bundle")"
  printf 'signed by %s\n' "$SIGNER" >&2

  step "Pulling the pinned images"
  for image in account_plane company_runtime native_documents; do
    docker pull --quiet "$(jq -er ".images.${image}" "$manifest")" >&2
  done

  step "Staging the appliance"
  tools="${HOME}/.local/lib/restless/host-tools/release-${revision:0:12}"
  mkdir -p "$(dirname "$tools")"
  stage_account_plane "$(jq -er '.images.account_plane' "$manifest")" "$work/stage" "$tools"
  write_release_environment "$revision" "$manifest" "$tools" "$work/stage/release.env"

  local verb=install
  [ -e "${HOME}/.local/lib/restless/current" ] && verb=upgrade
  if [ "$verb" = install ]; then
    step "Preparing the database"
    ensure_database
    ensure_linger
  fi
  step "Activating (appliance ${verb})"
  env -u RESTLESS_HOME -u RESTLESS_PORT_OFFSET -u RESTLESS_RESOURCE_NAMESPACE \
    -u RESTLESS_COMPANY_IMAGE -u RESTLESS_NATIVE_DOCUMENTS_IMAGE -u RESTLESS_OWNER_URL \
    RESTLESS_PROFILE=stable \
    "$work/stage/bin/restless" appliance "$verb" \
      --daemon "$work/stage/bin/restlessd" \
      --cockpit "$work/stage/cockpit" \
      --release-environment "$work/stage/release.env" \
      "${appliance_args[@]}"
  printf '\nRestless %s is active. Open the Cockpit with: restless open\n' "${revision:0:12}" >&2
}

# Sourcing defines the functions without installing anything.
if [ "${BASH_SOURCE[0]:-$0}" = "$0" ]; then
  main "$@"
fi
