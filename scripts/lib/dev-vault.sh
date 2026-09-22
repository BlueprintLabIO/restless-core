# A fresh workspace must be able to save the API key entered in its setup UI.
# An explicitly configured vault or an older installation stays authoritative.
dev_vault_ensure() {
  if [ -n "${INFISICAL_API_URL:-}" ]; then
    return 0
  fi
  local directory="$RESTLESS_HOME/infisical" authority="$RESTLESS_HOME/infisical/authority.env"
  if [ -f "$authority" ] && [ ! -f "$directory/dev-project" ]; then
    set -a
    source "$authority"
    set +a
    return 0
  fi
  mkdir -p "$directory"
  chmod 700 "$directory"
  export RESTLESS_INFISICAL_PROJECT="restless-${RESTLESS_RESOURCE_NAMESPACE}-vault"
  export RESTLESS_INFISICAL_PORT="$((7793 + RESTLESS_PORT_OFFSET))"
  export RESTLESS_INFISICAL_API_URL="http://127.0.0.1:${RESTLESS_INFISICAL_PORT}"
  printf '%s\n' "$RESTLESS_INFISICAL_PROJECT" > "$directory/dev-project"
  printf '\nPreparing the company vault…\n'
  "${STACK_REPO_ROOT}/infra/infisical/provision.sh" || return 1
  set -a
  source "$authority"
  set +a
}
