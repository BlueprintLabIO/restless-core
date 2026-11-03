#!/bin/sh
# Fails when release::SCHEMA_VERSION is not the highest migration number on disk.
# The same rule as restless-engine's release::tests::the_declared_schema_version_matches_the_migrations_on_disk,
# without compiling the engine, so qualification (local-ci and every Core release) catches a
# migration that merged without bumping the release identity.
#
#   scripts/check-schema-version.sh [checkout]
set -eu
root="${1:-$(cd "$(dirname "$0")/.." && pwd)}"
release="$root/crates/restless-engine/src/release.rs"
migrations="$root/crates/restless-orgintel/migrations"
declared="$(sed -n 's/^pub const SCHEMA_VERSION: u32 = \([0-9][0-9]*\);.*/\1/p' "$release")"
highest="$(ls "$migrations" | sed -n 's/^\([0-9][0-9]*\)_.*/\1/p' | sort -n | tail -n 1 | sed 's/^0*//')"
if [ -z "$declared" ] || [ -z "$highest" ]; then
  echo "check-schema-version: could not read SCHEMA_VERSION ($declared) or the highest migration ($highest)" >&2
  exit 1
fi
if [ "$declared" != "$highest" ]; then
  echo "release::SCHEMA_VERSION is $declared but the highest migration is $highest:" >&2
  echo "bump SCHEMA_VERSION in crates/restless-engine/src/release.rs to $highest." >&2
  exit 1
fi
echo "SCHEMA_VERSION $declared matches the highest migration"
