/** Real isolated owner journeys: file/editor/settings previews keep Inbox context and real
 * review decisions live in its fixed middle-pane footer. */
import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { chromium } from 'playwright';
const origin = process.env.RESTLESS_SMOKE_ORIGIN,
	company = process.env.RESTLESS_SMOKE_COMPANY;
assert(origin && company?.endsWith('_test') && new URL(origin).hostname === '127.0.0.1');
const out = process.env.RESTLESS_VERIFY_OUTPUT ?? '/tmp/restless-inbox-previews';
await mkdir(out, { recursive: true });
const requested = process.argv.slice(2),
	results = [];
const browser = await chromium.launch();
const api = async (path) => {
	const response = await fetch(origin + '/api/companies/' + company + path);
	assert(response.ok, path + ' must answer from the real daemon');
	return response.json();
};
async function item(name) {
	const { items } = await api('/attention');
	const row = items.find((row) => row.title === 'Inbox outcome ' + name + ' verification');
	assert(row, 'Seed missing outcome fixture ' + name);
	return row;
}
const href = (row) => origin + '/' + company + '?item=' + encodeURIComponent(row.id);
const pane = (page) => page.locator('.attention-focus');
const preview = (page) => page.getByLabel('Inbox preview', { exact: true });
async function inInbox(page, id) {
	const url = new URL(page.url());
	assert.equal(url.pathname, '/' + company);
	if (id) assert.equal(url.searchParams.get('item'), id);
	assert.equal(
		await page.getByRole('link', { name: /Inbox, .*items/ }).getAttribute('aria-current'),
		'page'
	);
}
async function footerAnchored(page) {
	const size = await page.locator('[data-inbox-actions]').evaluate((el) => {
		const rect = el.getBoundingClientRect(),
			pane = el.closest('.attention-focus').getBoundingClientRect();
		return {
			bottom: rect.bottom,
			right: rect.right,
			paneBottom: pane.bottom,
			paneRight: pane.right
		};
	});
	assert(
		Math.abs(size.bottom - size.paneBottom) < 2 && Math.abs(size.right - size.paneRight) < 2,
		'Decisions stay at the middle pane bottom-right'
	);
}
async function journey(name, viewport, run) {
	if (requested.length && !requested.includes(name)) return;
	const context = await browser.newContext({ viewport, reducedMotion: 'reduce' });
	context.setDefaultTimeout(15000);
	context.setDefaultNavigationTimeout(30000);
	await context.addInitScript(() => {
		window.__restlessStrict = true;
	});
	await context.tracing.start({ screenshots: true, snapshots: true });
	const page = await context.newPage(),
		errors = [];
	page.on('pageerror', (error) => errors.push(error.message));
	try {
		await run(page);
		assert.deepEqual(errors, [], 'No page errors');
		await page.screenshot({ path: out + '/' + name + '.png' });
		results.push({ name, ok: true });
		console.log(name + ' passed');
	} catch (error) {
		results.push({ name, ok: false, error: String(error) });
		await page.screenshot({ path: out + '/' + name + '-failed.png' }).catch(() => {});
		console.log('Failed URL:', page.url(), 'Page errors:', JSON.stringify(errors));
		await context.tracing.stop({ path: out + '/' + name + '-trace.zip' });
		console.log(
			'read states',
			JSON.stringify(
				await page.evaluate(() =>
					window.__restlessQueryClient
						?.getQueryCache()
						.findAll()
						.filter((q) => ['attention', 'work-graph', 'company-principal'].includes(q.queryKey[0]))
						.map((q) => ({
							key: q.queryKey,
							status: q.state.status,
							fetch: q.state.fetchStatus,
							health: q.state.data?.sourceHealth,
							hasData: !!q.state.data
						}))
				)
			)
		);
		throw error;
	} finally {
		await context.close();
	}
}
try {
	for (const width of [1440, 390]) {
		const viewport = { width, height: width === 390 ? 844 : 900 };
		await journey('outcome-accept-' + width, viewport, async (page) => {
			const row = await item('accept-' + width);
			await page.goto(href(row));
			await pane(page).getByRole('heading', { name: 'Prepared outcome', exact: true }).waitFor();
			assert.equal(
				await page.getByRole('button', { name: 'Accept outcome', exact: true }).count(),
				1,
				'One actual Accept control'
			);
			assert.equal(
				await page.getByRole('button', { name: 'Hold to accept', exact: true }).count(),
				0,
				'No duplicate decision in the chat rail'
			);
			await footerAnchored(page);
			const before = await page.locator('[data-inbox-actions]').boundingBox();
			await page.locator('.outcome-stage').evaluate((el) => (el.scrollTop = el.scrollHeight));
			const after = await page.locator('[data-inbox-actions]').boundingBox();
			assert.equal(before.y + before.height, after.y + after.height);
			const [decision] = await Promise.all([
				page.waitForResponse(
					(response) =>
						response.request().method() === 'POST' &&
						response.url().endsWith('/handoffs/' + row.source.reference + '/review')
				),
				pane(page).getByRole('button', { name: 'Accept outcome', exact: true }).click()
			]);
			assert(decision.ok(), 'The real review endpoint accepts the outcome');
			await page.waitForFunction(
				async ({ origin, company, id }) => {
					const data = await (
						await fetch(origin + '/api/companies/' + company + '/attention')
					).json();
					return data.work_graph.work.find((work) => work.id === id)?.status === 'completed';
				},
				{ origin, company, id: row.work_id }
			);
			assert(!(await api('/attention')).items.some((item) => item.id === row.id));
		});
		await journey('outcome-changes-' + width, viewport, async (page) => {
			const row = await item('changes-' + width);
			await page.goto(href(row));
			await pane(page).getByRole('button', { name: 'Request changes', exact: true }).click();
			const note = pane(page).getByRole('textbox', { name: /Changes for/ });
			await note.waitFor();
			assert(
				await note.evaluate((el) => el === document.activeElement),
				'Request changes focuses its note'
			);
			assert(
				!(await pane(page).getByRole('button', { name: 'Send changes', exact: true }).isEnabled()),
				'Blank revision feedback cannot be submitted'
			);
			await note.fill(
				'Keep the reviewed candidate and make the requested ' + width + 'px revision clearer.'
			);
			await footerAnchored(page);
			const [decision] = await Promise.all([
				page.waitForResponse(
					(response) =>
						response.request().method() === 'POST' &&
						response.url().endsWith('/handoffs/' + row.source.reference + '/review')
				),
				pane(page).getByRole('button', { name: 'Send changes', exact: true }).click()
			]);
			assert(decision.ok(), 'The actual changes endpoint accepts the feedback');
			assert(!(await api('/attention')).items.some((item) => item.id === row.id));
		});
		await journey('files-editors-and-settings-' + width, viewport, async (page) => {
			const row = await item('site');
			await page.goto(href(row));
			const counter = page
				.frameLocator('iframe[title="Prepared interactive outcome"]')
				.getByRole('button', { name: '0', exact: true });
			await counter.click();
			const one = page
				.frameLocator('iframe[title="Prepared interactive outcome"]')
				.getByRole('button', { name: '1', exact: true });
			await one.waitFor();
			await pane(page).locator('summary').filter({ hasText: 'Context and files' }).click();
			await pane(page).getByRole('link', { name: 'Inbox preview shape.svg', exact: true }).click();
			await preview(page).getByRole('img').waitFor();
			assert(
				await preview(page)
					.getByRole('img')
					.evaluate((el) => el.complete && el.naturalWidth > 0),
				'The actual Company image loaded'
			);
			await inInbox(page, row.id);
			await footerAnchored(page);
			await pane(page)
				.locator('[data-inbox-actions]')
				.locator('summary[aria-label="More"]')
				.click();
			assert(
				await preview(page).getByRole('img').isVisible(),
				'Opening the footer menu keeps the supporting file in place'
			);
			await page.keyboard.press('Escape');
			await inInbox(page, row.id);
			assert(
				await preview(page).getByRole('img').isVisible(),
				'Closing the menu keeps the preview in place'
			);
			await preview(page).getByRole('link', { name: 'Back to Inbox item', exact: true }).click();
			await one.waitFor();
			await pane(page).getByRole('link', { name: 'Open a kept file', exact: true }).click();
			await preview(page).getByRole('img').waitFor();
			assert(
				await preview(page)
					.getByRole('img')
					.evaluate((el) => el.complete && el.naturalWidth === 1),
				'The retained upload is read through its actual scoped attachment'
			);
			await inInbox(page, row.id);
			await footerAnchored(page);
			await preview(page).getByRole('link', { name: 'Back to Inbox item', exact: true }).click();
			await one.waitFor();
			await pane(page).getByRole('link', { name: 'Open company Limits', exact: true }).click();
			await preview(page).getByRole('button', { name: 'Edit limit', exact: true }).click();
			const limit = preview(page).getByRole('textbox', { name: 'Limit', exact: true });
			await limit.fill(String(Number(await limit.inputValue()) + 1));
			const [saved] = await Promise.all([
				page.waitForResponse(
					(response) =>
						response.request().method() === 'POST' &&
						response.url().endsWith('/company/spend-limit')
				),
				preview(page).getByRole('button', { name: /^(Raise|Lower) to / }).click()
			]);
			assert(saved.ok(), 'Budget is saved through the real Company form');
			await inInbox(page, row.id);
			await footerAnchored(page);
			await preview(page).getByRole('link', { name: '← Back to Inbox item', exact: true }).click();
			await one.waitFor();
			await pane(page).getByRole('link', { name: 'Open the shared document', exact: true }).click();
			await preview(page).getByRole('heading', { name: 'Discussion', exact: true }).waitFor();
			const backToBody = preview(page).getByRole('button', {
				name: 'Back to the document',
				exact: true
			});
			if (await backToBody.isVisible()) await backToBody.click();
			const body = preview(page).getByRole('textbox', {
				name: 'Collaborative document body',
				exact: true
			});
			await body.waitFor();
			const edit = 'Inbox in-place edit ' + Date.now() + ' verified at ' + width + ' pixels.';
			await body.press('Control+End');
			await body.press('Enter');
			await body.pressSequentially(edit);
			const document = new URL(
				new URL(page.url()).searchParams.get('preview'),
				origin
			).searchParams.get('document');
			await page.waitForFunction(
				async ({ origin, company, document, edit }) => {
					const view = await (
						await fetch(origin + '/api/companies/' + company + '/documents/' + document)
					).json();
					return view.current_version?.markdown?.includes(edit);
				},
				{ origin, company, document, edit }
			);
			await inInbox(page, row.id);
			await preview(page).getByRole('link', { name: '← Back to Inbox item', exact: true }).click();
			await one.waitFor();
			await pane(page).getByRole('link', { name: 'Open the shared sheet', exact: true }).click();
			await preview(page)
				.getByRole('heading', { name: 'Inbox preview sheet', exact: true })
				.waitFor();
			await inInbox(page, row.id);
			await footerAnchored(page);
			await page.goBack();
			await one.waitFor();
			await page.goForward();
			await preview(page)
				.getByRole('heading', { name: 'Inbox preview sheet', exact: true })
				.waitFor();
		});

		await journey('native-review-preview-and-recovery-' + width, viewport, async (page) => {
			const rows = (await api('/attention')).items;
			const review = rows.find((row) => row.native_document?.kind === 'review');
			assert(review, 'Seed a native document review');
			await page.goto(
				href(review) + '&preview=' + encodeURIComponent('/' + company + '/company/identity')
			);
			await preview(page).getByRole('heading', { name: 'Brand', exact: true, level: 2 }).waitFor();
			assert.equal(
				await preview(page).getByRole('heading', { name: 'Brand & voice', exact: true }).count(),
				0
			);
			await page.goto(
				href(review) + '&preview=' + encodeURIComponent('/' + company + '/company/resources')
			);
			await preview(page).getByRole('heading', { name: 'Limits', exact: true }).waitFor();
			await pane(page).getByRole('button', { name: 'Request changes', exact: true }).click();
			const note = pane(page).getByRole('textbox', { name: 'Note', exact: true });
			await note.waitFor();
			assert(
				await note.evaluate((el) => el === document.activeElement),
				'A native review preview returns to and focuses its actual feedback form'
			);
			await footerAnchored(page);
			const charter = rows.find((row) => row.charter_proposal);
			assert(charter, 'Seed a real charter proposal');
			await page.goto(
				href(charter) + '&preview=' + encodeURIComponent('/' + company + '/company/resources')
			);
			await preview(page).getByRole('heading', { name: 'Limits', exact: true }).waitFor();
			await pane(page)
				.locator('[data-inbox-actions]')
				.getByRole('button', { name: 'Edit', exact: true })
				.click();
			await pane(page).getByRole('textbox', { name: 'Company charter', exact: true }).waitFor();
			await footerAnchored(page);
			const row = await item('site');
			await page.goto(
				href(row) +
					'&preview=' +
					encodeURIComponent('/' + company + '/library?file=missing-inbox-fixture')
			);
			await preview(page)
				.getByText('This file is unavailable, or your access has changed.', { exact: true })
				.waitFor();
			await footerAnchored(page);
			await preview(page).getByRole('link', { name: 'Back to Inbox item', exact: true }).click();
			await page
				.frameLocator('iframe[title="Prepared interactive outcome"]')
				.getByRole('button', { name: '0', exact: true })
				.waitFor();
			await inInbox(page, row.id);
			const docs = await api('/documents?limit=100');
			const doc = docs.items.find(
				(row) => row.document.title === 'Inbox footer collaboration test'
			).document;
			await page.goto(origin + '/' + company + '/library/documents?document=' + doc.id);
			await page
				.getByRole('textbox', { name: 'Collaborative document body', exact: true })
				.waitFor();
			assert.equal(
				await page.getByRole('link', { name: 'Library', exact: true }).getAttribute('aria-current'),
				'page'
			);
			const sheets = await api('/sheets'),
				sheet = sheets.find((row) => row.title === 'Inbox preview sheet');
			await page.goto(origin + '/' + company + '/library/sheets?sheet=' + sheet.id);
			await page.getByRole('heading', { name: 'Inbox preview sheet', exact: true }).waitFor();
			assert.equal(
				await page.getByRole('link', { name: 'Library', exact: true }).getAttribute('aria-current'),
				'page'
			);
		});
		await journey('preview-conversation-context-' + width, viewport, async (page) => {
			const row = await item('site');
			await page.goto(
				href(row) + '&preview=' + encodeURIComponent('/' + company + '/company/resources#spend')
			);
			await preview(page).getByRole('heading', { name: 'Limits', exact: true }).waitFor();
			const toggle = page.getByRole('button', { name: 'Exec', exact: true });
			await pane(page).locator('summary[aria-label="More"]').click();
			await pane(page).getByRole('button', { name: 'Ask Exec about this', exact: true }).click();
			assert(
				new URL(page.url()).searchParams.has('preview'),
				'Asking Exec keeps the actual settings surface in the middle pane'
			);
			const rail = page.getByRole('complementary', { name: 'Exec conversation', exact: true });
			const draft = rail.getByRole('textbox', { name: 'Ask Exec', exact: true });
			await draft.waitFor();
			assert((await draft.inputValue()).includes(encodeURIComponent(row.id)), 'The draft links the item into the one Exec conversation');
			await draft.fill(
				'Verify this prepared Limits page as part of the isolated Inbox preview check.'
			);
			const [sent] = await Promise.all([
				page.waitForResponse(
					(response) =>
						response.request().method() === 'POST' &&
						/\/actors\/exec\/conversation$/.test(response.url())
				),
				draft.press('Enter')
			]);
			assert(
				sent.ok(),
				'The actual general Exec message is accepted: ' +
					sent.status() +
					' ' +
					(sent.ok() ? '' : await sent.text())
			);
			const request = sent.request();
			const form = await new Response(request.postData(), {
				headers: { 'content-type': await request.headerValue('content-type') }
			}).formData();
			assert.equal(
				form.get('context_path'),
				'/' + company + '/company/resources',
				'Exec gets the actual settings page rather than the opaque preview wrapper'
			);
			assert.equal(form.get('attention_id'), null, 'The item does not open a scoped conversation');
			assert.equal(form.get('work_id'), null, 'The one Exec conversation is not scoped to this Work');
			assert((await sent.json()).context_attached, 'The server accepted the linked native context');
			await toggle.click();
			// Request an ordinary cross-window refresh instead of depending on a background poll.
			const refreshed = page.waitForResponse(
				(response) =>
					response.request().method() === 'GET' &&
					new URL(response.url()).pathname.endsWith('/attention'),
				{ timeout: 20000 }
			);
			await page.evaluate(company => {
				const channel = new BroadcastChannel('restless-attention');
				channel.postMessage(company);
				channel.close();
			}, company);
			await refreshed;
			assert(
				!(await rail.isVisible()),
				'Closing chat stays closed through the next actual attention refresh'
			);
			await inInbox(page, row.id);
			await footerAnchored(page);
			await pane(page).locator('summary[aria-label="More"]').click();
			await pane(page).getByRole('button', { name: 'Ask Exec about this', exact: true }).click();
			await rail.waitFor();
		});
		await journey('update-has-its-own-selection-' + width, viewport, async (page) => {
			const mention = (await api('/for-you')).items.find((row) =>
				row.excerpt.includes('Inbox preview mention verification')
			);
			assert(mention, 'Seed a real actor-authored mention');
			await page.goto(origin + '/' + company);
			await page
				.getByRole('link', { name: /mentioned you/ })
				.first()
				.click();
			await preview(page).getByRole('heading', { name: 'Discussion', exact: true }).waitFor();
			const url = new URL(page.url());
			assert.equal(url.pathname, '/' + company);
			assert.equal(url.searchParams.get('update'), mention.key);
			assert(!url.searchParams.has('item'), 'An update must not borrow another attention item');
			assert.equal(
				await page.getByRole('button', { name: 'Accept outcome', exact: true }).count(),
				0
			);
			await footerAnchored(page);
		});
	}
} finally {
	await writeFile(out + '/report.json', JSON.stringify(results, null, 2));
	await browser.close();
}
