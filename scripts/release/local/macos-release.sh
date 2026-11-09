#!/usr/bin/env bash
# The macOS half of a Core release, run on the owner's Mac (macOS binaries need a Mac; ADR 0010 in
# restless-cloud retired the GitHub job that built them). For one published Linux release this:
#   1. builds restless and restlessd for Apple silicon and Intel from exactly that commit;
#   2. seals the commit and CPU into the archive and signs it with the release key;
#   3. checks the signature the way the installer will, and publishes it beside the Linux bundle
#      as <revision>-darwin-<arch>; and
#   4. verifies it from registry.restless.run, then moves `latest-darwin-<arch>`, which is what
#      a Mac's installer follows.
#
#   infisical run --projectId <id> --env prod --domain https://eu.infisical.com/api -- \
#     scripts/release/local/macos-release.sh [<revision>]
#
# Without a revision it builds the newest published Linux release. A build already published is
# left as it is. Needs Rust (rustup), Docker, Node, jq, the restless-cloud checkout beside this one
# ($RESTLESS_CLOUD_DIR, default ~/code/restless-cloud, with `npm ci` done if its registry scripts
# need it), and from Infisical RELEASE_SIGNING_KEY, RELEASE_SIGNING_PASSWORD,
# CLOUDFLARE_RELEASE_TOKEN and CLOUDFLARE_ACCOUNT_ID. Runs on macOS's bash 3.2.
set -euo pipefail
root="$(cd "$(dirname "$0")/../../.." && pwd)"
cloud="${RESTLESS_CLOUD_DIR:-$HOME/code/restless-cloud}"
target_dir="${RESTLESS_MACOS_TARGET_DIR:-$HOME/.cache/restless-macos-target}"
[ "$(uname -s)" = Darwin ] || { echo 'macos-release: this has to run on a Mac' >&2; exit 2; }
: "${RELEASE_SIGNING_KEY:?run under infisical run}" "${RELEASE_SIGNING_PASSWORD:?run under infisical run}"
: "${CLOUDFLARE_RELEASE_TOKEN:?run under infisical run}" "${CLOUDFLARE_ACCOUNT_ID:?run under infisical run}"
[ -f "$cloud/scripts/registry/push.mjs" ] || { echo "macos-release: no restless-cloud checkout at $cloud" >&2; exit 2; }

# The installer's own digest checks, signature check and download of a macOS build.
# shellcheck source=../../install-core.sh
. "$root/scripts/install-core.sh"
log() { printf '\n== %s  %s\n' "$(date -u +%H:%M:%S)" "$*"; }

WORK="$(mktemp -d)"
source_tree="$WORK/source"
cleanup_release() {
  git -C "$root" worktree remove --force "$source_tree" >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup_release EXIT

# with-r2.sh derives its secret with sha256sum, which not every macOS has.
if ! command -v sha256sum >/dev/null 2>&1; then
  mkdir -p "$WORK/shim"
  printf '#!/bin/sh\nexec shasum -a 256 "$@"\n' >"$WORK/shim/sha256sum"
  chmod +x "$WORK/shim/sha256sum"
  export PATH="$WORK/shim:$PATH"
fi

revision="${1:-}"
if [ -z "$revision" ]; then
  fetch_bundle latest amd64 "$WORK" >/dev/null
  manifest="$(jq -er '.release_manifest.path' "$WORK/bundle/core-release-bundle.json")"
  revision="$(jq -er '.source_revision' "$WORK/bundle/$manifest")"
fi
[[ "$revision" =~ ^[0-9a-f]{40}$ ]] || { echo "macos-release: not a commit: $revision" >&2; exit 2; }
published "$revision" amd64 || { echo "macos-release: $revision has no published Linux release" >&2; exit 1; }
log "macOS release of $revision"

git -C "$root" fetch -q origin
git -C "$root" worktree add -q --detach "$source_tree" "$revision"
mkdir -p "$target_dir"

build_arch() {
  local arch="$1" target="$2" out="$WORK/$1" bin binary
  if published "$revision" "darwin-$arch"; then
    log "$arch is already published"
    return 0
  fi
  log "build $arch"
  (cd "$source_tree" && rustup target add "$target" >/dev/null 2>&1 \
    && RESTLESS_SOURCE_REVISION="$revision" CARGO_TARGET_DIR="$target_dir" \
      cargo build --release --locked --target "$target" -p restlessd -p restless)
  mkdir -p "$out"
  for bin in restless restlessd; do
    binary="$target_dir/$target/release/$bin"
    file "$binary" | grep -q "Mach-O 64-bit executable ${arch}" \
      || { echo "macos-release: $binary is not a Mach-O ${arch} executable" >&2; exit 1; }
    cp "$binary" "$out/$bin"
  done
  # Intel builds run here through Rosetta when it is present.
  if [ "$arch" = arm64 ]; then
    "$out/restless" --help >/dev/null
  elif arch -x86_64 /usr/bin/true 2>/dev/null; then
    arch -x86_64 "$out/restless" --help >/dev/null
  else
    echo 'Rosetta is not installed; the Intel build was checked as a Mach-O file only.' >&2
  fi

  log "sign and publish $arch"
  jq -n --arg revision "$revision" --arg arch "$arch" \
    '{source_revision: $revision, arch: $arch, built_by: "scripts/release/local/macos-release.sh"}' >"$out/release.json"
  COPYFILE_DISABLE=1 tar -czf "$out/binaries.tar.gz" -C "$out" restless restlessd release.json
  pull_image "$COSIGN_IMAGE"
  COSIGN_PRIVATE_KEY="$RELEASE_SIGNING_KEY" COSIGN_PASSWORD="$RELEASE_SIGNING_PASSWORD" \
    docker run --rm -u "$(id -u):$(id -g)" -e HOME=/tmp -e COSIGN_PRIVATE_KEY -e COSIGN_PASSWORD -v "$out:/work" \
    "$COSIGN_IMAGE" sign-blob --yes --key env://COSIGN_PRIVATE_KEY --timeout 90s \
    --bundle /work/binaries.sigstore.json /work/binaries.tar.gz >/dev/null
  verify_signature "$out" binaries.sigstore.json binaries.tar.gz "the macOS binaries'"
  (cd "$out" && node "$cloud/scripts/registry/artifact.mjs" layout layout \
    application/vnd.restless.core.macos-release.v1 \
    "binaries.tar.gz:$MACOS_MEDIA_TYPE" "binaries.sigstore.json:$SIGSTORE_MEDIA_TYPE" \
    "release.json:$MACOS_INFO_MEDIA_TYPE" >/dev/null)
  "$cloud/scripts/registry/with-r2.sh" node "$cloud/scripts/registry/push.mjs" --layout "$out/layout" \
    --repo "$REPOSITORY" --tag "$revision-darwin-$arch"
}

# Only now, with the build verified from the public registry exactly as a Mac will fetch it, is it
# the latest a Mac is offered.
verify_and_tag() {
  local arch="$1" stage="$WORK/verify-$1" digest
  mkdir -p "$stage"
  log "verify $arch from registry.restless.run"
  fetch_macos_binaries "$revision" "$arch" "$WORK" "$stage"
  digest="sha256:$(curl -fsS -H 'Accept: application/vnd.oci.image.manifest.v1+json' \
    "https://${REGISTRY}/v2/${REPOSITORY}/manifests/${revision}-darwin-${arch}" | shasum -a 256 | cut -d' ' -f1)"
  "$cloud/scripts/registry/with-r2.sh" node "$cloud/scripts/registry/tag.mjs" \
    --repo "$REPOSITORY" --tag "latest-darwin-$arch" --digest "$digest"
}

build_arch arm64 aarch64-apple-darwin
build_arch x86_64 x86_64-apple-darwin
verify_and_tag arm64
verify_and_tag x86_64
log "released macOS builds of $revision"
