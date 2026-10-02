# The API-backed harness needs a host model broker as well as the guest agent.
# Keep its runtime pinned and checkout-local, without a global npm installation.
dev_tools_ensure() {
  local sheets="$STACK_REPO_ROOT/services/native-sheets"
  if [ ! -d "$sheets/node_modules/@odoo/o-spreadsheet" ]; then
    printf 'Installing the pinned spreadsheet engine…\n'
    npm --prefix "$sheets" ci --no-audit --no-fund || return 1
  fi
  export RESTLESS_SHEETS_WORKER="$sheets/src/worker.mjs"
  export RESTLESS_NODE_BIN="$(node -p 'process.execPath')"
  if [ -n "${RESTLESS_OMP_BIN:-}" ]; then
    return 0
  fi
  local tools="$STACK_REPO_ROOT/infra/host-tools"
  if [ ! -x "$tools/node_modules/.bin/omp" ] || [ ! -x "$tools/node_modules/.bin/bun" ]; then
    printf 'Installing the pinned model broker…\n'
    npm --prefix "$tools" ci --no-audit --no-fund || return 1
  fi
  source "$tools/resolve-bun.sh"
  local bun_bin
  bun_bin="$(host_tools_find_bun "$tools")" || {
    printf 'The model broker runtime could not start. Check your operating system and CPU support.\n' >&2
    return 1
  }
  "$bun_bin" "$tools/apply-pi-ai-patch.mjs" || return 1
  export RESTLESS_BUN_BIN="$bun_bin"
  export RESTLESS_OMP_BIN="$tools/omp-source"
}
