/* US-246: real seeded Inbox items keep a shared bottom-right decision footer.
 * No requests are answered. Test short viewports too: scrolling content must leave controls put. */
import { chromium } from 'playwright';
import { mkdir, writeFile } from 'node:fs/promises';
import assert from 'node:assert/strict';

const origin = process.env.RESTLESS_SMOKE_ORIGIN;
const company = process.env.RESTLESS_SMOKE_COMPANY;
if (!origin || !company?.endsWith('_test'))
	throw new Error('Set RESTLESS_SMOKE_ORIGIN and a _test RESTLESS_SMOKE_COMPANY');
const out = process.env.RESTLESS_VERIFY_OUTPUT ?? '/tmp/restless-inbox-actions';
await mkdir(out, { recursive: true });
const browser = await chromium.launch();
const report = [];
try {
	const response = await fetch(origin + '/api/companies/' + company + '/attention');
	assert(response.ok, 'Attention must load from the real daemon');
	const { items } = await response.json();
	const kinds = new Map();
	for (const item of items) {
		const kind =
			item.native_document?.kind ??
			(item.charter_proposal
				? 'charter'
				: item.direction_request
					? 'direction'
					: item.actions.some((action) => action.id === 'accept-review')
						? 'outcome'
						: 'handoff');
		if (!kinds.has(kind)) kinds.set(kind, item);
	}
	for (const kind of ['direction', 'charter', 'review', 'collaboration', 'handoff', 'outcome'])
		assert(
			kinds.has(kind),
			'Seed missing ' + kind + ' with scripts/seed-inbox-actions.mjs and seed-reno-scale.mjs'
		);
	for (const viewport of [
		{ width: 1440, height: 900 },
		{ width: 390, height: 844 },
		{ width: 960, height: 420 },
		{ width: 390, height: 420 }
	]) {
		const context = await browser.newContext({ viewport, reducedMotion: 'reduce' });
		const page = await context.newPage();
		const errors = [];
		page.on('pageerror', (error) => errors.push(error.message));
		for (const [kind, item] of kinds) {
			await page.goto(origin + '/' + company + '?item=' + encodeURIComponent(item.id), {
				waitUntil: 'networkidle'
			});
			const exec = page.getByRole('button', { name: 'Exec', exact: true });
			if (viewport.width >= 1200 && (await exec.getAttribute('aria-expanded')) === 'false')
				await exec.click();
			const footer = page.locator('[data-inbox-actions]');
			await footer.waitFor({ state: 'visible' });
			if (kind === 'review') {
				await footer.getByRole('button', { name: 'Accept this version', exact: true }).waitFor();
				await page.getByRole('button', { name: 'Hide comments and history', exact: true }).click();
				await footer.getByRole('button', { name: 'Accept this version', exact: true }).waitFor();
				await footer.getByRole('button', { name: 'Request changes', exact: true }).click();
				await page.getByRole('textbox', { name: 'Note', exact: true }).waitFor();
				assert(
					await page
						.getByRole('textbox', { name: 'Note', exact: true })
						.evaluate((element) => document.activeElement === element),
					'Request changes reveals and focuses its note'
				);
			}
			const geometry = async () =>
				footer.evaluate((element) => {
					const rect = element.getBoundingClientRect();
					const pane = element.closest('.attention-focus').getBoundingClientRect();
					const controls = [...element.querySelectorAll('button,a.btn')].filter(
						(node) => node.getBoundingClientRect().height > 0
					);
					const boxes = controls.map((node) => node.getBoundingClientRect());
					return {
						top: rect.top,
						bottom: rect.bottom,
						right: rect.right,
						paneBottom: pane.bottom,
						paneRight: pane.right,
						viewportHeight: innerHeight,
						controlRight: Math.max(...boxes.map((box) => box.right)),
						count: controls.length,
						visible: boxes.every(
							(box) =>
								box.left >= rect.left &&
								box.right <= rect.right + 1 &&
								box.top >= rect.top &&
								box.bottom <= rect.bottom + 1
						)
					};
				});
			const before = await geometry();
			assert(before.count > 0 && before.visible, kind + ' has visible controls');
			assert(Math.abs(before.bottom - before.paneBottom) < 2, kind + ' footer reaches pane bottom');
			assert(Math.abs(before.right - before.paneRight) < 2, kind + ' footer reaches pane right');
			assert(before.bottom <= before.viewportHeight, kind + ' footer fits viewport');
			assert(before.right - before.controlRight <= 26, kind + ' controls align right');
			const scroll = page
				.locator(
					'.request-content,.inbox-detail > .body,.charter-content,.requested-version,.native-document-body'
				)
				.filter({ visible: true })
				.first();
			if (await scroll.count()) {
				await scroll.evaluate((element) => {
					element.scrollTop = element.scrollHeight;
				});
				const after = await geometry();
				assert(
					Math.abs(before.bottom - after.bottom) < 1 && Math.abs(before.right - after.right) < 1,
					kind + ' controls stay still while content scrolls'
				);
			}
			if (kind === 'direction') {
				await footer.getByRole('button', { name: /Decline…/ }).click();
				await footer.getByRole('textbox').fill('A draft reason, never sent');
				const decline = await geometry();
				assert(
					decline.visible && Math.abs(decline.bottom - before.bottom) < 1,
					'Decline form remains in footer'
				);
				await footer.getByRole('button', { name: 'Cancel', exact: true }).click();
			}
			if (kind === 'handoff') {
				await footer.getByRole('button', { name: 'Not doing this', exact: true }).click();
				await footer
					.getByRole('textbox', { name: 'Why not', exact: true })
					.fill('Long reply context. '.repeat(80));
				const reply = await geometry();
				assert(
					reply.visible && Math.abs(reply.bottom - before.bottom) < 1,
					'Growing reply leaves controls at pane bottom'
				);
				await footer.getByRole('button', { name: 'Back', exact: true }).click();
			}
			if (kind === 'outcome') {
				await footer.getByRole('button', { name: 'Accept outcome', exact: true }).waitFor();
				await footer.getByRole('button', { name: 'Request changes', exact: true }).click();
				await footer
					.getByRole('textbox', { name: /Changes for/ })
					.fill('A long review note. '.repeat(100));
				const change = await geometry();
				assert(
					change.visible && Math.abs(change.bottom - before.bottom) < 1,
					'Outcome feedback keeps decisions anchored'
				);
				await footer.getByRole('button', { name: 'Back', exact: true }).click();
			}
			if (kind === 'charter') {
				await footer.getByRole('button', { name: 'Edit', exact: true }).click();
				await footer.getByRole('button', { name: 'Publish', exact: true }).waitFor();
				const edit = await geometry();
				assert(
					edit.visible && Math.abs(edit.bottom - before.bottom) < 1,
					'Charter edit actions remain in footer'
				);
				await footer.getByRole('button', { name: 'Cancel', exact: true }).click();
			}
			await page.screenshot({
				path: out + '/' + kind + '-' + viewport.width + 'x' + viewport.height + '.png'
			});
			report.push({ kind, viewport, ...before });
			console.log(kind + ' ' + viewport.width + 'x' + viewport.height + ' passed');
		}
		assert.deepEqual(errors, [], 'No browser errors');
		await context.close();
	}
	console.log(
		'Inbox footer: direction, handoff, charter, native review, outcome review and collaboration stay bottom-right at four desktop/phone viewport sizes; scroll, decline and charter editing passed.'
	);
} finally {
	await writeFile(out + '/report.json', JSON.stringify(report, null, 2));
	await browser.close();
}
