#!/usr/bin/env node
/* Token parity on the subscription route: give a disposable dev stack's model broker a scripted
 * Claude sign-in, so the company can run on `omp-oauth:anthropic` (the route the owner's companies
 * use) against tools/mock-model instead of an API key.
 *
 *   node seed-sign-in.mjs --broker http://127.0.0.1:<7789+offset> --token-file <profile>/auth-broker.token
 *
 * The token file is the dev stack's own broker bearer under its port-offset OMP profile; it is read,
 * never printed. The credential is a synthetic `sk-ant-oat01-` token the mock accepts and Anthropic
 * would refuse; the stack's broker and gateway send it only to RESTLESS_ANTHROPIC_BASE_URL (the
 * mock). It expires in 30 days so the broker never tries to refresh it. */
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const args = Object.fromEntries(
	process.argv.slice(2).reduce((pairs, value, index, all) => {
		if (value.startsWith('--')) pairs.push([value.slice(2), all[index + 1]]);
		return pairs;
	}, [])
);
const broker = args.broker;
assert(broker && new URL(broker).hostname === '127.0.0.1', '--broker must be the local dev broker');
const token = (await readFile(args['token-file'], 'utf8')).trim();
assert(token.length >= 32, 'the broker token file is empty');
const headers = { authorization: `Bearer ${token}`, 'content-type': 'application/json' };

async function snapshot() {
	const response = await fetch(`${broker}/v1/snapshot`, { headers });
	assert(response.ok, `broker snapshot ${response.status}`);
	return (await response.json()).credentials ?? [];
}

const signedIn = (rows) =>
	rows.filter((row) => row.provider === 'anthropic' && row.credential?.type === 'oauth');
if (signedIn(await snapshot()).length === 0) {
	const response = await fetch(`${broker}/v1/credential`, {
		method: 'POST',
		headers,
		body: JSON.stringify({
			provider: 'anthropic',
			credential: {
				type: 'oauth',
				access: 'sk-ant-oat01-tokparity-mock-sign-in-not-a-real-token',
				refresh: 'sk-ant-ort01-tokparity-mock-refresh-not-a-real-token',
				expires: Date.now() + 30 * 24 * 3600 * 1000
			}
		})
	});
	assert(response.ok, `broker refused the scripted sign-in: ${response.status}`);
}
assert.equal(signedIn(await snapshot()).length, 1, 'the broker holds one scripted Claude sign-in');

/* After the company switches to the sign-in, the gateway reload disables every other Anthropic
 * credential: wait for that, so the first measured call goes through the subscription route. */
if (args.wait) {
	const deadline = Date.now() + Number(args.wait) * 1000;
	for (;;) {
		const anthropic = (await snapshot()).filter((row) => row.provider === 'anthropic');
		if (anthropic.length === 1 && anthropic[0].credential?.type === 'oauth') break;
		assert(Date.now() < deadline, 'the gateway did not converge on the sign-in');
		await new Promise((resolve) => setTimeout(resolve, 2000));
	}
}
console.log('scripted Claude sign-in ready');
