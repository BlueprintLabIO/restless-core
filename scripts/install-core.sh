#!/usr/bin/env bash
# Install or upgrade the Restless appliance from one published Core release.
#
#   curl -fsSL https://restless.run/install | bash
#
# (restless.run/install redirects to this file on main.)
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
#      of the image (on macOS: verifies the release's signed macOS binaries and
#      fetches Node, Bun and the model broker for macOS at the image's versions); and
#   5. hands them to `restless appliance install` (or `upgrade` when an
#      appliance is already installed), which drains work, activates the
#      release under systemd --user (launchd on macOS) and rolls back if it does
#      not become ready;
#   6. opens the Cockpit, or on a server prints the port forward that reaches it.
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
# macOS: the same release's restless and restlessd, built and signed by the macOS release workflow
# and published beside the Linux bundle as <revision>-darwin-<arm64|x86_64>.
MACOS_SIGNER='https://github.com/BlueprintLabIO/restless-core/.github/workflows/macos-core-release.yml@refs/heads/main'
MACOS_MEDIA_TYPE='application/vnd.restless.core.macos-binaries.v1+tar+gzip'
SIGSTORE_MEDIA_TYPE='application/vnd.dev.sigstore.bundle.v0.3+json'

fail() { printf 'install-core: %s\n' "$*" >&2; exit 1; }
step() { printf '\n==> %s\n' "$*" >&2; }
is_macos() { [ "$(uname -s)" = Darwin ]; }
# sha256 hex of a file on Linux (coreutils) and macOS (shasum).
sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d' ' -f1
}

usage() {
  sed -n '2,5p' "${BASH_SOURCE[0]:-/dev/null}" 2>/dev/null >&2 || true
  printf 'usage: install-core.sh [<revision>] [--environment <file>] [--force]\n' >&2
  exit 2
}

require_tools() {
  local tool tools
  case "$(uname -s)" in
    Linux) tools="curl docker jq tar sha256sum systemctl" ;;
    Darwin) tools="curl docker jq tar shasum unzip launchctl" ;;
    *) fail "Restless installs on Linux and macOS; on Windows, run this in an Ubuntu (WSL2) terminal" ;;
  esac
  for tool in $tools; do
    command -v "$tool" >/dev/null 2>&1 && continue
    if is_macos && [ "$tool" = jq ]; then fail "jq is required: brew install jq"; fi
    if is_macos && [ "$tool" = docker ]; then fail "Docker is required: install Docker Desktop or OrbStack and start it"; fi
    fail "$tool is required"
  done
  docker info >/dev/null 2>&1 || fail "Docker is not reachable by this user; is Docker running?"
}

host_platform() {
  case "$(uname -m)" in
    x86_64 | amd64) printf 'amd64\n' ;;
    aarch64 | arm64) printf 'arm64\n' ;;
    *) fail "unsupported CPU architecture $(uname -m)" ;;
  esac
}

# Whether the registry holds <revision>-<suffix> (a Linux bundle or a macOS build).
published() {
  curl -fsS -o /dev/null -H "Authorization: Bearer $1" -H 'Accept: application/vnd.oci.image.manifest.v1+json' \
    "https://${REGISTRY}/v2/${REPOSITORY}/manifests/$2-$3"
}

# The newest successful release on main that this machine can install. Each platform is published
# by its own run (linux/amd64, then linux/arm64, then macOS a few minutes later), so take the newest
# release that already has this machine's build.
latest_revision() {
  local revisions revision token wanted
  revisions="$(curl -fsS "https://api.github.com/repos/${GITHUB_REPOSITORY}/actions/workflows/immutable-core-release.yml/runs?branch=main&status=success&per_page=10" \
    | jq -er 'reduce .workflow_runs[].head_sha as $s ([]; if any(.[]; . == $s) then . else . + [$s] end) | .[]')" \
    || fail "could not find the latest release; pass a revision"
  if is_macos; then wanted="darwin-$(uname -m)"; else wanted="$(host_platform)"; fi
  token="$(registry_token)" || fail "could not get registry access for ${REPOSITORY}"
  for revision in $revisions; do
    if published "$token" "$revision" "$wanted"; then
      printf '%s\n' "$revision"
      return 0
    fi
  done
  fail "no recent release is built for ${wanted} yet; try again in a few minutes"
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
  is_macos && return 0 # launchd keeps a user agent running while the user is logged in.
  [ "$(loginctl show-user "$USER" --property=Linger --value 2>/dev/null)" = yes ] && return 0
  loginctl enable-linger "$USER" 2>/dev/null && return 0
  printf 'note: Restless stops when you log out until you run: sudo loginctl enable-linger %s\n' "$USER" >&2
}

# A bearer token for pulling from the registry: anonymous for public packages,
# GHCR_TOKEN otherwise.
registry_token() {
  local auth=()
  if [ -n "${GHCR_TOKEN:-}" ]; then auth=(-u "${GHCR_USER:-token}:${GHCR_TOKEN}"); fi
  # ${a[@]+...}: macOS's bash 3.2 treats an empty array as unset under set -u.
  curl -fsS ${auth[@]+"${auth[@]}"} "https://${REGISTRY}/token?service=${REGISTRY}&scope=repository:${REPOSITORY}:pull" \
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
  manifest_digest="sha256:$(sha256_of "$work/bundle-manifest.json")"
  layer="$(jq -er --arg type "$BUNDLE_MEDIA_TYPE" '[.layers[] | select(.mediaType == $type)] | if length == 1 then .[0].digest else error("expected one bundle layer") end' "$work/bundle-manifest.json")" \
    || fail "the release bundle manifest is not a Core release bundle"
  [[ "$layer" =~ ^sha256:[0-9a-f]{64}$ ]] || fail "the bundle layer digest is malformed"
  curl -fsSL -H "Authorization: Bearer ${token}" -o "$work/bundle.tar.gz" \
    "https://${REGISTRY}/v2/${REPOSITORY}/blobs/${layer}" || fail "could not download the release bundle"
  [ "sha256:$(sha256_of "$work/bundle.tar.gz")" = "$layer" ] \
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
  [ "sha256:$(sha256_of "$bundle/$manifest")" = "$expected" ] \
    || fail "the release manifest does not match the bundle index"
  # As this user: the bundle sits in a private (0700) temporary directory that
  # the image's own non-root user cannot read.
  docker run --rm -u "$(id -u):$(id -g)" -e HOME=/tmp -v "$bundle:/work:ro" "$COSIGN_IMAGE" verify-blob \
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

# macOS: fetch this release's signed restless and restlessd into <stage>/bin. The artifact holds the
# binaries' tarball and its Sigstore bundle; both are checked against their digests, then the
# signature must come from the trusted macOS release workflow on main.
fetch_macos_binaries() {
  local revision="$1" arch="$2" work="$3" stage="$4" token manifest binaries signature
  token="$(registry_token)" || fail "could not get registry access for ${REPOSITORY}"
  manifest="$work/macos-manifest.json"
  curl -fsS -H "Authorization: Bearer ${token}" -H 'Accept: application/vnd.oci.image.manifest.v1+json' \
    -o "$manifest" "https://${REGISTRY}/v2/${REPOSITORY}/manifests/${revision}-darwin-${arch}" \
    || fail "no macOS build of ${revision:0:12} for ${arch} yet; it is published a few minutes after each release"
  mkdir -p "$work/macos"
  local type file digest
  for type in "$MACOS_MEDIA_TYPE" "$SIGSTORE_MEDIA_TYPE"; do
    digest="$(jq -er --arg type "$type" '[.layers[] | select(.mediaType == $type)] | if length == 1 then .[0].digest else error("expected one layer") end' "$manifest")" \
      || fail "the macOS artifact is missing its ${type} layer"
    [[ "$digest" =~ ^sha256:[0-9a-f]{64}$ ]] || fail "a macOS artifact digest is malformed"
    if [ "$type" = "$MACOS_MEDIA_TYPE" ]; then file="$work/macos/binaries.tar.gz"; else file="$work/macos/binaries.sigstore.json"; fi
    curl -fsSL -H "Authorization: Bearer ${token}" -o "$file" "https://${REGISTRY}/v2/${REPOSITORY}/blobs/${digest}" \
      || fail "could not download the macOS artifact"
    [ "sha256:$(sha256_of "$file")" = "$digest" ] || fail "the macOS artifact does not match its digest ${digest}"
  done
  docker run --rm -u "$(id -u):$(id -g)" -e HOME=/tmp -v "$work/macos:/work:ro" "$COSIGN_IMAGE" verify-blob \
    --bundle /work/binaries.sigstore.json --certificate-identity "$MACOS_SIGNER" --certificate-oidc-issuer "$ISSUER" \
    /work/binaries.tar.gz >&2 \
    || fail "the macOS binaries' signature did not verify against ${MACOS_SIGNER}"
  mkdir -p "$stage/bin" "$work/macos/out"
  tar -xzf "$work/macos/binaries.tar.gz" --no-same-owner -C "$work/macos/out" restless restlessd release.json \
    || fail "the macOS artifact does not hold restless, restlessd and release.json"
  # release.json is inside the signed archive: refuse a build of any other release or CPU.
  jq -e --arg revision "$revision" --arg arch "$arch" '.source_revision == $revision and .arch == $arch' \
    "$work/macos/out/release.json" >/dev/null \
    || fail "the macOS binaries were not built from release ${revision:0:12} for ${arch}"
  mv "$work/macos/out/restless" "$work/macos/out/restlessd" "$stage/bin/"
  chmod 0755 "$stage/bin/restless" "$stage/bin/restlessd"
}

# macOS: the Cockpit and the sheets worker's source come from the release's account-plane image;
# Node, Bun and the model broker are fetched for macOS at the exact versions that image pins, and
# the sheets worker's dependencies are installed for macOS (some of them are native code).
stage_macos_host_tools() {
  local image="$1" stage="$2" tools="$3" work="$4" container versions node_version bun_version omp_version
  local node_arch bun_arch
  case "$(uname -m)" in
    arm64) node_arch=arm64; bun_arch=aarch64 ;;
    x86_64) node_arch=x64; bun_arch=x64 ;;
    *) fail "unsupported CPU architecture $(uname -m)" ;;
  esac
  versions="$(docker run --rm --entrypoint /bin/sh "$image" -c 'printf "%s %s %s\n" "$(node --version)" "$(bun --version)" "$(omp --version)"')"
  read -r node_version bun_version omp_version <<<"$versions"
  node_version="${node_version#v}"
  omp_version="${omp_version#omp/}"
  [[ "$node_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ && "$bun_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ && "$omp_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] \
    || fail "could not read the pinned Node, Bun and model broker versions from the release image"

  container="restless-install-$$"
  docker create --name "$container" "$image" >/dev/null
  STAGING_CONTAINER="$container"
  rm -rf "$tools.partial"
  mkdir -p "$stage" "$tools.partial/bin" "$tools.partial/native-sheets"
  docker cp "$container:/opt/restless/cockpit" "$stage/cockpit"
  local part
  for part in package.json package-lock.json src NOTICE; do
    docker cp "$container:/opt/restless/native-sheets/$part" "$tools.partial/native-sheets/$part"
  done
  docker rm -f "$container" >/dev/null
  STAGING_CONTAINER=""

  # Node, checked against the release's published SHASUMS256.
  local node_name="node-v${node_version}-darwin-${node_arch}"
  curl -fsSL -o "$work/$node_name.tar.gz" "https://nodejs.org/dist/v${node_version}/${node_name}.tar.gz" \
    || fail "could not download Node ${node_version}"
  curl -fsSL "https://nodejs.org/dist/v${node_version}/SHASUMS256.txt" | grep -q "^$(sha256_of "$work/$node_name.tar.gz")  ${node_name}.tar.gz\$" \
    || fail "Node ${node_version} does not match its published checksum"
  tar -xzf "$work/$node_name.tar.gz" -C "$work"
  cp "$work/$node_name/bin/node" "$tools.partial/bin/node"
  PATH="$work/$node_name/bin:$PATH" "$work/$node_name/bin/npm" --prefix "$tools.partial/native-sheets" \
    ci --omit=dev --no-audit --no-fund >&2 || fail "could not install the sheets worker's dependencies"

  # Bun, checked against the release's published SHASUMS256, then the model broker.
  local bun_zip="bun-darwin-${bun_arch}.zip" bun_url="https://github.com/oven-sh/bun/releases/download/bun-v${bun_version}"
  curl -fsSL -o "$work/$bun_zip" "$bun_url/$bun_zip" || fail "could not download Bun ${bun_version}"
  curl -fsSL "$bun_url/SHASUMS256.txt" | grep -q "^$(sha256_of "$work/$bun_zip")  ${bun_zip}\$" \
    || fail "Bun ${bun_version} does not match its published checksum"
  unzip -q -o "$work/$bun_zip" -d "$work"
  cp "$work/bun-darwin-${bun_arch}/bun" "$tools.partial/bin/bun"
  chmod 0755 "$tools.partial/bin/node" "$tools.partial/bin/bun"
  BUN_INSTALL="$tools.partial/bun" "$tools.partial/bin/bun" install --global "@oh-my-pi/pi-coding-agent@${omp_version}" >&2 \
    || fail "could not install the model broker ${omp_version}"
  local package="$tools.partial/bun/install/global/node_modules/@oh-my-pi/pi-coding-agent" entry
  entry="$(jq -er 'if (.bin | type) == "string" then .bin else (.bin.omp // first(.bin[])) end' "$package/package.json")" \
    || fail "the model broker package names no executable"
  # The wrapper names final paths: the tools directory is renamed into place below.
  printf '#!/bin/sh\nexec "%s" "%s" "$@"\n' "$tools/bin/bun" "$tools/bun/install/global/node_modules/@oh-my-pi/pi-coding-agent/${entry#./}" \
    >"$tools.partial/bin/omp"
  chmod 0755 "$tools.partial/bin/omp"
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
  # On a Mac the company computers are Linux containers in Docker's VM. Apple silicon uses this
  # release's linux/arm64 images when they are published, and otherwise linux/amd64 through Rosetta.
  if is_macos && [ "$platform" = arm64 ] && ! published "$(registry_token)" "$revision" arm64; then platform=amd64; fi
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
  if is_macos; then
    fetch_macos_binaries "$revision" "$(uname -m)" "$work" "$work/stage"
    stage_macos_host_tools "$(jq -er '.images.account_plane' "$manifest")" "$work/stage" "$tools" "$work"
  else
    stage_account_plane "$(jq -er '.images.account_plane' "$manifest")" "$work/stage" "$tools"
  fi
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
      ${appliance_args[@]+"${appliance_args[@]}"}
  open_cockpit "${revision:0:12}"
}

# The install ends in the product: on a desktop the Cockpit opens; on a server
# the address and the port forward that reaches it are printed instead.
open_cockpit() {
  local url="http://127.0.0.1:7788"
  printf '\nRestless %s is running at %s\n' "$1" "$url" >&2
  if is_macos; then
    open "$url" >/dev/null 2>&1 || printf 'Open %s in your browser.\n' "$url" >&2
    return 0
  fi
  if [ -n "${DISPLAY:-}${WAYLAND_DISPLAY:-}" ] && command -v xdg-open >/dev/null 2>&1; then
    xdg-open "$url" >/dev/null 2>&1 &
    return 0
  fi
  printf 'From another computer: ssh -L 7788:127.0.0.1:7788 %s@%s, then open %s\n' \
    "$USER" "$(hostname -f 2>/dev/null || hostname)" "$url" >&2
  printf 'Later, on this machine: restless open\n' >&2
}

# Sourcing defines the functions without installing anything.
if [ "${BASH_SOURCE[0]:-$0}" = "$0" ]; then
  main "$@"
fi
