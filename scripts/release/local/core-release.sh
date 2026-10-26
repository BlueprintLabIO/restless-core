#!/usr/bin/env bash
# Cut a Core release on this computer, with no GitHub (ADR 0010 in restless-cloud):
#   1. qualify the exact source;
#   2. publish the images into the local staging registry;
#   3. seal with the release key (from Infisical): sign, attest and verify the images, sign the
#      release documents, push the bundle;
#   4. mirror staging into R2, so registry.restless.run serves it;
#   5. verify the bundle and its handoff from the public registry, as a consumer would.
#
#   infisical run --projectId <id> --env prod --domain https://eu.infisical.com/api -- \
#     scripts/release/local/core-release.sh [--skip-qualify]
#
# Needs: a clean checkout (the release names this exact commit), the restless-cloud checkout beside
# it ($RESTLESS_CLOUD_DIR, default ~/code/work/cloud-s106) for the mirror, and from Infisical
# RELEASE_SIGNING_KEY, RELEASE_SIGNING_PASSWORD, CLOUDFLARE_RELEASE_TOKEN, CLOUDFLARE_ACCOUNT_ID.
# Prints the bundle reference Cloud's release takes, and writes everything to $out.
set -euo pipefail
root="$(cd "$(dirname "$0")/../../.." && pwd)"
cloud="${RESTLESS_CLOUD_DIR:-$HOME/code/work/cloud-s106}"
dagger="${DAGGER_BIN:-$HOME/.local/bin/dagger}"
platform=linux/amd64
qualify=yes
[ "${1:-}" = --skip-qualify ] && qualify=no
: "${RELEASE_SIGNING_KEY:?run under infisical run}" "${RELEASE_SIGNING_PASSWORD:?run under infisical run}"
: "${CLOUDFLARE_RELEASE_TOKEN:?run under infisical run}"

cd "$root"
if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
  echo "core-release: the checkout has uncommitted changes; a release names one exact commit" >&2
  exit 1
fi
revision="$(git rev-parse HEAD)"
out="${RESTLESS_RELEASE_OUT:-$HOME/.local/state/restless-release/core-$revision}"
mkdir -p "$out"
log() { printf '\n== %s  %s\n' "$(date -u +%H:%M:%S)" "$*"; }
eval "$("$root/scripts/release/local/staging.sh" up)"
export LOCAL_REGISTRY_PASSWORD=unused   # staging needs no login; the pipeline still takes the argument

if [ "$qualify" = yes ]; then
  log "qualify $revision"
  "$dagger" --silent call qualify --source=. | tail -5
fi

log "publish into staging ($platform)"
rm -rf "$out/artifacts"
"$dagger" --silent call publish --source=. --revision="$revision" --username=local \
  --password=env:LOCAL_REGISTRY_PASSWORD --platform="$platform" --scan-period="$(date -u +%F)" \
  export --path="$out/artifacts" >/dev/null
node scripts/release/timings-summary.mjs publish "$out/artifacts/timings" 2>/dev/null | tail -3 || true

log "seal with the release key"
rm -rf "$out/release"
"$dagger" --silent call seal --source=. --artifacts="$out/artifacts" --revision="$revision" --username=local \
  --password=env:LOCAL_REGISTRY_PASSWORD --signing-key=env:RELEASE_SIGNING_KEY \
  --signing-password=env:RELEASE_SIGNING_PASSWORD --platform="$platform" \
  export --path="$out/release" >/dev/null

log "mirror staging into R2"
repos=()
for repo in core-release runtime-tools cockpit account-plane company-runtime native-documents-collaboration; do
  repos+=(--repo "blueprintlabio/restless-$repo")
done
"$cloud/scripts/registry/with-r2.sh" node "$cloud/scripts/registry/mirror.mjs" --from "$RESTLESS_STAGING_URL" "${repos[@]}"

log "verify from registry.restless.run"
bundle="$(jq -er .bundle "$out/release/core-release-handoff.json")"
case "$bundle" in registry.restless.run/blueprintlabio/restless-core-release@sha256:*) ;; *)
  echo "core-release: the handoff names $bundle, not the public registry" >&2; exit 1 ;;
esac
cosign verify --key "$root/release/signing.pub" "$bundle" >/dev/null 2>&1 \
  || { echo "core-release: the bundle's signature does not verify from the public registry" >&2; exit 1; }
cosign verify-blob --key "$root/release/signing.pub" --bundle "$out/release/core-release-handoff.sigstore.json" \
  "$out/release/core-release-handoff.json" >/dev/null 2>&1 \
  || { echo "core-release: the handoff signature does not verify" >&2; exit 1; }
for image in $(jq -r '.images[]' "$out/release/assembly.json" 2>/dev/null); do
  cosign verify --key "$root/release/signing.pub" "$image" >/dev/null 2>&1 \
    || { echo "core-release: $image does not verify from the public registry" >&2; exit 1; }
done
# Only now, with every signature verified from the public registry, is it the latest the
# installer offers.
"$cloud/scripts/registry/with-r2.sh" node "$cloud/scripts/registry/tag.mjs" \
  --repo blueprintlabio/restless-core-release --tag "latest-${platform#linux/}" --digest "${bundle##*@}"
log "released Core $revision"
echo "bundle: $bundle"
echo "artifacts: $out"
