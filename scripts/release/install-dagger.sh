#!/usr/bin/env bash
set -euo pipefail
version=0.21.10
destination="${DAGGER_INSTALL_DIR:-$HOME/.local/bin}"
if [[ -x "$destination/dagger" && "$("$destination/dagger" version)" == "dagger v$version "* ]]; then exit 0; fi
# Checksums from the release's checksums.txt: the x86_64 self-hosted builder and the arm64
# GitHub-hosted runner that releases linux/arm64.
case "$(uname -s)/$(uname -m)" in
  Linux/x86_64) arch=amd64; checksum=f9ee083767dd12cdac583f9db3fedbebbbb3064f69152998be1f121d1a6cc103 ;;
  Linux/aarch64) arch=arm64; checksum=d1b20d4b8815badc5a8478e7c8fc1031c355c02a7c1beb05509aff32965dc58e ;;
  *) echo 'this install path qualifies Linux AMD64 and ARM64 builders' >&2; exit 1 ;;
esac
temporary="$(mktemp -d)"
trap 'rm -f "$temporary/dagger.tar.gz" "$temporary/dagger"; rmdir "$temporary"' EXIT
curl --fail --silent --show-error --location "https://github.com/dagger/dagger/releases/download/v$version/dagger_v${version}_linux_${arch}.tar.gz" --output "$temporary/dagger.tar.gz"
printf '%s  %s\n' "$checksum" "$temporary/dagger.tar.gz" | sha256sum --check --status
tar -xzf "$temporary/dagger.tar.gz" -C "$temporary" dagger
mkdir -p "$destination"
install -m 755 "$temporary/dagger" "$destination/dagger"
"$destination/dagger" version
