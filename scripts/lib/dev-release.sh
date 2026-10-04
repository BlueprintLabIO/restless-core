# Build artifacts shared by a development stack and a promoted appliance
# release. Sourced by scripts/restless-dev; every function takes explicit roots
# so a promotion never builds from the working tree it was launched in.

# sha256 of stdin on both Linux (coreutils) and macOS (perl shasum).
dev_sha256() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum; else shasum -a 256; fi
}

# Build the company Runtime image from <root> as <tag>. <revision> labels the
# source it came from (a commit for a release, a digest for a dev checkout).
dev_build_company_image() {
  local root="$1" tag="$2" revision="$3"
  local source_digest api_contract assertion_contract schema_version
  source_digest="$(
    cd "$root"
    find Cargo.toml Cargo.lock crates infra/company-image -type f -print \
      | LC_ALL=C sort \
      | while IFS= read -r image_input; do
          printf '%s\0' "$image_input"
          command cat "$image_input"
          printf '\0'
        done \
      | dev_sha256 \
      | awk '{print $1}'
  )"
  api_contract="$(sed -n 's/.*API_CONTRACT_VERSION: u32 = \([0-9]*\);.*/\1/p' "${root}/crates/restless-engine/src/release.rs")"
  assertion_contract="$(sed -n 's/.*ASSERTION_CONTRACT_VERSION: u32 = \([0-9]*\);.*/\1/p' "${root}/crates/restless-engine/src/entry.rs")"
  schema_version="$(sed -n 's/.*SCHEMA_VERSION: u32 = \([0-9]*\);.*/\1/p' "${root}/crates/restless-engine/src/release.rs")"
  docker build \
    --file "${root}/infra/company-image/Dockerfile" \
    --tag "$tag" \
    --label "io.restless.source-digest=${source_digest}" \
    --build-arg RESTLESS_CORE_VERSION=0.0.0-dev \
    --build-arg "RESTLESS_SOURCE_REVISION=${revision:-dev-${source_digest}}" \
    --build-arg "RESTLESS_API_CONTRACT_VERSION=${api_contract}" \
    --build-arg "RESTLESS_ASSERTION_CONTRACT_VERSION=${assertion_contract}" \
    --build-arg "RESTLESS_SCHEMA_VERSION=${schema_version}" \
    "$root"
}

# Install the pinned, patched model broker for <root> into a directory keyed by
# its lockfile and patch, shared by every release that pins the same tools.
# Prints the directory.
dev_install_sheets() {
  local root="$1" tools_root="$2" source key target node_bin node_license
  source="$root/services/native-sheets"
  node_bin="$(node -p 'process.execPath')"
  node_license="$(dirname "$node_bin")/../LICENSE"
  [ -f "$node_license" ] || { printf 'The Node runtime license is missing beside its installation.\n' >&2; return 1; }
  node -e 'if(Number(process.versions.node.split(".")[0])<22) process.exit(1)' || return 1
  key="$( { cd "$source" && find package.json package-lock.json NOTICE src -type f -print | LC_ALL=C sort | while IFS= read -r input; do printf '%s\0' "$input"; command cat "$input"; done; command cat "$node_bin"; command cat "$node_license"; } | dev_sha256 | cut -c1-16)"
  target="${tools_root}/native-sheets-${key}"
  if [ ! -f "$target/.installed" ]; then
    mkdir -p "$tools_root"
    rm -rf "${target}.partial"
    mkdir -p "${target}.partial"
    (cd "$source" && tar --exclude=node_modules --exclude=test --exclude=probe.mjs -cf - .) | (cd "${target}.partial" && tar -xf -)
    npm --prefix "${target}.partial" ci --omit=dev --no-audit --no-fund >&2
    mkdir -p "${target}.partial/bin"
    cp "$node_bin" "${target}.partial/bin/node"
    cp "$node_license" "${target}.partial/NODE_LICENSE"
    touch "${target}.partial/.installed"
    mv "${target}.partial" "$target"
  fi
  printf '%s\n' "$target"
}

dev_install_host_tools() {
  local root="$1" tools_root="$2"
  local source="${root}/infra/host-tools" key target
  key="$(
    cd "$source"
    find package.json package-lock.json apply-pi-ai-patch.mjs omp-source resolve-bun.sh patches \
      -type f -print | LC_ALL=C sort | while IFS= read -r input; do
        printf '%s\0' "$input"; command cat "$input"
      done | dev_sha256 | cut -c1-16
  )"
  target="${tools_root}/${key}"
  if [ ! -x "${target}/omp-source" ] || [ ! -f "${target}/.installed" ]; then
    rm -rf "${target}.partial"
    mkdir -p "${target}.partial"
    (cd "$source" && tar --exclude=node_modules -cf - .) | (cd "${target}.partial" && tar -xf -)
    npm --prefix "${target}.partial" ci --no-audit --no-fund >&2
    source "${target}.partial/resolve-bun.sh"
    local bun_bin
    bun_bin="$(host_tools_find_bun "${target}.partial")" || {
      printf 'The model broker runtime could not start on this host.\n' >&2
      return 1
    }
    "$bun_bin" "${target}.partial/apply-pi-ai-patch.mjs" >&2
    touch "${target}.partial/.installed"
    rm -rf "$target"
    mv "${target}.partial" "$target"
  fi
  printf '%s\n' "$target"
}
