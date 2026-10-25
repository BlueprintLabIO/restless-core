// Verify the actual Markdown dependency's lazy KaTeX import, CSS/fonts and streaming updates.
// Every server/browser/cache belongs to this check; no company or API is used.
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { chromium } from 'playwright';
import { createServer } from 'vite';

const root = resolve(import.meta.dirname, '..');
const cache = await mkdtemp(join(tmpdir(), 'restless-math-cache-'));
let server, browser;
const results = [];
try {
	server = await createServer({
		root,
		configFile: false,
		appType: 'mpa',
		publicDir: false,
		cacheDir: cache,
		plugins: [svelte({ configFile: false })],
		server: { host: '127.0.0.1', port: 0, strictPort: true }
	});
	await server.listen();
	const origin = `http://127.0.0.1:${server.httpServer.address().port}`;
	browser = await chromium.launch({
		headless: true,
		executablePath: process.env.RESTLESS_TEST_CHROMIUM
	});
	for (const width of [1440, 390]) {
		const context = await browser.newContext({
			viewport: { width, height: 900 },
			reducedMotion: 'reduce'
		});
		let page;
		const errors = [],
			fonts = [];
		try {
			page = await context.newPage();
			page.on('pageerror', (error) => errors.push(error.message));
			page.on('requestfailed', (request) =>
				errors.push('Failed asset: ' + new URL(request.url()).pathname)
			);
			page.on('response', (response) => {
				if (/\.woff2?(?:\?|$)/.test(response.url()))
					fonts.push({ path: new URL(response.url()).pathname, status: response.status() });
			});
			await page.goto(origin + '/scripts/fixtures/math-compatibility/index.html');
			await page.locator('.katex').nth(2).waitFor();
			assert.equal(await page.locator('.katex-display').count(), 2);
			await page.getByRole('button', { name: 'Complete stream' }).click();
			await page.locator('.katex').nth(3).waitFor();
			await page
				.locator('[data-streamdown-inline-math]')
				.last()
				.getByText('2', { exact: true })
				.waitFor();
			await page.evaluate(() => document.fonts.ready);
			assert.equal(await page.locator('.katex-error').count(), 0);
			assert(
				fonts.length > 0 && fonts.every(({ status }) => status === 200),
				'KaTeX fonts must load from the tested graph'
			);
			assert(
				await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth),
				'Math must fit the viewport'
			);
			assert.deepEqual(errors, []);
			const result = {
				width,
				equations: await page.locator('.katex').count(),
				fonts: fonts.length
			};
			results.push(result);
			if (process.env.RESTLESS_VERIFY_OUTPUT) {
				await mkdir(process.env.RESTLESS_VERIFY_OUTPUT, { recursive: true });
				await page.screenshot({
					path: join(process.env.RESTLESS_VERIFY_OUTPUT, `math-${width}.png`),
					fullPage: true
				});
			}
		} catch (error) {
			console.error(
				'Math rendering failed:',
				JSON.stringify({
					width,
					errors,
					text: await page
						?.locator('body')
						.innerText()
						.catch(() => '')
				})
			);
			if (page && process.env.RESTLESS_VERIFY_OUTPUT) {
				await mkdir(process.env.RESTLESS_VERIFY_OUTPUT, { recursive: true });
				await page.screenshot({
					path: join(process.env.RESTLESS_VERIFY_OUTPUT, `math-${width}-failed.png`),
					fullPage: true
				});
			}
			throw error;
		} finally {
			await context.close();
		}
	}
	if (process.env.RESTLESS_VERIFY_OUTPUT)
		await writeFile(
			join(process.env.RESTLESS_VERIFY_OUTPUT, 'math.json'),
			JSON.stringify(results, null, 2)
		);
	console.log('Math compatibility passed:', JSON.stringify(results));
} finally {
	await browser?.close();
	await server?.close();
	await rm(cache, { recursive: true, force: true });
}
