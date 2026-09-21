# Explicit first-run configuration shared by the development launcher and its tests.
# Existing company files are never rewritten by these helpers.
dev_company_validate() {
  if [[ ! "${RESTLESS_DEV_MODEL:-}" =~ ^[a-zA-Z0-9_.:-]+/[a-zA-Z0-9_./:-]+$ ]]; then
    cat >&2 <<'HELP'
Choose a model before creating a development company:
  export RESTLESS_DEV_MODEL=provider/model
For an API connection, also set its credential reference:
  export RESTLESS_DEV_CREDENTIAL_REFERENCE=env:YOUR_API_KEY
Or omit the credential reference and connect Codex, Claude or an API provider
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
  printf 'name = "%s"\nmission = "Develop and verify Restless safely in this isolated checkout."\nmodel = "%s"\nreasoning_effort = "high"\n' \
    "$company_name" "$RESTLESS_DEV_MODEL"
  if [ -n "$credential" ]; then
    credential="${credential//\\/\\\\}"
    credential="${credential//\"/\\\"}"
    printf '\n[credentials]\n"model.inference" = "%s"\n' "$credential"
  fi
}
