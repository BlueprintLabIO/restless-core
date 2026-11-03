/** US-379, US-380: a pack the owner asked to review lands as one Inbox item whose pictures all show.
 * On 3 November (Reno) a staff-made send pack reached the owner as a chat message, and its
 * pictures, named by paths relative to the pack, came up blank. This seeds an isolated `_test`
 * company with that pack as Staff make it (a Markdown ReviewTarget beside its pictures, an
 * owner-review handoff briefed and escalated through the daemon) and asks Exec, through the
 * scripted model, for the chat version that names it. Then, at 1440 and 390, light and dark, it
 * checks: one review item per pack, every picture reference style shows its picture or says why
 * in the same fixed frame, nothing moves while pictures load, and the Library and chat agree.
 *
 *   RESTLESS_HOME=… RESTLESS_SMOKE_ORIGIN=http://127.0.0.1:<port> RESTLESS_SMOKE_COMPANY=<x>_test \
 *     node web/scripts/verify-pack-review.mjs
 *
 * The chat half needs the stack on tools/mock-model with tools/mock-model/scenarios/default.json. */
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { mkdir, readFile } from 'node:fs/promises';
import { createConnection } from 'node:net';
import { createRequire } from 'node:module';
import { basename, join } from 'node:path';
import { crc32, deflateSync } from 'node:zlib';
import { chromium } from 'playwright';

const origin = process.env.RESTLESS_SMOKE_ORIGIN,
	company = process.env.RESTLESS_SMOKE_COMPANY,
	home = process.env.RESTLESS_HOME;
assert(
	origin && company?.endsWith('_test') && new URL(origin).hostname === '127.0.0.1',
	'set RESTLESS_SMOKE_ORIGIN to the local stack and RESTLESS_SMOKE_COMPANY to a _test company'
);
assert(home?.includes('/.restless-dev/'), 'set RESTLESS_HOME to the isolated dev profile');
const out = process.env.RESTLESS_VERIFY_OUTPUT ?? '/tmp/restless-pack-review';
await mkdir(out, { recursive: true });
const api = origin + '/api/companies/' + encodeURIComponent(company);
const dir = '/company/outputs/pack-verify';
const pack = dir + '/final-send-pack.md';
const title = 'Final send pack for the four prospects';

/* ---------- the pack, as Staff make it ---------- */
const browser = await chromium.launch();
/* A real PNG of the given size and shape: a colour field with a darker band across the middle. */
function picture(width, height, [r, g, b]) {
	const chunk = (type, data) => {
		const head = Buffer.alloc(8);
		head.writeUInt32BE(data.length, 0);
		head.write(type, 4, 'ascii');
		const crc = Buffer.alloc(4);
		crc.writeUInt32BE(crc32(Buffer.concat([head.subarray(4), data])), 0);
		return Buffer.concat([head, data, crc]);
	};
	const header = Buffer.alloc(13);
	header.writeUInt32BE(width, 0);
	header.writeUInt32BE(height, 4);
	header.set([8, 2, 0, 0, 0], 8);
	const rows = Buffer.alloc((width * 3 + 1) * height);
	for (let y = 0; y < height; y++) {
		const band = Math.abs(y - height / 2) < height / 10 ? 0.6 : 1;
		for (let x = 0; x < width; x++)
			rows.set([r * band, g * band, b * band].map(Math.round), y * (width * 3 + 1) + 1 + x * 3);
	}
	return Buffer.concat([
		Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
		chunk('IHDR', header),
		chunk('IDAT', deflateSync(rows)),
		chunk('IEND', Buffer.alloc(0))
	]).toString('base64');
}
const files = {
	'final-send-pack-evidence/aleon-phone.png': picture(390, 844, [150, 190, 220]),
	'final-send-pack-evidence/aleon-desktop.png': picture(1440, 900, [160, 210, 170]),
	'magnet/preview.png': picture(1063, 647, [235, 200, 150]),
	'magnet/mockup.png': picture(900, 900, [200, 170, 225])
};
/* Every reference style agents write, and the ones that cannot be shown. */
const shown = [
	['Aleon on a phone', 'final-send-pack-evidence/aleon-phone.png'],
	['Aleon on a desktop', `${dir}/final-send-pack-evidence/aleon-desktop.png`],
	['The magnet artwork', './magnet/preview.png'],
	['The magnet on a fridge', `file://${dir}/magnet/mockup.png`]
];
const missing = [
	['A picture that was never saved', 'magnet/back.png'],
	['A preview on the company computer', 'http://localhost:4173/shot.png'],
	['The print file', 'magnet/print.svg']
];
const markdown = [
	`# ${title}`,
	'Four emails, each with its live site, its pictures and its price.',
	'## Aleon Electrical',
	'**Subject:** Aleon Electrical’s website',
	'> Hi Alex and Vicki, you have what people look for in an electrician…',
	'[Live site](https://example.com/aleon/) · A$500 upfront, then A$75 a month.',
	...[...shown, ...missing].map(([alt, href]) => `![${alt}](${href})`),
	'## The magnet',
	'A free fridge-magnet design each prospect can print themselves.'
].join('\n\n');
const container = 'restless-' + basename(home) + '-co-' + company;
const writer =
	"import sys,json,base64,pathlib; root=pathlib.Path(sys.argv[1]); data=json.load(sys.stdin)\nfor name,body in data.items():\n p=root/name; p.parent.mkdir(parents=True,exist_ok=True)\n p.write_bytes(base64.b64decode(body) if name.endswith('.png') else body.encode())";
execFileSync('docker', ['exec', '-i', '-u', 'company', container, 'python3', '-c', writer, dir], {
	input: JSON.stringify({
		...files,
		'final-send-pack.md': markdown,
		'magnet/print.svg': "<svg xmlns='http://www.w3.org/2000/svg' width='90' height='55'/>"
	})
});

const pg = createRequire(new URL('../../services/identity/package.json', import.meta.url))('pg');
const adminUrl = (await readFile(join(home, 'orgintel.toml'), 'utf8')).match(
	/database_url\s*=\s*"([^"]+)"/
)[1];
const admin = new pg.Client({ connectionString: adminUrl });
await admin.connect();
const { rows: databases } = await admin.query(
	"SELECT datname FROM pg_database WHERE datname LIKE '%\\_' || $1 || '\\_%' ESCAPE '\\'",
	[company]
);
await admin.end();
assert.equal(databases.length, 1, 'one isolated company database');
const url = new URL(adminUrl);
url.pathname = '/' + databases[0].datname;
const db = new pg.Client({ connectionString: url.toString() });
await db.connect();
await db.query('SET search_path TO "' + company + '"');
const daemon = (request) =>
	new Promise((resolve, reject) => {
		const socket = createConnection(join(home, 'restlessd.sock'));
		let buffer = '';
		socket.setEncoding('utf8');
		socket.setTimeout(10_000, () => (socket.destroy(), reject(new Error('daemon timed out'))));
		socket.on('connect', () =>
			socket.write(JSON.stringify({ principal: 'owner', company, ...request }) + '\n')
		);
		socket.on('data', (chunk) => {
			buffer += chunk;
			if (!buffer.includes('\n')) return;
			socket.end();
			const response = JSON.parse(buffer.split('\n')[0]);
			response.ok ? resolve(response.data) : reject(new Error(JSON.stringify(response)));
		});
		socket.on('error', reject);
	});
try {
	const existing = await db.query('SELECT id FROM work WHERE title=$1', [title]);
	if (!existing.rowCount) {
		await db.query(
			"INSERT INTO actors (id,kind,actor_class,role,display) VALUES ('eames','staff','agent','Designer','Eames') ON CONFLICT (id) DO NOTHING"
		);
		const work = randomUUID(),
			handoff = randomUUID();
		await db.query(
			"INSERT INTO work (id,owner_id,title,outcome,status,resolution,priority,commissioned_by,owner_review_required) VALUES ($1,'eames',$2,'One pack the owner reviews before any email goes','blocked','Waiting for the owner review',10,'exec',true)",
			[work, title]
		);
		for (const [kind, uri, label] of [
			['review_target', pack, 'Final send pack'],
			...Object.keys(files).map((name) => ['output', `${dir}/${name}`, name.split('/').pop()])
		])
			await db.query(
				"INSERT INTO artifact_refs (id,kind,uri,created_by,work_id,label) VALUES ($1,$2,$3,'eames',$4,$5)",
				[randomUUID(), kind, uri, work, label]
			);
		await db.query(
			"INSERT INTO owner_handoffs (id,work_id,requested_by,assigned_to,category,requested_action,prepared_state,resume_condition) VALUES ($1,$2,'eames','exec','owner_judgement','Review the send pack and accept it or ask for changes','The pack passed its independent check.','An explicit owner review decision')",
			[handoff, work]
		);
		await daemon({
			cmd: 'work-handoff-prepare-brief',
			id: handoff,
			as_actor: 'exec',
			owner_kind: 'outcome_review',
			headline: 'Approve the four emails and their magnet designs',
			situation: 'The send pack passed its independent check.',
			impact: 'No email goes until you approve it.',
			recommendation: 'Read each email with its site and pictures, then approve the pack.',
			no_action: 'The pack waits here and nothing is sent.'
		});
		await daemon({
			cmd: 'work-handoff-escalate',
			id: handoff,
			as_actor: 'exec',
			reason: 'The owner asked to approve the whole pack at once.'
		});
	}
} finally {
	await db.end();
}

/* ---------- Exec's chat reply naming the pack, through the scripted model ---------- */
const read = async (path) => {
	const response = await fetch(api + path);
	assert(response.ok, path + ' answers from the real daemon');
	return response.json();
};
const conversation = async () => (await read('/actors/exec/conversation')).messages;
if (!(await conversation()).some((message) => message.intent?.ownerReview === 'Final send pack')) {
	const form = new FormData();
	form.set('client_command_id', randomUUID());
	form.set('body', 'Where is the final send pack?');
	const response = await fetch(api + '/actors/exec/conversation', {
		method: 'POST',
		headers: { Origin: origin },
		body: form
	});
	assert(response.ok, 'the owner can write to Exec: ' + response.status);
	const deadline = Date.now() + 90_000;
	while (!(await conversation()).some((m) => m.intent?.ownerReview === 'Final send pack')) {
		assert(Date.now() < deadline, 'Exec answers through the scripted model');
		await new Promise((resolve) => setTimeout(resolve, 1000));
	}
}
const { items } = await read('/attention');
const reviews = items.filter((item) => item.review_target?.uri === pack);
assert.equal(reviews.length, 1, 'the staff-made pack is exactly one Inbox review');
const review = reviews[0];
assert.equal(review.category, 'review');
assert.equal(review.review_target.kind, 'runtime-text');
const chatReview = items.find((item) => item.source.kind === 'conversation_owner_review');
assert(chatReview, "Exec's message naming the pack is an Inbox item");
assert(
	chatReview.evidence.some((row) => row.content === pack),
	'the chat item carries the pack it links'
);
assert(
	items.some((item) => item.source.kind === 'conversation_owner_need'),
	'the question asked beside it is its own item'
);

/* ---------- what the owner sees ---------- */
const results = [];
async function frames(page, scope) {
	return scope.locator('.md-figure').evaluateAll((figures) =>
		figures.map((figure) => {
			const frame = figure.querySelector('.md-figure-frame').getBoundingClientRect();
			const img = figure.querySelector('img');
			return {
				state: figure.dataset.imageState,
				alt: img?.alt ?? figure.querySelector('.md-figure-name')?.textContent ?? '',
				loaded: !!img && img.complete && img.naturalWidth > 0,
				// The picture fits inside its frame rather than being cropped by it.
				fits:
					!img ||
					(img.getBoundingClientRect().height <= frame.height + 0.5 &&
						img.getBoundingClientRect().width <= frame.width + 0.5),
				why: figure.querySelector('.md-figure-why')?.textContent ?? '',
				// Unaffected by scrolling, so a shift is a real one.
				top: figure.offsetTop,
				height: Math.round(frame.height),
				width: Math.round(frame.width)
			};
		})
	);
}
/* Bring each picture into view (they load lazily), let it settle, then prove none of the frames
 * moved or resized while they did. */
async function pictures(page, scope, expectShown, expectMissing, name) {
	await scope.locator('.md-figure').first().waitFor();
	const first = await frames(page, scope);
	const count = await scope.locator('.md-figure').count();
	for (let index = 0; index < count; index++) {
		const figure = scope.locator('.md-figure').nth(index);
		await figure.scrollIntoViewIfNeeded();
		await figure.evaluate(
			(el) =>
				new Promise((resolve) => {
					const settled = () => {
						const img = el.querySelector('img');
						return !img || (img.complete && el.dataset.imageState);
					};
					const check = () => (settled() ? resolve() : requestAnimationFrame(check));
					check();
				})
		);
	}
	await page.waitForTimeout(300);
	const settled = await frames(page, scope);
	const showing = settled.filter((row) => row.state === 'shown');
	const saying = settled.filter((row) => row.state === 'missing');
	assert.deepEqual(
		showing.map((row) => row.alt).sort(),
		[...expectShown].sort(),
		name + ': each company picture shows'
	);
	assert(
		showing.every((row) => row.loaded),
		name + ': each shown picture has loaded'
	);
	assert.deepEqual(
		saying.map((row) => row.alt).sort(),
		[...expectMissing].sort(),
		name + ': each unreachable picture says so'
	);
	assert(
		saying.every((row) => row.why.length > 10),
		name + ': a missing picture says why'
	);
	assert(
		settled.every((row) => row.fits),
		name + ': every picture fits inside its frame'
	);
	assert(
		settled.every((row) => Math.abs(row.width * 0.625 - row.height) <= 1),
		name + ': every picture keeps its fixed frame'
	);
	assert.deepEqual(
		settled.map((row) => [row.top, row.height]),
		first.map((row) => [row.top, row.height]),
		name + ': nothing moved while pictures loaded'
	);
}
const shownAlts = shown.map(([alt]) => alt),
	missingAlts = missing.map(([alt]) => alt);
const surfaces = [
	// The staff-made pack: one review item, read in place.
	[
		'review',
		`/${company}?item=${encodeURIComponent(review.id)}`,
		'.outcome-text',
		shownAlts,
		missingAlts
	],
	// Exec's chat item: the message in full, its pictures in place.
	[
		'chat-item',
		`/${company}?item=${encodeURIComponent(chatReview.id)}`,
		'.result',
		['Magnet design', 'Magnet mockup'],
		[]
	],
	// The same pack opened in the Library.
	[
		'library',
		`/${company}/library?path=${encodeURIComponent(pack)}`,
		'.text-paper',
		shownAlts,
		missingAlts
	]
];
for (const width of [1440, 390])
	for (const scheme of ['light', 'dark'])
		for (const [surface, path, selector, expectShown, expectMissing] of surfaces) {
			const name = `${width}-${scheme}-${surface}`;
			// A fresh load each time: the owner's first look is what must arrive whole.
			const context = await browser.newContext({
				viewport: { width, height: width > 500 ? 900 : 844 },
				colorScheme: scheme,
				reducedMotion: 'reduce'
			});
			await context.addInitScript(() => {
				window.__shifts = [];
				new PerformanceObserver((list) => {
					for (const entry of list.getEntries())
						for (const source of entry.sources)
							if (
								source.node?.closest?.('.md-figure') ||
								source.node?.querySelector?.('.md-figure')
							)
								window.__shifts.push(entry.value);
				}).observe({ type: 'layout-shift', buffered: true });
			});
			const page = await context.newPage(),
				errors = [];
			page.on('pageerror', (error) => errors.push(error.message));
			try {
				await page.goto(origin + path);
				const scope = page.locator(selector);
				await scope.waitFor({ timeout: 30_000 });
				await pictures(page, scope, expectShown, expectMissing, name);
				assert.deepEqual(
					await page.evaluate(() => window.__shifts),
					[],
					name + ': no layout shift touches a picture'
				);
				await scope.locator('.md-figure').first().scrollIntoViewIfNeeded();
				await page.screenshot({ path: `${out}/${name}.png` });
				assert.deepEqual(errors, [], 'no page errors');
				results.push({ name, ok: true });
				console.log(name + ' passed');
			} catch (error) {
				results.push({ name, ok: false, error: String(error) });
				await page.screenshot({ path: `${out}/${name}-failed.png` }).catch(() => {});
				console.log(name + ' FAILED: ' + error);
			} finally {
				await context.close();
			}
		}
await browser.close();
console.log(JSON.stringify(results));
process.exitCode = results.every((row) => row.ok) ? 0 : 1;
