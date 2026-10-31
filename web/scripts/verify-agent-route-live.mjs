import { chromium } from 'playwright';
import assert from 'node:assert/strict';

const origin = process.env.RESTLESS_SMOKE_ORIGIN;
const company = process.env.RESTLESS_SMOKE_COMPANY;
assert(origin && new URL(origin).hostname === '127.0.0.1' && company?.endsWith('_test'));
const browser = await chromium.launch();
try {
	for (const width of [1440, 390]) {
		const page = await browser.newPage({
			viewport: { width, height: 900 },
			isMobile: width < 600,
			hasTouch: width < 600
		});
		await page.route('**/*', (route) =>
			new URL(route.request().url()).hostname === '127.0.0.1' ? route.continue() : route.abort()
		);
		await page.goto(`${origin}/${company}/people?person=site-writer`);
		const selected = page.locator('.tb-exec');
		await selected.waitFor({ state: 'visible', timeout: 30000 });
		await page.locator('.tb-exec.live:not(.unavailable)').waitFor({ timeout: 30000 });
		await page.locator('.chip summary').waitFor({ timeout: 30000 });
		assert.equal(await page.getByText("Exec can't reply").count(), 0);
		if (width === 1440) {
			await page.locator('.chip summary').click();
			await page.getByRole('searchbox', { name: 'Find a model' }).fill('gpt-4o-mini');
			const choice = page.getByRole('option', { name: 'GPT-4o mini' });
			await choice.waitFor({ timeout: 30000 });
			const response = page.waitForResponse(
				(r) => r.url().endsWith('/intelligence/site-writer') && r.request().method() === 'PUT',
				{ timeout: 30000 }
			);
			await choice.click();
			const saved = await response;
			assert.equal(saved.status(), 200, `model save failed: ${await saved.text()}`);
			const view = await (
				await page.request.get(`${origin}/api/companies/${company}/intelligence`)
			).json();
			const agent = view.agents.find((a) => a.id === 'site-writer');
			assert.deepEqual(agent.assignment, { connection: 'direct:openai', model: 'gpt-4o-mini' });
			assert.equal(agent.effective_model, 'openai/gpt-4o-mini');
		}
		await page.goto(`${origin}/${company}/people?person=exec`);
		await page.getByText("Exec can't reply", { exact: false }).waitFor({ timeout: 30000 });
		await page.locator('.tb-exec.unavailable:not(.live)').waitFor({ timeout: 30000 });
		await page.getByText('Model unavailable', { exact: true }).waitFor({ timeout: 30000 });
		console.log(`PASS ${width}px lead is ready and Exec is unavailable`);
		await page.close();
	}
} finally {
	await browser.close();
}
