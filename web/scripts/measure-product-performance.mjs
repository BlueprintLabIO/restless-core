/* Same populated company, same backend, same browser: compare frontend work at two source refs.
 * Development-server timings are diagnostic, not production latency promises. Reads only. */
import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
const company = process.env.RESTLESS_SMOKE_COMPANY;
const baseline = process.env.RESTLESS_BASELINE_ORIGIN,
	candidate = process.env.RESTLESS_SMOKE_ORIGIN;
const out = process.env.RESTLESS_VERIFY_OUTPUT;
assert(company?.endsWith('_test') && baseline && candidate && out);
for (const origin of [baseline, candidate]) assert.equal(new URL(origin).hostname, '127.0.0.1');
await mkdir(out, { recursive: true });
const browser = await chromium.launch();
const report = [];
try {
	for (const [label, origin] of [
		['baseline', baseline],
		['candidate', candidate]
	]) {
		const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
		for (let run = 0; run < 3; run++) {
			const page = await context.newPage();
			const errors = [];
			page.on('pageerror', (error) => errors.push(error.message));
			const cdp = await context.newCDPSession(page);
			await cdp.send('Performance.enable');
			const start = Date.now();
			await page.goto(origin + '/' + company + '/library');
			await page
				.getByRole('list', { name: 'Files', exact: true })
				.getByRole('listitem')
				.nth(29)
				.waitFor({ timeout: 30000 });
			const readyMs = Date.now() - start;
			const counts = await page.evaluate(() => ({
				files: document.querySelectorAll('[role=list][aria-label=Files] [role=listitem]').length,
				nodes: document.querySelectorAll('*').length,
				menus: document.querySelectorAll('details.action-menu').length
			}));
			const listeners = await cdp.send('Runtime.evaluate', {
				expression: `JSON.stringify(Object.fromEntries([['window',window],['document',document]].map(([name,target])=>[name,Object.fromEntries(Object.entries(getEventListeners(target)).map(([event,list])=>[event,list.length]))])))`,
				includeCommandLineAPI: true,
				returnByValue: true
			});
			const before = Object.fromEntries(
				(await cdp.send('Performance.getMetrics')).metrics.map((x) => [x.name, x.value])
			);
			await page.evaluate(async () => {
				const list = document.querySelector('[role=list][aria-label=Files]');
				let scroller = list;
				while (
					scroller &&
					!(
						scroller.scrollHeight > scroller.clientHeight &&
						['auto', 'scroll'].includes(getComputedStyle(scroller).overflowY)
					)
				)
					scroller = scroller.parentElement;
				if (!scroller) throw new Error('No library scroller');
				for (let step = 0; step < 60; step++) {
					scroller.scrollTop = step % 2 ? 0 : scroller.scrollHeight;
					await new Promise((resolve) => requestAnimationFrame(resolve));
				}
			});
			const after = Object.fromEntries(
				(await cdp.send('Performance.getMetrics')).metrics.map((x) => [x.name, x.value])
			);
			report.push({
				label,
				run,
				readyMs,
				...counts,
				listeners: JSON.parse(listeners.result.value),
				scroll: {
					scriptMs: (after.ScriptDuration - before.ScriptDuration) * 1000,
					layoutMs: (after.LayoutDuration - before.LayoutDuration) * 1000,
					layoutCount: after.LayoutCount - before.LayoutCount,
					recalcCount: after.RecalcStyleCount - before.RecalcStyleCount
				},
				errors
			});
			assert.deepEqual(errors, []);
			await page.close();
		}
		await context.close();
	}
} finally {
	await browser.close();
	await writeFile(out + '/performance.json', JSON.stringify(report, null, 2));
}
console.log(JSON.stringify(report, null, 2));
