import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
const origin = process.env.RESTLESS_SMOKE_ORIGIN,
	company = process.env.RESTLESS_SMOKE_COMPANY;
assert(origin && new URL(origin).hostname === '127.0.0.1' && company?.endsWith('_test'));
const out = process.env.RESTLESS_VERIFY_OUTPUT ?? '/tmp/restless-ta94';
await mkdir(out, { recursive: true });
const graph = await (
	await fetch(`${origin}/api/companies/${company}/work-graph?context=true`)
).json();
const topic = graph.work.find((w) => w.owner_id === 'studio-design' && w.title.length > 75);
assert(topic, 'Seed the Reno-scale long Work fixture');
const browser = await chromium.launch();
const results = [];
for (const [name, viewport] of [
	['desktop', { width: 1440, height: 900 }],
	['phone', { width: 390, height: 844 }]
]) {
	const context = await browser.newContext({ viewport, reducedMotion: 'reduce' });
	// A prior Work conversation, the same persisted recents the product already reads.
	await context.addInitScript(
		({ company, topic }) =>
			localStorage.setItem(
				`restless:${company}:rail-topics`,
				JSON.stringify([{ actorId: topic.owner_id, workId: topic.id }])
			),
		{ company, topic }
	);
	const page = await context.newPage();
	try {
		await page.goto(`${origin}/${company}/work`);
		await page.getByRole('heading', { name: 'All work', exact: true }).waitFor();
		if (!(await page.locator('.topic-switch').isVisible()))
			await page.getByRole('button', { name: 'Exec', exact: true }).click();
		await page.locator('.topic-switch summary').waitFor();
		assert.equal(await page.locator('.topic-label').count(), 0, 'Default topic has no subtitle');
		await page.locator('.topic-switch summary').click();
		await page.locator('.topic-switch button').filter({ hasText: topic.title }).click();
		await page.locator('.topic-label').waitFor();
		assert.equal(await page.locator('.topic-label').getAttribute('title'), topic.title);
		const layout = await page.locator('.exr-head').evaluate((el) => {
			const r = el.getBoundingClientRect(),
				name = el.querySelector('.exr-name').getBoundingClientRect(),
				topic = el.querySelector('.topic-label');
			return {
				height: r.height,
				nameY: name.y,
				topicY: topic.getBoundingClientRect().y,
				clipped: topic.scrollWidth > topic.clientWidth,
				overflows: [...el.querySelectorAll('button,summary,a')].filter(
					(b) =>
						b.getBoundingClientRect().width &&
						(b.getBoundingClientRect().right > r.right + 1 ||
							b.getBoundingClientRect().left < r.left - 1)
				).length
			};
		});
		assert.equal(layout.height, 44);
		assert(Math.abs(layout.nameY - layout.topicY) < 5);
		assert(layout.clipped);
		assert.equal(layout.overflows, 0);
		await page.screenshot({ path: `${out}/topic-${name}.png` });
		results.push({ name, passed: true, ...layout });
	} catch (error) {
		await page.screenshot({ path: `${out}/topic-${name}-failed.png` });
		results.push({ name, passed: false, error: String(error) });
	} finally {
		await context.close();
	}
}
await browser.close();
await writeFile(`${out}/topic-results.json`, JSON.stringify(results, null, 2));
console.log(JSON.stringify(results));
assert(results.every((r) => r.passed));
