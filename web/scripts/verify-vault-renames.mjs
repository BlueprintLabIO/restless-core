// Real Authority/Vault round trip, using only disposable synthetic values in an isolated profile.
// Source that profile's infisical/authority.env before running. No secret value is logged.
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';

const origin = process.env.RESTLESS_SMOKE_ORIGIN;
const company = process.env.RESTLESS_SMOKE_COMPANY;
assert(origin && new URL(origin).hostname === '127.0.0.1' && company?.endsWith('_test'));
const base = `${origin}/api/companies/${encodeURIComponent(company)}/vault`;
const api = async (path = '', body, expected = 200) => {
	const response = await fetch(
		base + path,
		body
			? {
					method: 'POST',
					headers: {
						'Content-Type': 'application/json',
						Origin: origin,
						'Idempotency-Key': randomUUID()
					},
					body: JSON.stringify(body)
				}
			: {}
	);
	assert(
		response.status === expected,
		`Vault ${path || 'read'}: expected ${expected}, got ${response.status}`
	);
	return response.json();
};
const vaultUrl = process.env.INFISICAL_API_URL;
assert(vaultUrl && new URL(vaultUrl).hostname === '127.0.0.1', 'Use the isolated local Vault');
const login = await fetch(`${vaultUrl}/api/v1/auth/universal-auth/login`, {
	method: 'POST',
	headers: { 'Content-Type': 'application/json' },
	body: JSON.stringify({
		clientId: process.env.INFISICAL_UNIVERSAL_AUTH_CLIENT_ID,
		clientSecret: process.env.INFISICAL_UNIVERSAL_AUTH_CLIENT_SECRET
	})
});
assert(login.ok, 'Local Vault authentication must succeed');
const token = (await login.json()).accessToken;
async function assertStoredValue(reference, expected) {
	assert(
		reference.startsWith(`infisical:/companies/${company}/`),
		'Only this test company may be read'
	);
	const path = reference.slice('infisical:'.length);
	const index = path.lastIndexOf('/');
	const url = new URL(`${vaultUrl}/api/v4/secrets/${encodeURIComponent(path.slice(index + 1))}`);
	url.search = new URLSearchParams({
		projectId: process.env.INFISICAL_PROJECT_ID,
		environment: process.env.INFISICAL_ENVIRONMENT ?? 'prod',
		secretPath: path.slice(0, index),
		type: 'shared',
		viewSecretValue: 'true'
	}).toString();
	const response = await fetch(url, { headers: { Authorization: `Bearer ${token}` } });
	assert(response.ok, 'Renamed secret must exist in the real Vault');
	const body = await response.json();
	assert(
		body.secret.secretValue === expected,
		'Rename must preserve the exact value; edit must replace it'
	);
}

const initial = await api();
assert(initial.status === 'connected', 'Vault must be connected');
assert(
	!initial.references.some((r) => r.name === 'resend.production'),
	'Use a fresh test company without an email key'
);
const before = new Set(initial.secrets.map((secret) => secret.reference));
const value = `synthetic-${randomUUID()}`;
const newName = `mail-${randomUUID().slice(0, 8)}`;
try {
	await api('/secret', {
		binding: 'resend',
		secret: value,
		revision: initial.revision,
		create: true
	});
	let view = await api();
	const binding = view.references.find((r) => r.name === 'resend.production');
	assert(
		binding && !view.references.some((r) => r.name === 'resend'),
		'Email and Vault must use one canonical binding'
	);
	await assertStoredValue(binding.reference, value);
	await api(
		'/secret/rename',
		{ reference: binding.reference, name: '../other', revision: view.revision },
		400
	);
	await api(
		'/secret/rename',
		{ reference: `infisical:/companies/another_test/key`, name: newName, revision: view.revision },
		400
	);
	await api(
		'/secret/rename',
		{ reference: binding.reference, name: newName, revision: 'stale' },
		409
	);
	await api('/secret/rename', {
		reference: binding.reference,
		name: newName,
		revision: view.revision
	});
	view = await api();
	const renamed = view.secrets.find((secret) => secret.name === newName);
	assert(
		renamed && !view.secrets.some((secret) => secret.reference === binding.reference),
		'Rename must remove the old row'
	);
	assert(
		view.references.find((r) => r.name === 'resend.production')?.reference === renamed.reference,
		'The sending integration must follow the renamed key'
	);
	await assertStoredValue(renamed.reference, value);
	const replacement = `replacement-${randomUUID()}`;
	await api('/secret', {
		reference: renamed.reference,
		secret: replacement,
		revision: view.revision
	});
	await assertStoredValue(renamed.reference, replacement);
	await api(
		'/secret',
		{ binding: newName, secret: value, revision: view.revision, create: true },
		409
	);
	await api('/secret/rename', {
		reference: renamed.reference,
		name: newName,
		revision: view.revision
	});
	console.log(
		'PASS canonical Resend save, exact-value rename/edit, consumer references, stale revision, path/company isolation and collision refusal'
	);
} finally {
	const view = await api();
	for (const secret of view.secrets.filter((secret) => !before.has(secret.reference))) {
		const current = await api();
		await api('/secret/delete', { reference: secret.reference, revision: current.revision });
	}
	const cleaned = await api();
	assert(
		cleaned.secrets.every((secret) => before.has(secret.reference)),
		'Synthetic Vault values must be removed'
	);
	assert(!cleaned.references.some(reference => reference.name === 'resend.production' || reference.name === newName),
		'Deleting the renamed value must remove its integration binding and name alias');
}
