/* What an open, untouched company page costs in requests. Every request a hosted cockpit makes goes
 * through the edge router, which is billed per request: on 3 Nov 2026 a few idle tabs on a busy
 * company made about 45 a minute each and the Workers daily cap took restless.run down. This opens
 * each surface (all side by side), lets it settle, then counts every request it makes while nobody
 * touches it, once visible and once hidden, and fails when a page goes over its budget.
 *
 *   RESTLESS_SMOKE_ORIGIN=http://127.0.0.1:<web port> RESTLESS_SMOKE_COMPANY=<name>_test \
 *     node scripts/verify-idle-requests.mjs
 *
 * With RESTLESS_HOME=~/.restless-dev/<namespace> and HINT_EVERY_MS=4000 the company is busy as
 * well: a change hint every 4 s, as a company at work sends (kinds the scheduler ignores, written
 * straight to the company's change channel; nothing is stored).
 *
 * MINUTES sets the window (default 5) and SETTLE_MS the wait before it (default 4 minutes: past the
 * first minute, when a page still counts as in use, and the first repair read it schedules then).
 * PAGES picks surfaces (inbox,work,library,people), STATES visible and/or hidden, BUDGET=0 only
 * measures. Reads only, apart from the presence heartbeat an open page sends. */
import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const origin = process.env.RESTLESS_SMOKE_ORIGIN;
const company = process.env.RESTLESS_SMOKE_COMPANY;
assert(origin && company?.endsWith('_test'), 'set RESTLESS_SMOKE_ORIGIN and a _test company');
assert(['127.0.0.1', 'localhost'].includes(new URL(origin).hostname), 'local stacks only');
const minutes = Number(process.env.MINUTES ?? 5);
const settleMs = Number(process.env.SETTLE_MS ?? 4 * 60_000);
const enforce = process.env.BUDGET !== '0';
const out = process.env.RESTLESS_VERIFY_OUTPUT ?? '/tmp/restless-idle-requests';
const hintEvery = Number(process.env.HINT_EVERY_MS ?? 0);
/* Requests a minute an untouched page may make. A visible page's live updates arrive over its one
 * change stream and polling is only a slow repair; on a busy company each thing on screen reads
 * about once a minute. A hidden page makes none until it is shown. Before 3 Nov 2026: about 15 a
 * minute on a quiet company, about 230 on a busy one, and 3 while hidden. */
const VISIBLE_BUDGET = Number(process.env.VISIBLE_BUDGET ?? (hintEvery ? 25 : 4));
const HIDDEN_BUDGET = Number(process.env.HIDDEN_BUDGET ?? 0.5);

/* The busy company: one body-free hint at a time on the company's own change channel. */
async function busyCompany() {
	if (!hintEvery) return () => {};
	const home = process.env.RESTLESS_HOME;
	assert(home, 'HINT_EVERY_MS needs RESTLESS_HOME, the dev stack home');
	const repo = join(dirname(fileURLToPath(import.meta.url)), '../..');
	const pg = createRequire(join(repo, 'services/identity/package.json'))('pg');
	const adminUrl = (await readFile(join(home, 'orgintel.toml'), 'utf8')).match(
		/database_url\s*=\s*"([^"]+)"/
	)[1];
	const admin = new pg.Client({ connectionString: adminUrl });
	await admin.connect();
	const { rows } = await admin.query(
		"SELECT datname FROM pg_database WHERE datname LIKE '%\\_' || $1 || '\\_%' ESCAPE '\\'",
		[company]
	);
	await admin.end();
	assert.equal(rows.length, 1, `one database for ${company}`);
	const url = new URL(adminUrl);
	url.pathname = '/' + rows[0].datname;
	const db = new pg.Client({ connectionString: url.href });
	await db.connect();
	const kinds = ['work_progress', 'library_changed', 'room_event', 'work_progress'];
	let sent = 0;
	const timer = setInterval(() => {
		const kind = kinds[sent++ % kinds.length];
		void db
			.query("SELECT pg_notify('restless_orgintel', $1)", [
				JSON.stringify({ company, kind, body: {} })
			])
			.catch(() => {});
	}, hintEvery);
	return async () => {
		clearInterval(timer);
		await db.end();
	};
}

const surfaces = {
	inbox: `/${company}`,
	work: `/${company}/work`,
	library: `/${company}/library`,
	people: `/${company}/people`
};
const pick = (process.env.PAGES ?? Object.keys(surfaces).join(',')).split(',');

/* The development server's own module and HMR traffic is not something a hosted page sends. */
const devOnly = (url) =>
	/^\/(@vite|@fs|@id|node_modules|src|\.svelte-kit)\//.test(url.pathname) ||
	url.pathname === '/' + '@vite/client';

/* One name per endpoint: ids and keys are folded so repeats of the same read group together. */
function endpoint(method, url) {
	const path = url.pathname
		.replace(`/${company}`, '/:company')
		.replace(/\/[0-9a-f]{8}-[0-9a-f-]{27,}/g, '/:id')
		.replace(/\/\d+(?=\/|$)/g, '/:n');
	const query = [...url.searchParams.keys()].sort().join('&');
	return `${method} ${path}${query ? '?' + query : ''}`;
}

/* A page that is hidden the way a background tab is: the document says so and the event fires.
 * Chromium's own timer throttling in a real background tab only lowers what this counts. */
const visibilityControl = () => {
	let hidden = false;
	Object.defineProperty(Document.prototype, 'visibilityState', {
		configurable: true,
		get: () => (hidden ? 'hidden' : 'visible')
	});
	Object.defineProperty(Document.prototype, 'hidden', { configurable: true, get: () => hidden });
	window.__restlessSetHidden = (value) => {
		hidden = value;
		document.dispatchEvent(new Event('visibilitychange'));
		window.dispatchEvent(new Event(value ? 'blur' : 'focus'));
	};
};

async function measure(browser, name, path, state) {
	const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
	await context.addInitScript(visibilityControl);
	const page = await context.newPage();
	const errors = [];
	page.on('pageerror', (error) => errors.push(error.message));
	let counting = false;
	const hits = [];
	const sockets = [];
	page.on('request', (request) => {
		const url = new URL(request.url());
		if (url.origin !== new URL(origin).origin || devOnly(url)) return;
		if (counting) hits.push({ at: Date.now(), name: endpoint(request.method(), url) });
	});
	/* On localhost the change and activity streams ride a socket; hosted they are EventSource, one
	 * request each time one opens. Either way an opening is one request through the edge. */
	page.on('websocket', (socket) => {
		const url = new URL(socket.url());
		if (url.pathname.startsWith('/@vite') || url.searchParams.has('token')) return;
		if (counting) hits.push({ at: Date.now(), name: `STREAM ${endpoint('GET', url).slice(4)}` });
		sockets.push(url.pathname);
	});
	/* A cold development server can reload the first visit while it optimises dependencies; the
	 * page is ready once its company has answered its first Inbox read. */
	for (let attempt = 0; ; attempt++) {
		const ready = page.waitForResponse((response) => /\/attention(\?|$)/.test(response.url()), {
			timeout: 60_000
		});
		await page.goto(origin + path);
		try {
			await ready;
			break;
		} catch (error) {
			if (attempt === 2) throw error;
		}
	}
	await page.waitForTimeout(settleMs);
	if (state === 'hidden') {
		await page.evaluate(() => window.__restlessSetHidden(true));
		await page.waitForTimeout(2_000);
	}
	counting = true;
	const start = Date.now();
	await page.waitForTimeout(minutes * 60_000);
	counting = false;
	let shown = null;
	if (state === 'hidden') {
		/* Coming back must catch up at once: count what the page reads in the next few seconds. */
		const before = hits.length;
		counting = true;
		await page.evaluate(() => window.__restlessSetHidden(false));
		await page.waitForTimeout(4_000);
		counting = false;
		shown = hits.length - before;
		hits.length = before;
	}
	const elapsed = (Date.now() - start) / 60_000;
	await context.close();
	const groups = new Map();
	for (const hit of hits) {
		const list = groups.get(hit.name) ?? [];
		list.push(hit.at);
		groups.set(hit.name, list);
	}
	const endpoints = [...groups]
		.map(([name, times]) => {
			const gaps = times.slice(1).map((at, index) => (at - times[index]) / 1000);
			gaps.sort((a, b) => a - b);
			return {
				name,
				count: times.length,
				perMinute: +(times.length / elapsed).toFixed(2),
				medianGapS: gaps.length ? Math.round(gaps[Math.floor(gaps.length / 2)]) : null,
				atS: times.map((at) => Math.round((at - start) / 1000))
			};
		})
		.sort((a, b) => b.count - a.count);
	return {
		name,
		state,
		minutes: +elapsed.toFixed(2),
		requests: hits.length,
		perMinute: +(hits.length / elapsed).toFixed(2),
		readsOnShow: shown,
		endpoints,
		errors
	};
}

await mkdir(out, { recursive: true });
const browser = await chromium.launch();
const report = [];
const stopHints = await busyCompany();
try {
	/* Pages are measured side by side, each in its own browser context: one page's requests never
	 * reach another's count, and hints from the company reach every page as they would. */
	const runs = pick.flatMap((name) =>
		(process.env.STATES ?? 'visible,hidden')
			.split(',')
			.map((state) => measure(browser, name, surfaces[name], state))
	);
	for (const result of await Promise.all(runs)) {
		report.push(result);
		console.log(
			`${result.name.padEnd(8)} ${result.state.padEnd(7)} ${String(result.perMinute).padStart(6)}/min` +
				(result.readsOnShow === null ? '' : `  (${result.readsOnShow} reads on show)`)
		);
		for (const entry of result.endpoints)
			console.log(
				`    ${String(entry.count).padStart(4)}  ${String(entry.perMinute).padStart(6)}/min  every ~${entry.medianGapS ?? '-'}s  ${entry.name}`
			);
	}
} finally {
	await stopHints();
	await browser.close();
	await writeFile(`${out}/idle-requests.json`, JSON.stringify(report, null, 2));
}
if (enforce) {
	const over = report.filter(
		(result) => result.perMinute > (result.state === 'hidden' ? HIDDEN_BUDGET : VISIBLE_BUDGET)
	);
	assert.deepEqual(
		over.map((result) => `${result.name} ${result.state} ${result.perMinute}/min`),
		[],
		'idle pages over their request budget'
	);
	const blind = report.filter((result) => result.state === 'hidden' && !result.readsOnShow);
	assert.deepEqual(
		blind.map((result) => result.name),
		[],
		'a page shown again must catch up at once'
	);
}
