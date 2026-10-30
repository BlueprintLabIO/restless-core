// Run against a seeded disposable _test company through its local web server.
// Set RESTLESS_SMOKE_ORIGIN and RESTLESS_SMOKE_COMPANY.
import { chromium } from 'playwright';
import assert from 'node:assert/strict';
const origin = process.env.RESTLESS_SMOKE_ORIGIN,
	company = process.env.RESTLESS_SMOKE_COMPANY;
assert(company?.endsWith('_test'));
const browser = await chromium.launch();
try {
	const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
	const errors = [];
	page.on('pageerror', (e) => errors.push(e.message));
	for (const [path, label, pane] of [
		['', 'Resize attention panes', '.attention-index'],
		['/work', 'Resize the Work sidebar', '.sidebar-nav'],
		['/library', 'Resize the Library sidebar', '.sidebar-nav'],
		['/apps', 'Resize the Apps sidebar', '.sidebar-nav'],
		['/company', 'Resize the Company sidebar', '.sidebar-nav']
	]) {
		await page.goto(`${origin}/${company}${path}`);
		const handle = page.getByRole('separator', { name: label, exact: true });
		await handle.waitFor({ timeout: 90000 });
		await handle.focus();
		await page.keyboard.press('Home');
		await page.waitForTimeout(500);
		assert.equal(Number(await handle.getAttribute('aria-valuenow')), 160, `${label}: min`);
		const box = await page.locator(pane).boundingBox();
		assert(Math.abs(box.width - 160) < 1, `${label}: width`);
		const clipped = await page
			.locator(
				`${pane} header button, ${pane} header summary, ${pane} .sidebar-head button, ${pane} .sidebar-head summary`
			)
			.evaluateAll((buttons) =>
				buttons
					.filter((b) => {
						const a = b.getBoundingClientRect(),
							p = b.closest('aside,nav').getBoundingClientRect();
						return a.width > 0 && (a.left < p.left - 1 || a.right > p.right + 1);
					})
					.map((b) => b.getAttribute('aria-label') || b.textContent)
			);
		assert.deepEqual(clipped, [], `${label}: clipped controls`);
		if (!path) {
			await page.locator('summary[aria-label^="Filter Inbox"]').click();
			await page.getByRole('button', { name: 'Skipped', exact: true }).waitFor();
			await page.keyboard.press('Escape');
		}

		await page.reload();
		await handle.waitFor({ timeout: 90000 });
		assert.equal(Number(await handle.getAttribute('aria-valuenow')), 160, `${label}: restored`);
		console.log(
			JSON.stringify({
				path: path || '/inbox',
				width: box.width,
				controlsClipped: clipped,
				restored: true
			})
		);
	}
	assert.deepEqual(errors, []);
} finally {
	await browser.close();
}
