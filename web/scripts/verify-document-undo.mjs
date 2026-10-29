/* Native document Undo against the official isolated daemon, real PostgreSQL and real browser. */
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
		home?.includes('/.restless-dev/')
);
const out = process.env.RESTLESS_VERIFY_OUTPUT ?? '/tmp/restless-document-undo';
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
const api = '/api/companies/' + company,
	report = [],
	errors = [];
function record(entry) {
	report.push(entry);
	console.log(JSON.stringify(entry));
}
const browser = await chromium.launch();
const wait = (ms) => new Promise((r) => setTimeout(r, ms));
async function until(fn, label) {
	for (let i = 0; i < 150; i++) {
		if (await fn()) return;
		await wait(100);
	}
	throw Error(label);
}
async function json(path, body, method = 'POST') {
	const response = await fetch(
		origin + api + path,
		body
			? {
					method,
					headers: {
						Origin: origin,
						'Content-Type': 'application/json',
						'Idempotency-Key': randomUUID()
					},
					body: JSON.stringify(body)
				}
			: {}
	);
	const result = await response.json();
	assert(response.ok, JSON.stringify({ status: response.status, result }));
	return result;
}
async function fixture(label) {
	const title = 'Native Undo ' + label + ' ' + randomUUID().slice(0, 8);
	const created = await json('/documents', {
		title,
		kind: 'review',
		visibility: 'company',
		linked_room_id: null,
		inherit_room_visibility: false,
		reason: 'Isolated browser verification',
		content_json: {
			type: 'doc',
			content: [
				{
					type: 'paragraph',
					attrs: { block_id: randomUUID() },
					content: [
						{
							type: 'text',
							text: 'Review this saved document. Undo must keep the note unpublished.'
						}
					]
				}
			]
		}
	});
	const document = created.document_id,
		view = await json('/documents/' + document);
	await json(
		'/documents/' + document + '/participants/exec',
		{ expected_document_version: view.document.version, access: 'edit' },
		'PUT'
	);
	const shared = await json('/documents/' + document),
		request = randomUUID();
	await db.query('BEGIN');
	try {
		await db.query("UPDATE native_documents SET status='in_review',version=version+1 WHERE id=$1", [
			document
		]);
		await db.query(
			"INSERT INTO native_document_reviews(id,document_id,requested_version_id,requested_by_actor_id,reviewer_actor_id,summary) VALUES($1,$2,$3,'exec','owner','Review the browser verification document')",
			[request, document, shared.current_version.version.id]
		);
		await db.query('COMMIT');
	} catch (error) {
		await db.query('ROLLBACK');
		throw error;
	}
	return {
		document,
		request,
		title,
		item: 'document:review:' + request,
		note: 'Private review note ' + label,
		name: 'Accepted ' + label
	};
}
async function groupReceipts(group) {
	const rows = [];
	for (const f of group) rows.push(await receipt(f.receipt));
	return rows;
}
async function receipt(id) {
	return (await db.query('SELECT * FROM owner_document_decisions WHERE id=$1', [id])).rows[0];
}
async function state(f) {
	return (await db.query('SELECT status::text FROM native_documents WHERE id=$1', [f.document]))
		.rows[0].status;
}
async function notes(f) {
	return (
		await db.query(
			"SELECT id,mentioned_actor_ids FROM native_document_comments WHERE document_id=$1 AND position($2 in plain_text)>0 AND author_actor_id='owner'",
			[f.document, f.note]
		)
	).rows;
}
async function open(page, f) {
	await page.goto(origin + '/' + company + '?item=' + encodeURIComponent(f.item), {
		waitUntil: 'networkidle'
	});
	await page.getByRole('button', { name: 'Accept this version', exact: true }).waitFor();
	await page.getByRole('textbox', { name: 'Note', exact: true }).waitFor();
	const note = await page.getByRole('textbox', { name: 'Note', exact: true }).boundingBox();
	const action = await page
		.getByRole('button', { name: 'Accept this version', exact: true })
		.boundingBox();
	assert(
		note && action && note.y < action.y - 30,
		'The review note is reachable above its decision bar without scrolling through a blank preview'
	);
}
async function fill(page, f) {
	await page.getByLabel('Version name', { exact: true }).fill(f.name);
	await page.getByRole('textbox', { name: 'Note', exact: true }).fill(f.note);
}
async function queue(page) {
	const pending = page.waitForResponse(
		(r) => r.url().endsWith('/pending-decision') && r.request().method() === 'POST'
	);
	const started = Date.now();
	await page.getByRole('button', { name: 'Accept this version', exact: true }).click();
	const response = await pending;
	assert(response.ok(), await response.text());
	return { ...(await response.json()), latency: Date.now() - started };
}
async function clearNotices(page) {
	for (let i = 0; i < 8; i++) {
		const close = page.getByRole('button', { name: 'Dismiss', exact: true });
		if (!(await close.count())) break;
		await close.first().click();
	}
}
try {
	// This entire explicitly disposable profile belongs to the runner. Acknowledge results
	// retained by an earlier interrupted run through the real owner API.
	for (const previous of (await json('/document-decisions')).items) {
		const title = (
			await db.query('SELECT title FROM native_documents WHERE id=$1', [previous.document_id])
		).rows[0]?.title;
		if (
			previous.state !== 'pending' &&
			(title?.startsWith('Native Undo ') || title === 'Native review across daemon restart')
		)
			await json('/document-decisions/' + previous.id + '/seen', {});
	}
	for (const width of [1440, 390]) {
		const context = await browser.newContext({
			viewport: { width, height: 900 },
			reducedMotion: 'reduce'
		});
		let page = await context.newPage();
		page.on('pageerror', (e) => errors.push(e.message));
		const undo = await fixture('cancel-' + width);
		await open(page, undo);
		await fill(page, undo);
		const saved = await queue(page);
		assert.equal(saved.state, 'pending');
		assert.equal(await state(undo), 'in_review');
		assert.equal((await notes(undo)).length, 0);
		await page.getByRole('button', { name: 'Undo', exact: true }).click();
		await until(async () => (await receipt(saved.id)).state === 'canceled', 'Undo persisted');
		await wait(5200);
		assert.equal(await state(undo), 'in_review');
		assert.equal((await notes(undo)).length, 0);
		await open(page, undo);
		await until(
			async () =>
				(await page.getByRole('textbox', { name: 'Note', exact: true }).inputValue()) === undo.note,
			'Canceled form note restored after re-open'
		);
		assert.equal(await page.getByLabel('Version name', { exact: true }).inputValue(), undo.name);
		await page.screenshot({ path: out + '/restored-' + width + '.png' });
		record({
			width,
			scenario: 'Undo keeps review and unpublished note; form restored after re-open',
			latency: saved.latency
		});
		const network = await fixture('network-' + width);
		await open(page, network);
		await fill(page, network);
		await page.route('**/pending-decision', (route) => route.abort('connectionfailed'));
		await page.getByRole('button', { name: 'Accept this version', exact: true }).click();
		await page.getByRole('alert').waitFor();
		assert.equal(await state(network), 'in_review');
		assert.equal((await notes(network)).length, 0);
		assert.equal(
			await page.getByRole('textbox', { name: 'Note', exact: true }).inputValue(),
			network.note
		);
		assert.equal(
			(
				await db.query(
					'SELECT count(*)::int AS n FROM owner_document_decisions WHERE request_id=$1',
					[network.request]
				)
			).rows[0].n,
			0
		);
		await page.unroute('**/pending-decision');
		await clearNotices(page);
		record({ width, scenario: 'Failed durable save retains request and typed note' });
		const lost = await fixture('lost-reply-' + width);
		await open(page, lost);
		await fill(page, lost);
		let lostReceipt, lostInput;
		await page.route('**/pending-decision', async (route) => {
			lostInput = route.request().postDataJSON();
			const response = await route.fetch();
			assert(response.ok());
			lostReceipt = await response.json();
			await route.abort('connectionfailed');
		});
		await page.getByRole('button', { name: 'Accept this version', exact: true }).click();
		await until(async () => Boolean(lostReceipt), 'Server saved request before connection broke');
		await page.unroute('**/pending-decision');
		await page.getByRole('button', { name: 'Undo', exact: true }).waitFor({ timeout: 4500 });
		await page.getByRole('button', { name: 'Undo', exact: true }).click();
		await until(
			async () => (await receipt(lostReceipt.id)).state === 'canceled',
			'Lost-response intent recovered and undone'
		);
		const lostReplay = await json(
			'/documents/' + lost.document + '/reviews/' + lost.request + '/pending-decision',
			lostInput
		);
		assert.equal(lostReplay.id, lostReceipt.id);
		assert.equal(lostReplay.state, 'canceled');
		assert.equal(lostReplay.undo_until, lostReceipt.undo_until);
		await wait(5200);
		assert.equal(await state(lost), 'in_review');
		assert.equal((await notes(lost)).length, 0);
		record({
			width,
			scenario: 'Lost save response recovers Undo; replay retains the canceled receipt and deadline'
		});
		const reconnect = await fixture('reconnect-' + width);
		await open(page, reconnect);
		await fill(page, reconnect);
		const pending = await queue(page);
		const deadline = Date.parse(pending.undo_until);
		await page.close();
		page = await context.newPage();
		page.on('pageerror', (e) => errors.push(e.message));
		await page.goto(origin + '/' + company, { waitUntil: 'networkidle' });
		await page.getByRole('button', { name: 'Undo', exact: true }).waitFor();
		assert(Date.now() < deadline);
		await page.screenshot({ path: out + '/reconnect-' + width + '.png' });
		await page.getByRole('button', { name: 'Undo', exact: true }).hover();
		await wait(Math.max(0, deadline - Date.now() + 150));
		assert.equal(await page.getByRole('button', { name: 'Undo', exact: true }).count(), 0);
		await until(
			async () => (await receipt(pending.id)).state === 'applied',
			'Acceptance commits after original deadline'
		);
		assert.equal(await state(reconnect), 'accepted');
		const committed = await notes(reconnect);
		assert.equal(committed.length, 1);
		assert.deepEqual(committed[0].mentioned_actor_ids, ['exec']);
		record({
			width,
			scenario: 'Reconnect and hover preserve deadline; one acceptance and note to requester'
		});
		const stale = await fixture('late-change-' + width);
		await open(page, stale);
		await fill(page, stale);
		const failed = await queue(page);
		await json(
			'/documents/' + stale.document,
			{
				expected_version: (await json('/documents/' + stale.document)).document.version,
				title: stale.title + ' updated',
				kind: 'review',
				visibility: 'company'
			},
			'PATCH'
		);
		await until(
			async () => (await receipt(failed.id)).state === 'failed',
			'Changed review refused'
		);
		assert.equal(await state(stale), 'in_review');
		assert.equal((await notes(stale)).length, 0);
		await page.getByRole('button', { name: 'Open request', exact: true }).waitFor();
		await page.mouse.move(5, 5);
		await page.screenshot({ path: out + '/failed-' + width + '.png' });
		await open(page, undo);
		await page.reload({ waitUntil: 'networkidle' });
		await page.getByRole('button', { name: 'Open request', exact: true }).waitFor();
		await until(
			async () =>
				page.evaluate(() => {
					const toast = document.querySelector('.toasts'),
						bar = document.querySelector('[data-inbox-actions]');
					return Boolean(
						toast &&
						bar &&
						Math.abs(toast.getBoundingClientRect().bottom - bar.getBoundingClientRect().top + 8) < 2
					);
				}),
			'Cold-load notice follows the final decision bar position'
		);
		await page.screenshot({ path: out + '/cold-failure-' + width + '.png' });
		assert.equal((await receipt(failed.id)).seen_at, null);
		await page.getByRole('button', { name: 'Open request', exact: true }).click();
		await until(
			async () => Boolean((await receipt(failed.id)).seen_at),
			'Explicit failure acknowledgement'
		);
		record({
			width,
			scenario: 'Changed review refuses both effects; durable failure survives reload'
		});
		const collaboration = await fixture('collaboration-' + width);
		collaboration.request = randomUUID();
		collaboration.item = 'document:collaboration:' + collaboration.request;
		await db.query(
			"INSERT INTO native_document_collaboration_requests(id,document_id,requested_by_actor_id,requested_owner_actor_id,summary) VALUES($1,$2,'exec','owner','Finish editing this shared document')",
			[collaboration.request, collaboration.document]
		);
		const openCollaboration = async () => {
			await page.goto(origin + '/' + company + '?item=' + encodeURIComponent(collaboration.item), {
				waitUntil: 'networkidle'
			});
			await until(
				async () => await page.getByRole('button', { name: 'Done', exact: true }).isEnabled(),
				'Saved collaboration ready to complete'
			);
		};
		const queueCollaboration = async () => {
			const response = page.waitForResponse(
				(r) => r.url().endsWith('/pending-decision') && r.request().method() === 'POST'
			);
			await page.getByRole('button', { name: 'Done', exact: true }).click();
			const r = await response;
			assert(r.ok(), await r.text());
			return await r.json();
		};
		const collaborationResolved = async () =>
			(
				await db.query(
					'SELECT resolved_at FROM native_document_collaboration_requests WHERE id=$1',
					[collaboration.request]
				)
			).rows[0].resolved_at;
		await openCollaboration();
		const undone = await queueCollaboration();
		assert.equal(await collaborationResolved(), null);
		await page.getByRole('button', { name: 'Undo', exact: true }).click();
		await until(
			async () => (await receipt(undone.id)).state === 'canceled',
			'Collaboration Undo persisted'
		);
		await wait(5200);
		assert.equal(await collaborationResolved(), null);
		await openCollaboration();
		const completed = await queueCollaboration();
		assert.equal(await collaborationResolved(), null);
		await until(
			async () => (await receipt(completed.id)).state === 'applied',
			'Collaboration completed after deadline'
		);
		assert(await collaborationResolved());
		record({
			width,
			scenario: 'Saved collaboration Mark done can be undone, then completed after a fresh deadline'
		});
		await context.close();
	}
	// Four independently failed receipts with identical wording must all remain reachable.
	const group = [];
	for (let i = 0; i < 4; i++) {
		const f = await fixture('queued-failure-' + i),
			view = await json('/documents/' + f.document);
		const r = await json(
			'/documents/' + f.document + '/reviews/' + f.request + '/pending-decision',
			{
				id: randomUUID(),
				decision: {
					kind: 'accept_review',
					expected_document_version: view.document.version,
					expected_review_version: 1,
					accepted_version_name: f.name,
					feedback: f.note
				}
			}
		);
		group.push({ ...f, receipt: r.id });
		await json(
			'/documents/' + f.document,
			{
				expected_version: (await json('/documents/' + f.document)).document.version,
				title: f.title + ' updated',
				kind: 'review',
				visibility: 'company'
			},
			'PATCH'
		);
	}
	await until(
		async () => (await groupReceipts(group)).every((row) => row.state === 'failed'),
		'All four fail independently'
	);
	const context = await browser.newContext({
			viewport: { width: 390, height: 900 },
			reducedMotion: 'reduce'
		}),
		page = await context.newPage();
	page.on('pageerror', (e) => errors.push(e.message));
	await page.goto(origin + '/' + company, { waitUntil: 'networkidle' });
	await page.getByRole('button', { name: 'Open request', exact: true }).first().waitFor();
	assert.equal(await page.getByRole('button', { name: 'Open request', exact: true }).count(), 3);
	for (let i = 0; i < 4; i++) {
		await page.getByRole('button', { name: 'Dismiss', exact: true }).first().click();
		await until(
			async () =>
				(await groupReceipts(group)).filter((row) => Boolean(row.seen_at)).length === i + 1,
			'Only explicit dismissal acknowledges one result'
		);
	}
	assert.equal(await page.getByRole('button', { name: 'Open request', exact: true }).count(), 0);
	record({
		scenario: 'Four identical failures remain independently visible and acknowledged'
	});
	await context.close();
	assert.deepEqual(errors, []);
	await writeFile(out + '/report.json', JSON.stringify({ report, errors }, null, 2));
	console.log(JSON.stringify(report));
} catch (error) {
	for (const context of browser.contexts())
		for (const page of context.pages()) {
			await page.screenshot({ path: out + '/failure.png' });
			await writeFile(
				out + '/failure.json',
				JSON.stringify({ url: page.url(), body: await page.locator('body').innerText() }, null, 2)
			);
		}
	throw error;
} finally {
	await browser.close();
	await db.end();
}
