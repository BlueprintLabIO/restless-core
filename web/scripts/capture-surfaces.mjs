/* Owner-surface capture for visual review: every primary route at desktop,
 * tablet and phone widths, with overflow and page-error probes, and the jank
 * probes of Sprint 84: content clipped by its box, boxes that scroll sideways,
 * and raw ids, codes, paths or shell commands shown to a person. It reads only;
 * point it at a `_test` company on a running restless-dev stack, ideally one
 * filled by scripts/seed-reno-scale.mjs. Exits non-zero when anything is found. */
import { chromium } from 'playwright';
import fs from 'node:fs/promises';

const origin = process.env.RESTLESS_SMOKE_ORIGIN;
const company = process.env.RESTLESS_SMOKE_COMPANY;
if (!origin || !company?.endsWith('_test'))
	throw new Error('set RESTLESS_SMOKE_ORIGIN and a _test RESTLESS_SMOKE_COMPANY');
const out = process.env.RESTLESS_VERIFY_OUTPUT ?? '/tmp/restless-surfaces';
const only = process.env.ROUTES?.split(',');
const widths = (process.env.WIDTHS ?? '1440,1024,390').split(',').map(Number);
const reduced = process.env.REDUCED === '1';
const scheme = process.env.SCHEME === 'dark' ? 'dark' : 'light';
/* Optional deep links for seeded data: WORK_ID and ROOM_ID. */
const workId = process.env.WORK_ID;
const roomId = process.env.ROOM_ID;

const routes = [
	['home', '/'],
	['attention', `/${company}`],
	['work', `/${company}/work`],
	['work-board', `/${company}/work?lens=board`],
	...(workId ? [['work-item', `/${company}/work/${workId}`]] : []),
	['work-documents', `/${company}/work/documents`],
	['people', `/${company}/people`],
	['people-rooms', `/${company}/people/rooms`],
	...(roomId ? [['people-room', `/${company}/people?room=${roomId}`]] : []),
	['authority', `/${company}/authority`],
	['company', `/${company}/company`],
	['company-identity', `/${company}/company/identity`],
	['company-provider', `/${company}/company/provider`],
	['company-members', `/${company}/company/members`],
	['company-resources', `/${company}/company/resources`],
	['company-schedules', `/${company}/company/schedules`],
	['company-skills', `/${company}/company/skills`],
	['company-harnesses', `/${company}/company/harnesses`],
	['company-vault', `/${company}/company/vault`],
	['company-decisions', `/${company}/company/decisions`],
	['company-actions', `/${company}/company/actions`],
	['company-doctor', `/${company}/company/doctor`],
	['company-computer', `/${company}/company/computer`],
	['company-authority', `/${company}/company/authority`],
	['account', '/account/connections'],
	['not-found', `/${company}/no-such-surface`]
].filter(([name]) => !only || only.includes(name));

await fs.mkdir(out, { recursive: true });
const browser = await chromium.launch(
	process.env.RESTLESS_BROWSER_EXECUTABLE
		? { executablePath: process.env.RESTLESS_BROWSER_EXECUTABLE }
		: {}
);
const report = [];
try {
	for (const width of widths) {
		const context = await browser.newContext({
			viewport: { width, height: width < 600 ? 844 : 900 },
			deviceScaleFactor: width < 600 ? 2 : 1,
			isMobile: width < 600,
			hasTouch: width < 600,
			colorScheme: scheme,
			reducedMotion: reduced ? 'reduce' : 'no-preference'
		});
		for (const [name, path] of routes) {
			const page = await context.newPage();
			const errors = [];
			page.on('pageerror', (e) => errors.push(e.message));
			page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
			await page
				.goto(origin + path, { waitUntil: 'networkidle' })
				.catch((e) => errors.push(String(e)));
			await page.waitForTimeout(700);
			const overflow = await page.evaluate(() => {
				const doc = document.documentElement;
				const wide = [...document.querySelectorAll('body *')]
					.filter((el) => {
						const r = el.getBoundingClientRect();
						return (
							r.width > 0 &&
							r.right > doc.clientWidth + 1 &&
							getComputedStyle(el).position !== 'fixed'
						);
					})
					.slice(0, 5)
					.map((el) => `${el.tagName.toLowerCase()}.${[...el.classList].join('.')}`);
				return { scroll: doc.scrollWidth > doc.clientWidth + 1, wide };
			});
			/* Sprint 84: the jank a page-level check never sees (14 October report). */
			const jank = await page.evaluate(() => {
				const name = (el) =>
					`${el.tagName.toLowerCase()}${[...el.classList]
						.filter((c) => !c.startsWith('svelte-'))
						.map((c) => `.${c}`)
						.join('')}`;
				const visible = (el) => {
					const r = el.getBoundingClientRect();
					const cs = getComputedStyle(el);
					return r.width > 0 && r.height > 0 && cs.visibility !== 'hidden' && cs.opacity !== '0';
				};
				/* Screen-reader text is clipped on purpose; code, tables and grids scroll by design. */
				const exempt = (el) =>
					el.closest(
						'.sr-only, pre, code, table, [role=grid], .sheet-grid, canvas, svg, .cm-editor, .ProseMirror'
					);
				/* Content wider than the box that clips it: a row cut at the sidebar edge. */
				const clipped = [];
				/* A box that scrolls sideways inside the page. */
				const sideways = [];
				for (const el of document.querySelectorAll('body *')) {
					if (!visible(el) || exempt(el)) continue;
					const cs = getComputedStyle(el);
					const scrolls = cs.overflowX === 'auto' || cs.overflowX === 'scroll';
					if (scrolls && el.scrollWidth > el.clientWidth + 2 && el.clientWidth > 40)
						sideways.push(`${name(el)} ${el.clientWidth}<${el.scrollWidth}`);
					if (cs.overflowX === 'hidden' || cs.overflowX === 'clip') {
						for (const child of el.children) {
							if (!visible(child) || getComputedStyle(child).position === 'absolute') continue;
							const box = el.getBoundingClientRect();
							const inner = child.getBoundingClientRect();
							const ellipsized = getComputedStyle(child).textOverflow === 'ellipsis';
							/* A box a few pixels wide is collapsed on purpose, not cutting anything off. */
							if (
								!ellipsized &&
								box.width >= 8 &&
								inner.right > box.right + 2 &&
								inner.width > box.width + 2
							)
								clipped.push(
									`${name(child)} in ${name(el)} ${Math.round(box.width)}<${Math.round(inner.width)}`
								);
						}
					}
				}
				/* Words a person should never be shown. */
				const text = document.body.innerText;
				const raw = [
					[/\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b/i, 'uuid'],
					[/\bcompany_[0-9a-f]{8,}/i, 'company handle'],
					[/\[(budget|error|warn|internal)\]/i, 'bracket code'],
					[
						/\brestless (company|work|people|direction|request|charter|message|teams?) [a-z-]+/i,
						'shell command'
					],
					[/\$\(cat /, 'shell command'],
					[/\/company\/(outputs|archive|mission)/i, 'company path']
				]
					.map(([pattern, label]) => {
						const hit = text.match(pattern);
						return hit ? `${label}: ${hit[0].slice(0, 60)}` : null;
					})
					.filter(Boolean);
				return {
					clipped: [...new Set(clipped)].slice(0, 6),
					sideways: [...new Set(sideways)].slice(0, 6),
					raw
				};
			});
			const file = `${out}/${name}-${width}-${scheme}${reduced ? '-rm' : ''}.png`;
			await page.screenshot({ path: file });
			report.push({
				name,
				width,
				scheme,
				url: page.url().replace(origin, ''),
				errors,
				overflow,
				jank
			});
			await page.close();
		}
		await context.close();
	}
} finally {
	await browser.close();
}
await fs.writeFile(`${out}/report.json`, JSON.stringify(report, null, 2));
for (const r of report) {
	/* Clipped off-canvas panes (a closed rail) are wide but invisible; only a
	 * page that actually scrolls sideways is a defect worth naming. */
	const flags = [
		r.overflow.scroll && `H-SCROLL ${r.overflow.wide.join(' ')}`,
		r.errors.length && `errors:${r.errors.length}`,
		r.jank.clipped.length && `CLIPPED ${r.jank.clipped.join(' | ')}`,
		r.jank.sideways.length && `SIDEWAYS ${r.jank.sideways.join(' | ')}`,
		r.jank.raw.length && `RAW ${r.jank.raw.join(' | ')}`
	].filter(Boolean);
	console.log(
		`${r.name.padEnd(18)} ${String(r.width).padEnd(5)} ${r.url.padEnd(40)} ${flags.join(' ') || 'ok'}`
	);
}
if (
	report.some(
		(r) => r.overflow.scroll || r.jank.clipped.length || r.jank.sideways.length || r.jank.raw.length
	)
)
	process.exitCode = 1;
