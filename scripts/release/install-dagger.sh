#!/usr/bin/env bash
set -euo pipefail
version=0.21.10
checksum=f9ee083767dd12cdac583f9db3fedbebbbb3064f69152998be1f121d1a6cc103
destination="${DAGGER_INSTALL_DIR:-$HOME/.local/bin}"
if [[ -x "$destination/dagger" && "$("$destination/dagger" version)" == "dagger v$version "* ]]; then exit 0; fi
[[ "$(uname -s)" == Linux && "$(uname -m)" == x86_64 ]] || { echo 'this install path qualifies the Linux AMD64 alpha builder' >&2; exit 1; }
temporary="$(mktemp -d)"
trap 'rm -f "$temporary/dagger.tar.gz" "$temporary/dagger"; rmdir "$temporary"' EXIT
curl --fail --silent --show-error --location "https://github.com/dagger/dagger/releases/download/v$version/dagger_v${version}_linux_amd64.tar.gz" --output "$temporary/dagger.tar.gz"
printf '%s  %s\n' "$checksum" "$temporary/dagger.tar.gz" | sha256sum --check --status
tar -xzf "$temporary/dagger.tar.gz" -C "$temporary" dagger
mkdir -p "$destination"
install -m 755 "$temporary/dagger" "$destination/dagger"
"$destination/dagger" version
