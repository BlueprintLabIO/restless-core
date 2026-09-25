// Run against the local cockpit with RESTLESS_TEST_SOURCE_COMPANY set.
// All writes are intercepted; real company data is read only. The SSE server
// and browser are closed in finally, including on assertion failures.
import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import http from 'node:http';
const real = process.env.RESTLESS_TEST_SOURCE_COMPANY,
	test = 'markdown_renderer_test',
	base = process.env.RESTLESS_TEST_BASE_URL ?? 'http://127.0.0.1:8788';
assert(
	real,
	'Set RESTLESS_TEST_SOURCE_COMPANY to an existing company. Only GET requests are proxied; fixture state stays in this browser.'
);
let streams = [],
	done = false;
const srv = http.createServer((req, res) => {
	res.writeHead(200, {
		'Content-Type': 'text/event-stream',
		'Access-Control-Allow-Origin': '*',
		'Cache-Control': 'no-cache'
	});
	res.write(': connected\n\n');
	streams.push(res);
	req.on('close', () => (streams = streams.filter((s) => s !== res)));
});
await new Promise((r) => srv.listen(0, '127.0.0.1', r));
let b;
try {
	b = await chromium.launch({ headless: true, executablePath: process.env.RESTLESS_TEST_CHROMIUM });
	const p = await b.newPage({ viewport: { width: 1440, height: 1000 } });
	// The rail is under test; open it as the owner would, whatever the source company's setup.
	await p.addInitScript(() => {
		localStorage.setItem(`restless:exec-rail:${location.pathname.split('/')[1]}`, 'open');
	});
	let errors = [];
	p.on('pageerror', (e) => errors.push(e.message));
	const now = new Date().toISOString(),
		message = {
			id: 100,
			from_actor: 'owner',
			body: 'Show the streaming reply',
			created_at: now,
			attachments: []
		};
	let finalReply = '';
	await p.route('**/api/**', async (route) => {
		const req = route.request(),
			url = req.url();
		if (req.method() !== 'GET') return route.fulfill({ json: {} });
		if (url.includes('/activity?'))
			return route.fulfill({
				status: 307,
				headers: { Location: `http://127.0.0.1:${srv.address().port}/events` }
			});
		if (url.includes('/actors/exec/conversation'))
			return route.fulfill({
				json: {
					actor: { id: 'exec', display: 'Exec', kind: 'exec', role: 'exec' },
					focus: { after_message_id: 0 },
					messages: done
						? [message, { ...message, id: 101, from_actor: 'exec', body: finalReply }]
						: [message]
				}
			});
		if (url.includes(test)) {
			const response = await route.fetch({ url: url.replaceAll(test, real) });
			return route.fulfill({ response });
		}
		return route.continue();
	});
	await p.goto(`${base}/${test}`);
	await p.locator('.turn-summary').waitFor();
	const deadline = Date.now() + 10000;
	while (!streams.length && Date.now() < deadline) await p.waitForTimeout(100);
	assert(streams.length, 'EventSource did not connect');
	let seq = 0;
	const send = (reply, phase = 'responding', updatedAt = now) => {
		const state = {
			streamId: 'fixture',
			sequence: ++seq,
			company: test,
			actorId: 'exec',
			triggerMessageId: 100,
			workId: null,
			attemptId: null,
			phase,
			reply,
			activity: [],
			startedAt: now,
			updatedAt,
			contextUsage: { used: 120000, size: 200000 },
			generatedOutputTokens: 12000
		};
		for (const s of streams) s.write(`event: activity\ndata: ${JSON.stringify(state)}\n\n`);
	};
	send('First streamed words');
	await p
		.getByLabel('Exec streaming reply')
		.getByText('First streamed words', { exact: true })
		.waitFor();
	assert.equal(await p.locator('.turn-summary').getAttribute('aria-expanded'), 'false');
	send('First streamed words, followed by the next chunk.');
	await p
		.getByLabel('Exec streaming reply')
		.getByText('First streamed words, followed by the next chunk.', { exact: true })
		.waitFor();
	const intro = 'Stable **introduction**.\n\n';
	send(intro + '| Provider | Status |\n| --- | --- |\n| Codex |');
	await p.getByLabel('Exec streaming reply').locator('table').waitFor();
	await p.evaluate(() => {
		window.stableParagraph = document.querySelector('.visible-reply [data-streamdown-paragraph]');
		window.stableTable = document.querySelector('.visible-reply table');
	});
	send(intro + '| Provider | Status |\n| --- | --- |\n| Codex | Ready |\n| Claude | Connecting');
	await p.getByLabel('Exec streaming reply').getByText('Connecting', { exact: true }).waitFor();
	assert(
		await p.evaluate(
			() =>
				window.stableParagraph ===
					document.querySelector('.visible-reply [data-streamdown-paragraph]') &&
				window.stableTable === document.querySelector('.visible-reply table')
		),
		'streaming replaced stable paragraph/table'
	);
	const prefix = intro + '| Provider | Status |\n| --- | --- |\n| Codex | Ready |\n\n';
	send(prefix + '```typescript\nconst greeting = "hello";');
	await p.locator('.visible-reply [data-streamdown-code]').waitFor();
	assert.equal(
		await p.locator('.visible-reply [data-streamdown-code]').getAttribute('data-incomplete'),
		'true'
	);
	await p.evaluate(
		() => (window.stableCode = document.querySelector('.visible-reply [data-streamdown-code]'))
	);
	send(
		prefix +
			'```typescript\nconst greeting = "hello";\nconsole.log(greeting);\n```\n\nNext paragraph'
	);
	await p.getByLabel('Exec streaming reply').getByText('Next paragraph', { exact: true }).waitFor();
	assert(
		await p.evaluate(
			() =>
				window.stableCode === document.querySelector('.visible-reply [data-streamdown-code]') &&
				window.stableParagraph.isConnected
		)
	);
	assert(
		(await p.locator('.visible-reply .th-token[style]').count()) > 0,
		'syntax highlighting missing'
	);
	await p.context().grantPermissions(['clipboard-read', 'clipboard-write']);
	await p.locator('.visible-reply').getByRole('button', { name: 'Copy code', exact: true }).click();
	assert.equal(
		await p.evaluate(() => navigator.clipboard.readText()),
		'const greeting = "hello";\nconsole.log(greeting);'
	);
	const hostile =
		'\n\n<img src=x onerror="window.markdownExecuted=true">\n\n<script>window.markdownExecuted=true</script>\n\n[unsafe](javascript:alert(1)) ![unsafe](data:image/svg+xml;base64,PHN2Zz4=)\n\n[safe](https://example.com) [email](mailto:test@example.com) [anchor](#test)';
	send(prefix + hostile);
	await p.locator('.visible-reply a[href="https://example.com/"]').waitFor();
	assert.equal(
		await p
			.locator(
				'.visible-reply script,.visible-reply img[onerror],.visible-reply a[href^="javascript:"],.visible-reply img[src^="data:"]'
			)
			.count(),
		0
	);
	assert(!(await p.evaluate(() => window.markdownExecuted)));
	assert((await p.locator('.visible-reply a[href^="mailto:"]').count()) === 1);
	assert((await p.locator('.visible-reply a[href$="#test"]').count()) === 1);
	const long =
		Array.from({ length: 150 }, (_, i) => `Paragraph ${i}: stable content.`).join('\n\n') + '\n\n';
	send(long + 'Growing tail');
	await p
		.locator('.visible-reply')
		.getByText('Paragraph 0: stable content.', { exact: true })
		.waitFor();
	await p.evaluate(
		() => (window.longFirst = document.querySelector('.visible-reply [data-streamdown-paragraph]'))
	);
	for (let i = 0; i < 12; i++) {
		send(long + 'Growing tail ' + i);
		await p
			.locator('.visible-reply')
			.getByText('Growing tail ' + i, { exact: true })
			.waitFor();
		assert(await p.evaluate(() => window.longFirst.isConnected));
	}
	console.log(
		'PASS: partial table and open code fence; stable DOM across incremental updates and long replies; code highlighting/copy; raw HTML and unsafe URLs blocked; normal links preserved.'
	);
	// Exercise real layout/events while chunks arrive in both scrolling surfaces.
	async function checkFollow(selector, label) {
		await p.waitForFunction((selector) => {
			const e = document.querySelector(selector);
			return (
				e &&
				e.scrollHeight > e.clientHeight + 200 &&
				e.scrollHeight - e.clientHeight - e.scrollTop < 3
			);
		}, selector);
		const paused = await p.locator(selector).evaluate((e) => {
			e.dispatchEvent(new WheelEvent('wheel', { deltaY: -250 }));
			e.scrollTop -= 250;
			return e.scrollTop;
		});
		await p.waitForTimeout(100);
		send(long + `Paused ${label}`);
		await p.getByText(`Paused ${label}`, { exact: true }).waitFor();
		await p.waitForTimeout(150);
		assert(
			Math.abs((await p.locator(selector).evaluate((e) => e.scrollTop)) - paused) < 3,
			`${label} pulled the reader back down during streaming`
		);
		await p.locator(selector).evaluate((e) => {
			e.scrollTop = e.scrollHeight;
		});
		await p.waitForTimeout(100);
		send(long + `Resumed ${label}\n\nNew paragraph after returning to the end.`);
		await p.getByText(`Resumed ${label}`, { exact: true }).waitFor();
		await p.waitForFunction((selector) => {
			const e = document.querySelector(selector);
			return e.scrollHeight - e.clientHeight - e.scrollTop < 3;
		}, selector);
	}
	await checkFollow('.exr-msgs', 'transcript');
	await p.locator('.turn-summary').click();
	await checkFollow('.turn-body', 'activity');
	await p.locator('.turn-summary').click();
	console.log(
		'PASS: streaming follows the end, pauses for reading history, and resumes in transcript and expanded activity.'
	);
	const seam = p.locator('.bridge-body > .pane-resizer');
	await seam.focus();
	await p.keyboard.press('Home');
	await p.waitForTimeout(150);
	const before = Number(await seam.getAttribute('aria-valuenow'));
	const r = await seam.boundingBox();
	await p.mouse.move(r.x + 5, r.y + 100);
	await p.mouse.down();
	await p.mouse.move(r.x - 95, r.y + 100, { steps: 10 });
	await p.mouse.up();
	assert.equal(Number(await seam.getAttribute('aria-valuenow')), before + 100);
	await seam.focus();
	await p.keyboard.press('Home');
	finalReply =
		'First streamed words\n\n```text\n' +
		'long_value_'.repeat(60) +
		'\n```\n\n| Column | Another |\n| --- | --- |\n| ' +
		'long'.repeat(80) +
		' | value |';
	send(finalReply);
	await p.waitForTimeout(250);
	const overflow = await p.evaluate(() =>
		[...document.querySelectorAll('.exr-msgs, .exr-msgs *')]
			.filter((e) => e.clientWidth && e.scrollWidth > e.clientWidth + 2)
			.map((e) => ({ tag: e.tagName, cls: e.className, w: e.clientWidth, scroll: e.scrollWidth }))
	);
	console.log('overflow', overflow);
	assert.deepEqual(overflow, []);
	await p.locator('.turn-summary').click();
	assert.equal(await p.locator('.turn-summary').getAttribute('aria-expanded'), 'true');
	assert.equal(await p.locator('.visible-reply').count(), 0);
	await p.locator('.turn-summary').click();
	// A terminal frame remains correct even before the durable reply catches up.
	const finishedAt = new Date(new Date(now).getTime() + 7000).toISOString();
	send(finalReply, 'failed', finishedAt);
	await p.waitForTimeout(250);
	assert.equal(await p.locator('.turn-status').innerText(), 'Reply interrupted');
	assert.equal(await p.locator('.turn-summary time').innerText(), '7s');
	await p.waitForTimeout(2200);
	assert.equal(await p.locator('.turn-summary time').innerText(), '7s');
	send(finalReply, 'complete', finishedAt);
	await p.waitForTimeout(250);
	assert.equal(await p.locator('.turn-status').innerText(), 'Exec finished replying');
	assert.equal(await p.locator('.pixel-glimmer, .turn-status.shimmer').count(), 0);
	// Connection closure must not replace a completed state with "Reconnecting".
	for (const stream of [...streams]) stream.end();
	await p.waitForTimeout(2200);
	assert.equal(await p.locator('.turn-status').innerText(), 'Exec finished replying');
	assert.equal(await p.locator('.turn-summary time').innerText(), '7s');
	done = true;
	const reconnected = Date.now() + 10000;
	while (!streams.length && Date.now() < reconnected) await p.waitForTimeout(100);
	assert(streams.length, 'EventSource did not reconnect after terminal frame');
	send(finalReply, 'complete', finishedAt);

	await p.waitForTimeout(1000);
	assert.equal(await p.locator('.visible-reply').count(), 0);
	assert.equal(
		await p
			.locator('.exr-msgs .conversation-message .md')
			.filter({ hasText: 'First streamed words' })
			.count(),
		1
	);
	await p.screenshot({ path: '../../work/browser/exec-completed-desktop.png' });
	await p.setViewportSize({ width: 390, height: 844 });
	await p.waitForTimeout(200);
	assert(await p.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
	assert.equal(await p.getByRole('separator').count(), 0);
	await p.screenshot({ path: '../../work/browser/exec-completed-mobile.png' });
	assert.deepEqual(errors, []);
	console.log(
		'PASS: terminal durations frozen for success/failure, completion survives disconnect; real EventSource partial chunks visible before completion; collapsed tools; one durable final reply; long code/table fits narrow rail; pointer resize; mobile fits.'
	);
} finally {
	for (const c of b?.contexts() ?? [])
		for (const p of c.pages()) await p.unrouteAll({ behavior: 'ignoreErrors' });
	await b?.close();
	for (const s of streams) s.end();
	await new Promise((r) => srv.close(r));
}
