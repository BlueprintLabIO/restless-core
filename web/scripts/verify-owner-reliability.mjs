/* Real _test company journeys for S89. Faults delay/fail reads; all mutations still use the
 * real daemon. These checks never touch an owner profile or an external model provider. */
import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';

const origin = process.env.RESTLESS_SMOKE_ORIGIN,
	company = process.env.RESTLESS_SMOKE_COMPANY;
assert(origin && company?.endsWith('_test'), 'Set a localhost smoke origin and a _test company');
assert.equal(new URL(origin).hostname, '127.0.0.1');
const out = process.env.RESTLESS_VERIFY_OUTPUT ?? '/tmp/restless-owner-reliability';
await mkdir(out, { recursive: true });
const browser = await chromium.launch();
const results = [];
const requested = process.argv.slice(2);
const api = async (path) => {
	const response = await fetch(origin + '/api/companies/' + company + path);
	assert(response.ok, 'Real API ' + path + ' must answer');
	return response.json();
};
async function journey(name, run, viewport = { width: 1440, height: 900 }) {
	if (requested.length && !requested.includes(name)) return;
	const context = await browser.newContext({ viewport, reducedMotion: 'reduce' });
	context.setDefaultTimeout(15000);
	context.setDefaultNavigationTimeout(30000);
	const page = await context.newPage();
	const errors = [];
	page.on('pageerror', (error) => errors.push(error.message));
	try {
		await run(page);
		assert.deepEqual(errors, [], 'No page errors');
		await page.screenshot({ path: out + '/' + name + '.png' });
		results.push({ name, ok: true });
		console.log(name + ' passed');
	} catch (error) {
		await page.screenshot({ path: out + '/' + name + '-failed.png' }).catch(() => {});
		results.push({ name, ok: false, error: String(error) });
		throw error;
	} finally {
		await context.close();
	}
}
try {
	await journey('people-stable-and-monitored', async (page) => {
		await page.goto(origin + '/' + company + '/people?person=exec');
		// Section labels may be represented as regions or generic sections by browser engines.
		await page.locator('.work-panel .monitoring').waitFor();
		assert((await page.locator('.work-panel .monitoring').innerText()).includes('Waiting'));
		assert.equal(
			await page.getByRole('link', { name: 'All company work', exact: true }).getAttribute('href'),
			'/' + company + '/work'
		);
		const row = page.locator('.directory-person[href*="person=exec"]');
		const word = row.locator('.status-word');
		await word.waitFor();
		const before = await word.boundingBox();
		await row.hover();
		const after = await word.boundingBox();
		assert.deepEqual(before, after, 'Hover must not move the status');
		assert.equal(await word.innerText(), 'Needs you');
		const radio = page.getByRole('radio', { name: 'Everyone', exact: true });
		await radio.press('ArrowRight');
		await page.waitForFunction(
			() =>
				document.activeElement?.getAttribute('aria-checked') === 'true' &&
				document.activeElement?.textContent?.includes('Working')
		);
		assert((await page.url()).includes('show=working'));
	});
	await journey('team-dialog-save-and-rename', async (page) => {
		await page.goto(origin + '/' + company + '/people?person=exec');
		let cockpit = await api('/cockpit');
		if (
			!cockpit.teams.some((team) => ['Owner QA team', 'Owner QA team renamed'].includes(team.name))
		) {
			await page.getByRole('button', { name: 'New team…', exact: true }).click();
			const dialog = page.getByRole('dialog', { name: 'New team', exact: true });
			await dialog.getByRole('textbox', { name: 'Name', exact: true }).fill('Owner QA team');
			await dialog
				.getByRole('textbox', { name: 'Accountable for', exact: true })
				.fill('A focused, reversible team form verification.');
			await dialog.getByRole('combobox', { name: 'Lead', exact: true }).selectOption('copy-editor');
			await dialog.getByRole('button', { name: 'Form team', exact: true }).click();
			await dialog.waitFor({ state: 'hidden' });
			cockpit = await api('/cockpit');
			assert(cockpit.teams.some((team) => team.name === 'Owner QA team'));
		}
		const teamName = cockpit.teams.find((team) =>
			['Owner QA team', 'Owner QA team renamed'].includes(team.name)
		).name;
		await page.locator('summary[aria-label="Change ' + teamName + '"]').click();
		await page.getByRole('button', { name: 'Rename…', exact: true }).click();
		const edit = page.getByRole('dialog', { name: 'Edit team', exact: true });
		const newName = teamName === 'Owner QA team' ? 'Owner QA team renamed' : 'Owner QA team';
		await edit.getByRole('textbox', { name: 'Name', exact: true }).fill(newName);
		await edit.getByRole('button', { name: 'Save', exact: true }).click();
		await edit.waitFor({ state: 'hidden' });
		cockpit = await api('/cockpit');
		assert(cockpit.teams.some((team) => team.name === newName));
	});
	await journey('staff-direct-message', async (page) => {
		await page.goto(origin + '/' + company + '/people?person=studio-design');
		const textbox = page.locator('.conversation-main textarea');
		await textbox.fill('Direct staff message verification: please explain your current draft.');
		const sent = page.waitForResponse(
			(response) =>
				response.request().method() === 'POST' &&
				response.url().includes('/actors/studio-design/conversation')
		);
		await page.locator('.conversation-main .hc-send').click();
		const response = await sent;
		assert(response.ok(), 'Non-lead staff must accept the direct message');
		const conversation = await api('/actors/studio-design/conversation');
		assert(JSON.stringify(conversation).includes('Direct staff message verification'));
		assert(
			!(await page.locator('.conversation-main').innerText()).includes('not a directly addressable')
		);
	});
	await journey('work-toolbar-goal-archive-undo', async (page) => {
		const before = await api('/cockpit');
		const goal = before.goals.find((goal) =>
			['Completed goal for owner reliability test', 'Edited goal reliability fixture'].includes(
				goal.title
			)
		);
		assert(goal && !goal.closed_at, 'The completed-only goal fixture must be open');
		await page.goto(origin + '/' + company + '/work?goal=' + goal.id + '&view=board');
		const head = page.locator('.head');
		await head.getByRole('button', { name: 'Clear done', exact: true }).click();
		const confirm = page.getByRole('dialog');
		await confirm.getByRole('button', { name: 'Clear and archive', exact: true }).click();
		await page.waitForFunction(
			async ({ origin, company, id }) => {
				const data = await (await fetch(origin + '/api/companies/' + company + '/cockpit')).json();
				return !!data.goals.find((goal) => goal.id === id)?.closed_at;
			},
			{ origin, company, id: goal.id }
		);
		await page.getByRole('button', { name: 'Undo', exact: true }).click();
		await page.waitForFunction(
			async ({ origin, company, id }) => {
				const data = await (await fetch(origin + '/api/companies/' + company + '/cockpit')).json();
				return !data.goals.find((goal) => goal.id === id)?.closed_at;
			},
			{ origin, company, id: goal.id }
		);
		await head.locator('summary[aria-label="Goal actions"]').click();
		await page.getByRole('button', { name: 'Edit goal…', exact: true }).click();
		const edit = page.getByRole('dialog', { name: 'Edit goal', exact: true });
		const newTitle =
			goal.title === 'Completed goal for owner reliability test'
				? 'Edited goal reliability fixture'
				: 'Completed goal for owner reliability test';
		await edit.getByRole('textbox', { name: 'Goal', exact: true }).fill(newTitle);
		await edit.getByRole('button', { name: 'Save', exact: true }).click();
		await edit.waitFor({ state: 'hidden' });
		assert(
			(await api('/cockpit')).goals.some((row) => row.id === goal.id && row.title === newTitle)
		);
	});
	await journey(
		'phone-layout-and-retired-ai-apps',
		async (page) => {
			await page.goto(origin + '/' + company + '/work');
			await page.getByRole('combobox', { name: 'Work view', exact: true }).waitFor();
			assert.equal(
				await page.evaluate(() => document.documentElement.scrollWidth > innerWidth),
				false
			);
			await page.goto(origin + '/account/ai-apps');
			await page.waitForURL('**/account/connections');
			assert.equal(await page.getByRole('link', { name: 'AI apps', exact: true }).count(), 0);
			assert.equal(
				await page.evaluate(() => document.documentElement.scrollWidth > innerWidth),
				false
			);
		},
		{ width: 390, height: 844 }
	);
	await journey('slow-entry-keeps-loading-wave', async (page) => {
		await page.route('**/api/companies/' + company + '/attention', async (route) => {
			await new Promise((resolve) => setTimeout(resolve, 3000));
			await route.continue().catch(() => {});
		});
		await page.goto(origin + '/' + company, { waitUntil: 'domcontentloaded' });
		await page.waitForTimeout(1600);
		assert(
			await page.locator('.first-paint-pending').count(),
			'Slow critical read must hold the entry barrier beyond 1.2 seconds'
		);
		assert(
			await page
				.locator('body > .boot')
				.evaluate((element) => getComputedStyle(element).display !== 'none'),
			'The loading wave stays visible'
		);
		await page.locator('.first-paint-pending').waitFor({ state: 'detached', timeout: 15000 });
		await page.locator('.attention-index').waitFor();
	});
	await journey(
		'health-diagnostic-wraps-and-sends',
		async (page) => {
			await page.route('**/api/companies/' + company + '/company*', async (route) => {
				const response = await route.fetch();
				const body = await response.json();
				body.computer.doctor.status = 'degraded';
				body.computer.doctor.actions = [];
				body.computer.doctor.checks.push({
					id: 'qa-diagnostic',
					label: 'Disk memory and processor',
					source: 'runtime',
					status: 'degraded',
					summary:
						'qa-diagnostic-geometry: Company files have 2.8 GB free. ' +
						'Read all of this diagnostic without truncation. '.repeat(20),
					detail: 'A controlled degraded read for the diagnostic action verification.'
				});
				await route.fulfill({ response, json: body });
			});
			await page.goto(origin + '/' + company + '/company/doctor');
			const diagnostic = page.locator('p.detail').filter({ hasText: 'qa-diagnostic-geometry' });
			await diagnostic.waitFor();
			assert(
				await diagnostic.evaluate((element) => element.scrollWidth <= element.clientWidth),
				'The full diagnostic wraps'
			);
			const sent = page.waitForResponse(
				(response) =>
					response.request().method() === 'POST' &&
					response.url().includes('/actors/exec/conversation')
			);
			void sent.catch(() => {});
			await page.screenshot({ path: out + '/health-before-click.png' });
			await page
				.getByRole('button', { name: 'Ask Exec to fix', exact: true })
				.click({ timeout: 8000 });
			assert((await sent).ok(), 'Diagnostic must be sent to the real daemon in one click');
			assert(
				JSON.stringify(await api('/actors/exec/conversation')).includes('qa-diagnostic-geometry')
			);
		},
		{ width: 390, height: 844 }
	);
	for (const actor of ['exec', 'studio-design'])
		await journey(
			actor === 'exec' ? 'stop-and-redirect-stay-anchored' : 'staff-stop-and-redirect',
			async (page) => {
				await page.goto(origin + '/' + company + '/people?person=' + actor);
				const input = page.locator('.conversation-main textarea');
				async function send(body) {
					await input.fill(body);
					const [response] = await Promise.all([
						page.waitForResponse(
							(response) =>
								response.request().method() === 'POST' &&
								response.url().includes('/actors/' + actor + '/conversation') &&
								!response.url().endsWith('/interrupt')
						),
						page.locator('.conversation-main .hc-send').click()
					]);
					assert(response.ok(), 'Message accepted by the real daemon');
					return response.json();
				}
				await send('long reply');
				await page.locator('.visible-reply[aria-busy="true"]').waitFor({ timeout: 30000 });
				const stop = page.getByRole('button', { name: 'Stop reply', exact: true });
				await stop.waitFor();
				assert.equal(
					await page.locator('.conversation-turn-dock').count(),
					1,
					'Only one run indicator'
				);
				assert.equal(
					await page.getByRole('button', { name: 'Interrupt', exact: true }).count(),
					0,
					'No duplicate interrupt control'
				);
				const [stopped] = await Promise.all([
					page.waitForResponse(
						(response) =>
							response.request().method() === 'POST' && response.url().endsWith('/interrupt')
					),
					stop.click()
				]);
				assert(stopped.ok(), 'Stop endpoint succeeded');
				const result = await stopped.json();
				assert(
					result.interrupted || result.cancelled,
					'The active reply was interrupted or cancelled'
				);
				await page
					.locator('.conversation-turn-dock[data-phase="failed"]')
					.waitFor({ timeout: 15000 });
				assert((await page.locator('.conversation-turn-dock').innerText()).includes('Stopped'));
				await send('long reply');
				await page.locator('.visible-reply[aria-busy="true"]').waitFor({ timeout: 30000 });
				await stop.waitFor();
				const before = await stop.boundingBox();
				await input.fill('Acknowledge this new direction and leave the earlier reply stopped.');
				const redirect = page.getByRole('button', {
					name: 'Send now and redirect the current reply',
					exact: true
				});
				const after = await redirect.boundingBox();
				assert(
					Math.abs(before.x + before.width - after.x - after.width) < 1 &&
						Math.abs(before.y + before.height - after.y - after.height) < 1,
					'Stop and redirect occupy the same anchored position'
				);
				const [redirected] = await Promise.all([
					page.waitForResponse(
						(response) =>
							response.request().method() === 'POST' &&
							response.url().includes('/actors/' + actor + '/conversation') &&
							!response.url().endsWith('/interrupt')
					),
					redirect.click()
				]);
				assert(redirected.ok());
				const receipt = await redirected.json();
				assert(receipt.interrupted, 'Sending new direction interrupts the earlier reply');
				await page
					.locator('[id="turn-' + receipt.message_id + '"]')
					.waitFor({ state: 'attached', timeout: 15000 });
			}
		);
	await journey('stale-brand-draft-has-recovery', async (page) => {
		const before = await api('/company/identity');
		const proposal = before.pending_proposals.find(
			(proposal) => proposal.rationale === 'Stale brand proposal reliability test'
		);
		assert(
			proposal && proposal.expected_predecessor !== before.current_release?.id,
			'Seed a real stale proposal'
		);
		await page.goto(origin + '/' + company + '/company/identity');
		const head = page.getByRole('button', { name: /Stale brand proposal reliability test/ });
		if ((await head.getAttribute('aria-pressed')) !== 'true') await head.click();
		await page.getByText('Brand and voice changed since this draft.', { exact: false }).waitFor();
		const action = page.getByRole('button', { name: 'Ask Exec to update', exact: true });
		assert(await action.isEnabled());
		const [sent] = await Promise.all([
			page.waitForResponse(
				(response) =>
					response.request().method() === 'POST' &&
					response.url().endsWith('/actors/exec/conversation')
			),
			action.click()
		]);
		assert(sent.ok(), 'Recovery reaches the real Exec conversation');
		const after = await api('/company/identity');
		assert.equal(
			after.current_release.id,
			before.current_release.id,
			'Recovery preserves the newer owner wording'
		);
		assert(
			JSON.stringify(await api('/actors/exec/conversation')).includes(proposal.id),
			'Exec gets the actual stale proposal id'
		);
		await page
			.getByText('Asked Exec for a draft based on the latest brand and voice.', { exact: true })
			.waitFor();
	});

	await journey('stored-provider-failure-is-readable', async (page) => {
		await page.goto(origin + '/' + company + '/people?person=exec');
		await page
			.getByText('The model provider is busy right now. Try again in a minute.', { exact: true })
			.first()
			.waitFor();
		const text = await page.locator('.message-text').allTextContents();
		assert(
			!text.some((value) => /^\s*529\s*\{/.test(value)),
			'Stored provider JSON is folded into error details'
		);
	});

	for (const viewport of [
		{ width: 1440, height: 900 },
		{ width: 390, height: 844 }
	]) {
		await journey(
			'conversation-popovers-' + viewport.width,
			async (page) => {
				await page.goto(origin + '/' + company + '/people?person=exec');
				const trigger = page.getByRole('button', {
					name: 'Search this conversation with Exec',
					exact: true
				});
				await trigger.click();
				const input = page.getByRole('textbox', { name: 'Search this conversation', exact: true });
				await input.fill('Reno');
				await page.setViewportSize({ width: viewport.width === 1440 ? 960 : 320, height: 420 });
				await page.waitForFunction(() => {
					const panel = document.querySelector('.history-panel:popover-open');
					if (!panel) return false;
					const box = panel.getBoundingClientRect();
					return (
						box.left >= 7 &&
						box.right <= innerWidth - 7 &&
						box.top >= 7 &&
						box.bottom <= innerHeight - 7
					);
				});
				await input.press('Escape');
				await page.locator('.history-panel:popover-open').waitFor({ state: 'hidden' });
				assert(
					await trigger.evaluate((el) => el === document.activeElement),
					'Search returns focus to its trigger'
				);
				const menu = page.locator('summary[aria-label="Conversation actions"]');
				await menu.press('Enter');
				await page.getByRole('button', { name: 'Copy loaded messages', exact: true }).waitFor();
				const box = await page.locator('.action-menu-panel:popover-open').boundingBox();
				const size = page.viewportSize();
				assert(
					box.x >= 0 &&
						box.y >= 0 &&
						box.x + box.width <= size.width &&
						box.y + box.height <= size.height,
					'Conversation menu stays in the viewport'
				);
				await page.keyboard.press('Escape');
				await page.locator('.action-menu-panel:popover-open').waitFor({ state: 'hidden' });
			},
			viewport
		);
		await journey(
			'profile-drawer-' + viewport.width,
			async (page) => {
				await page.goto(origin + '/' + company + '/people');
				await page.locator('summary[aria-label="More for Arlo"]').click();
				await page.getByRole('button', { name: 'Profile', exact: true }).click();
				const drawer = page.getByRole('dialog', { name: 'Arlo', exact: true });
				await drawer.waitFor();
				const rect = await drawer.boundingBox();
				assert(
					rect.y === 0 &&
						Math.abs(rect.height - viewport.height) < 1 &&
						Math.abs(rect.x + rect.width - viewport.width) < 1,
					'Profile fills the right edge'
				);
				assert(
					!/rgba\(/.test(await drawer.evaluate((el) => getComputedStyle(el).backgroundColor)),
					'Profile is opaque'
				);
				await drawer.getByRole('button', { name: 'Close', exact: true }).press('Escape');
				await drawer.waitFor({ state: 'hidden' });
			},
			viewport
		);
		await journey(
			'empty-inbox-floor-' + viewport.width,
			async (page) => {
				await page.route('**/api/companies/' + company + '/attention', async (route) => {
					const response = await route.fetch();
					const data = await response.json();
					await route.fulfill({ response, json: { ...data, items: [] } });
				});
				await page.route('**/api/companies/' + company + '/for-you*', (route) =>
					route.fulfill({ json: { items: [], unread_count: 0, next_before: null } })
				);
				await page.goto(origin + '/' + company);
				const floor = page.getByLabel('Interactive company floor', { exact: true });
				await floor.waitFor();
				const sizes = await floor.evaluate((el) => {
					const floor = el.getBoundingClientRect(),
						pane = el.closest('.clear-surface').getBoundingClientRect();
					return {
						floorHeight: floor.height,
						paneHeight: pane.height,
						floorWidth: floor.width,
						paneWidth: pane.width
					};
				});
				assert(sizes.floorHeight > viewport.height * 0.65, 'Floor uses the available height');
				assert(
					Math.abs(sizes.floorHeight - sizes.paneHeight) < 2 &&
						Math.abs(sizes.floorWidth - sizes.paneWidth) < 2,
					'Office fills its pane'
				);
				assert.equal(await page.getByText('Recent outcomes', { exact: true }).count(), 0);
			},
			viewport
		);
	}

	await journey('hanging-home-read-has-deadline', async (page) => {
		await page.route('**/api/companies', async () => new Promise(() => {}));
		await page.goto(origin + '/', { waitUntil: 'domcontentloaded' });
		await page.locator('button.failure-retry').waitFor({ timeout: 20000 });
		assert.equal(await page.locator('.first-paint-pending').count(), 0);
		assert.equal(await page.getByText('No companies yet', { exact: true }).count(), 0);
	});

	await journey('failed-home-has-recovery', async (page) => {
		await page.route('**/api/companies', (route) =>
			route.fulfill({
				status: 503,
				contentType: 'application/json',
				body: JSON.stringify({
					error: 'unavailable',
					message: 'The company catalogue is temporarily unavailable.'
				})
			})
		);
		await page.goto(origin + '/', { waitUntil: 'domcontentloaded' });
		await page.locator('button.failure-retry').waitFor({ timeout: 15000 });
		assert.equal(await page.getByText('No companies yet', { exact: true }).count(), 0);
		assert.equal(await page.locator('.first-paint-pending').count(), 0);
	});
} finally {
	await writeFile(out + '/report.json', JSON.stringify(results, null, 2));
	await browser.close();
}
