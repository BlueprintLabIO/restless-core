import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import { chromium } from 'playwright';
const origin = process.env.RESTLESS_SMOKE_ORIGIN,
	company = process.env.RESTLESS_SMOKE_COMPANY;
assert(
	origin && new URL(origin).hostname === '127.0.0.1' && company?.endsWith('_test'),
	'Use a local test company'
);
const output = process.env.RESTLESS_VERIFY_OUTPUT ?? '/tmp/restless-resource-states';
await fs.mkdir(output, { recursive: true });
const browser = await chromium.launch();
const report = [];
try {
	for (const width of [1440, 390]) {
		const context = await browser.newContext({
			viewport: { width, height: 900 },
			reducedMotion: 'reduce'
		});
		const page = await context.newPage();
		const errors = [];
		page.on('pageerror', (e) => errors.push(e.message));
		let mode = 'pending',
			release;
		const ready = new Promise((resolve) => (release = resolve));
		const sample = {
			observed_at: new Date().toISOString(),
			host_memory_bytes: 8 * 1024 ** 3,
			host_available_bytes: 2 * 1024 ** 3,
			placement_current: true,
			pool: {
				memory_used_bytes: 3 * 1024 ** 3,
				memory_limit_bytes: 6 * 1024 ** 3,
				cpu_used_cores: 1.5,
				cpu_limit_cores: 3
			},
			work: {
				memory_used_bytes: 1024 ** 3,
				memory_limit_bytes: 2 * 1024 ** 3,
				cpu_used_cores: 0.25,
				cpu_limit_cores: 1
			},
			computer: {
				memory_used_bytes: 80 * 1024 ** 2,
				memory_limit_bytes: 1024 ** 3,
				cpu_used_cores: 0,
				cpu_limit_cores: 1
			}
		};
		await page.route('**/api/companies/' + company + '/company/compute', async (route) => {
			if (mode === 'pending') await ready;
			if (mode === 'error')
				return route.fulfill({ status: 503, json: { error: 'temporary test outage' } });
			await route.fulfill({
				json:
					mode === 'not_configured'
						? { status: 'not_configured' }
						: mode === 'unavailable'
							? { status: 'unavailable' }
							: { status: 'available', sample }
			});
		});
		await page.goto(origin + '/' + company + '/company/resources');
		const compute = page.locator('#compute');
		await compute.waitFor();
		await compute.getByText('Reading…', { exact: true }).waitFor();
		const bounds = () =>
			page
				.locator('#ai-usage')
				.evaluate((node) => ({
					top: node.getBoundingClientRect().top,
					height: node.getBoundingClientRect().height
				}));
		const before = await bounds();
		mode = 'available';
		release();
		await compute.getByText('Live', { exact: true }).waitFor();
		const after = await bounds();
		assert(
			Math.abs(before.top - after.top) <= 1,
			'Compute readings must not move the following section'
		);
		mode = 'error';
		await compute.getByRole('button', { name: 'Refresh resource readings' }).click();
		await compute.getByText('Could not refresh', { exact: true }).waitFor();
		assert.match(
			await compute.innerText(),
			/3 GB/,
			'failed refresh retains previous actual readings'
		);
		mode = 'not_configured';
		await compute.getByRole('button', { name: 'Refresh resource readings' }).click();
		await compute.getByText('Shared limits not enabled', { exact: true }).waitFor();
		assert(
			!(await compute.innerText()).includes('3 GB'),
			'unconfigured limits must not present an old live sample'
		);
		mode = 'unavailable';
		await compute.getByRole('button', { name: 'Refresh resource readings' }).click();
		await compute.getByText('Readings unavailable', { exact: true }).waitFor();
		assert(
			!(await compute.innerText()).includes('0 cores'),
			'unavailable usage must not masquerade as zero'
		);
		await page.screenshot({ path: output + '/resources-unavailable-' + width + '.png' });
		mode = 'available';
		await compute.getByRole('button', { name: 'Refresh resource readings' }).click();
		await compute.getByText('Live', { exact: true }).waitFor();
		await page.screenshot({ path: output + '/resources-live-' + width + '.png' });
		assert.deepEqual(errors, []);
		report.push({
			width,
			firstReadingShiftPx: after.top - before.top,
			failedRefresh: 'retains readings',
			unconfigured: 'unknown',
			unavailable: 'unknown',
			recovery: 'live'
		});
		await context.close();
	}
	await fs.writeFile(output + '/report.json', JSON.stringify(report, null, 2) + '\n');
	console.log(JSON.stringify(report, null, 2));
} finally {
	await browser.close();
}
