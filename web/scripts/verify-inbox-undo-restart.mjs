// Operational restart companion: queue, stop/restart the isolated official dev stack, then verify.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFile, writeFile } from 'node:fs/promises';
import { randomUUID } from 'node:crypto';
const origin = process.env.RESTLESS_SMOKE_ORIGIN,
	company = process.env.RESTLESS_SMOKE_COMPANY,
	home = process.env.RESTLESS_HOME;
assert(
	origin &&
		new URL(origin).hostname === '127.0.0.1' &&
		company?.endsWith('_test') &&
		home?.includes('/.restless-dev/')
);
const mode = process.argv[2],
	path = process.env.RESTLESS_RESTART_RECORD;
assert(path && ['queue', 'paused', 'verify'].includes(mode));
const pg = createRequire(new URL('../../services/identity/package.json', import.meta.url))('pg');
const adminUrl = (await readFile(home + '/orgintel.toml', 'utf8')).match(
	/database_url\s*=\s*"([^"]+)"/
)[1];
const admin = new pg.Client({ connectionString: adminUrl });
await admin.connect();
const names = await admin.query(
	"SELECT datname FROM pg_database WHERE datname LIKE '%\\_' || $1 || '\\_%' ESCAPE '\\'",
	[company]
);
await admin.end();
assert.equal(names.rowCount, 1);
const url = new URL(adminUrl);
url.pathname = '/' + names.rows[0].datname;
const db = new pg.Client({ connectionString: url.toString() });
await db.connect();
await db.query('SET search_path TO "' + company + '"');
try {
	if (mode === 'queue') {
		const work = randomUUID(),
			handoff = randomUUID();
		await db.query(
			"INSERT INTO work(id,owner_id,title,outcome,status,resolution,priority,commissioned_by) VALUES($1,'exec','Undo across daemon restart','Confirm an isolated human step','blocked','Waiting for owner',1,'exec')",
			[work]
		);
		await db.query(
			"INSERT INTO owner_handoffs(id,work_id,requested_by,category,requested_action,prepared_state,resume_condition) VALUES($1,$2,'exec','mfa','Confirm the restart fixture','Synthetic step','Owner confirms')",
			[handoff, work]
		);
		const attention = await (
			await fetch(origin + '/api/companies/' + company + '/attention')
		).json();
		const item = attention.items.find((i) => i.source.reference === handoff);
		assert(item);
		const response = await fetch(
			origin + '/api/companies/' + company + '/handoffs/' + handoff + '/pending-decision',
			{
				method: 'POST',
				headers: { Origin: origin, 'Content-Type': 'application/json' },
				body: JSON.stringify({
					id: randomUUID(),
					expected_version: item.review_version,
					decision: { kind: 'complete_human_step' }
				})
			}
		);
		const saved = await response.json();
		assert(response.ok, JSON.stringify(saved));
		assert.equal(saved.state, 'pending');
		assert(Date.parse(saved.undo_until) > Date.now());
		assert.equal(
			(await db.query('SELECT state FROM owner_handoffs WHERE id=$1', [handoff])).rows[0].state,
			'pending'
		);
		await writeFile(path, JSON.stringify({ ...saved, company, work }), { mode: 0o600 });
		console.log('Pending restart fixture durably saved with its original deadline');
	} else if (mode === 'paused') {
		const record = JSON.parse(await readFile(path, 'utf8'));
		assert.equal(record.company, company);
		const result = (
			await db.query(
				'SELECT d.state,d.undo_until,d.undo_until=$2::timestamptz AS same_deadline,h.state AS handoff_state FROM owner_handoff_decisions d JOIN owner_handoffs h ON h.id=d.handoff_id WHERE d.id=$1',
				[record.id, record.undo_until]
			)
		).rows[0];
		assert.equal(result.state, 'pending');
		assert.equal(result.handoff_state, 'pending');
		assert(Date.parse(record.undo_until) < Date.now());
		assert.equal(result.same_deadline, true);
		console.log('Daemon stopped: expired original window remains pending, with no source effect');
	} else {
		const record = JSON.parse(await readFile(path, 'utf8'));
		assert.equal(record.company, company);
		let result;
		for (let i = 0; i < 100; i++) {
			result = (
				await db.query(
					'SELECT d.state,d.undo_until,d.undo_until=$2::timestamptz AS same_deadline,h.state AS handoff_state FROM owner_handoff_decisions d JOIN owner_handoffs h ON h.id=d.handoff_id WHERE d.id=$1',
					[record.id, record.undo_until]
				)
			).rows[0];
			if (result?.state === 'applied') break;
			await new Promise((r) => setTimeout(r, 100));
		}
		assert.equal(result.state, 'applied');
		assert.equal(result.handoff_state, 'resolved');
		assert.equal(result.same_deadline, true);
		const feedback = (
			await db.query('SELECT count(*)::int AS count FROM work_feedback WHERE work_id=$1', [
				record.work
			])
		).rows[0].count;
		assert.equal(feedback, 1);
		console.log(
			JSON.stringify({
				scenario: 'daemon restart retains original deadline and applies exactly once',
				deadline: record.undo_until,
				feedback,
				state: result.state
			})
		);
	}
} finally {
	await db.end();
}
