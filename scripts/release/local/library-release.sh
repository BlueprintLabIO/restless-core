#!/usr/bin/env bash
# Publish Core's product libraries (ui, office, issuer) on this computer, with no GitHub (ADR 0010
# in restless-cloud):
#   1. build, qualify and scan each library, and publish it into the local staging registry
#      (a library whose exact inputs were already sealed today is reused, not rebuilt);
#   2. seal each new one with the release key (from Infisical);
#   3. mirror staging into R2, so registry.restless.run serves it;
#   4. verify each library from the public registry, as Cloud's fetch-core-library.mjs will.
#
#   infisical run --projectId <id> --env prod --domain https://eu.infisical.com/api -- \
#     scripts/release/local/library-release.sh
#
# Needs the same as core-release.sh. Prints, per library, the reference and revision that Cloud's
# `node scripts/fetch-core-library.mjs <kind> <reference> <revision>` takes.
set -euo pipefail
root="$(cd "$(dirname "$0")/../../.." && pwd)"
cloud="${RESTLESS_CLOUD_DIR:-$HOME/code/work/cloud-s106}"
dagger="${DAGGER_BIN:-$HOME/.local/bin/dagger}"
: "${RELEASE_SIGNING_KEY:?run under infisical run}" "${RELEASE_SIGNING_PASSWORD:?run under infisical run}"
: "${CLOUDFLARE_RELEASE_TOKEN:?run under infisical run}"

cd "$root"
if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
  echo "library-release: the checkout has uncommitted changes; a library names one exact commit" >&2
  exit 1
fi
revision="$(git rev-parse HEAD)"
out="${RESTLESS_RELEASE_OUT:-$HOME/.local/state/restless-release/libraries-$revision}"
mkdir -p "$out"
log() { printf '\n== %s  %s\n' "$(date -u +%H:%M:%S)" "$*"; }
eval "$("$root/scripts/release/local/staging.sh" up)"
export LOCAL_REGISTRY_PASSWORD=unused   # staging needs no login; the pipeline still takes the argument

log "build, publish and seal the libraries ($revision)"
rm -rf "$out/libraries"
"$dagger" --silent call deliver-libraries --source=. --revision="$revision" --epoch="$(git log -1 --format=%ct)" \
  --scan-period="$(date -u +%F)" --username=local --password=env:LOCAL_REGISTRY_PASSWORD \
  --signing-key=env:RELEASE_SIGNING_KEY --signing-password=env:RELEASE_SIGNING_PASSWORD \
  export --path="$out/libraries" >/dev/null

log "mirror staging into R2"
"$cloud/scripts/registry/with-r2.sh" node "$cloud/scripts/registry/mirror.mjs" --from "$RESTLESS_STAGING_URL" \
  --repo blueprintlabio/restless-core-release

log "verify from registry.restless.run"
for kind in ui office issuer; do
  receipt="$out/libraries/$kind/library-receipt.json"
  reference="$(jq -er .reference "$receipt")"
  # A reused library keeps the receipt it was sealed with; either way it names the public registry.
  case "$reference" in registry.restless.run/blueprintlabio/restless-core-release@sha256:*) ;; *)
    reference="registry.restless.run/${reference#*/}" ;;
  esac
  cosign verify --key "$root/release/signing.pub" "$reference" >/dev/null 2>&1 \
    || { echo "library-release: $kind ($reference) does not verify from the public registry" >&2; exit 1; }
  printf '%s %s %s\n' "$kind" "$reference" "$(jq -er .source_revision "$receipt")"
done
log "published the libraries of $revision"
