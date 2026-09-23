import { chromium } from 'playwright';
import assert from 'node:assert/strict';

const origin = process.env.RESTLESS_SMOKE_ORIGIN ?? 'http://127.0.0.1:8788';
const source = process.env.RESTLESS_FIXTURE_COMPANY;
assert(source, 'RESTLESS_FIXTURE_COMPANY is required');
const executablePath = process.env.RESTLESS_BROWSER_EXECUTABLE;
assert(executablePath, 'RESTLESS_BROWSER_EXECUTABLE is required');
const company = 'provider_connections_test';
const models = [
	{ id: 'gpt-5.6-terra', name: 'GPT-5.6 Terra', default: true },
	{ id: 'claude-sonnet-4-6', name: 'Claude Sonnet', default: true }
];
let connections = [
	{
		id: 'direct:openai',
		provider: 'openai',
		kind: 'direct',
		model: models[0].id,
		models: [models[0]],
		loaded: true
	},
	{
		id: 'direct:anthropic',
		provider: 'anthropic',
		kind: 'direct',
		model: models[1].id,
		models: [models[1]],
		loaded: true
	}
];
let agents = [
	{
		id: 'exec',
		name: 'The Exec',
		role: 'exec',
		assignment: { connection: 'direct:openai', model: models[0].id },
		effective_model: 'openai/gpt-5.6-terra',
		harness: '',
		thinking_effort: 'medium'
	},
	{
		id: 'hosting-engineering',
		name: 'Daria',
		role: 'lead',
		assignment: { connection: 'direct:openai', model: models[0].id },
		effective_model: 'openai/gpt-5.6-terra',
		harness: '',
		thinking_effort: 'medium'
	}
];
const providerRows = () => [
	{
		provider: 'openai',
		reference: 'infisical:/companies/test/OPENAI_API_KEY',
		credential_status: 'present',
		credential_detail: null,
		gateway_loaded: true
	},
	{
		provider: 'anthropic',
		reference: 'infisical:/companies/test/ANTHROPIC_API_KEY',
		credential_status: 'present',
		credential_detail: null,
		gateway_loaded: true
	}
];
const intelligence = () => ({
	revision: 'fixture-revision',
	default: connections[0] ? { connection: connections[0].id, model: connections[0].model } : null,
	has_connections: !!connections.length,
	connections,
	agents
});
const browser = await chromium.launch({ executablePath });
let context;
let intelligenceMode = 'ready';
try {
	context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
	const page = await context.newPage();
	const writes = [];
	const errors = [];
	page.on('pageerror', (error) => errors.push(error.message));
	await context.route('**/api/**', async (route) => {
		const req = route.request(),
			url = new URL(req.url()),
			path = url.pathname;
		if (req.method() === 'GET') {
			if (path.endsWith('/intelligence')) {
				if (intelligenceMode === 'unavailable')
					return route.fulfill({ status: 503, json: { message: 'fixture unavailable' } });
				return route.fulfill({
					json: {
						...intelligence(),
						has_connections: intelligenceMode === 'unknown' ? null : intelligence().has_connections
					}
				});
			}
			if (path.endsWith('/provider'))
				return route.fulfill({
					json: {
						revision: 'fixture-revision',
						primary_provider: 'openai',
						connections: providerRows(),
						infisical_configured: true,
						infisical_status: 'present',
						infisical_detail: null,
						startup_issue: null
					}
				});
			if (path.endsWith('/harness-auth'))
				return route.fulfill({
					json: {
						connections: [
							{
								harness: 'codex',
								mode: 'oauth',
								model: 'gpt-5.6-terra',
								auth: { state: 'connected', account: { email: 'codex@fixture.test' } }
							},
							{
								harness: 'claude-agent',
								mode: 'oauth',
								model: 'claude-sonnet-4-6',
								auth: { state: 'waiting', desktop: true, message: 'Continue independently.' }
							}
						]
					}
				});
			if (path.endsWith('/activity'))
				return route.fulfill({ contentType: 'text/event-stream', body: '' });
			const upstream = await context.request.get(req.url().replaceAll(company, source));
			return route.fulfill({
				status: upstream.status(),
				contentType: upstream.headers()['content-type'] ?? 'application/json',
				body: await upstream.text()
			});
		}
		if (req.method() === 'PUT' && path.includes('/intelligence/')) {
			const body = req.postDataJSON();
			writes.push(body);
			assert.match(path, /\/intelligence\/exec$/);
			agents = agents.map((agent) =>
				agent.id === 'exec'
					? {
							...agent,
							assignment: { connection: body.connection, model: body.model },
							effective_model: `${body.connection.split(':')[1]}/${body.model}`
						}
					: agent
			);
			return route.fulfill({ json: intelligence() });
		}
		if (req.method() === 'POST' && path.includes('/harness-auth/'))
			return route.fulfill({
				json: {
					harness: path.split('/').at(-1),
					mode: 'oauth',
					model: 'fixture',
					auth: { state: 'waiting' }
				}
			});
		return route.fulfill({
			status: 403,
			contentType: 'application/json',
			body: '{"message":"writes disabled"}'
		});
	});
	await page.goto(`${origin}/${company}/company/provider`, { waitUntil: 'domcontentloaded' });
	await page
		.getByRole('region', { name: 'Your connections' })
		.getByText('OpenAI', { exact: true })
		.waitFor();
	await page
		.getByRole('region', { name: 'Your connections' })
		.getByText('Anthropic', { exact: true })
		.waitFor();
	await page.locator('.connection-row .badge.success').first().waitFor();
	await page.getByText('ChatGPT / Codex', { exact: true }).waitFor();
	await page.getByText('Claude Code', { exact: true }).waitFor();
	await page.getByText('Continue Claude sign-in in company desktop', { exact: false }).waitFor();
	const assignments = page.locator('.assignments');
	await assignments
		.getByRole('button', { name: 'Change intelligence for Exec', exact: true })
		.click();
	await assignments.getByLabel('Connection', { exact: true }).selectOption('direct:anthropic');
	await assignments.getByRole('button', { name: 'Save assignment', exact: true }).click();
	await page.reload();
	assert.equal(writes.length, 1);
	const execRow = assignments.locator('.agent-row').filter({ hasText: 'Exec' });
	const dariaRow = assignments.locator('.agent-row').filter({ hasText: 'Daria' });
	assert.match(await execRow.locator('.route').innerText(), /Anthropic[\s\S]*claude-sonnet-4-6/);
	assert.match(await dariaRow.locator('.route').innerText(), /OpenAI[\s\S]*gpt-5\.6-terra/);
	connections = [];
	agents = [];
	await page.goto(`${origin}/${company}`);
	const cta = page.getByRole('link', { name: /Add intelligence provider/i });
	await cta.waitFor();
	assert.match(await cta.getAttribute('href'), new RegExp(`/${company}/company/provider$`));
	intelligenceMode = 'unknown';
	await Promise.all([
		page.waitForResponse((r) => r.url().endsWith('/intelligence') && r.status() === 200),
		page.reload()
	]);
	await page.locator('.exr-composer textarea').waitFor();
	await page.evaluate(
		() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)))
	);
	assert.equal(await page.getByRole('link', { name: /Add intelligence provider/i }).count(), 0);
	assert.deepEqual(errors, []);
	intelligenceMode = 'unavailable';
	await Promise.all([
		page.waitForResponse((r) => r.url().endsWith('/intelligence') && r.status() === 503),
		page.reload()
	]);
	await page.locator('.exr-composer textarea').waitFor();
	await page.evaluate(
		() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)))
	);
	assert.equal(await page.getByRole('link', { name: /Add intelligence provider/i }).count(), 0);
	console.log(
		'PASS two direct connections, independent Codex/Claude auth states, per-agent reload isolation, empty-provider CTA and unknown/failed-read distinction'
	);
} finally {
	await context?.unrouteAll({ behavior: 'ignoreErrors' });
	await browser.close();
}
