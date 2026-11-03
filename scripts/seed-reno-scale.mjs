#!/usr/bin/env node
// Seed a `_test` company on a running restless-dev stack so it looks like a company in real use:
// Reno, a studio that renovates small-business websites. Long names, a lead with two staff, a goal
// with 30 finished pieces of Work, Work under way and blocked, a Library of several hundred files on
// the company computer (client site folders, screenshots, agents' notes, reviews and reports, a
// scratch folder, Exec's journal, downloads, documents, an open and a closed goal), long Exec
// threads with code blocks and an error, and direction requests.
//
// Checks run against an empty company miss what real use shows: lists that pile up, names that
// overflow, threads that open at the top. Run this before a sweep or a UI change and look there.
//
//   RESTLESS_HOME=~/.restless-dev/<namespace> node scripts/seed-reno-scale.mjs s73_test
//
// It writes through the daemon where a command exists. Finished Work and its files are written to
// the company's own tables, because finishing Work for real needs agents to run. It refuses any
// company whose name does not end in `_test`, and running it again adds nothing twice.
import { createConnection } from 'node:net';
import { createRequire } from 'node:module';
import { readFileSync, mkdirSync, writeFileSync, utimesSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { randomUUID, createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';

const company = process.argv[2];
const home = process.env.RESTLESS_HOME;
if (!company?.endsWith('_test')) throw new Error('usage: seed-reno-scale.mjs <company>_test');
if (!home) throw new Error('set RESTLESS_HOME to the dev stack home (~/.restless-dev/<namespace>)');

const repo = join(dirname(fileURLToPath(import.meta.url)), '..');
const pg = createRequire(join(repo, 'services/identity/package.json'))('pg');

async function daemon(request) {
	const line = await new Promise((resolve, reject) => {
		const socket = createConnection(join(home, 'restlessd.sock'));
		let buffer = '';
		socket.setEncoding('utf8');
		socket.on('connect', () => socket.write(JSON.stringify({ principal: 'owner', company, ...request }) + '\n'));
		socket.on('data', (chunk) => {
			buffer += chunk;
			const end = buffer.indexOf('\n');
			if (end >= 0) {
				socket.end();
				resolve(buffer.slice(0, end));
			}
		});
		socket.on('error', reject);
	});
	const response = JSON.parse(line);
	if (!response.ok) throw new Error(`${request.cmd}: ${JSON.stringify(response.error)}`);
	return response.data;
}
const tryDaemon = (request) => daemon(request).catch((error) => console.log(`  skipped: ${error.message}`));

// The company's database: one per company, named after the stack namespace and the company.
const adminUrl = readFileSync(join(home, 'orgintel.toml'), 'utf8').match(/database_url\s*=\s*"([^"]+)"/)[1];
const admin = new pg.Client({ connectionString: adminUrl });
await admin.connect();
const { rows: databases } = await admin.query(
	"SELECT datname FROM pg_database WHERE datname LIKE '%\\_' || $1 || '\\_%' ESCAPE '\\'",
	[company]
);
await admin.end();
if (databases.length !== 1) throw new Error(`expected one database for ${company}, found ${databases.length}`);
const url = new URL(adminUrl);
url.pathname = '/' + databases[0].datname;
const db = new pg.Client({ connectionString: url.toString() });
await db.connect();
await db.query(`SET search_path TO "${company}"`);

const day = 24 * 3600 * 1000;
const ago = (days) => new Date(Date.now() - days * day);

// ---------------------------------------------------------------- people and teams
console.log('people and teams');
await tryDaemon({ cmd: 'company-set', state: 'mission', body: CHARTER() });
const roster = [
	['studio-direction', 'Lead designer', 'Elif'],
	['studio-design', 'Visual designer', 'Emeka'],
	['site-critic', 'Independent reviewer', 'Esra'],
	['copy-editor', 'Copy editor', 'auto'],
];
const people = (await daemon({ cmd: 'people' })).actors ?? (await daemon({ cmd: 'people' }));
const existing = new Set((Array.isArray(people) ? people : []).map((actor) => actor.id ?? actor.actor_id));
for (const [id, role, display] of roster) {
	if (!existing.has(id))
		await tryDaemon({ cmd: 'actor-create', as_actor: id, role, name: display, actor: 'exec', reason: 'Reno-scale seed' });
}
const teams = (await daemon({ cmd: 'teams' })).teams;
// The copy editor is on the team and idle: no Work, nothing owed.
const reno = teams.find((team) => team.name === 'Reno Studio');
if (reno && !(reno.members ?? []).some((member) => member.actor_id === 'copy-editor'))
	await tryDaemon({ cmd: 'team-assign', as_actor: 'copy-editor', name: 'Reno Studio', actor: 'exec', reason: 'Reno-scale seed' });
if (!teams.some((team) => team.name === 'Reno Studio')) {
	await tryDaemon({
		cmd: 'team-create',
		name: 'Reno Studio',
		to: 'studio-direction',
		body: 'Renovate the websites of established small businesses: rebuild dated sites so they look credible, load fast and turn visitors into enquiries.',
		actor: 'exec',
	});
	for (const id of ['studio-design', 'site-critic'])
		await tryDaemon({ cmd: 'team-assign', as_actor: id, name: 'Reno Studio', actor: 'exec', reason: 'Reno-scale seed' });
}
const goals = await daemon({ cmd: 'goals' });
const goalList = Array.isArray(goals) ? goals : goals.goals ?? [];
let goal = goalList.find((row) => row.title?.startsWith('Grow a website-renovation service'));
if (!goal) {
	await daemon({
		cmd: 'goal-add',
		title: 'Grow a website-renovation service for established local businesses',
		body: 'Six concept sites, a price people accept, and the first paid renovation.',
		actor: 'exec',
		team_name: 'Reno Studio',
	});
	const again = await daemon({ cmd: 'goals' });
	goal = (Array.isArray(again) ? again : again.goals).find((row) => row.title.startsWith('Grow a website-renovation service'));
}

// ---------------------------------------------------------------- Work and its files
console.log('work and library');
const { rows: seeded } = await db.query("SELECT count(*)::int AS n FROM work WHERE title LIKE 'Ordway & Daughters%'");
const files = join(home, 'seed-files', company);
if (seeded[0].n === 0) {
	const clients = [
		'Taylor & Ward Landscapes', 'Ordway & Daughters Joinery', 'Bowral Creative Constructions',
		'Ustyle Homes', 'SI Constructions', 'Kidron Homes', 'Harbourside Physiotherapy & Pilates',
		'Mount Victoria Veterinary Clinic', 'The Corner Bakehouse on Bong Bong Street',
		'Southern Highlands Electrical Contracting',
	];
	const done = [];
	for (let i = 0; i < 30; i++) {
		const client = clients[i % clients.length];
		const version = Math.floor(i / clients.length) + 1;
		done.push(
			i % 3 === 2
				? [`Independent check of the ${client} concept website (version ${version})`, 'site-critic', 'review']
				: [`${client} concept website, version ${version}: quiet luxury, led by real photography and a single clear enquiry action`, i % 2 ? 'studio-design' : 'studio-direction', 'site']
		);
	}
	const open = [
		['Independent review of the Taylor & Ward concept, version 4', 'site-critic', 'proposed', ''],
		["Independent review of Reno's own website", 'site-critic', 'proposed', ''],
		["Reno's own website, version 5: one before-and-after idea carried through every section", 'studio-design', 'active', ''],
		["Reno's own website, built to the studio's premium editorial standard", 'studio-direction', 'blocked',
			"Blocked: Reno's own website, build waits on the designer's model connection, which is no longer granted to the company"],
	];
	let n = 0;
	for (const [title, owner, kind] of done) {
		const id = randomUUID();
		const at = ago(30 - n);
		await db.query(
			`INSERT INTO work (id, owner_id, title, outcome, status, resolution, created_at, updated_at, priority, commissioned_by, goal_id)
			 VALUES ($1,$2,$3,$4,'completed','accepted',$5,$5,1,'exec',$6)`,
			[id, owner, title, `${title}. Done when the page is live on a preview link and passes the independent check.`, at, goal?.id ?? null]
		);
		const outputs = kind === 'review' ? ['review.md'] : ['index.html', 'mobile-390.png', 'desktop-1440.png', 'notes.md'];
		for (const output of outputs) {
			const slug = title.toLowerCase().replace(/[^a-z0-9]+/g, '-').slice(0, 60);
			const uri = `/company/outputs/${slug}-${n}/${output}`;
			await db.query(
				`INSERT INTO artifact_refs (id, kind, uri, created_by, created_at, work_id, label, digest)
				 VALUES ($1,$2,$3,$4,$5,$6,$7,$8)`,
				[randomUUID(), output.endsWith('.png') ? 'image' : output.endsWith('.html') ? 'site' : 'document', uri, owner, at, id,
					output === 'index.html' ? `${title.split(':')[0]}.html` : output, createHash('sha256').update(uri).digest('hex')]
			);
			const path = join(files, uri);
			mkdirSync(dirname(path), { recursive: true });
			writeFileSync(path, output.endsWith('.html') ? `<!doctype html><title>${title}</title><h1>${title}</h1>` : `# ${title}\n`);
		}
		n++;
	}
	for (const [title, owner, status, resolution] of open) {
		await db.query(
			`INSERT INTO work (id, owner_id, title, outcome, status, resolution, created_at, updated_at, priority, commissioned_by, goal_id)
			 VALUES ($1,$2,$3,$4,$5,$6,now(),now(),2,'exec',$7)`,
			[randomUUID(), owner, title, `${title}.`, status, resolution, goal?.id ?? null]
		);
	}
}

// Reviews, newer versions and a stopped piece of Work, the way the engine records them:
// - an independent check is its own Work that reviews a producer's Work: a `revises` edge from the
//   reviewer to the producer, paired with a `requires` edge from the producer to the reviewer;
// - its verdict is its latest attempt: `produced` (accepted) or `changes_requested`;
// - a newer version is separate Work; the older version's outputs are marked superseded;
// - one early landing page was stopped with its files still recorded.
// Earlier seeds linked versions to each other with `revises`, which the engine never does; those
// edges are removed.
await db.query(
	`DELETE FROM work_edges e USING work a, work b
	 WHERE e.kind = 'revises' AND a.id = e.from_work_id AND b.id = e.to_work_id
	   AND a.title ~ ' concept website, version [0-9]+:' AND b.title ~ ' concept website, version [0-9]+:'`
);
const { rows: reviewed } = await db.query(
	"SELECT count(*)::int AS n FROM work_edges e JOIN work r ON r.id = e.from_work_id WHERE e.kind = 'revises' AND r.title LIKE 'Independent check of %'"
);
if (reviewed[0].n === 0) {
	const { rows: rows } = await db.query("SELECT id, title, owner_id FROM work WHERE title ~ 'concept website'");
	const sites = new Map();
	for (const row of rows) {
		const site = row.title.match(/^(.*) concept website, version (\d+):/);
		if (site) sites.set(`${site[1]}|${site[2]}`, row);
	}
	let index = 0;
	for (const row of rows) {
		const check = row.title.match(/^Independent check of the (.*) concept website \(version (\d+)\)/);
		if (!check) continue;
		// The check reviews the version before it in the same client's run, or that version itself.
		const producer = sites.get(`${check[1]}|${check[2]}`) ?? sites.get(`${check[1]}|${Number(check[2]) - 1}`);
		if (!producer) continue;
		await db.query("INSERT INTO work_edges (from_work_id, to_work_id, kind) VALUES ($1, $2, 'revises')", [row.id, producer.id]);
		await db.query("INSERT INTO work_edges (from_work_id, to_work_id, kind) VALUES ($1, $2, 'requires') ON CONFLICT DO NOTHING", [producer.id, row.id]);
		const verdict = index++ === 0 ? 'changes_requested' : 'produced';
		await db.query(
			`INSERT INTO work_attempts (id, work_id, revision, attempt_no, actor_id, session_id, state, trigger, input_fingerprint, summary)
			 VALUES ($1, $2, 1, 1, $3, $4, $5, 'seed', $6, $7)`,
			[randomUUID(), row.id, row.owner_id, `seed-${randomUUID().slice(0, 8)}`, verdict, createHash('sha256').update(row.id).digest('hex'),
				verdict === 'produced' ? 'Accepted: the page meets the brief at phone and desktop width.' : 'Changes requested: the hero image is squeezed on phones.']
		);
	}
	// Older versions' outputs are superseded by the newest version of each client's site.
	await db.query(
		`UPDATE artifact_refs a SET state = 'superseded', superseded_at = now()
		 FROM work w WHERE a.work_id = w.id AND w.title ~ ' concept website, version [12]:'
		   AND a.state = 'available'`
	);
}
const { rows: stopped } = await db.query("SELECT count(*)::int AS n FROM work WHERE status = 'abandoned'");
if (stopped[0].n === 0) {
	const id = randomUUID();
	await db.query(
		`INSERT INTO work (id, owner_id, title, outcome, status, resolution, created_at, updated_at, priority, commissioned_by, goal_id)
		 VALUES ($1,'studio-design',$2,$3,'abandoned','Stopped: the owner chose the before-and-after direction instead',$4,$4,1,'exec',$5)`,
		[id, "Reno's first landing page: quiet editorial, cream paper and a clay accent", "Reno's first landing page.", ago(12), goal?.id ?? null]
	);
	for (const output of ['index.html', 'desktop-1440.png']) {
		const uri = `/company/outputs/reno-first-landing/${output}`;
		await db.query(
			`INSERT INTO artifact_refs (id, kind, uri, created_by, created_at, work_id, label, digest)
			 VALUES ($1,$2,$3,'studio-design',$4,$5,$6,$7)`,
			[randomUUID(), output.endsWith('.png') ? 'image' : 'site', uri, ago(12), id,
				output === 'index.html' ? "Reno's first landing page.html" : output, createHash('sha256').update(uri).digest('hex')]
		);
	}
}

// ---------------------------------------------------------------- the Library at Reno scale
// Production Reno's Library holds hundreds of files: client sites in folders, version after
// version, with screenshots, agents' notes, reviews and reports beside them, loose images, a
// scratch folder, Exec's journal, downloads, files nothing recorded and a handful of documents.
// The records go to the company's tables and the files themselves onto the company computer
// (/company/outputs, /company/org, /company/downloads), so folders list and files open.
console.log('library at Reno scale');
const namespace = process.env.RESTLESS_RESOURCE_NAMESPACE ?? home.split('/').filter(Boolean).at(-1);
const computer = `restless-${namespace}-co-${company}`;
const PNG = Buffer.from(
	'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==',
	'base64'
);
const PDF = Buffer.from(
	'%PDF-1.1\n1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj 2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj 3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 300 144]>>endobj\ntrailer<</Root 1 0 R>>\n%%EOF\n'
);
function put(uri, body, at) {
	const path = join(files, uri);
	mkdirSync(dirname(path), { recursive: true });
	writeFileSync(path, body);
	utimesSync(path, at, at);
}
async function addWork(title, owner, status, at, goalId) {
	const id = randomUUID();
	await db.query(
		`INSERT INTO work (id, owner_id, title, outcome, status, resolution, created_at, updated_at, priority, commissioned_by, goal_id)
		 VALUES ($1,$2,$3,$4,$5,$6,$7,$7,1,'exec',$8)`,
		[id, owner, title, `${title}.`, status, status === 'completed' ? 'accepted' : '', at, goalId ?? null]
	);
	return id;
}
async function addFile(workId, owner, uri, label, kind, at, body) {
	await db.query(
		`INSERT INTO artifact_refs (id, kind, uri, created_by, created_at, work_id, label, digest)
		 VALUES ($1,$2,$3,$4,$5,$6,$7,$8)`,
		[randomUUID(), kind, uri, owner, at, workId, label, createHash('sha256').update(uri).digest('hex')]
	);
	if (body !== undefined && uri.startsWith('/company/')) put(uri, body, at);
}
const page = (title, body = '') =>
	`<!doctype html><meta charset="utf-8"><title>${title}</title><link rel="stylesheet" href="styles.css"><h1>${title}</h1>${body}`;
async function goalNamed(title, body) {
	const list = await daemon({ cmd: 'goals' });
	const found = (Array.isArray(list) ? list : list.goals ?? []).find((row) => row.title === title);
	if (found) return found;
	await daemon({ cmd: 'goal-add', title, body, actor: 'exec', team_name: 'Reno Studio' });
	const again = await daemon({ cmd: 'goals' });
	return (Array.isArray(again) ? again : again.goals).find((row) => row.title === title);
}
const { rows: libraryDone } = await db.query(
	"SELECT count(*)::int AS n FROM artifact_refs WHERE uri LIKE '/company/outputs/sites/%'"
);
if (libraryDone[0].n === 0) {
	const outreach = await goalNamed('Outreach test send', 'Ten hand-picked local businesses get a first email with their concept site.');
	const spring = await goalNamed('Spring promo', 'A spring flyer and social images for the studio.');
	const slug = (text) => text.toLowerCase().replace(/&/g, 'and').replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
	const clients = [
		'Aleon Electrical', 'Taylor & Ward Landscapes', 'Ordway & Daughters Joinery', 'Bowral Creative Constructions',
		'Ustyle Homes', 'SI Constructions', 'Kidron Homes', 'Harbourside Physiotherapy & Pilates',
		'Mount Victoria Veterinary Clinic', 'The Corner Bakehouse on Bong Bong Street', 'Southern Highlands Electrical Contracting',
	];
	for (const [index, client] of clients.entries()) {
		const folder = `/company/outputs/sites/${slug(client)}`;
		const versions = index === 0 ? 4 : 1 + (index % 3);
		for (let version = 1; version <= versions; version++) {
			// Aleon and Taylor & Ward are the busiest: their newest versions changed today.
			const at = index < 2 && version === versions ? ago(0.1 + index * 0.2) : ago(4 + index * 2 + (versions - version) * 3);
			const owner = version % 2 ? 'studio-design' : 'studio-direction';
			const latest = version === versions;
			const work = await addWork(`${client} concept website, version ${version}`, owner, latest && index < 2 ? 'active' : 'completed', at, goal?.id);
			const dir = `${folder}/v${version}`;
			await addFile(work, owner, `${dir}/index.html`, `${client} concept, version ${version}.html`, 'site', at, page(`${client}, version ${version}`));
			await addFile(work, owner, `${dir}/desktop-1440.png`, 'desktop-1440.png', 'image', at, PNG);
			await addFile(work, owner, `${dir}/mobile-390.png`, 'mobile-390.png', 'image', at, PNG);
			await addFile(work, owner, `${dir}/notes.md`, 'notes.md', 'document', at, `# Notes on ${client}, version ${version}\n\nWhat changed and why.\n`);
			// What the site needs that nothing records: its styles, script and photos.
			put(`${dir}/styles.css`, 'body{font-family:serif}', at);
			put(`${dir}/app.js`, '// enquiry form', at);
			put(`${dir}/assets/hero.png`, PNG, at);
			put(`${dir}/assets/team.png`, PNG, at);
			const review = await addWork(`Independent check of ${client}, version ${version}`, 'site-critic', 'completed', at, goal?.id);
			await addFile(review, 'site-critic', `${dir}/review.md`, 'review.md', 'document', at, `# Review of ${client}, version ${version}\n\nThe hero holds at 390.\n`);
		}
		const reporter = await addWork(`${client} upgrade report`, 'site-critic', 'completed', ago(6 + index), goal?.id);
		await addFile(reporter, 'site-critic', `/company/outputs/${slug(client)}-upgrade-report.md`, `${slug(client)}-upgrade-report.md`, 'document', ago(6 + index), `# ${client}: what to upgrade\n`);
	}
	// Outreach: an email preview, its recipients and the send log; a live landing page.
	const send = await addWork('Outreach test send: the first ten emails', 'studio-direction', 'active', ago(0.05), outreach?.id);
	await addFile(send, 'studio-direction', '/company/outputs/outreach/test-send/email-preview.html', 'Outreach email preview.html', 'site', ago(0.05), page('Outreach email preview'));
	await addFile(send, 'studio-direction', '/company/outputs/outreach/test-send/recipients.csv', 'recipients.csv', 'document', ago(0.05), 'name,email\nAleon Electrical,hello@aleon.example\n');
	await addFile(send, 'studio-direction', '/company/outputs/outreach/test-send/send-log.md', 'send-log.md', 'document', ago(0.05), '# Send log\n');
	await addFile(send, 'studio-direction', 'http://127.0.0.1:8650/', 'Outreach landing page', 'site', ago(0.3));
	// A goal that closed: its files leave Live now and stay reachable from Archive.
	const promo = await addWork('Spring promo flyer and social images', 'studio-design', 'completed', ago(25), spring?.id);
	await addFile(promo, 'studio-design', '/company/outputs/spring-promo/spring-flyer.pdf', 'Spring flyer.pdf', 'document', ago(25), PDF);
	for (let n = 1; n <= 6; n++)
		await addFile(promo, 'studio-design', `/company/outputs/spring-promo/social-${n}.png`, `social-${n}.png`, 'image', ago(25), PNG);
	// Loose images, scratch drafts, Exec's journal and a stray note at the top of outputs.
	const shots = await addWork('Photograph references for the concept sites', 'studio-design', 'completed', ago(18), goal?.id);
	for (let n = 1; n <= 60; n++)
		await addFile(shots, 'studio-design', `/company/outputs/images/reference-${String(n).padStart(2, '0')}.png`, `reference-${String(n).padStart(2, '0')}.png`, 'image', ago(18 + n / 10), PNG);
	const scratch = await addWork('Try three hero layouts', 'studio-design', 'completed', ago(9), goal?.id);
	for (let n = 1; n <= 12; n++)
		await addFile(scratch, 'studio-design', `/company/outputs/scratch/hero-try-${n}.html`, `hero-try-${n}.html`, 'site', ago(9), page(`Hero try ${n}`));
	const journal = await addWork('Keep the company journal', 'exec', 'active', ago(1), null);
	for (let n = 1; n <= 24; n++)
		await addFile(journal, 'exec', `/company/org/exec/journal/${String(n).padStart(4, '0')}.md`, `${String(n).padStart(4, '0')}.md`, 'document', ago(25 - n), `# Journal ${n}\n`);
	await addFile(journal, 'exec', '/company/outputs/lazarus-cc-notes.md', 'lazarus-cc-notes.md', 'document', ago(3), '# Lazarus notes\n');
	// Files on the computer that nothing recorded: a card site and two downloads.
	put('/company/outputs/calcifer-card/index.html', page("Calcifer's card"), ago(2));
	put('/company/outputs/calcifer-card/card.png', PNG, ago(2));
	put('/company/downloads/supplier-pricelist.pdf', PDF, ago(11));
	put('/company/downloads/council-brief.pdf', PDF, ago(14));
	// Documents: a few the studio wrote, two waiting on the owner's review.
	for (const [title, kind] of [
		['Reno pricing: what a renovation costs', 'decision_note'], ['Studio brief: who we renovate for', 'brief'],
		['Launch week plan', 'plan'], ['Outreach email: first touch', 'freeform'], ['Aleon Electrical proposal', 'report'],
		['How we review a concept site', 'handbook'], ['Spring promo copy', 'freeform'],
	])
		await tryDaemon({
			cmd: 'document-operation',
			document_operation: { operation: 'create', title, kind, visibility: 'company', content: { format: 'markdown', body: `# ${title}\n\nDraft.\n` }, reason: 'Reno-scale seed', key: randomUUID() },
		});
	await db
		.query("UPDATE native_documents SET status = 'in_review' WHERE title IN ('Aleon Electrical proposal', 'Outreach email: first touch')")
		.catch((error) => console.log(`  skipped review status: ${error.message}`));
	if (spring?.id && !spring.closed_at) await tryDaemon({ cmd: 'goal-close', goal: spring.id, actor: 'exec' });
}
// Every run puts the files onto the company computer, which a fresh computer starts without, each
// dated when its record says it was made, so "Updated" reads as it would in real use.
{
	const { execFileSync } = await import('node:child_process');
	const { existsSync } = await import('node:fs');
	const { rows: dated } = await db.query("SELECT uri, created_at FROM artifact_refs WHERE uri LIKE '/company/%'");
	for (const row of dated) {
		const path = join(files, row.uri);
		if (existsSync(path)) utimesSync(path, row.created_at, row.created_at);
	}
	try {
		execFileSync(
			'bash',
			['-c', `cd "$0/company" && tar -cf - $(ls) | docker exec -i -u company "$1" tar -xf - -C /company --no-same-owner --no-overwrite-dir`, files, computer],
			{ stdio: ['ignore', 'inherit', 'inherit'] }
		);
		console.log(`  files copied to ${computer}`);
	} catch (error) {
		console.log(`  files not copied to ${computer}: ${error.message}`);
	}
}

// ---------------------------------------------------------------- conversation
console.log('conversation');
const { rows: talked } = await db.query("SELECT count(*)::int AS n FROM messages WHERE body LIKE '%[seed]%'");
if (talked[0].n === 0) {
	for (const [from, to, body] of THREAD()) await tryDaemon({ cmd: 'message', from, to, body: body + ' [seed]' });
}

// ---------------------------------------------------------------- direction
console.log('direction');
const authority = new pg.Client({ connectionString: adminUrl });
await authority.connect();
const { rows: filed } = await authority.query(
	"SELECT count(*)::int AS n FROM restless_authority.records WHERE company = $1 AND kind = 'direction_request'",
	[company]
);
await authority.end();
for (const request of filed[0].n ? [] : [
	['studio-direction', 'staffing', 'Reno Studio', 'Give the designer a stronger model for launch week', 'Move Emeka to the frontier tier until the six sites are reviewed', 'The launch copy and layouts keep needing rewrites'],
	['studio-direction', null, null, 'Raise the monthly spend ceiling to $200', 'Company spend ceiling from $100 to $200 a month', ''],
])
	await tryDaemon({
		cmd: 'direction-operation',
		direction_operation: { operation: 'request_file', requester: request[0], class: request[1], team: request[2], summary: request[3], change: request[4], reason: request[5] },
	});

await db.end();
console.log(`seeded ${company}; open http://127.0.0.1:<owner port>/${company}`);

function CHARTER() {
	return [
		'Established businesses deserve a digital presence as good as the business behind it.',
		'Reno exists to close the gap between how good these businesses are and how they appear, are found and are trusted online.',
		'We renovate the websites of established small businesses and local service companies: we rebuild dated sites so they look credible, load fast and turn visitors into enquiries.',
	].join(' ');
}

function THREAD() {
	const code = ['```', '# Reno — Charter', '', '## Purpose', CHARTER(), '', '## What we do', ...Array.from({ length: 24 }, (_, i) => `- Commitment ${i + 1}: every page is checked at phone width before anyone sees it.`), '```'].join('\n');
	return [
		['owner', 'exec', 'Is the Reno charter showing in Company > Charter? No, it is empty.'],
		['exec', 'owner', `Here is the charter in one copyable block:\n\n${code}\n\nNeeds you: publish it in Company › Charter.`],
		['owner', 'exec', 'Archive the old sites please, my Library is full of them.'],
		['exec', 'owner', "Cleared the old sites and notes from your Library by retiring 69 stale records; the new studio's two site jobs are under way.\n\n- Reno's own site.\n- Taylor & Ward version 4, with the full-resolution original photos in place of the squeezed ones.\n\nI'll tell you when a first look is ready."],
		['exec', 'studio-direction', 'Owner says the Anthropic connection was re-checked and the designer cooldown is now null. Please resume the Reno site on Opus. If it fails again with the same grant error, report and I will switch to Sonnet 5.5 on owner say-so.'],
		['studio-direction', 'exec', "Reno site version 5 is blocked on the same revoked Opus 5.5 connection: I resumed it after the cooldown and it failed again ('account connection is no longer granted to the company'); retrying will keep failing."],
		['exec', 'owner', 'The company has spent $100.14 of its $100 ceiling, so charged work has stopped. Raise it in Company › Limits and I will carry on.'],
		['owner', 'exec', 'I raised the spend cap and renamed the company. Carry on with the Reno site.'],
		['exec', 'owner', Array.from({ length: 12 }, (_, i) => `Paragraph ${i + 1}: the current page isn't conveying anything specific. The look is a generic quiet-editorial skin: cream paper, serif type, one clay accent.`).join('\n\n')],
	];
}
