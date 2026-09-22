# Provision PostgreSQL only for a new, unconfigured development profile.
# Existing external database configuration remains authoritative.
dev_database_ensure() {
  local state="$RESTLESS_HOME" name="restless-${RESTLESS_RESOURCE_NAMESPACE}-postgres"
  local port="$((7797 + RESTLESS_PORT_OFFSET))" attempt owner
  if [ -n "${RESTLESS_PLANE_DATABASE_URL:-}" ]; then
    return 0
  fi
  if [ -f "$state/orgintel.toml" ] && [ ! -f "$state/postgres.env" ]; then
    return 0
  fi
  mkdir -p "$state"
  chmod 700 "$state"
  if [ ! -f "$state/orgintel.toml" ]; then
    node --input-type=module - "$state" "$port" <<'JS'
import { randomBytes } from 'node:crypto';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
const [root, port] = process.argv.slice(2);
const envPath = join(root, 'postgres.env');
let password;
if (existsSync(envPath)) {
  password = readFileSync(envPath, 'utf8').match(/^POSTGRES_PASSWORD=([a-f0-9]+)$/m)?.[1];
  if (!password) throw new Error('Invalid development database credential file');
} else {
  password = randomBytes(32).toString('hex');
  writeFileSync(envPath, `POSTGRES_USER=restless\nPOSTGRES_DB=restless\nPOSTGRES_PASSWORD=${password}\n`, {mode: 0o600, flag: 'wx'});
}
writeFileSync(join(root, 'orgintel.toml'),
  `database_url = "postgres://restless:${password}@127.0.0.1:${port}/restless"\n`,
  {mode: 0o600, flag: 'wx'});
JS
  fi
  if docker container inspect "$name" >/dev/null 2>&1; then
    owner="$(docker inspect --format '{{index .Config.Labels "io.restless.dev-state"}}' "$name")"
    if [ "$owner" != "$state" ]; then
      printf 'Database container %s belongs to another profile; choose a different RESTLESS_RESOURCE_NAMESPACE.\n' "$name" >&2
      return 1
    fi
    docker start "$name" >/dev/null
  else
    docker run --detach --name "$name" \
      --label "io.restless.dev-state=$state" \
      --cpus 1 --memory 512m --memory-swap 512m --pids-limit 128 \
      --publish "127.0.0.1:${port}:5432" \
      --env-file "$state/postgres.env" \
      --volume "${name}-data:/var/lib/postgresql/data" \
      postgres:17-alpine >/dev/null
  fi
  for ((attempt = 0; attempt < 60; attempt += 1)); do
    # The image starts a socket-only temporary server while initialising. Wait
    # for TCP readiness so the daemon cannot race that server's shutdown.
    if docker exec "$name" pg_isready -h 127.0.0.1 -U restless -d restless >/dev/null 2>&1; then
      printf 'PASS  development PostgreSQL is ready\n'
      return 0
    fi
    sleep 0.5
  done
  printf 'Development PostgreSQL did not become ready. Inspect: docker logs %s\n' "$name" >&2
  return 1
}
