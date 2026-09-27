#!/usr/bin/env bash
# Reuse only an image previously published and signed by this release workflow.
# A missing tag is buildable; an unverifiable existing tag is never overwritten.
set -euo pipefail

image="${1:?image repository required}"
revision="${2:?source revision required}"
workflow_ref="${3:?workflow ref required}"
[[ "$revision" =~ ^[0-9a-f]{40}$ ]]
[[ "$image" =~ ^ghcr\.io/blueprintlabio/restless-(account-plane|company-runtime|native-documents-collaboration)$ ]]
reference="${image}:${revision}"

if ! manifest="$(docker buildx imagetools inspect --format '{{json .Manifest}}' "$reference" 2>&1)"; then
  if grep -Eiq 'not found|manifest unknown|404' <<< "$manifest"; then
    echo 'exists=false'
    exit 0
  fi
  printf '%s\n' "$manifest" >&2
  exit 1
fi

digest="$(jq -er '.digest | select(test("^sha256:[0-9a-f]{64}$"))' <<< "$manifest")"
index="$(docker buildx imagetools inspect --raw "$reference")"
jq -e '
  ([.manifests[] | select(.platform.os == "linux") | .platform.architecture] | sort) == ["amd64", "arm64"] and
  ([.manifests[] | select(.platform.os == "unknown" and .annotations["vnd.docker.reference.type"] == "attestation-manifest")] | length) >= 2
' <<< "$index" >/dev/null
configs="$(docker buildx imagetools inspect --format '{{json .Image}}' "$reference")"
jq -e --arg revision "$revision" '
  all([. ["linux/amd64"], .["linux/arm64"]][];
    .config.Labels["org.opencontainers.image.revision"] == $revision and
    .config.Labels["org.opencontainers.image.source"] == "https://github.com/BlueprintLabIO/restless-core")
' <<< "$configs" >/dev/null
cosign verify \
  --certificate-identity "https://github.com/${workflow_ref}" \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  "${image}@${digest}" >/dev/null

printf 'Reusing signed %s@%s\n' "$image" "$digest" >&2
printf 'exists=true\ndigest=%s\n' "$digest"
