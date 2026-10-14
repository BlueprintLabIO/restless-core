#!/usr/bin/env node
// Seed a `_test` company on a running restless-dev stack so it looks like a company in real use:
// Reno, a studio that renovates small-business websites. Long names, a lead with two staff, a goal
// with 30 finished pieces of Work, Work under way and blocked, 80 Library files, long Exec threads
// with code blocks and an error, and direction requests.
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
import { readFileSync, mkdirSync, writeFileSync } from 'node:fs';
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
];
const people = (await daemon({ cmd: 'people' })).actors ?? (await daemon({ cmd: 'people' }));
const existing = new Set((Array.isArray(people) ? people : []).map((actor) => actor.id ?? actor.actor_id));
for (const [id, role, display] of roster) {
	if (!existing.has(id))
		await tryDaemon({ cmd: 'actor-create', as_actor: id, role, name: display, actor: 'exec', reason: 'Reno-scale seed' });
}
const teams = (await daemon({ cmd: 'teams' })).teams;
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
		['exec', 'owner', '[budget] the company has spent $100.14 of its $100.00 ceiling; the owner must raise it before charged work continues'],
		['owner', 'exec', 'I raised the spend cap and renamed the company. Carry on with the Reno site.'],
		['exec', 'owner', Array.from({ length: 12 }, (_, i) => `Paragraph ${i + 1}: the current page isn't conveying anything specific. The look is a generic quiet-editorial skin: cream paper, serif type, one clay accent.`).join('\n\n')],
	];
}
