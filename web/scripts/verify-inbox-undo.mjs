/* Real daemon + PostgreSQL verification of the handoff Undo window. No mocked replies. */
import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { randomUUID } from 'node:crypto';
const origin = process.env.RESTLESS_SMOKE_ORIGIN,
	company = process.env.RESTLESS_SMOKE_COMPANY,
	home = process.env.RESTLESS_HOME;
assert(
	origin &&
		new URL(origin).hostname === '127.0.0.1' &&
		company?.endsWith('_test') &&
		home?.includes('/.restless-dev/'),
	'isolated test profile required'
);
const out = process.env.RESTLESS_VERIFY_OUTPUT ?? '/tmp/restless-inbox-undo';
await mkdir(out, { recursive: true });
const pg = createRequire(new URL('../../services/identity/package.json', import.meta.url))('pg');
const adminUrl = (await readFile(home + '/orgintel.toml', 'utf8')).match(
	/database_url\s*=\s*"([^"]+)"/
)[1];
const admin = new pg.Client({ connectionString: adminUrl });
await admin.connect();
const databases = await admin.query(
	"SELECT datname FROM pg_database WHERE datname LIKE '%\\_' || $1 || '\\_%' ESCAPE '\\'",
	[company]
);
await admin.end();
assert.equal(databases.rowCount, 1);
const url = new URL(adminUrl);
url.pathname = '/' + databases.rows[0].datname;
const db = new pg.Client({ connectionString: url.toString() });
await db.connect();
await db.query('SET search_path TO "' + company + '"');
const old = (
	await db.query(
		"SELECT d.id FROM owner_handoff_decisions d JOIN owner_handoffs h ON h.id=d.handoff_id JOIN work w ON w.id=h.work_id WHERE w.outcome='Confirm an isolated human step' AND d.state<>'pending' AND d.seen_at IS NULL"
	)
).rows;
for (const row of old) {
	const r = await fetch(
		origin + '/api/companies/' + company + '/handoff-decisions/' + row.id + '/seen',
		{ method: 'POST', headers: { Origin: origin } }
	);
	assert(r.ok);
}
const browser = await chromium.launch();
const report = [],
	errors = [];
const api = '/api/companies/' + company;
const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
async function until(fn, label) {
	for (let i = 0; i < 100; i++) {
		if (await fn()) return;
		await wait(100);
	}
	throw Error(label);
}
async function fixture(label) {
	const work = randomUUID(),
		handoff = randomUUID(),
		title = 'Undo ' + label + ' ' + work.slice(0, 8);
	await db.query(
		"INSERT INTO work(id,owner_id,title,outcome,status,resolution,priority,commissioned_by) VALUES($1,'exec',$2,'Confirm an isolated human step','blocked','Waiting for owner',1,'exec')",
		[work, title]
	);
	await db.query(
		"INSERT INTO owner_handoffs(id,work_id,requested_by,category,requested_action,prepared_state,resume_condition) VALUES($1,$2,'exec','mfa',$3,'Synthetic step, no external action','Owner confirms')",
		[handoff, work, title]
	);
	return { work, handoff, title, item: 'orgintel:handoff:' + handoff };
}
async function state(f) {
	return (await db.query('SELECT state FROM owner_handoffs WHERE id=$1', [f.handoff])).rows[0]
		.state;
}
async function receipt(id) {
	return (await db.query('SELECT * FROM owner_handoff_decisions WHERE id=$1', [id])).rows[0];
}
async function open(page, f) {
	await page.goto(origin + '/' + company + '?item=' + encodeURIComponent(f.item), {
		waitUntil: 'networkidle'
	});
	try {
		await page
			.locator('[data-inbox-actions]')
			.getByRole('button', { name: 'Mark done', exact: true })
			.waitFor();
	} catch (cause) {
		await page.screenshot({ path: out + '/open-failure.png' });
		await writeFile(
			out + '/open-failure.json',
			JSON.stringify(
				{
					fixture: f,
					url: page.url(),
					headings: await page.getByRole('heading').allTextContents(),
					body: await page.locator('body').innerText(),
					source: await (await fetch(origin + api + '/attention')).json()
				},
				null,
				2
			)
		);
		throw cause;
	}
}
async function queue(page) {
	const response = page.waitForResponse(
		(r) => r.url().endsWith('/pending-decision') && r.request().method() === 'POST'
	);
	const start = Date.now();
	await page
		.locator('[data-inbox-actions]')
		.getByRole('button', { name: 'Mark done', exact: true })
		.click();
	const r = await response;
	assert(r.ok(), await r.text());
	const value = await r.json();
	return { ...value, latency: Date.now() - start };
}
try {
	for (const width of [1440, 390]) {
		const context = await browser.newContext({
			viewport: { width, height: 900 },
			reducedMotion: 'reduce'
		});
		let page = await context.newPage();
		page.on('pageerror', (e) => errors.push(e.message));
		const undo = await fixture('cancel-' + width);
		await open(page, undo);
		const queued = await queue(page);
		assert.equal(queued.state, 'pending');
		assert.equal(await state(undo), 'pending');
		if (width === 390 && (await page.getByRole('link', { name: '← Inbox', exact: true }).count()))
			await page.getByRole('link', { name: '← Inbox', exact: true }).click();
		await until(
			async () => !new URL(page.url()).searchParams.get('item')?.includes(undo.handoff),
			'next request selected immediately'
		);
		assert.equal(await page.getByText('This request was already resolved.').count(), 0);
		await page.getByRole('button', { name: 'Undo', exact: true }).click();
		await until(async () => (await receipt(queued.id)).state === 'canceled', 'Undo persisted');
		await wait(5400);
		assert.equal(await state(undo), 'pending');
		if (width === 390 && (await page.getByRole('link', { name: '← Inbox', exact: true }).count()))
			await page.getByRole('link', { name: '← Inbox', exact: true }).click();
		await until(
			async () => (await page.getByRole('link', { name: new RegExp(undo.title) }).count()) > 0,
			'Canceled request returns to the live Inbox'
		);
		report.push({ width, scenario: 'Undo prevents source effect', latency: queued.latency });
		const reload = await fixture('reconnect-' + width);
		await open(page, reload);
		const saved = await queue(page);
		const deadline = Date.parse(saved.undo_until);
		await page.close();
		page = await context.newPage();
		page.on('pageerror', (e) => errors.push(e.message));
		await page.goto(origin + '/' + company, { waitUntil: 'networkidle' });
		await page.getByRole('button', { name: 'Undo', exact: true }).waitFor();
		assert(Date.now() < deadline, 'Reconnect retained time for Undo');
		await page.screenshot({ path: out + '/reconnect-' + width + '.png' });
		console.log(
			'Reconnect Undo geometry',
			width,
			await page.getByRole('button', { name: 'Undo', exact: true }).boundingBox()
		);
		await page.getByRole('button', { name: 'Undo', exact: true }).hover();
		await wait(Math.max(0, deadline - Date.now() + 150));
		assert.equal(
			await page.getByRole('button', { name: 'Undo', exact: true }).count(),
			0,
			'Hover cannot extend deadline'
		);
		await until(
			async () => (await receipt(saved.id)).state === 'applied',
			'source commit after original deadline'
		);
		assert.equal(await state(reload), 'resolved');
		report.push({
			width,
			scenario: 'Reconnect preserves deadline; hover cannot extend; source applies'
		});
		const fail = await fixture('network-' + width);
		await open(page, fail);
		await page.route('**/pending-decision', (route) => route.abort('connectionfailed'));
		await page
			.locator('[data-inbox-actions]')
			.getByRole('button', { name: 'Mark done', exact: true })
			.click();
		await page
			.getByRole('alert')
			.filter({ hasNot: page.getByRole('button', { name: 'Open request', exact: true }) })
			.waitFor();
		assert.equal(await state(fail), 'pending');
		assert.equal(
			(
				await db.query(
					'SELECT count(*)::int AS count FROM owner_handoff_decisions WHERE handoff_id=$1',
					[fail.handoff]
				)
			).rows[0].count,
			0
		);
		await page
			.locator('[data-inbox-actions]')
			.getByRole('button', { name: 'Mark done', exact: true })
			.waitFor();
		await page.unroute('**/pending-decision');
		report.push({ width, scenario: 'Failed save leaves original request' });
		const stale = await fixture('late-change-' + width);
		await open(page, stale);
		const staleReceipt = await queue(page);
		await db.query(
			"UPDATE owner_handoffs SET requested_action=requested_action || ' updated' WHERE id=$1",
			[stale.handoff]
		);
		await until(
			async () => (await receipt(staleReceipt.id)).state === 'failed',
			'changed request refused'
		);
		assert.equal(await state(stale), 'pending');
		await page.getByRole('button', { name: 'Open request', exact: true }).waitFor();
		await page.screenshot({ path: out + '/late-change-' + width + '.png' });
		await page.reload({ waitUntil: 'networkidle' });
		await page.getByRole('button', { name: 'Open request', exact: true }).waitFor();
		assert.equal(
			(await receipt(staleReceipt.id)).seen_at,
			null,
			'Failure survives reload until explicit acknowledgement'
		);
		await page.getByRole('button', { name: 'Open request', exact: true }).click();
		await until(
			async () => Boolean((await receipt(staleReceipt.id)).seen_at),
			'Explicit action acknowledges failure'
		);
		report.push({ width, scenario: 'Late change returns request; failure survives reload' });
		await context.close();
	}
	assert.deepEqual(errors, []);
	await writeFile(out + '/report.json', JSON.stringify({ report, errors }, null, 2));
	console.log(JSON.stringify(report));
} catch (cause) {
	for (const context of browser.contexts())
		for (const page of context.pages()) {
			await page.screenshot({ path: out + '/failure.png' });
			await writeFile(
				out + '/failure.json',
				JSON.stringify({ url: page.url(), body: await page.locator('body').innerText() }, null, 2)
			);
		}
	throw cause;
} finally {
	await browser.close();
	await db.end();
}
