#!/usr/bin/env bash
set -euo pipefail

# Provision one local Infisical instance plus one project-scoped Universal
# Auth identity for Restless Authority. Secret values never go on argv or
# stdout. Generated bootstrap material lives outside the checkout with 0600
# permissions; provider values remain in Infisical after import.

script_dir=$(CDPATH= cd "$(dirname "$0")" && pwd -P)
repo_root=$(CDPATH= cd "$script_dir/../.." && pwd -P)
state_root=${RESTLESS_HOME:-"${HOME:?HOME is required}/.restless"}
state_dir="$state_root/infisical"
runtime_env="$state_dir/runtime.env"
authority_env="$state_dir/authority.env"
admin_env="$state_dir/admin.env"
checkout_env="$repo_root/.env"
api_url=${RESTLESS_INFISICAL_API_URL:-http://127.0.0.1:7793}
install_checkout=false

usage() {
  printf '%s\n' \
    'Usage: infra/infisical/provision.sh [--install-checkout]' \
    '' \
    'Starts the pinned loopback-only Infisical service and creates a dedicated' \
    'Restless project plus project-scoped Universal Auth identity.' \
    '--install-checkout also installs the non-provider Infisical service' \
    'configuration into the ignored repository .env.'
}

while (($#)); do
  case "$1" in
    --install-checkout) install_checkout=true ;;
    -h|--help) usage; exit 0 ;;
    *) printf 'unknown option: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done

for command_name in curl docker jq openssl; do
  command -v "$command_name" >/dev/null 2>&1 || {
    printf '%s is required\n' "$command_name" >&2
    exit 1
  }
done

if [[ -L "$state_dir" || -L "$checkout_env" ]]; then
  printf 'refusing a symbolic-link credential path\n' >&2
  exit 1
fi
mkdir -p "$state_dir"
chmod 700 "$state_dir"

write_runtime_env() {
  local temporary encryption_key auth_secret postgres_password
  temporary=$(mktemp "$state_dir/.runtime.env.XXXXXX")
  chmod 600 "$temporary"
  encryption_key=$(openssl rand -hex 16)
  auth_secret=$(openssl rand -base64 48 | tr -d '\n')
  postgres_password=$(openssl rand -hex 32)
  {
    printf 'ENCRYPTION_KEY=%s\n' "$encryption_key"
    printf 'AUTH_SECRET=%s\n' "$auth_secret"
    printf 'POSTGRES_PASSWORD=%s\n' "$postgres_password"
    printf 'POSTGRES_USER=infisical\n'
    printf 'POSTGRES_DB=infisical\n'
    printf 'DB_CONNECTION_URI=postgres://infisical:%s@db:5432/infisical\n' "$postgres_password"
    printf 'REDIS_URL=redis://redis:6379\n'
    printf 'SITE_URL=%s\n' "$api_url"
    printf 'OTEL_TELEMETRY_COLLECTION_ENABLED=false\n'
  } >"$temporary"
  mv "$temporary" "$runtime_env"
}

if [[ ! -f "$runtime_env" ]]; then
  write_runtime_env
fi
chmod 600 "$runtime_env"

# Infisical v0.162.19's bootstrap endpoint emits a legacy no-expiry instance
# admin token, while the release's compiled legacy-token cutoff has elapsed.
# Open a fresh-instance-only window long enough to create the scoped identity.
# It is removed and the backend is recreated below.
if [[ ! -f "$authority_env" ]] &&
  ! grep -q '^LEGACY_IDENTITY_ACCESS_TOKEN_EXPIRATION_ENFORCED_AT=' "$runtime_env"; then
  printf 'LEGACY_IDENTITY_ACCESS_TOKEN_EXPIRATION_ENFORCED_AT=%s\n' \
    "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" >>"$runtime_env"
fi

export RESTLESS_INFISICAL_RUNTIME_ENV="$runtime_env"
export RESTLESS_INFISICAL_PORT=${RESTLESS_INFISICAL_PORT:-7793}
docker compose --project-name restless-infisical \
  --file "$script_dir/compose.yml" \
  --env-file "$runtime_env" \
  up --detach --wait

ready=false
for _ in $(seq 1 120); do
  if curl --fail --silent --show-error --max-time 2 \
    "$api_url/api/status" >/dev/null 2>&1; then
    ready=true
    break
  fi
  sleep 1
done
if [[ "$ready" != true ]]; then
  printf 'Infisical did not become ready at %s\n' "$api_url" >&2
  exit 1
fi

tmp_dir=$(mktemp -d "${TMPDIR:-/tmp}/restless-infisical.XXXXXX")
cleanup() {
  if [[ -n "${tmp_dir:-}" && -d "$tmp_dir" && "$tmp_dir" == "${TMPDIR:-/tmp}"/restless-infisical.* ]]; then
    rm -rf "$tmp_dir"
  fi
}
trap cleanup EXIT
chmod 700 "$tmp_dir"

api_json() {
  local method=$1 endpoint=$2 header_file=${3:-} request_file=${4:-}
  local -a arguments
  arguments=(--fail-with-body --silent --show-error --request "$method"
    --header 'Content-Type: application/json')
  if [[ -n "$header_file" ]]; then
    arguments+=(--header "@$header_file")
  fi
  if [[ -n "$request_file" ]]; then
    arguments+=(--data-binary "@$request_file")
  fi
  curl "${arguments[@]}" "$api_url$endpoint"
}

if [[ ! -f "$authority_env" ]]; then
  admin_password_file="$tmp_dir/admin-password"
  bootstrap_request="$tmp_dir/bootstrap-request.json"
  bootstrap_response="$tmp_dir/bootstrap-response.json"
  admin_token_file="$tmp_dir/admin-token"
  admin_header_file="$tmp_dir/admin-header"
  project_request="$tmp_dir/project-request.json"
  project_response="$tmp_dir/project-response.json"
  identity_request="$tmp_dir/identity-request.json"
  identity_response="$tmp_dir/identity-response.json"
  auth_request="$tmp_dir/auth-request.json"
  auth_response="$tmp_dir/auth-response.json"
  client_request="$tmp_dir/client-request.json"
  client_response="$tmp_dir/client-response.json"

  if [[ ! -f "$admin_env" ]]; then
    admin_temporary=$(mktemp "$state_dir/.admin.env.XXXXXX")
    chmod 600 "$admin_temporary"
    {
      printf 'INFISICAL_ADMIN_EMAIL=owner@restless.local\n'
      printf 'INFISICAL_ADMIN_PASSWORD='
      openssl rand -base64 48 | tr -d '\n'
      printf '\n'
    } >"$admin_temporary"
    mv "$admin_temporary" "$admin_env"
  fi
  sed -n 's/^INFISICAL_ADMIN_PASSWORD=//p' "$admin_env" >"$admin_password_file"
  chmod 600 "$admin_password_file"
  jq --null-input --rawfile password "$admin_password_file" '{
    email: "owner@restless.local",
    password: $password,
    organization: "Restless"
  }' >"$bootstrap_request"
  api_json POST /api/v1/admin/bootstrap '' "$bootstrap_request" >"$bootstrap_response"
  jq --exit-status --raw-output '.identity.credentials.token' \
    "$bootstrap_response" >"$admin_token_file"
  chmod 600 "$admin_token_file"
  {
    printf 'Authorization: Bearer '
    tr -d '\n' <"$admin_token_file"
    printf '\n'
  } >"$admin_header_file"
  chmod 600 "$admin_header_file"

  jq --null-input '{
    projectName: "Restless Authority",
    projectDescription: "Provider credentials brokered by the Restless Authority plane",
    slug: "restless-authority",
    template: "default",
    type: "secret-manager",
    shouldCreateDefaultEnvs: true,
    hasDeleteProtection: true
  }' >"$project_request"
  api_json POST /api/v1/projects "$admin_header_file" "$project_request" >"$project_response"
  project_id=$(jq --exit-status --raw-output '.project.id' "$project_response")

  create_folder() {
    local name=$1 parent_path=$2 request_file response_file
    request_file="$tmp_dir/folder-$name-request.json"
    response_file="$tmp_dir/folder-$name-response.json"
    jq --null-input \
      --arg project_id "$project_id" \
      --arg name "$name" \
      --arg parent_path "$parent_path" '{
        projectId: $project_id,
        environment: "prod",
        name: $name,
        path: $parent_path
      }' >"$request_file"
    api_json POST /api/v2/folders "$admin_header_file" "$request_file" >"$response_file"
    jq --exit-status '.folder.id' "$response_file" >/dev/null
  }
  create_folder companies /
  create_folder aris /companies
  create_folder providers /
  create_folder moonshot /providers

  jq --null-input '{
    name: "restless-authority",
    hasDeleteProtection: true,
    metadata: [{key: "owner", value: "restlessd"}],
    roles: [{role: "admin", isTemporary: false}]
  }' >"$identity_request"
  api_json POST "/api/v1/projects/$project_id/identities" \
    "$admin_header_file" "$identity_request" >"$identity_response"
  identity_id=$(jq --exit-status --raw-output '.identity.id' "$identity_response")

  jq --null-input '{
    clientSecretTrustedIps: [{ipAddress: "0.0.0.0/0"}, {ipAddress: "::/0"}],
    accessTokenTrustedIps: [{ipAddress: "0.0.0.0/0"}, {ipAddress: "::/0"}],
    accessTokenTTL: 3600,
    accessTokenMaxTTL: 3600,
    accessTokenNumUsesLimit: 0,
    accessTokenPeriod: 0,
    lockoutEnabled: true,
    lockoutThreshold: 3,
    lockoutDurationSeconds: 300,
    lockoutCounterResetSeconds: 30
  }' >"$auth_request"
  api_json POST "/api/v1/auth/universal-auth/identities/$identity_id" \
    "$admin_header_file" "$auth_request" >"$auth_response"
  client_id=$(jq --exit-status --raw-output '.identityUniversalAuth.clientId' "$auth_response")

  jq --null-input '{
    description: "Restless Authority host",
    numUsesLimit: 0,
    ttl: 31536000
  }' >"$client_request"
  api_json POST "/api/v1/auth/universal-auth/identities/$identity_id/client-secrets" \
    "$admin_header_file" "$client_request" >"$client_response"
  client_secret=$(jq --exit-status --raw-output '.clientSecret' "$client_response")
  organization_slug=$(jq --exit-status --raw-output '.organization.slug' "$bootstrap_response")

  authority_temporary=$(mktemp "$state_dir/.authority.env.XXXXXX")
  chmod 600 "$authority_temporary"
  {
    printf 'INFISICAL_API_URL=%s\n' "$api_url"
    printf 'INFISICAL_PROJECT_ID=%s\n' "$project_id"
    printf 'INFISICAL_ENVIRONMENT=prod\n'
    printf 'INFISICAL_UNIVERSAL_AUTH_CLIENT_ID=%s\n' "$client_id"
    printf 'INFISICAL_UNIVERSAL_AUTH_CLIENT_SECRET=%s\n' "$client_secret"
    printf 'INFISICAL_ORGANIZATION_SLUG=%s\n' "$organization_slug"
  } >"$authority_temporary"
  mv "$authority_temporary" "$authority_env"

fi
chmod 600 "$authority_env" "$admin_env"

# Close the compatibility window now that the broad bootstrap token has been
# discarded and the scoped Universal Auth identity exists. Recreating only the
# backend preserves the Infisical database and Redis volumes.
if grep -q '^LEGACY_IDENTITY_ACCESS_TOKEN_EXPIRATION_ENFORCED_AT=' "$runtime_env"; then
  runtime_temporary=$(mktemp "$state_dir/.runtime.env.XXXXXX")
  chmod 600 "$runtime_temporary"
  awk '!/^LEGACY_IDENTITY_ACCESS_TOKEN_EXPIRATION_ENFORCED_AT=/' \
    "$runtime_env" >"$runtime_temporary"
  mv "$runtime_temporary" "$runtime_env"
  docker compose --project-name restless-infisical \
    --file "$script_dir/compose.yml" \
    --env-file "$runtime_env" \
    up --detach --wait --force-recreate backend
fi

if [[ "$install_checkout" == true ]]; then
  checkout_temporary=$(mktemp "$repo_root/.env.infisical.XXXXXX")
  chmod 600 "$checkout_temporary"
  if [[ -f "$checkout_env" ]]; then
    awk '!/^(INFISICAL_API_URL|INFISICAL_PROJECT_ID|INFISICAL_ENVIRONMENT|INFISICAL_UNIVERSAL_AUTH_CLIENT_ID|INFISICAL_UNIVERSAL_AUTH_CLIENT_SECRET|INFISICAL_ORGANIZATION_SLUG|RESTLESS_RESEND_WEBHOOK_CREDENTIAL)=/' \
      "$checkout_env" >"$checkout_temporary"
  fi
  {
    printf '\n# Durable host credential backend; generated by infra/infisical/provision.sh.\n'
    sed -n '/^INFISICAL_/p' "$authority_env"
    printf 'RESTLESS_RESEND_WEBHOOK_CREDENTIAL=infisical:/companies/aris/RESEND_WEBHOOK_SECRET\n'
  } >>"$checkout_temporary"
  mv "$checkout_temporary" "$checkout_env"
  chmod 600 "$checkout_env"
fi

printf 'Infisical is ready on loopback; Authority identity and project are provisioned.\n'
printf 'Service configuration: %s\n' "$authority_env"
if [[ "$install_checkout" == true ]]; then
  printf 'Installed Infisical service configuration into the ignored checkout .env.\n'
fi
