# The API-backed harness needs a host model broker as well as the guest agent.
# Keep its runtime pinned and checkout-local, without a global npm installation.
dev_tools_ensure() {
  if [ -n "${RESTLESS_OMP_BIN:-}" ]; then
    return 0
  fi
  local tools="$STACK_REPO_ROOT/infra/host-tools"
  if [ ! -x "$tools/node_modules/.bin/omp" ] || [ ! -x "$tools/node_modules/.bin/bun" ]; then
    printf 'Installing the pinned model broker…\n'
    npm --prefix "$tools" ci --no-audit --no-fund || return 1
  fi
  export PATH="$tools/node_modules/.bin:$PATH"
  export RESTLESS_OMP_BIN="$tools/node_modules/.bin/omp"
  "$tools/node_modules/.bin/bun" --version >/dev/null || {
    printf 'The model broker runtime could not start. Check your operating system and CPU support.\n' >&2
    return 1
  }
}
