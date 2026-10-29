/* Two real clients reviewing one isolated Runtime outcome. No mocked review response or judge. */
import { chromium } from 'playwright';
import { createConnection } from 'node:net';
import { join } from 'node:path';
import { mkdir, writeFile } from 'node:fs/promises';
import assert from 'node:assert/strict';
const origin = process.env.RESTLESS_SMOKE_ORIGIN;
const company = process.env.RESTLESS_SMOKE_COMPANY;
const home = process.env.RESTLESS_HOME;
if (
	!origin ||
	!['127.0.0.1', 'localhost'].includes(new URL(origin).hostname) ||
	!company?.endsWith('_test') ||
	!home?.includes('/.restless-dev/')
)
	throw new Error('Use a local isolated development profile and _test company');
const out = process.env.RESTLESS_VERIFY_OUTPUT ?? '/tmp/restless-review-versions';
await mkdir(out, { recursive: true });
const api = `${origin}/api/companies/${company}`;
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext();
const report = {};
const rpc = (request) =>
	new Promise((resolve, reject) => {
		const socket = createConnection(join(home, 'restlessd.sock'));
		let line = '';
		socket.setEncoding('utf8');
		socket.setTimeout(15000, () => {
			socket.destroy();
			reject(new Error('Fixture command timed out'));
		});
		socket.on('connect', () =>
			socket.write(JSON.stringify({ principal: 'owner', company, ...request }) + '\n')
		);
		socket.on('data', (chunk) => {
			line += chunk;
			if (!line.includes('\n')) return;
			socket.end();
			const response = JSON.parse(line.split('\n')[0]);
			response.ok ? resolve(response.data) : reject(new Error(JSON.stringify(response.error)));
		});
		socket.on('error', reject);
	});
async function attention() {
	const response = await context.request.get(api + '/attention');
	assert(response.ok());
	return response.json();
}
async function decide(item, version) {
	return context.request.post(`${api}/handoffs/${item.source.reference}/review`, {
		headers: { Origin: origin },
		data: { decision: 'accept', feedback: '', expected_version: version }
	});
}
try {
	const original = (await attention()).items.find(
		(item) => item.title === 'Inbox outcome version verification'
	);
	assert(original?.review_version, 'Seed a pending real outcome review with its version');
	const noVersion = await decide(original, undefined);
	assert.equal(noVersion.status(), 409, 'A blind/old-client acceptance needs a current review');
	const desktop = await context.newPage({ viewport: { width: 1440, height: 900 } });
	const phone = await context.newPage({ viewport: { width: 390, height: 844 } });
	const href = `${origin}/${company}?review=${encodeURIComponent(original.id)}`;
	await desktop.goto(`${origin}/${company}/work/${original.work_id}`);
	await desktop.getByRole('button', { name: 'Open the full brief →', exact: true }).click();
	const reviewLink = desktop.getByRole('link', { name: 'Review in Inbox', exact: true });
	await reviewLink.waitFor();
	assert.equal(
		await desktop.getByRole('button', { name: 'Accept', exact: true }).count(),
		0,
		'Work summaries send the owner to the candidate rather than accepting unseen work'
	);
	assert.equal(
		await reviewLink.getAttribute('href'),
		`/${company}?review=${encodeURIComponent(original.id)}`
	);
	for (const page of [desktop, phone]) {
		await page.goto(href);
		await page
			.frameLocator('.outcome-preview iframe')
			.getByRole('button', { name: '0', exact: true })
			.waitFor({ timeout: 30000 });
		await page
			.frameLocator('.outcome-preview iframe')
			.getByRole('button', { name: '0', exact: true })
			.click();
	}
	const before = await desktop
		.getByRole('button', { name: 'Accept outcome', exact: true })
		.boundingBox();
	const headline = 'Updated candidate for the version check';
	await rpc({
		cmd: 'work-handoff-prepare-brief',
		id: original.source.reference,
		as_actor: 'exec',
		owner_kind: 'outcome_review',
		headline,
		situation: original.what_happened,
		impact: original.why_it_matters,
		recommendation: original.requested_action,
		no_action: original.if_no_action
	});
	await rpc({
		cmd: 'work-handoff-escalate',
		id: original.source.reference,
		as_actor: 'exec',
		reason: 'The refreshed isolated candidate explanation is ready for owner review.'
	});
	const stale = await decide(original, original.review_version);
	assert.equal(stale.status(), 409, 'The real endpoint refuses the old tab version');
	assert.match(await stale.text(), /changed after you opened/i);
	assert(
		(await attention()).items.some((item) => item.id === original.id),
		'Stale acceptance does not consume the review'
	);
	for (const page of [desktop, phone]) {
		// The same cross-window notification used after an ordinary owner decision; no data is faked.
		await page.evaluate((company) => {
			const channel = new BroadcastChannel('restless-attention');
			channel.postMessage(company);
			channel.close();
		}, company);
		await page
			.getByRole('button', { name: 'Review new version', exact: true })
			.waitFor({ timeout: 20000 });
		await page.getByRole('heading', { name: original.title, exact: true }).waitFor();
		await page
			.frameLocator('.outcome-preview iframe')
			.getByRole('button', { name: '1', exact: true })
			.waitFor();
	}
	const after = await desktop
		.getByRole('button', { name: 'Review new version', exact: true })
		.boundingBox();
	for (const key of ['x', 'y', 'width', 'height'])
		assert(Math.abs(before[key] - after[key]) <= 1, `${key} moved when a newer version arrived`);
	await desktop.screenshot({ path: `${out}/new-version-desktop.png`, fullPage: true });
	await phone.screenshot({ path: `${out}/new-version-phone.png`, fullPage: true });
	await desktop.getByRole('button', { name: 'Review new version', exact: true }).click();
	await desktop.getByRole('heading', { name: headline, exact: true }).waitFor();
	await desktop
		.frameLocator('.outcome-preview iframe')
		.getByRole('button', { name: '0', exact: true })
		.waitFor();
	const saved = desktop.waitForResponse(
		(response) =>
			response.url().endsWith(`/handoffs/${original.source.reference}/pending-decision`) &&
			response.request().method() === 'POST'
	);
	await desktop.getByRole('button', { name: 'Accept outcome', exact: true }).click();
	const queued = await saved;
	assert(queued.ok(), 'The reviewed current version saves through the actual UI');
	const receipt = await queued.json();
	assert.equal(receipt.state, 'pending');
	await desktop.waitForFunction(
		async ({ api, id }) => {
			const data = await (await fetch(api + '/attention')).json();
			return data.work_graph.work.some((row) => row.id === id && row.status === 'completed');
		},
		{ api, id: original.work_id }
	);
	const replay = await decide(original, original.review_version);
	assert(!replay.ok(), 'An old tab cannot decide the consumed review again');
	assert(!(await attention()).items.some((item) => item.id === original.id));
	Object.assign(report, {
		missingVersion: noVersion.status(),
		staleVersion: stale.status(),
		oldTabReplay: replay.status(),
		previewStayedOnDisplayedVersion: true,
		buttonGeometryStable: true,
		workSummaryOpensReview: true,
		currentVersionAccepted: true
	});
	console.log(
		'PASS missing/stale versions refused; two clients keep their displayed candidate; explicit newer review accepts once'
	);
} finally {
	await writeFile(`${out}/report.json`, JSON.stringify(report, null, 2));
	await browser.close();
}
