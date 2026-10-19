// Sprint 82: nothing on screen moves unless the person moved it, or a newer fact did.
//
// Runs against a local dev stack and one `_test` company:
//   RESTLESS_TEST_BASE_URL=http://127.0.0.1:22298 RESTLESS_TEST_COMPANY=s82_test \
//     node scripts/verify-steady.mjs [journey ...]
//
// Journeys:
//   entry    entering the cockpit reveals once; nothing swaps after the reveal
//   answer   tapping a suggested answer while a stale read lands: the answer never vanishes and the
//            question never reopens (the 14 October report)
//   hints    a message hint refetches conversations, never settings, the principal or intelligence
//   charter  Exec's charter draft shows on the real charter, in the Inbox, and publishes once
//
// The answer journey intercepts its conversation and the model route so it needs no model and
// writes nothing. The charter journey writes only to the `_test` company, through the dev CLI
// (RESTLESS_TEST_CLI, a wrapper that targets this stack).
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { chromium } from 'playwright';

const base = process.env.RESTLESS_TEST_BASE_URL ?? 'http://127.0.0.1:22298';
const company = process.env.RESTLESS_TEST_COMPANY ?? 's82_test';
const cli = process.env.RESTLESS_TEST_CLI;
assert(company.endsWith('_test'), 'only a _test company is written to');
const wanted = process.argv.slice(2);
const run = (name) => wanted.length === 0 || wanted.includes(name);

const browser = await chromium.launch(
	process.env.PW_CHROMIUM ? { executablePath: process.env.PW_CHROMIUM } : {}
);
const report = [];

async function journey(name, body) {
	if (!run(name)) return;
	const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
	await context.addInitScript(() => {
		window.__restlessStrict = true;
	});
	const page = await context.newPage();
	const errors = [];
	page.on('pageerror', (error) => errors.push(String(error)));
	page.on('console', (message) => {
		if (message.type() === 'error' && message.text().startsWith('Steady:'))
			errors.push(message.text());
	});
	const started = Date.now();
	try {
		const detail = await body(page, context);
		const violations = await page.evaluate(() => window.__restlessViolations ?? []).catch(() => []);
		assert.deepEqual(violations, [], `${name}: tripwire`);
		assert.deepEqual(errors, [], `${name}: page errors`);
		report.push({ journey: name, ok: true, ms: Date.now() - started, ...detail });
	} catch (error) {
		report.push({ journey: name, ok: false, error: String(error?.message ?? error) });
	} finally {
		await context.close();
	}
}

/* Watch the visible text of the cockpit's regions: every distinct state counts as a swap. */
async function watchRegions(page, ms) {
	return page.evaluate(async (ms) => {
		const text = () =>
			[...document.querySelectorAll('main, aside, nav, header')]
				.map((node) => node.innerText.replace(/\s+/g, ' ').trim())
				.join(' || ');
		const states = [text()];
		const until = performance.now() + ms;
		while (performance.now() < until) {
			await new Promise((resolve) => requestAnimationFrame(resolve));
			const now = text();
			if (now !== states.at(-1)) states.push(now);
		}
		return states;
	}, ms);
}

await journey('entry', async (page) => {
	const api = [];
	page.on('request', (request) => {
		const url = new URL(request.url());
		if (url.pathname.startsWith('/api/companies/')) api.push(url.pathname);
	});
	// A dev server compiles each module on its first request; production serves a built
	// bundle. One warm-up visit keeps that compile out of what is measured.
	await page.goto(`${base}/${company}`);
	await page.waitForLoadState('networkidle', { timeout: 60_000 }).catch(() => {});
	await page.goto('about:blank');
	api.length = 0;
	await page.goto(`${base}/${company}`);
	// The page is hidden until it settles, then revealed once.
	await page.waitForSelector('.company-browser-link-capture:not(.first-paint-pending)', {
		timeout: 8_000
	});
	const revealedAt = api.length;
	const states = await watchRegions(page, 4_000);
	// One swap is allowed for the office canvas's own late art; anything beyond is reshuffling.
	assert(
		states.length <= 2,
		`entry: ${states.length} visible states after reveal:\n${states.join('\n---\n')}`
	);
	const avatar = await page.locator('.face.me').first().innerText();
	assert.match(avatar, /^\S$/, 'entry: the avatar shows the real initial once revealed');
	return {
		statesAfterReveal: states.length,
		readsBeforeReveal: revealedAt,
		states: states.map((state) => state.slice(0, 400)),
		reads: [...new Set(api)]
	};
});

await journey('hints', async (page) => {
	// A real hint: a message to the owner from Exec, written through the dev CLI, which the
	// server's change stream reports as `message`.
	assert(cli, 'Set RESTLESS_TEST_CLI to the dev CLI wrapper for the hints journey');
	await page.goto(`${base}/${company}`);
	await page.waitForSelector('.company-browser-link-capture:not(.first-paint-pending)');
	await page.waitForTimeout(4_000);
	const reads = [];
	page.on('request', (request) => {
		const url = new URL(request.url());
		if (url.pathname.startsWith(`/api/companies/${company}/`))
			reads.push({ path: url.pathname.split(`/${company}/`)[1], at: Date.now() });
	});
	const hintAt = Date.now();
	execFileSync(
		cli,
		['message', '-c', company, '--from', 'exec', '--to', 'owner', 'Steady hint check'],
		{
			stdio: 'ignore'
		}
	);
	await page.waitForTimeout(3_000);
	const paths = [...new Set(reads.map((read) => read.path))];
	assert(
		paths.some((path) => path.startsWith('actors/exec/conversation')),
		`hints: the conversation did not refetch: ${paths}`
	);
	const settings = paths.filter((path) =>
		/^(principal|intelligence|company\/identity|company$|connections|browser)/.test(path ?? '')
	);
	assert.deepEqual(settings, [], `hints: a message hint reloaded settings: ${settings.join(', ')}`);
	return { readsAfterHint: paths, firstReadMs: reads[0] ? reads[0].at - hintAt : null };
});

await journey('answer', async (page, context) => {
	const now = Date.now();
	const iso = (offset) => new Date(now + offset).toISOString();
	const question = {
		id: 9001,
		from_actor: 'exec',
		to_actor: null,
		body: 'Should the first site lead with the before and after photos?',
		attachments: [],
		details: null,
		intent: {
			kind: 'conversation',
			summary: 'Lead with photos?',
			ownerNeed: 'Should the first site lead with the before and after photos?',
			ownerReplies: ['Yes, lead with photos', 'No, lead with the offer']
		},
		context_path: null,
		created_at: iso(-60_000),
		read_at: null
	};
	const viewer = (await (await fetch(`${base}/api/companies/${company}/principal`)).json())
		.actor_id;
	let answer = null;
	let postHeld = false;
	let sentBody = '';
	await context.route(`**/api/companies/${company}/actors/exec/conversation**`, async (route) => {
		const request = route.request();
		if (request.method() === 'POST') {
			postHeld = true;
			// While the send is in flight, the server's commit fires a hint and a read starts.
			await page.evaluate(() => {
				void window.__restlessQueryClient?.refetchQueries({ queryKey: ['conversation'] });
			});
			await new Promise((resolve) => setTimeout(resolve, 1_500));
			// The send is JSON, or multipart when it carries files: take its body as written.
			const raw = request.postData() ?? '';
			let body = '';
			try {
				body = JSON.parse(raw).body ?? '';
			} catch {
				body = /name="body"\r\n\r\n([\s\S]*?)\r\n--/.exec(raw)?.[1] ?? '';
			}
			sentBody = body;
			answer = {
				...question,
				id: 9002,
				from_actor: viewer,
				to_actor: 'exec',
				body,
				intent: null,
				created_at: iso(0)
			};
			postHeld = false;
			return route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({
					message_id: 9002,
					created: true,
					interrupted: false,
					context_attached: false,
					context_omitted: false,
					focus: null
				})
			});
		}
		// A read that started while the send was in flight answers with the old transcript,
		// slightly changed so it is new data rather than a structurally shared copy.
		const stale = postHeld || !answer;
		const messages = stale
			? [{ ...question, read_at: postHeld ? iso(-1) : null }]
			: [question, answer];
		return route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				actor: { id: 'exec', display: 'Exec', kind: 'exec', role: 'Exec' },
				focus: null,
				messages
			})
		});
	});
	await context.route(`**/api/companies/${company}/actors/exec/activity**`, (route) =>
		route.fulfill({ status: 200, contentType: 'text/event-stream', body: ': idle\n\n' })
	);
	// Exec's route is ready, as it is on a company with a model.
	await context.route(`**/api/companies/${company}/intelligence`, async (route) => {
		const response = await route.fetch();
		const view = await response.json();
		const connection = {
			...(view.connections?.[0] ?? {}),
			id: 'steady',
			kind: 'direct',
			provider: 'steady',
			loaded: true
		};
		view.has_connections = true;
		view.connections = [connection, ...(view.connections ?? [])];
		view.agents = (view.agents ?? []).map((agent) => ({
			...agent,
			assignment: { ...(agent.assignment ?? {}), connection: 'steady', model: 'steady/model' }
		}));
		return route.fulfill({ response, json: view });
	});
	await context.route(`**/api/companies`, async (route) => {
		const response = await route.fetch();
		const rows = await response.json();
		return route.fulfill({
			response,
			json: Array.isArray(rows) ? rows.map((row) => ({ ...row, unstartable_reason: null })) : rows
		});
	});

	await page.goto(`${base}/${company}/people?person=exec`);
	const asking = page.getByRole('group', { name: 'Exec is asking' });
	await asking.waitFor({ timeout: 10_000 });
	await asking.getByRole('button', { name: 'Yes, lead with photos' }).click();
	// From the tap on: the rail's states, each with when it appeared. The tap shows "Sending…"
	// with Undo for two seconds, then sends; once the answer is on screen it must stay, and the
	// question must never offer its buttons again.
	const timeline = await page.evaluate(async () => {
		const seen = [];
		const t0 = performance.now();
		const until = t0 + 9_000;
		while (performance.now() < until) {
			await new Promise((resolve) => requestAnimationFrame(resolve));
			const rail = document.getElementById('bridge-exrail');
			const pending = !!rail?.querySelector('.answer-pending');
			const buttons = !!rail?.querySelector('[aria-label="Exec is asking"] .open-replies');
			const answered = [...(rail?.querySelectorAll('[id^="rail-message-"]') ?? [])].some((node) =>
				node.textContent?.includes('Yes, lead with photos')
			);
			const state = pending
				? 'undo-window'
				: `${buttons ? 'buttons' : 'closed'}:${answered ? 'answer' : 'no-answer'}`;
			if (state !== seen.at(-1)?.state)
				seen.push({ state, at: Math.round(performance.now() - t0) });
		}
		return seen;
	});
	const states = timeline.map((entry) => entry.state);
	const firstAnswer = states.indexOf('closed:answer');
	assert(firstAnswer >= 0, `answer: the answer never showed: ${JSON.stringify(timeline)}`);
	assert(
		states.slice(firstAnswer).every((state) => state === 'closed:answer'),
		`answer: after it showed, the answer vanished or the question reopened: ${JSON.stringify(timeline)}`
	);
	const reopened = timeline;
	const sentAt = Date.now();
	return { afterSend: reopened, sentBody, sentAfterMs: sentAt - now };
});

await journey('charter', async (page) => {
	assert(cli, 'Set RESTLESS_TEST_CLI to the dev CLI wrapper for the charter journey');
	const restless = (...args) =>
		execFileSync(cli, [args[0], '-c', company, ...args.slice(1)], {
			encoding: 'utf8',
			stdio: ['ignore', 'pipe', 'pipe']
		});
	const draftFile = `/tmp/${company}-charter-draft.md`;
	const marker = `Steady charter ${Date.now()}`;
	await import('node:fs').then((fs) =>
		fs.writeFileSync(draftFile, `# ${company}\n\n${marker}: we renovate small business websites.\n`)
	);
	const proposed = JSON.parse(
		restless(
			'charter',
			'propose',
			'--requester',
			'owner',
			'--file',
			draftFile,
			'--reason',
			'You confirmed the read-back.'
		)
	);
	assert.match(
		JSON.stringify(proposed),
		/Drafted in Company › Charter/,
		'charter: propose answers with where it is'
	);

	// The Inbox item is the charter itself, with the draft on it.
	await page.goto(`${base}/${company}`);
	await page.waitForSelector('.company-browser-link-capture:not(.first-paint-pending)');
	await page
		.getByText(/Publish the (company )?charter/)
		.first()
		.click();
	const draft = page.getByRole('region', { name: "Exec's draft of the charter" });
	await draft.waitFor({ timeout: 8_000 });
	assert(await draft.getByText(marker).isVisible(), 'charter: the Inbox shows the draft text');
	await draft.getByRole('button', { name: 'Publish' }).click();
	await page.getByText('Charter', { exact: true }).first().waitFor();
	await page.waitForFunction(
		() => !document.querySelector('[aria-label="Exec\'s draft of the charter"]'),
		null,
		{
			timeout: 8_000
		}
	);

	// The real charter now says it, as read back by Exec.
	const shown = JSON.parse(restless('charter', 'show'));
	assert(shown.markdown.includes(marker), 'charter: published text is the charter');
	assert.equal(shown.proposal, null, 'charter: no draft remains');

	// Discard path: a second draft, discarded on the Charter page.
	await import('node:fs').then((fs) =>
		fs.writeFileSync(draftFile, `# ${company}\n\n${marker} (second draft)\n`)
	);
	restless('charter', 'propose', '--requester', 'owner', '--file', draftFile);
	await page.goto(`${base}/${company}/company/charter`);
	const second = page.getByRole('region', { name: "Exec's draft of the charter" });
	await second.waitFor({ timeout: 8_000 });
	await second.getByRole('button', { name: 'Discard' }).click();
	await page.getByText('Draft discarded. The charter is unchanged.').waitFor();
	assert.equal(
		JSON.parse(restless('charter', 'show')).proposal,
		null,
		'charter: discarded draft is gone'
	);

	// A member's direction cannot set the charter; it is refused with where to draft instead.
	let refused = '';
	try {
		restless('charter', 'apply', '--requester', 'human-nobody', '--file', draftFile);
	} catch (error) {
		refused = String(error.stderr ?? error.message);
	}
	assert.match(refused, /Only .* sets the charter/, 'charter: apply refuses anyone but the owner');
	return { marker };
});

/* Every distinct state of the page's main region from an action until it has been still for a
 * second and a half: a save should read Saving…, then saved, and nothing else should move. */
async function statesAfter(page, action) {
	const watching = page.evaluate(async () => {
		const text = () => document.querySelector('main')?.innerText.replace(/\s+/g, ' ').trim() ?? '';
		const states = [text()];
		let still = performance.now();
		const started = performance.now();
		while (performance.now() - still < 1_500 && performance.now() - started < 8_000) {
			await new Promise((resolve) => requestAnimationFrame(resolve));
			const now = text();
			if (now !== states.at(-1)) {
				states.push(now);
				still = performance.now();
			}
		}
		return states;
	});
	await action();
	return watching;
}

/* What changed between two page states, as the words removed and added. */
function delta(before, after) {
	const a = new Set(before.split(' '));
	const b = new Set(after.split(' '));
	const removed = [...a]
		.filter((word) => !b.has(word))
		.slice(0, 12)
		.join(' ');
	const added = [...b]
		.filter((word) => !a.has(word))
		.slice(0, 12)
		.join(' ');
	return `-[${removed}] +[${added}]`;
}

await journey('settings', async (page) => {
	const out = {};
	// The charter: edit and save.
	await page.goto(`${base}/${company}/company/charter`);
	await page.waitForSelector('.company-browser-link-capture:not(.first-paint-pending)');
	await page.getByRole('button', { name: 'Edit', exact: true }).first().click();
	await page
		.getByLabel('Company charter in Markdown')
		.fill(`# ${company}\n\nSaved by the steady suite at ${Date.now()}.`);
	const charterStates = await statesAfter(page, () =>
		page.getByRole('button', { name: 'Save', exact: true }).click()
	);
	out.charter = charterStates.slice(1).map((state, index) => delta(charterStates[index], state));
	assert(
		charterStates.length <= 4,
		`settings: charter save passed through ${charterStates.length} states: ${out.charter.join(' | ')}`
	);

	// The company name: saved as the owner types.
	const field = page.getByRole('textbox', { name: /name/i }).first();
	const original = await field.inputValue();
	const nameStates = await statesAfter(page, async () => {
		await field.fill(`${original} x`);
		await field.blur();
	});
	out.name = nameStates.slice(1).map((state, index) => delta(nameStates[index], state));
	await field.fill(original);
	await field.blur();
	await page.waitForTimeout(2_000);
	assert(
		nameStates.length <= 5,
		`settings: rename passed through ${nameStates.length} states: ${out.name.join(' | ')}`
	);
	return out;
});

await journey('screens', async (page) => {
	// Every settings screen that now reads and writes through the model layer opens without a
	// page error or a server failure, and a save on one of them sticks.
	const failures = [];
	page.on('response', (response) => {
		const url = new URL(response.url());
		if (url.pathname.startsWith('/api/') && response.status() >= 500)
			failures.push(`${response.status()} ${response.request().method()} ${url.pathname}`);
	});
	const opened = {};
	for (const path of [
		`/${company}/company/provider`,
		'/account/settings/accounts',
		`/${company}/company/vault`,
		`/${company}/company/resources`,
		`/${company}/company/doctor`,
		`/${company}/company/charter`,
		`/${company}/company/members`,
		`/${company}/people`,
		`/${company}/library`,
		`/${company}/work`
	]) {
		await page.goto(`${base}${path}`);
		await page.waitForLoadState('networkidle', { timeout: 15_000 }).catch(() => {});
		// A company screen appears once its first reads are in (first-paint.svelte.ts).
		await page.waitForFunction(() => !document.querySelector('.first-paint-pending'), null, {
			timeout: 10_000
		});
		await page.waitForTimeout(800);
		// Rendered: the main region holds the screen, not a blank or a loading line.
		const text = (
			await page
				.locator('main')
				.first()
				.innerText()
				.catch(() => '')
		).trim();
		opened[path] = text.length > 40 ? text.split(/\n/).slice(0, 2).join(' · ') : '';
	}
	assert.deepEqual(failures, [], `screens: server failures ${failures.join(', ')}`);
	const blank = Object.entries(opened).filter(([, heading]) => !heading);
	assert.deepEqual(
		blank,
		[],
		`screens: nothing rendered on ${blank.map(([path]) => path).join(', ')}`
	);

	// The spend limit, saved through the model and shown from the save's own answer.
	await page.goto(`${base}/${company}/company/resources`);
	await page.getByRole('button', { name: 'Edit limit', exact: true }).click();
	const field = page.locator('#spend-limit');
	const before = await field.inputValue();
	const next = before === '37' ? '38' : '37';
	await field.fill(next);
	const states = await statesAfter(page, () =>
		page.getByRole('button', { name: 'Save', exact: true }).first().click()
	);
	await page.reload();
	await page.waitForLoadState('networkidle', { timeout: 15_000 }).catch(() => {});
	const shown = await page.locator('main').innerText();
	assert(shown.includes(next), `screens: the saved limit ${next} shows after a reload`);
	return { opened, limitSave: states.slice(1).map((state, index) => delta(states[index], state)) };
});

await browser.close();
console.log(JSON.stringify(report, null, 2));
process.exit(report.every((entry) => entry.ok) ? 0 : 1);
