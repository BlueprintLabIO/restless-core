#!/usr/bin/env bash
# Cut a Core release on this computer, with no GitHub (ADR 0010 in restless-cloud):
#   1. qualify the exact source (builds, type checks, unit and contract tests, about 8 minutes);
#   2. publish the images into the local staging registry;
#   3. seal with the release key (from Infisical): sign, attest and verify the images, sign the
#      release documents, push the bundle;
#   4. mirror staging into R2, so registry.restless.run serves it;
#   5. verify the bundle and its handoff from the public registry, as a consumer would.
#
#   infisical run --projectId <id> --env prod --domain https://eu.infisical.com/api -- \
#     scripts/release/local/core-release.sh [--skip-qualify] [--with-gates]
#
# The slow gates (connections smoke, zero-jank, constrained-host resources) don't block a release:
# post-merge-gates.sh runs them on every new main commit in the background and posts the
# `post-merge-gates` status on it. --with-gates runs them here too, before publishing.
#
# Needs: a clean checkout (the release names this exact commit), the restless-cloud checkout beside
# it ($RESTLESS_CLOUD_DIR, default ~/code/restless-cloud) for the mirror, and from Infisical
# RELEASE_SIGNING_KEY, RELEASE_SIGNING_PASSWORD, CLOUDFLARE_RELEASE_TOKEN, CLOUDFLARE_ACCOUNT_ID.
# Prints the bundle reference Cloud's release takes, and writes everything to $out.
set -euo pipefail
root="$(cd "$(dirname "$0")/../../.." && pwd)"
cloud="${RESTLESS_CLOUD_DIR:-$HOME/code/restless-cloud}"
dagger="${DAGGER_BIN:-$HOME/.local/bin/dagger}"
platform=linux/amd64
qualify=yes gates=no
for arg in "$@"; do
  case "$arg" in
    --skip-qualify) qualify=no ;;
    --with-gates) gates=yes ;;
    *) echo "core-release: unknown argument $arg" >&2; exit 2 ;;
  esac
done
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
staging_env="$("$root/scripts/release/local/staging.sh" up)"
eval "$staging_env"
export LOCAL_REGISTRY_PASSWORD=unused   # staging needs no login; the pipeline still takes the argument

if [ "$qualify" = yes ]; then
  log "qualify $revision"
  "$dagger" --silent call qualify --source=. | tail -5
fi
if [ "$gates" = yes ]; then
  log "connections smoke"
  # Without the release secrets, like every test.
  env -i HOME="$HOME" USER="${USER:-}" PATH="$PATH" LANG="${LANG:-C.UTF-8}" TERM="${TERM:-dumb}" \
    "$root/scripts/release/local/connections-smoke.sh"
  log "zero-jank gate"
  # Sprint 109 C7: every surface and card, loading, empty, failure and populated, at desktop and
  # phone widths, against an isolated company; also without the release secrets.
  env -i HOME="$HOME" USER="${USER:-}" PATH="$PATH" LANG="${LANG:-C.UTF-8}" TERM="${TERM:-dumb}" \
    "$root/scripts/release/local/jank-gate.sh"
  log "constrained-host resource gate"
  # Sprint 109 A0.6: a production-sized account pool, small Runtime, simultaneous wakes, idle and
  # restart, with owner entry probed throughout; also without the release secrets.
  env -i HOME="$HOME" USER="${USER:-}" PATH="$PATH" LANG="${LANG:-C.UTF-8}" TERM="${TERM:-dumb}" \
    RESTLESS_CLOUD_DIR="$cloud" ${RESTLESS_CLOUD_AGENT:+RESTLESS_CLOUD_AGENT="$RESTLESS_CLOUD_AGENT"} \
    "$root/scripts/release/local/resource-gate.sh"
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
"$cloud/scripts/registry/with-r2.sh" node "$cloud/scripts/registry/installer.mjs" --file "$root/scripts/install-core.sh"
log "released Core $revision"
echo "bundle: $bundle"
echo "artifacts: $out"
