# Find a working Bun without trusting npm's executable shim: installs with
# ignored lifecycle scripts leave node_modules/.bin/bun executable but unusable.
host_tools_find_bun() {
  local tools_dir="$1" candidate
  for candidate in \
    "${RESTLESS_BUN_BIN:-}" \
    "$(command -v bun || true)" \
    "$tools_dir/node_modules/.bin/bun" \
    "$tools_dir"/node_modules/@oven/bun-*/bin/bun; do
    if [ -n "$candidate" ] && [ -x "$candidate" ] && "$candidate" --version >/dev/null 2>&1; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
  return 1
}
