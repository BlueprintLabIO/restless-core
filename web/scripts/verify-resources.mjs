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
		// Capture the real owner editor before response-state fixtures replace readings.
		await page.goto(origin + '/' + company + '/company/resources');
		const edit = page.getByRole('button', { name: 'Edit agent computer limits', exact: true });
		await edit.waitFor();
		await edit.click();
		const form = page.getByRole('form', { name: 'Agent computer limits', exact: true });
		await form.getByRole('button', { name: 'Save limits', exact: true }).scrollIntoViewIfNeeded();
		assert(
			Number(await form.getByRole('spinbutton', { name: 'CPU cores', exact: true }).inputValue()) >
				0
		);
		assert(
			Number(
				await form.getByRole('spinbutton', { name: 'Memory (MB)', exact: true }).inputValue()
			) > 0
		);
		await page.screenshot({ path: output + '/resources-real-editor-' + width + '.png' });
		await form.getByRole('button', { name: 'Cancel', exact: true }).click();
		let mode = 'pending',
			release;
		const ready = new Promise((resolve) => (release = resolve));
		const sample = {
			observed_at: new Date().toISOString(),
			host_memory_bytes: 8 * 1024 ** 3,
			host_available_bytes: 2 * 1024 ** 3,
			placement_current: true,
			work_admission: {
				scope: 'shared',
				waiting_for_memory: false,
				working_set_bytes: 2 * 1024 ** 3,
				resume_below_bytes: 5.4 * 1024 ** 3
			},
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
		const actorCapacity = {
			company_slots: 2,
			shared_slots: 3,
			company_active: 0,
			shared_active: 0,
			waiting_for_memory: false
		};
		let actorsAvailable = true;
		const targetLimits = () => ({
			saved: null,
			effective: { cpu_millis: 1000, memory_mib: 1024 },
			maximum: { cpu_millis: 2000, memory_mib: 2048 },
			applied: { cpu_millis: 1000, memory_mib: 1024 },
			state: 'running',
			pending: false
		});
		const limits = { runtime: targetLimits(), computer: targetLimits() };
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
							: {
									status: 'available',
									sample,
									actor_capacity: actorsAvailable ? actorCapacity : null,
									limits
								}
			});
		});
		await page.goto(origin + '/' + company + '/company/resources');
		const compute = page.locator('#compute');
		await compute.waitFor();
		await compute.getByText('Reading…', { exact: true }).first().waitFor();
		const bounds = () =>
			page.locator('#ai-usage').evaluate((node) => ({
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
		await compute.getByText('Memory available', { exact: true }).waitFor();
		limits.runtime.pending = true;
		limits.runtime.effective.memory_mib = 768;
		await compute.getByRole('button', { name: 'Refresh resource readings' }).click();
		await compute.getByText('Pending', { exact: true }).waitFor();
		const pendingLimit = await bounds();
		assert(
			Math.abs(pendingLimit.top - after.top) <= 1,
			'Pending saved limits must not move the next section'
		);
		limits.runtime.pending = false;
		limits.runtime.applied.memory_mib = 768;
		await compute.getByRole('button', { name: 'Refresh resource readings' }).click();
		await compute.getByText('Pending', { exact: true }).waitFor({ state: 'hidden' });
		actorCapacity.company_active = 2;
		actorCapacity.shared_active = 2;
		await compute.getByRole('button', { name: 'Refresh resource readings' }).click();
		await compute.getByText('2 / 2 here · 2 / 3 shared', { exact: true }).waitFor();
		await compute.getByText('Waiting for a slot', { exact: true }).waitFor();
		assert(
			Math.abs((await bounds()).top - after.top) <= 1,
			'Turn capacity changes must not move the next section'
		);
		actorCapacity.company_active = 0;
		actorCapacity.shared_active = 3;
		await compute.getByRole('button', { name: 'Refresh resource readings' }).click();
		await compute.getByText('0 / 2 here · 3 / 3 shared', { exact: true }).waitFor();
		await compute.getByText('Waiting for a slot', { exact: true }).waitFor();
		actorCapacity.shared_active = 0;
		sample.work_admission.waiting_for_memory = true;
		await compute.getByRole('button', { name: 'Refresh resource readings' }).click();
		await compute.getByText('Waiting for memory', { exact: true }).waitFor();
		const waiting = await bounds();
		assert(
			Math.abs(waiting.top - after.top) <= 1,
			'Memory admission changes must not move the next section'
		);
		sample.work_admission.scope = 'company';
		await compute.getByRole('button', { name: 'Refresh resource readings' }).click();
		await compute.getByText('Company RAM busy', { exact: true }).waitFor();
		assert(
			Math.abs((await bounds()).top - after.top) <= 1,
			'Company memory wait must not move the next section'
		);
		sample.work_admission.scope = 'shared';
		sample.work_admission.waiting_for_memory = false;
		sample.work_admission.waiting_for_processes = true;
		await compute.getByRole('button', { name: 'Refresh resource readings' }).click();
		await compute.getByText('Waiting for process capacity', { exact: true }).waitFor();
		assert(Math.abs((await bounds()).top - after.top) <= 1, 'Process capacity wait must not move the next section');
		sample.work_admission.waiting_for_processes = false;
		await compute.getByRole('button', { name: 'Refresh resource readings' }).click();
		await compute.getByText('Memory available', { exact: true }).waitFor();
		actorCapacity.waiting_for_memory = true;
		await compute.getByRole('button', { name: 'Refresh resource readings' }).click();
		await compute.getByText('Waiting for memory', { exact: true }).waitFor();
		actorCapacity.waiting_for_memory = false;
		await compute.getByRole('button', { name: 'Refresh resource readings' }).click();
		await compute.getByText('Memory available', { exact: true }).waitFor();
		actorsAvailable = false;
		await compute.getByRole('button', { name: 'Refresh resource readings' }).click();
		await compute.getByText('No reading', { exact: true }).first().waitFor();
		assert.equal(await compute.getByText('No reading', { exact: true }).count(), 2);
		actorsAvailable = true;
		await compute.getByRole('button', { name: 'Refresh resource readings' }).click();
		await compute.getByText('0 / 2 here · 0 / 3 shared', { exact: true }).waitFor();
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
		await compute.getByText('Account limits not enabled', { exact: true }).waitFor();
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
			memoryWaitShiftPx: waiting.top - after.top,
			memoryAdmission: 'waiting and resumed',
			actorAdmission: 'company/shared slots and startup memory',
			pendingLimitShiftPx: pendingLimit.top - after.top,
			recovery: 'live'
		});
		await context.close();
	}
	await fs.writeFile(output + '/report.json', JSON.stringify(report, null, 2) + '\n');
	console.log(JSON.stringify(report, null, 2));
} finally {
	await browser.close();
}
