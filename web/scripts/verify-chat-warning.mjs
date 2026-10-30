import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import { mkdir, copyFile, rm } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

// Mount the real rail under a temporary local route with simulated outcomes.
// The server cannot reach the user's local appliance or a model provider.
const web = join(dirname(fileURLToPath(import.meta.url)), '..');
const route = join(web, 'src/routes/s109-chat-warning-fixture');
await mkdir(route); // Refuse to replace an existing route.
let server;
let browser;
try {
	await copyFile(
		join(web, 'scripts/fixtures/s109-chat-warning.svelte'),
		join(route, '+page.svelte')
	);
	const port = await new Promise((resolve, reject) => {
		const socket = createServer();
		socket.once('error', reject);
		socket.listen(0, '127.0.0.1', () => {
			const port = socket.address().port;
			socket.close(() => resolve(port));
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
	let ready = false;
	for (let i = 0; i < 120; i++) {
		if (server.exitCode !== null) throw new Error('Fixture server exited');
		try {
			await fetch(origin);
			ready = true;
			break;
		} catch {
			await new Promise((resolve) => setTimeout(resolve, 500));
		}
	}
	assert(ready, 'Fixture server did not start');
	browser = await chromium.launch(
		process.env.RESTLESS_BROWSER_EXECUTABLE
			? { executablePath: process.env.RESTLESS_BROWSER_EXECUTABLE }
			: {}
	);
	for (const width of [1440, 390]) {
		const page = await browser.newPage({ viewport: { width, height: 900 } });
		const errors = [];
		page.on('pageerror', (e) => errors.push(e.message));
		await page.goto(`${origin}/s109-chat-warning-fixture`);
		await page.locator('.fixture-controls[data-ready="true"]').waitFor({ timeout: 60000 });
		await page.waitForLoadState('networkidle');
		await page.waitForTimeout(3000); // Let the dev server finish dependency optimization.
		await page.locator('.fixture-controls[data-ready="true"]').waitFor({ timeout: 60000 });
		const messages = page.locator('.exr-msgs');
		await messages.waitFor({ timeout: 60000 });
		const input = page.locator('.exr-inner textarea').first();
		await input.waitFor({ timeout: 30000 });
		const before = await messages.boundingBox(),
			fieldBefore = await input.boundingBox();
		await page.getByRole('button', { name: 'Break conversation' }).click();
		await page.locator('.fixture-controls[data-broken="true"]').waitFor({ timeout: 10000 });
		const warning = page.locator('.toast.failure').filter({ hasText: 'Reconnecting.' });
		await warning.waitFor({ timeout: 30000 });
		const during = await messages.boundingBox(),
			fieldDuring = await input.boundingBox();
		assert(
			Math.abs(before.y - during.y) < 1 && Math.abs(before.height - during.height) < 1,
			'reconnect moved transcript'
		);
		assert(Math.abs(fieldBefore.y - fieldDuring.y) < 1, 'reconnect moved composer');
		const indicator = page.getByRole('button', { name: 'Show conversation problem' });
		await warning.getByRole('button', { name: 'Dismiss' }).click();
		await indicator.click();
		await warning.waitFor();
		await warning.getByRole('button', { name: 'Try again' }).click();
		assert(Number(await page.locator('[data-retries]').innerText()) >= 1);
		await indicator.click();
		await warning.waitFor();
		await page.getByRole('button', { name: 'Restore conversation' }).click();
		await warning.waitFor({ state: 'detached' });
		await indicator.waitFor({ state: 'detached' });
		await input.fill('A draft that must survive');
		const inputBefore = await input.boundingBox();
		await input.press('Enter');
		const failure = page
			.locator('.toast.failure')
			.filter({ hasText: 'Could not confirm the send.' });
		await failure.waitFor({ timeout: 10000 });
		assert.equal(await input.inputValue(), 'A draft that must survive');
		const inputAfter = await input.boundingBox();
		assert(Math.abs(inputBefore.y - inputAfter.y) < 1, 'send error moved composer');
		await failure.getByRole('button', { name: 'Dismiss' }).click();
		await indicator.click();
		await failure.waitFor();
		await page.getByRole('button', { name: 'Allow send' }).click();
		await input.press('Enter');
		await failure.waitFor({ state: 'detached' });
		await indicator.waitFor({ state: 'detached' });
		assert.deepEqual(errors, []);
		console.log(
			JSON.stringify({
				width,
				reconnectShift: during.y - before.y,
				sendShift: inputAfter.y - inputBefore.y,
				draftPreserved: true,
				retry: true,
				rediscoverable: true
			})
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
			new Promise((resolve) => server.once('exit', resolve)),
			new Promise((resolve) => setTimeout(resolve, 5000))
		]);
		if (server.exitCode === null) {
			try {
				process.kill(-server.pid, 'SIGKILL');
			} catch {}
		}
	}
	await rm(route, { recursive: true, force: true });
}
