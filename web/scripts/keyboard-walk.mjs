/* Keyboard-only walk of the everyday screens (S75). From the top of each screen it tabs to the
 * end and reports every stop that has no name, cannot be seen, or shows no focus, and any place
 * where focus gets stuck. It leaves a sheet's grid the documented way: Escape, then Tab. A stop
 * shows focus when its pixels change between focused and blurred.
 *
 * Point it at a `_test` company on a running restless-dev stack. It makes one document and one
 * sheet there. Exits 1 on any finding outside the editors, where the caret is the cue. */
import { chromium } from 'playwright';
import { randomUUID } from 'node:crypto';

const origin = process.env.RESTLESS_SMOKE_ORIGIN;
const company = process.env.RESTLESS_SMOKE_COMPANY;
if (!origin || !company?.endsWith('_test'))
	throw new Error('set RESTLESS_SMOKE_ORIGIN and a _test RESTLESS_SMOKE_COMPANY');
const only = process.env.ROUTES?.split(',');

/* Editing surfaces show a caret or the grid's own selection rather than a ring. */
const CARET = ['.paper-title', '.ProseMirror', '.o-grid-composer'];

const browser = await chromium.launch();
const context = await browser.newContext({ viewport: { width: 1280, height: 860 } });
const api = async (path, data) =>
	(
		await context.request.fetch(origin + path, {
			method: 'POST',
			headers: {
				Origin: origin,
				'Content-Type': 'application/json',
				'Idempotency-Key': randomUUID()
			},
			data: JSON.stringify(data)
		})
	)
		.json()
		.catch(() => null);
const base = `/api/companies/${company}`;
const doc = await api(`${base}/documents`, {
	title: 'Keyboard walk',
	kind: 'brief',
	visibility: 'company',
	reason: 'keyboard walk',
	content_json: {
		type: 'doc',
		content: [
			{
				type: 'paragraph',
				attrs: { block_id: randomUUID() },
				content: [{ type: 'text', text: 'Walk.' }]
			}
		]
	}
});
const sheet = randomUUID();
await api(`${base}/sheets`, { id: sheet, title: 'Keyboard walk', visibility: 'company' });
const page = await context.newPage();
const screens = [
	['inbox', `/${company}`],
	['library', `/${company}/library`],
	['document', `/${company}/library/documents?document=${doc.document_id}`],
	[
		'document-comments',
		`/${company}/library/documents?document=${doc.document_id}&inspect=comments`
	],
	['sheet', `/${company}/library/sheets?sheet=${sheet}`],
	['sheet-comments', `/${company}/library/sheets?sheet=${sheet}&inspect=comments`],
	['people', `/${company}/people`],
	['members', `/${company}/company/members`],
	['work', `/${company}/work`]
].filter(([name]) => !only || only.includes(name));

function describe(caret) {
	const el = document.activeElement;
	if (!el || el === document.body) return null;
	const r = el.getBoundingClientRect();
	const name =
		el.getAttribute('aria-label') ||
		el
			.getAttribute('aria-labelledby')
			?.split(/\s+/)
			.map((id) => document.getElementById(id)?.textContent ?? '')
			.join(' ') ||
		(el.id && document.querySelector(`label[for="${CSS.escape(el.id)}"]`)?.textContent) ||
		el.getAttribute('title') ||
		el.getAttribute('placeholder') ||
		(el.innerText ?? '').trim().slice(0, 60) ||
		'';
	const path = [];
	for (let n = el; n && n !== document.body && path.length < 4; n = n.parentElement)
		path.unshift(
			n.tagName.toLowerCase() +
				(typeof n.className === 'string' && n.className
					? '.' + n.className.trim().split(/\s+/)[0]
					: '')
		);
	return {
		name: name.replace(/\s+/g, ' ').trim(),
		path: path.join('>'),
		box: {
			x: Math.round(r.x),
			y: Math.round(r.y),
			w: Math.round(r.width),
			h: Math.round(r.height)
		},
		visible:
			r.width > 0 &&
			r.height > 0 &&
			r.bottom > 0 &&
			r.right > 0 &&
			r.top < innerHeight &&
			r.left < innerWidth &&
			getComputedStyle(el).visibility !== 'hidden',
		caret: caret.some((selector) => el.closest(selector)),
		grid: !!el.closest('.o-spreadsheet')
	};
}

async function focusShows(box) {
	const clip = {
		x: Math.max(0, box.x - 4),
		y: Math.max(0, box.y - 4),
		width: Math.min(box.w + 8, 1280),
		height: Math.min(box.h + 8, 860)
	};
	if (clip.width < 2 || clip.height < 2 || clip.y > 860 || clip.x > 1280) return null;
	const focused = await page.screenshot({ clip }).catch(() => null);
	const handle = await page.evaluateHandle(() => document.activeElement);
	await page.evaluate(() => document.activeElement?.blur());
	const blurred = await page.screenshot({ clip }).catch(() => null);
	await handle.evaluate((el) => el?.focus?.({ preventScroll: true }));
	return focused && blurred ? !focused.equals(blurred) : null;
}

let findings = 0;
for (const [name, path] of screens) {
	await page.goto(origin + path);
	await page.waitForTimeout(3500);
	await page.mouse.click(1, 1).catch(() => {});
	await page.evaluate(() => document.activeElement?.blur());
	const seen = new Map();
	const problems = [];
	let stops = 0;
	let first = '';
	let leftGrid = false;
	for (let i = 0; i < 150; i++) {
		await page.keyboard.press('Tab');
		await page.waitForTimeout(40);
		const stop = await page.evaluate(describe, CARET);
		if (!stop) break; // past the last control: the walk is complete
		if (i === 0) first = stop.name;
		const key = `${stop.path}|${stop.name}|${stop.box.x},${stop.box.y}`;
		if (seen.has(key)) {
			if (stop.grid && !leftGrid) {
				leftGrid = true;
				await page.keyboard.press('Escape');
				continue;
			}
			if (i - seen.get(key) <= 2) problems.push(`focus stuck at "${stop.name || stop.path}"`);
			break;
		}
		seen.set(key, i);
		stops += 1;
		if (!stop.name) problems.push(`no name: ${stop.path}`);
		if (!stop.visible) problems.push(`cannot be seen: "${stop.name}" ${stop.path}`);
		else if (!stop.caret && (await focusShows(stop.box)) === false)
			problems.push(`no visible focus: "${stop.name}" ${stop.path}`);
	}
	if (first !== 'Skip to content')
		problems.push(`the first stop is "${first}", not Skip to content`);
	console.log(`${problems.length ? 'FAIL' : 'PASS'} ${name}: ${stops} stops`);
	for (const problem of problems) console.log(`  ${problem}`);
	findings += problems.length;
}
await browser.close();
process.exit(findings ? 1 : 0);
