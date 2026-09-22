# First-run configuration shared by the development launcher and its tests.
# Existing company files are never rewritten by these helpers.
dev_company_validate() {
  local model="${RESTLESS_DEV_MODEL:-}"
  if { [ -n "$model" ] && [[ ! "$model" =~ ^[a-zA-Z0-9_.:-]+/[a-zA-Z0-9_./:-]+$ ]]; } ||
     { [ -n "${RESTLESS_DEV_CREDENTIAL_REFERENCE:-}" ] && [ -z "${RESTLESS_DEV_MODEL:-}" ]; }; then
    cat >&2 <<'HELP'
Choose a model for your API connection:
  export RESTLESS_DEV_MODEL=provider/model
For an API connection, also set its credential reference:
  export RESTLESS_DEV_CREDENTIAL_REFERENCE=env:YOUR_API_KEY
Or leave both variables unset and connect Codex, Claude or an API provider
in Company > Intelligence after the workspace opens.
HELP
    return 1
  fi
  if [[ "${RESTLESS_DEV_CREDENTIAL_REFERENCE:-}" == *$'\n'* || "${RESTLESS_DEV_CREDENTIAL_REFERENCE:-}" == *$'\r'* ]]; then
    printf 'The credential reference must be a single line.\n' >&2
    return 1
  fi
}

dev_company_write() {
  local company_name="$1" credential="${RESTLESS_DEV_CREDENTIAL_REFERENCE:-}"
  dev_company_validate || return 1
  printf 'name = "%s"\nmission = "Help the owner and their team turn business goals into useful work. Ask for their direction before starting new projects."\n' \
    "$company_name"
  if [ -n "${RESTLESS_DEV_MODEL:-}" ]; then
    printf 'model = "%s"\n' "$RESTLESS_DEV_MODEL"
  fi
  printf 'reasoning_effort = "high"\n'
  if [ -n "$credential" ]; then
    credential="${credential//\\/\\\\}"
    credential="${credential//\"/\\\"}"
    printf '\n[credentials]\n"model.inference" = "%s"\n' "$credential"
  fi
}
