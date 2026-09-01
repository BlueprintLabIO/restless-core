#!/usr/bin/env node
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const composePath = join(root, 'infra/account-plane/compose.yml');
const digestRef = (letter) => `registry.example.test/restless/image@sha256:${letter.repeat(64)}`;
const env = {
  ...process.env,
  RESTLESS_ACCOUNT_PLANE_IMAGE: digestRef('a'),
  RESTLESS_COMPANY_IMAGE: digestRef('b'),
  RESTLESS_ENTRY_ISSUER: 'https://fleet.example.test',
  RESTLESS_ENTRY_JWKS_URL: 'https://fleet.example.test/.well-known/jwks.json',
  RESTLESS_ENTRY_OWNER_ID: '00000000-0000-0000-0000-000000000001',
  RESTLESS_ENTRY_PLANE_ID: '00000000-0000-0000-0000-000000000002',
  RESTLESS_ENTRY_HOST: 'owner.example.test',
  RESTLESS_DESIRED_REVISION: '1',
  RESTLESS_RELEASE_MANIFEST_DIGEST: `sha256:${'c'.repeat(64)}`,
  RESTLESS_PLANE_DATABASE_PASSWORD: 'database-test-password',
  RESTLESS_PLANE_DATABASE_URL: 'postgres://restless:database-test-password@plane-database/restless',
  RESTLESS_PLANE_READINESS_TOKEN: 'plane-readiness-test-token-at-least-32-characters',
  RESTLESS_CELL_READINESS_TOKEN: 'cell-readiness-test-token-distinct-and-over-32-characters',
  INFISICAL_API_URL: 'https://app.infisical.com',
  INFISICAL_PROJECT_ID: 'project-test',
  INFISICAL_UNIVERSAL_AUTH_CLIENT_ID: 'client-test',
  INFISICAL_UNIVERSAL_AUTH_CLIENT_SECRET: 'infisical-test-secret',
};
const template = readFileSync(composePath, 'utf8');
const expectedMarkers = [
  'ACCOUNT_PLANE_IMAGE',
  'COMPANY_RUNTIME_IMAGE',
  'CORE_RELEASE_MANIFEST_DIGEST',
  'DESIRED_REVISION',
  'FLEET_ENTRY_ISSUER',
  'FLEET_ENTRY_JWKS_URL',
  'HOSTNAME',
  'OWNER_ID',
  'PLANE_ID',
];
const actualMarkers = [...new Set([...template.matchAll(/\{\{([A-Z0-9_]+)\}\}/g)].map((match) => match[1]))].sort();
if (actualMarkers.join(',') !== expectedMarkers.sort().join(',')) {
  throw new Error(`account-plane template marker contract drifted: ${actualMarkers.join(',')}`);
}
if (template.includes('{{RUNTIME_BOOTSTRAP_TOKEN}}')) {
  throw new Error('runtime bootstrap secret must never be embedded as a provisioning marker');
}
const renderedTemplate = template
  .replaceAll('{{ACCOUNT_PLANE_IMAGE}}', env.RESTLESS_ACCOUNT_PLANE_IMAGE)
  .replaceAll('{{COMPANY_RUNTIME_IMAGE}}', env.RESTLESS_COMPANY_IMAGE)
  .replaceAll('{{CORE_RELEASE_MANIFEST_DIGEST}}', env.RESTLESS_RELEASE_MANIFEST_DIGEST)
  .replaceAll('{{FLEET_ENTRY_ISSUER}}', env.RESTLESS_ENTRY_ISSUER)
  .replaceAll('{{FLEET_ENTRY_JWKS_URL}}', env.RESTLESS_ENTRY_JWKS_URL)
  .replaceAll('{{OWNER_ID}}', env.RESTLESS_ENTRY_OWNER_ID)
  .replaceAll('{{PLANE_ID}}', env.RESTLESS_ENTRY_PLANE_ID)
  .replaceAll('{{HOSTNAME}}', env.RESTLESS_ENTRY_HOST)
  .replaceAll('{{DESIRED_REVISION}}', env.RESTLESS_DESIRED_REVISION);
if (/\{\{[A-Z0-9_]+\}\}/.test(renderedTemplate)) {
  throw new Error('account-plane template contains an unverified provisioning marker');
}
const rendered = spawnSync(
  'docker',
  ['compose', '-f', '-', 'config', '--format', 'json'],
  { cwd: root, env, input: renderedTemplate, encoding: 'utf8' },
);
if (rendered.status !== 0) {
  process.stderr.write(rendered.stderr);
  process.exit(rendered.status ?? 1);
}
const model = JSON.parse(rendered.stdout);
const problems = [];
const services = model.services ?? {};
if (Object.keys(services).sort().join(',') !== 'account-plane,plane-database') {
  problems.push('template must contain exactly account-plane and plane-database');
}
const plane = services['account-plane'] ?? {};
const database = services['plane-database'] ?? {};
const networkNames = (service) => Object.keys(service.networks ?? {}).sort().join(',');
if (networkNames(database) !== 'plane-db') {
  problems.push('plane-database must have only the internal plane-db network');
}
if (networkNames(plane) !== 'plane-db,plane-egress,public-proxy') {
  problems.push('account-plane must have only database, outbound, and public-proxy networks');
}
if (model.networks?.['plane-db']?.internal !== true || model.networks?.['public-proxy']?.external !== true) {
  problems.push('database network must be internal and proxy network external');
}
if (!/@sha256:[0-9a-f]{64}$/.test(plane.image ?? '') || !/@sha256:[0-9a-f]{64}$/.test(database.image ?? '')) {
  problems.push('both deployed images must be immutable digests');
}
if (plane.read_only !== true || !(plane.tmpfs ?? []).some((mount) => String(mount.target ?? mount).startsWith('/tmp'))) {
  problems.push('account-plane root filesystem must be read-only with bounded temporary storage');
}
for (const [name, service] of Object.entries(services)) {
  if (service.privileged || service.ports?.length) problems.push(`${name} may not be privileged or publish a host port`);
  if ((service.volumes ?? []).some((mount) => mount.type === 'bind' || String(mount.source).includes('docker.sock'))) {
    problems.push(`${name} may not receive a host bind or Docker socket`);
  }
  if (!(service.cap_drop ?? []).includes('ALL') || !(service.security_opt ?? []).includes('no-new-privileges:true')) {
    problems.push(`${name} must drop ambient capabilities and forbid privilege gain`);
  }
}
const planeSecrets = new Set((plane.secrets ?? []).map((secret) => secret.source));
for (const required of ['plane_database_url', 'plane_readiness_token', 'cell_readiness_token', 'infisical_client_secret']) {
  if (!planeSecrets.has(required)) problems.push(`account-plane is missing secret ${required}`);
}
if (planeSecrets.has('plane_database_password')) problems.push('account-plane must not receive the database bootstrap password');
const databaseSecrets = new Set((database.secrets ?? []).map((secret) => secret.source));
if (databaseSecrets.size !== 1 || !databaseSecrets.has('plane_database_password')) {
  problems.push('plane-database must receive only its bootstrap password');
}
for (const forbidden of ['RESTLESS_PLANE_DATABASE_URL', 'RESTLESS_PLANE_READINESS_TOKEN', 'RESTLESS_CELL_READINESS_TOKEN', 'INFISICAL_UNIVERSAL_AUTH_CLIENT_SECRET']) {
  if (forbidden in (plane.environment ?? {})) problems.push(`${forbidden} must be a mounted secret, not plaintext environment`);
}
if (plane.environment?.RESTLESS_ENTRY_MODE !== 'network'
    || plane.environment?.RESTLESS_DATABASE_URL_FILE !== '/run/secrets/plane_database_url'
    || plane.environment?.RESTLESS_PLANE_READINESS_TOKEN_FILE !== '/run/secrets/plane_readiness_token'
    || plane.environment?.RESTLESS_CELL_READINESS_TOKEN_FILE !== '/run/secrets/cell_readiness_token') {
  problems.push('account-plane network entry and file-backed secret configuration drifted');
}
const labels = plane.labels ?? {};
if (labels['traefik.enable'] !== 'true'
    || !Object.entries(labels).some(([key, value]) => key.endsWith('.rule') && value === 'Host(`owner.example.test`)')
    || !Object.entries(labels).some(([key, value]) => key.endsWith('.tls') && value === 'true')) {
  problems.push('account-plane must publish only its exact TLS hostname through Traefik');
}

if (problems.length) {
  console.error('account-plane Compose verification FAILED:');
  for (const problem of problems) console.error(`  - ${problem}`);
  process.exit(1);
}
const sha256 = createHash('sha256').update(readFileSync(composePath)).digest('hex');
console.log(`account-plane Compose OK — sha256:${sha256}`);
