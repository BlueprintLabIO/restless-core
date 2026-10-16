/* S90: reported product failures on a populated, isolated company. Model provider is scripted;
 * app reads and UI are real. Catalogue faults test resilience, not upstream availability. */
import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { mkdir, writeFile } from 'node:fs/promises';

const origin = process.env.RESTLESS_SMOKE_ORIGIN;
const company = process.env.RESTLESS_SMOKE_COMPANY;
assert(origin && new URL(origin).hostname === '127.0.0.1' && company?.endsWith('_test'));
const out = process.env.RESTLESS_VERIFY_OUTPUT ?? '/tmp/restless-product-quality';
await mkdir(out, { recursive: true });
const require = createRequire(import.meta.url);
const browser = await chromium.launch();
const results = [];
const requested = process.argv.slice(2);
const available = new Set();
async function journey(name, viewport, run) {
	available.add(name);
	if (requested.length && !requested.includes(name)) return;
	const context = await browser.newContext({ viewport, reducedMotion: 'reduce' });
	context.setDefaultTimeout(15000);
	await context.tracing.start({ screenshots: true, snapshots: true });
	const page = await context.newPage();
	const errors = [];
	page.on('pageerror', (error) => errors.push(error.message));
	try {
		await run(page);
		assert.deepEqual(errors, [], 'No uncaught page errors');
		await page.screenshot({ path: out + '/' + name + '.png' });
		results.push({ name, ok: true });
		console.log('PASS ' + name);
	} catch (error) {
		await page.screenshot({ path: out + '/' + name + '-failed.png' }).catch(() => {});
		await context.tracing.stop({ path: out + '/' + name + '-trace.zip' });
		results.push({ name, ok: false, error: String(error) });
		console.log('FAIL ' + name + ': ' + error);
		console.log('page errors', JSON.stringify(errors));
	} finally {
		await page.unrouteAll({ behavior: 'ignoreErrors' });
		await context.close();
	}
}
async function visible(locator) {
	await locator.waitFor({ state: 'visible' });
}
async function accessibility(page, name) {
	await page.addScriptTag({ path: require.resolve('axe-core/axe.min.js') });
	const report = await page.evaluate(async () =>
		window.axe.run(document.body, {
			runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21aa'] }
		})
	);
	const severe = report.violations
		.filter((item) => ['serious', 'critical'].includes(item.impact))
		.map((item) => ({
			id: item.id,
			impact: item.impact,
			nodes: item.nodes.map((node) => node.target)
		}));
	await writeFile(
		out + '/' + name + '-accessibility.json',
		JSON.stringify(report.violations, null, 2)
	);
	assert.deepEqual(severe, [], 'No serious/critical automated accessibility failures');
}
const apps = origin + '/' + company + '/apps';
try {
	for (const [name, viewport] of [
		['desktop', { width: 1440, height: 900 }],
		['phone', { width: 390, height: 844 }]
	]) {
		await journey('discovery-' + name, viewport, async (page) => {
			let catalogueReads = 0;
			page.on('request', (request) => {
				if (new URL(request.url()).pathname === '/api/app-catalogue') catalogueReads++;
			});
			await page.goto(apps);
			await visible(page.getByRole('heading', { name: 'Featured', exact: true }));
			await visible(page.getByRole('searchbox', { name: 'Search apps', exact: true }));
			assert.equal(
				await page.locator('.sidebar-nav').getByText('Browse', { exact: true }).count(),
				0
			);
			const card = page.getByRole('link', { name: 'Intercom', exact: true });
			await card.click();
			await visible(page.getByRole('heading', { name: 'Intercom', exact: true }));
			await visible(page.locator('.app-overview .description'));
			assert.match(
				await page.locator('.app-overview').innerText(),
				/Customer conversations[\s\S]*Source[\s\S]*Setup[\s\S]*Access/
			);
			assert.equal(await page.getByRole('button', { name: 'Add', exact: true }).count(), 1);
			await page.goBack();
			await visible(page.getByRole('heading', { name: 'Featured', exact: true }));
			assert.equal(catalogueReads, 1, 'Detail and Back reuse the directory cache');
			await page.getByRole('searchbox', { name: 'Search apps', exact: true }).fill('Intercom');
			await visible(page.getByRole('link', { name: 'Intercom', exact: true }));
			await page.getByRole('link', { name: 'Intercom', exact: true }).click();
			await visible(page.locator('.app-overview'));
			await page.locator('a.back').click();
			assert.equal(
				await page.getByRole('searchbox', { name: 'Search apps', exact: true }).inputValue(),
				'Intercom'
			);
			await page.getByRole('searchbox', { name: 'Search apps', exact: true }).fill('');
			await page.getByRole('link', { name: 'Browse all', exact: true }).click();
			await visible(page.locator('.app-card').nth(23));
			const firstPage = await page.locator('.app-card').first().getAttribute('href');
			await page.getByRole('button', { name: 'Next', exact: true }).click();
			await page.waitForURL((url) => url.searchParams.get('page') === '2');
			assert.equal(await page.locator('.app-card').count(), 24);
			assert.notEqual(await page.locator('.app-card').first().getAttribute('href'), firstPage);
			assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
			await accessibility(page, 'discovery-' + name);
			await page.goto(apps + '?view=in-use');
			const installed = page.getByRole('table', { name: 'Installed', exact: true });
			await visible(installed);
			const all = await installed.getByRole('row').count();
			await page.getByRole('searchbox', { name: 'Search apps', exact: true }).fill('Landing page');
			await visible(installed.getByRole('link', { name: 'Landing page design', exact: true }));
			assert(
				(await installed.getByRole('row').count()) < all,
				'Installed search filters the actual owned list'
			);
			await accessibility(page, 'installed-' + name);
			await page.getByRole('searchbox', { name: 'Search apps', exact: true }).fill('');
			await installed.getByRole('link').last().scrollIntoViewIfNeeded();
			await page.getByRole('link', { name: 'Discover', exact: true }).click();
			const search = await page
				.getByRole('searchbox', { name: 'Search apps', exact: true })
				.boundingBox();
			assert(
				search && search.y > 0 && search.y + search.height < viewport.height,
				'A new Apps view starts at its top, rather than inheriting the previous list scroll'
			);
		});
	}
	await journey('picker-short-window', { width: 1280, height: 720 }, async (page) => {
		let changes = 0;
		page.on('request', (request) => {
			if (request.method() !== 'GET' && request.url().includes('/intelligence')) changes++;
		});
		await page.goto(apps);
		const trigger = page.locator('summary[aria-label="Exec’s model"]');
		await visible(trigger);
		await trigger.click();
		const panel = page.locator('.selection-popover-panel:popover-open');
		await visible(panel);
		await visible(panel.getByText('Effort for every agent', { exact: true }));
		const before = await panel.locator('.models button').count();
		await panel.getByRole('button', { name: /^Older models/ }).click();
		await visible(panel.getByRole('button', { name: 'Fewer models', exact: true }));
		assert((await panel.locator('.models button').count()) > before);
		for (const size of [
			{ width: 1280, height: 420 },
			{ width: 320, height: 420 }
		]) {
			await page.setViewportSize(size);
			await panel
				.getByRole('link', { name: 'All intelligence settings', exact: true })
				.scrollIntoViewIfNeeded();
			const box = await panel.boundingBox();
			assert(
				box &&
					box.x >= 7 &&
					box.y >= 7 &&
					box.x + box.width <= size.width - 7 &&
					box.y + box.height <= size.height - 7
			);
			const last = panel.locator('.models button').last();
			await last.scrollIntoViewIfNeeded();
			const lastBox = await last.boundingBox();
			assert(lastBox && lastBox.y >= 0 && lastBox.y + lastBox.height <= size.height);
		}
		assert.equal(changes, 0, 'Disclosure and scrolling make no model or effort changes');
		await page.keyboard.press('Escape');
		await panel.waitFor({ state: 'hidden' });
		assert(await trigger.evaluate((element) => element === document.activeElement));
	});
	await journey('installed-with-directory-held', { width: 1280, height: 720 }, async (page) => {
		let release;
		const held = new Promise((resolve) => {
			release = resolve;
		});
		await page.route('**/api/app-catalogue', async (route) => {
			await held;
			await route.continue();
		});
		try {
			await page.goto(apps + '?view=in-use');
			await visible(page.getByRole('table', { name: 'Installed', exact: true }));
			assert(
				(await page
					.getByRole('table', { name: 'Installed', exact: true })
					.getByRole('row')
					.count()) > 1
			);
		} finally {
			release();
		}
	});
	await journey('directory-failure', { width: 1280, height: 720 }, async (page) => {
		await page.route('**/api/app-catalogue', (route) =>
			route.fulfill({
				status: 503,
				contentType: 'application/json',
				body: JSON.stringify({ error: 'Temporarily unavailable' })
			})
		);
		await page.goto(apps);
		await visible(page.getByRole('heading', { name: 'Featured', exact: true }));
		await visible(
			page.getByText(
				'Online discovery could not be fully refreshed. Showing the available catalogue.',
				{ exact: true }
			)
		);
		assert(await page.getByRole('link', { name: 'Intercom', exact: true }).isVisible());
		assert.equal(
			await page.getByText('Everything here is already added', { exact: true }).count(),
			0
		);
	});
	await journey('focused-setup-dialogs', { width: 390, height: 640 }, async (page) => {
		await page.goto(apps + '?add=link');
		const dialog = page.getByRole('dialog', { name: 'Add from a link', exact: true });
		await visible(dialog);
		const input = dialog.getByRole('textbox', { name: 'Link or command', exact: true });
		await input.fill('https://fixture.invalid/mcp');
		await page.route('**/api/companies/' + company + '/tool-connections', (route) => {
			if (route.request().method() === 'POST')
				return route.fulfill({
					status: 503,
					contentType: 'application/json',
					body: JSON.stringify({ error: 'Fixture service is unavailable' })
				});
			return route.continue();
		});
		await dialog.getByRole('button', { name: 'Add', exact: true }).click();
		await visible(dialog.getByText(/That could not be saved/));
		assert.equal(
			await input.inputValue(),
			'https://fixture.invalid/mcp',
			'A failed add keeps the draft'
		);
		await page.keyboard.press('Escape');
		await dialog.waitFor({ state: 'hidden' });
		await visible(page.getByRole('heading', { name: 'Featured', exact: true }));
		await page.goto(apps + '?add=skill');
		const skill = page.getByRole('dialog', { name: 'Write know-how', exact: true });
		await visible(skill);
		await skill.getByLabel('Name', { exact: true }).fill('A bounded test draft');
		await skill.getByLabel('When to use it', { exact: true }).fill('During interface verification');
		await skill.getByLabel('How to do it', { exact: true }).fill('Keep this draft unsubmitted.');
		await accessibility(page, 'setup-dialog');
		await skill.getByRole('button', { name: 'Cancel', exact: true }).click();
		await visible(page.getByRole('heading', { name: 'Featured', exact: true }));
		assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
	});
	await journey('short-composer-and-fixed-actions', { width: 1280, height: 420 }, async (page) => {
		const response = await page.request.get(
			origin + '/api/companies/' + company + '/attention?include_work_graph=false'
		);
		const { items } = await response.json();
		assert(items.length, 'A real decision is present');
		await page.goto(origin + '/' + company + '?item=' + encodeURIComponent(items[0].id));
		const footer = page.locator('[data-inbox-actions]');
		await visible(footer);
		const bounds = await footer.evaluate((el) => {
			const pane = el.closest('.attention-focus').getBoundingClientRect(),
				rect = el.getBoundingClientRect();
			return {
				bottom: rect.bottom,
				right: rect.right,
				paneBottom: pane.bottom,
				paneRight: pane.right
			};
		});
		assert(
			Math.abs(bounds.bottom - bounds.paneBottom) < 2 &&
				Math.abs(bounds.right - bounds.paneRight) < 2
		);
		const before = await footer.boundingBox();
		await page.locator('.attention-focus').evaluate((el) => {
			for (const node of el.querySelectorAll('*'))
				if (node.scrollHeight > node.clientHeight) node.scrollTop = node.scrollHeight;
		});
		const after = await footer.boundingBox();
		assert.equal(
			before.y + before.height,
			after.y + after.height,
			'The footer does not move when content scrolls'
		);
		await page.goto(origin + '/' + company + '/people?person=exec');
		const composer = page.locator('.exr-composer textarea').first();
		await visible(composer);
		await composer.fill(
			Array.from({ length: 35 }, (_, i) => 'Unsent long draft line ' + i).join('\n')
		);
		await composer.press('End');
		await composer.scrollIntoViewIfNeeded();
		const scroller = page.locator('.exr-msgs').first();
		await scroller.evaluate((el) => {
			el.scrollTop = el.scrollHeight;
		});
		const read = await scroller.evaluate((el) => ({
			gap: el.scrollHeight - el.clientHeight - el.scrollTop,
			height: el.clientHeight
		}));
		assert(read.height > 35 && read.gap <= 2, 'The transcript still has a usable, reachable end');
		const send = page.locator('.exr-composer button[type=submit]').first();
		await visible(send);
		const box = await send.boundingBox();
		assert(
			box && box.y >= 0 && box.y + box.height <= 420,
			'Send is reachable in a short window with a long draft'
		);
		await composer.fill('');
	});
	await journey('session-without-production-work', { width: 1280, height: 720 }, async (page) => {
		await page.route('**/api/companies/' + company + '/cockpit*', async (route) => {
			const response = await route.fetch();
			const data = await response.json();
			for (const person of data.people ?? [])
				if (person.actor_id === 'exec') {
					person.session_running = true;
					person.session_observed_at = new Date().toISOString();
				}
			await route.fulfill({ response, json: data });
		});
		await page.route('**/api/companies/' + company + '/attention*', async (route) => {
			const response = await route.fetch();
			const data = await response.json();
			data.items = [];
			await route.fulfill({ response, json: data });
		});
		await page.goto(origin + '/' + company + '/people?person=exec');
		const row = page.locator('.directory-person.executive');
		await visible(row);
		await visible(row.getByText('Working', { exact: true }));
		assert.equal(
			await row.getByText('Idle', { exact: true }).count(),
			0,
			'A fresh conversation session is not labelled Idle'
		);
	});

	await journey(
		'know-how-review-edit-remove-restore',
		{ width: 1280, height: 720 },
		async (page) => {
			const slug = 'polish-quality-' + Date.now();
			await page.goto(apps + '?add=skill');
			const dialog = page.getByRole('dialog', { name: 'Write know-how', exact: true });
			await dialog.getByLabel('Name', { exact: true }).fill(slug);
			await dialog
				.getByLabel('When to use it', { exact: true })
				.fill('Only for this disposable interface verification.');
			await dialog
				.getByLabel('How to do it', { exact: true })
				.fill('Preserve the owner review before using the instructions.');
			await dialog.getByRole('button', { name: 'Save for review', exact: true }).click();
			await page.waitForURL((url) => url.pathname.endsWith('/s-' + slug));
			const state = async () => {
				const data = await (
					await page.request.get(origin + '/api/companies/' + company + '/skills')
				).json();
				return data.skills.find((row) => row.name === slug);
			};
			assert.equal(
				(await state()).disposition,
				'candidate',
				'Saving does not silently accept new know-how'
			);
			await page.getByRole('button', { name: 'Accept', exact: true }).click();
			await page.getByRole('button', { name: 'Remove', exact: true }).waitFor();
			assert.equal((await state()).disposition, 'accepted');
			await page.getByRole('button', { name: 'Edit', exact: true }).click();
			const text = page.locator('.skill-edit textarea');
			await text.fill((await text.inputValue()) + '\n\nA verified edit in the existing package.');
			await page.locator('.skill-edit').getByRole('button', { name: 'Save', exact: true }).click();
			await text.waitFor({ state: 'hidden' });
			await page.getByRole('button', { name: 'Remove', exact: true }).click();
			await page.getByRole('button', { name: 'Restore', exact: true }).waitFor();
			assert.equal((await state()).disposition, 'retired');
			await page.getByRole('button', { name: 'Restore', exact: true }).click();
			await page.getByRole('button', { name: 'Remove', exact: true }).waitFor();
			assert.equal((await state()).disposition, 'accepted');
			await page.getByRole('button', { name: 'Remove', exact: true }).click();
			await page.getByRole('button', { name: 'Restore', exact: true }).waitFor();
		}
	);

	await journey('receipt-failure-and-recovery', { width: 1280, height: 720 }, async (page) => {
		// Stage a connection read, not a new grant or real external service.
		const name = 'ui-receipt-fixture';
		await page.route('**/api/companies/' + company + '/tool-connections', async (route) => {
			assert.equal(route.request().method(), 'GET');
			await route.fulfill({
				json: {
					connections: [
						{
							name,
							kind: 'remote',
							endpoint: 'https://fixture.invalid/mcp',
							args: [],
							auth_type: 'none',
							status: 'working',
							frozen: false,
							server_name: 'Receipt fixture',
							tools: [],
							grants: [],
							proposed: [],
							changed: []
						}
					]
				}
			});
		});
		let fail = true,
			reads = 0;
		await page.route(
			'**/api/companies/' + company + '/tool-connections/' + name + '/receipts*',
			(route) => {
				reads++;
				return fail
					? route.fulfill({ status: 503, json: { message: 'Receipt source is unavailable.' } })
					: route.fulfill({
							json: {
								reads: [
									{
										id: 'fixture-read',
										actor: 'exec',
										tool: 'fixture_note',
										status: 'complete',
										observed_at: new Date().toISOString()
									}
								],
								effects: []
							}
						});
			}
		);
		await page.goto(apps + '/c-' + name);
		await visible(page.getByText('Activity could not be read', { exact: true }));
		assert.equal(
			await page.getByText('Nothing used it lately.', { exact: true }).count(),
			0,
			'A failed activity source is not empty history'
		);
		fail = false;
		await page
			.getByRole('status')
			.filter({ hasText: 'Activity could not be read' })
			.getByRole('button', { name: 'Try again', exact: true })
			.click();
		await visible(page.getByRole('heading', { name: 'Activity', exact: true }));
		assert(reads >= 2, 'Retry rereads the activity source');
		assert.equal(await page.getByText('Activity could not be read', { exact: true }).count(), 0);
	});
	await journey(
		'archived-only-groups-remain-reachable',
		{ width: 1280, height: 720 },
		async (page) => {
			const id = '11111111-2222-4333-8444-555555555555';
			await page.route('**/api/companies/' + company + '/rooms/archived', (route) =>
				route.fulfill({
					json: {
						rooms: [
							{
								id,
								kind: 'group',
								title: 'Retained project discussion',
								created_by: 'owner',
								canonical_key: null,
								created_at: new Date().toISOString(),
								archived_at: new Date().toISOString()
							}
						],
						owned_room_ids: [id]
					}
				})
			);
			await page.goto(origin + '/' + company + '/people?person=exec');
			const archived = page.locator('details.archived-chats');
			await visible(archived.locator('summary'));
			await archived.locator('summary').click();
			await visible(
				archived.getByRole('link', { name: 'Retained project discussion', exact: true })
			);
			await visible(archived.getByRole('button', { name: 'Restore', exact: true }));
		}
	);

	await journey('added-app-opens-after-shared-read', { width: 1280, height: 720 }, async (page) => {
		let connections = [];
		let writes = 0;
		await page.route('**/api/companies/' + company + '/tool-connections', async (route) => {
			if (route.request().method() === 'POST') {
				writes++;
				const input = route.request().postDataJSON();
				connections = [
					{
						name: input.name,
						kind: 'remote',
						endpoint: input.endpoint,
						source: input.source,
						args: [],
						auth_type: 'none',
						status: 'awaiting_sign_in',
						frozen: false,
						server_name: 'Intercom',
						tools: [],
						grants: [],
						proposed: [],
						changed: []
					}
				];
				return route.fulfill({ json: { connection: connections[0] } });
			}
			return route.fulfill({ json: { connections } });
		});
		await page.route('**/api/companies/' + company + '/tool-connections/*/receipts*', (route) =>
			route.fulfill({ json: { reads: [], effects: [] } })
		);
		await page.goto(apps + '/intercom');
		await page.getByRole('button', { name: 'Add', exact: true }).click();
		await page.waitForURL((url) => url.pathname.endsWith('/c-intercom'));
		await visible(page.getByRole('heading', { name: 'Intercom', exact: true }));
		assert.equal(await page.getByText('This app is not here', { exact: true }).count(), 0);
		assert.equal(writes, 1, 'Adding does not silently retry or grant extra access');
		await visible(page.getByRole('button', { name: 'Sign in', exact: true }));
	});

	await journey(
		'late-company-read-cannot-replace-selection',
		{ width: 1280, height: 720 },
		async (page) => {
			// Two read-only company projections, with the earlier company's app read held back.
			// The second projection uses the real fixture APIs; no second Runtime or grant is created.
			const next = company.replace(/_test$/, '_next_test');
			await page.route('**/api/companies', async (route) => {
				const response = await route.fetch();
				const rows = await response.json();
				const source = rows.find((row) => row.id === company);
				assert(source);
				await route.fulfill({
					response,
					json: [...rows, { ...source, id: next, name: 'Next company' }]
				});
			});
			await page.route('**/api/companies/' + next + '/**', async (route) => {
				// Read markers/presence may be emitted on entry; no staged company write reaches a server.
				if (route.request().method() !== 'GET')
					return route.fulfill({ status: 403, json: { error: 'Read-only UI projection' } });
				if (route.request().resourceType() === 'eventsource') return route.abort();
				const url = route
					.request()
					.url()
					.replace('/' + next + '/', '/' + company + '/');
				const response = await route.fetch({ url });
				const body = (await response.text()).replaceAll(company, next);
				await route.fulfill({ response, body });
			});
			let settled;
			const finished = new Promise((resolve) => {
				settled = resolve;
			});
			let release;
			const held = new Promise((resolve) => {
				release = resolve;
			});
			let requested;
			const started = new Promise((resolve) => {
				requested = resolve;
			});
			await page.route('**/api/companies/' + company + '/tool-connections', async (route) => {
				requested();
				await held;
				await route
					.fulfill({
						json: {
							connections: [
								{
									name: 'earlier-only',
									server_name: 'Earlier company only',
									kind: 'remote',
									endpoint: 'https://fixture.invalid/mcp',
									args: [],
									auth_type: 'none',
									status: 'working',
									frozen: false,
									tools: [],
									grants: [],
									proposed: [],
									changed: []
								}
							]
						}
					})
					.catch(() => {});
				settled();
			});
			try {
				await page.goto(apps + '?view=in-use');
				await started;
				await page.locator('.company-switcher > summary').click();
				await page
					.locator('.company-switcher-menu')
					.getByRole('link', { name: /Next company/ })
					.click();
				await page.waitForURL((url) => url.pathname === `/${next}/apps`);
				await page.waitForResponse((response) =>
					response.url().includes('/' + next + '/tool-connections')
				);
				await page
					.locator(`a[href="/${next}/apps?view=in-use"]:visible`)
					.waitFor({ state: 'visible' });
				await page.locator(`a[href="/${next}/apps?view=in-use"]:visible`).click();
				const table = page.getByRole('table', { name: 'Installed', exact: true });
				await visible(table);
				release();
				await finished;
				await page.evaluate(
					() =>
						new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)))
				);
				assert.equal(
					await page.locator('.sidebar-nav a[href^="/' + company + '/"]').count(),
					0,
					'All wide navigation follows the selected company'
				);
				await page
					.getByRole('searchbox', { name: 'Search apps', exact: true })
					.fill('Earlier company only');
				await visible(page.getByText('No installed apps match', { exact: true }));
				assert.equal(new URL(page.url()).pathname, `/${next}/apps`);
			} finally {
				release();
			}
		}
	);

	await journey('brand-and-disclosures', { width: 1280, height: 720 }, async (page) => {
		await page.goto(origin + '/' + company + '/company/identity');
		await visible(page.getByRole('heading', { name: 'Brand', level: 1, exact: true }));
		assert.equal(await page.locator('.page h1:visible').count(), 1);
		assert.equal(
			await page.getByRole('heading', { name: 'Brand', level: 2, exact: true }).count(),
			0
		);
		await accessibility(page, 'brand');
		await page.emulateMedia({ media: 'print' });
		assert.equal(
			await page.getByRole('heading', { name: 'Brand', level: 1, exact: true }).count(),
			1,
			'Printed Brand has one visible title'
		);
		await page.screenshot({ path: out + '/brand-print.png' });
		await page.emulateMedia({ media: 'screen' });
		await page.goto(origin + '/' + company + '/company/provider');
		await page.locator('summary').filter({ hasText: 'Diagnostics' }).click();
		assert.equal(
			await page
				.locator('summary')
				.filter({ hasText: /^Installed agent software$/ })
				.count(),
			0
		);
		await page.locator('summary').filter({ hasText: 'Custom harnesses' }).click();
		assert.equal(
			await page.getByRole('heading', { name: 'Other harnesses', exact: true }).count(),
			0
		);
		await visible(page.getByRole('button', { name: '+ Add harness', exact: true }));
	});
} finally {
	await writeFile(out + '/results.json', JSON.stringify(results, null, 2));
	await browser.close();
}
assert(results.length, 'At least one product journey must run');
assert.deepEqual(
	requested.filter((name) => !available.has(name)),
	[],
	'Unknown journey requested'
);
if (results.some((result) => !result.ok)) process.exitCode = 1;
