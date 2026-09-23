// UI fixtures only: no company writes, installations, keys or agent assignment reach the live backend.
import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
const base = process.env.RESTLESS_SMOKE_ORIGIN ?? 'http://127.0.0.1:18788';
const company = process.env.RESTLESS_FIXTURE_COMPANY;
assert(company, 'RESTLESS_FIXTURE_COMPANY is required');
const presets = JSON.parse(
	await fs.readFile(new URL('../../tools/custom-harness/presets.json', import.meta.url), 'utf8')
);
const output = process.env.RESTLESS_VERIFY_OUTPUT ?? '/tmp/restless-custom-harness-ui_test';
await fs.mkdir(output, { recursive: true });
const browser = await chromium.launch({
	executablePath: process.env.RESTLESS_BROWSER_EXECUTABLE,
	headless: true
});
try {
	const page = await browser.newPage();
	const errors = [];
	await page.clock.install();
	page.on('pageerror', (e) => errors.push(e.message));
	let revision = 1,
		rows = [],
		failed = false,
		mutations = 0;
	await page.route('**/api/**', async (route) => {
		const req = route.request(),
			url = new URL(req.url());
		if (url.pathname.includes('/custom-harnesses')) {
			if (req.method() === 'GET') {
				for (const row of rows) {
					if (row.checking && ++row.autoChecks >= 2) {
						row.checking = false;
						row.discovery = {
							state: 'compatible',
							checked_at: new Date().toISOString(),
							models: [{ id: 'custom:local:vendor/model', name: 'Example model' }]
						};
						if (row.nextModel)
							row.discovery.models.push({ id: row.nextModel, name: 'New native model' });
					}
				}
				return route.fulfill({
					status: failed ? 503 : 200,
					json: failed
						? { message: 'Test computer unavailable' }
						: { revision: String(revision), harnesses: rows, presets }
				});
			}
			const body = req.postDataJSON();
			assert.equal(body.revision, String(revision));
			mutations++;
			const id = url.pathname.split('/').at(-1);
			if (req.method() === 'PUT') {
				rows.push({ id, config: body.config, installation: { state: 'not_installed' } });
				revision++;
				return route.fulfill({ json: { saved: true, revision: String(revision) } });
			}
			assert.equal(req.method(), 'POST');
			const row = rows.find((r) => r.id === id);
			assert(row);
			if (body.action === 'install') {
				row.installation = { state: 'installed' };
				row.checking = true;
				row.autoChecks = 0;
			}
			if (body.action === 'probe')
				row.discovery = {
					state: 'compatible',
					models: [{ id: 'custom:local:vendor/model', name: 'Example model' }]
				};
			if (body.action === 'api_key') {
				assert.equal(body.secret, 'fixture-key-not-real');
				assert.equal(body.environment_name, 'OPENROUTER_API_KEY');
				revision++;
			}
			return route.fulfill({ json: { state: 'ok' } });
		}
		if (url.pathname.endsWith('/intelligence') && req.method() === 'GET')
			return route.fulfill({
				json: {
					revision: 'fixture',
					default: null,
					has_connections: true,
					connections: rows
						.filter((r) => r.discovery?.state === 'compatible')
						.map((r) => ({
							id: `harness:custom:${r.id}`,
							provider: r.config.name,
							kind: 'harness',
							models: r.discovery.models,
							loaded: true
						})),
					agents: [
						{
							id: 'exec',
							name: 'Exec',
							role: 'executive',
							harness: 'custom-acp',
							effective_model: 'native-custom-hermes/custom:local:vendor/model'
						}
					]
				}
			});
		assert.equal(req.method(), 'GET', 'all non-fixture API requests must be read-only');
		return route.continue();
	});
	await page.goto(`${base}/${company}/company/provider`);
	const closeRail = async () => {
		const toggle = page
			.locator('button[aria-controls="bridge-exrail"][aria-expanded="true"]')
			.filter({ visible: true });
		if (await toggle.count()) await toggle.click();
	};
	const section = page.locator('.custom-harnesses');
	await section.getByRole('button', { name: '+ Add harness' }).click();
	await section.getByRole('button', { name: 'Install harness', exact: true }).click();
	await section.getByText('Checking models', { exact: true }).waitFor();
	await page.clock.fastForward(4000);
	await section.getByText('Models available', { exact: true }).waitFor();
	await section.getByRole('button', { name: 'API key', exact: true }).click();
	await section.getByLabel('API key', { exact: true }).fill('fixture-key-not-real');
	await section.getByRole('button', { name: 'Save in Vault' }).click();
	await section.getByText('Models available', { exact: true }).waitFor();
	const assignments = page.locator('.assignments');
	await assignments
		.getByRole('button', { name: 'Change intelligence for Exec', exact: true })
		.click();
	assert.equal(
		await assignments.getByLabel('Connection', { exact: true }).inputValue(),
		'harness:custom:hermes'
	);
	assert.equal(
		await assignments.getByLabel('Model', { exact: true }).inputValue(),
		'custom:local:vendor/model'
	);
	assert.equal(
		await assignments
			.getByLabel('Connection', { exact: true })
			.locator('option:checked')
			.textContent(),
		'Hermes'
	);
	// A catalogue refresh arrives without pressing Check models or replacing an open selection.
	rows[0].checking = true;
	rows[0].autoChecks = 0;
	rows[0].nextModel = 'custom:local:vendor/new-model';
	await page.clock.fastForward(31000);
	await section.getByText('Refreshing models', { exact: true }).waitFor();
	await page.clock.fastForward(4000);
	await section.getByText('Models available', { exact: true }).waitFor();
	await assignments
		.getByLabel('Model', { exact: true })
		.locator('option[value="custom:local:vendor/new-model"]')
		.waitFor({ state: 'attached' });
	assert.equal(
		await assignments.getByLabel('Model', { exact: true }).inputValue(),
		'custom:local:vendor/model'
	);
	for (const width of [1440, 390, 320]) {
		await page.setViewportSize({ width, height: 1000 });
		await closeRail();
		await section.scrollIntoViewIfNeeded();
		const overflow = await section.evaluate((e) => e.scrollWidth > e.clientWidth + 1);
		assert(!overflow, `harness overflow at ${width}`);
		await page.screenshot({ path: `${output}/harnesses-${width}.png` });
	}
	await section.getByRole('button', { name: '+ Add harness' }).click();
	await section.getByLabel('Harness', { exact: true }).selectOption('custom');
	await section.getByLabel('Name', { exact: true }).fill('Example custom ACP');
	await section.getByLabel('Identifier', { exact: true }).fill('example');
	await section.getByLabel('Install command', { exact: true }).fill('echo install-fixture-only');
	await section
		.getByLabel('ACP command (JSON array)', { exact: true })
		.fill('["example-agent", "acp", "--system-prompt-file", "${SYSTEM_PROMPT_FILE}"]');
	await section.getByRole('button', { name: 'Install harness', exact: true }).click();
	await section.getByText('Example custom ACP', { exact: true }).waitFor();
	failed = true;
	await page.reload();
	await closeRail();
	await section.getByText('Test computer unavailable').waitFor();
	failed = false;
	await section.getByRole('button', { name: 'Retry', exact: true }).click();
	await section.getByText('Example custom ACP', { exact: true }).waitFor();
	assert.deepEqual(errors, []);
	assert.equal(mutations, 6);
	console.log(
		'PASS automatic install discovery/catalogue refresh preserves selection, preset/custom forms, independent keys, exact native models, retry, 1440/390/320px; fixture mutations only'
	);
} finally {
	await browser.close();
}
