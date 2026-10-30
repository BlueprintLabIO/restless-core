import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import { mkdir, copyFile, rm } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

// Real picker with synthetic identity and intelligence responses. No model provider is called.
const web = join(dirname(fileURLToPath(import.meta.url)), '..');
const fixture = join(web, 'src/routes/s109-agent-model-fixture');
await mkdir(fixture);
let server, browser;
try {
	await copyFile(
		join(web, 'scripts/fixtures/s109-agent-model.svelte'),
		join(fixture, '+page.svelte')
	);
	const port = await new Promise((resolve, reject) => {
		const s = createServer();
		s.once('error', reject);
		s.listen(0, '127.0.0.1', () => {
			const p = s.address().port;
			s.close(() => resolve(p));
		});
	});
	server = spawn(
		join(web, 'node_modules/.bin/vite'),
		['dev', '--host', '127.0.0.1', '--port', String(port), '--strictPort'],
		{
			cwd: web,
			env: { ...process.env, RESTLESS_OWNER_URL: 'http://127.0.0.1:1' },
			stdio: 'ignore',
			detached: true
		}
	);
	const origin = `http://127.0.0.1:${port}`;
	for (let i = 0; i < 120; i++) {
		if (server.exitCode !== null) throw Error('fixture server exited');
		try {
			if (await fetch(origin)) break;
		} catch {}
		await new Promise((r) => setTimeout(r, 500));
	}
	browser = await chromium.launch();
	for (const width of [1440, 390]) {
		const page = await browser.newPage({ viewport: { width, height: 900 } });
		let saved = null;
		const view = {
			revision: 'test-revision',
			default: null,
			base_default: { connection: 'direct:anthropic', model: 'claude-base' },
			has_connections: true,
			company_issue: null,
			connections: [
				{
					id: 'account:openai@fixture',
					provider: 'openai',
					kind: 'direct',
					loaded: true,
					models: [
						{ id: 'gpt-example', name: 'GPT Example' },
						{ id: 'gpt-next', name: 'GPT Next' }
					]
				},
				{
					id: 'direct:anthropic',
					provider: 'anthropic',
					kind: 'direct',
					loaded: true,
					models: [{ id: 'claude-base', name: 'Claude Base' }]
				}
			],
			agents: [
				{
					id: 'eames',
					name: 'Eames',
					role: 'lead',
					assignment: null,
					effective_model: 'openai/gpt-example',
					harness: 'restless-managed',
					thinking_effort: 'medium'
				}
			]
		};
		await page.route('**/api/**', async (route) => {
			const path = new URL(route.request().url()).pathname;
			if (path.endsWith('/principal'))
				return route.fulfill({
					json: { actor_id: 'owner', membership_role: 'owner', cache_partition: 'fixture' }
				});
			if (path.endsWith('/intelligence/eames') && route.request().method() === 'PUT') {
				saved = route.request().postDataJSON();
				view.agents[0].assignment = { connection: saved.connection, model: saved.model };
				view.agents[0].effective_model = `openai/${saved.model}`;
				return route.fulfill({ json: { ok: true } });
			}
			if (path.endsWith('/intelligence')) return route.fulfill({ json: view });
			if (path === '/api/model-catalog')
				return route.fulfill({ json: { updatedAt: Date.now(), catalog: [] } });
			if (path === '/api/connections') return route.fulfill({ json: { connections: [] } });
			return route.fulfill({ json: {} });
		});
		await page.goto(`${origin}/s109-agent-model-fixture`);
		await page.locator('.chip summary').waitFor({ timeout: 30000 });
		await page.locator('.chip summary').click();
		await page.getByRole('option', { name: 'GPT Next' }).waitFor();
		assert.equal(
			await page.getByRole('option', { name: 'Claude Base' }).count(),
			0,
			'base provider must not appear in agent picker'
		);
		assert.equal(
			await page.getByRole('option', { name: 'GPT Example' }).getAttribute('aria-selected'),
			'true'
		);
		await page.getByRole('option', { name: 'GPT Next' }).click();
		assert.deepEqual(saved, {
			connection: 'account:openai@fixture',
			model: 'gpt-next',
			reset: false,
			revision: 'test-revision'
		});
		await page.getByText('Eames will use GPT Next from their next session.').waitFor();
		console.log(
			`PASS ${width}px agent picker shows the effective provider and sends the correct assignment`
		);
		await page.close();
	}
} finally {
	if (browser) await browser.close();
	if (server && server.exitCode === null) {
		try {
			process.kill(-server.pid, 'SIGTERM');
		} catch {}
		await Promise.race([
			new Promise((r) => server.once('exit', r)),
			new Promise((r) => setTimeout(r, 5000))
		]);
	}
	await rm(fixture, { recursive: true, force: true });
}
