// Queue, stop/restart the isolated official stack, then verify the original Native Document decision.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFile, writeFile } from 'node:fs/promises';
import { randomUUID } from 'node:crypto';
const origin = process.env.RESTLESS_SMOKE_ORIGIN,
	company = process.env.RESTLESS_SMOKE_COMPANY,
	home = process.env.RESTLESS_HOME,
	path = process.env.RESTLESS_RESTART_RECORD;
assert(
	origin &&
		new URL(origin).hostname === '127.0.0.1' &&
		company?.endsWith('_test') &&
		home?.includes('/.restless-dev/') &&
		path
);
const mode = process.argv[2];
assert(['queue', 'paused', 'verify'].includes(mode));
const pg = createRequire(new URL('../../services/identity/package.json', import.meta.url))('pg');
const adminUrl = (await readFile(home + '/orgintel.toml', 'utf8')).match(
	/database_url\s*=\s*"([^"]+)"/
)[1];
const admin = new pg.Client({ connectionString: adminUrl });
await admin.connect();
const databases = await admin.query(
	"SELECT datname FROM pg_database WHERE datname LIKE '%\\_' || $1 || '\\_%' ESCAPE '\\'",
	[company]
);
await admin.end();
assert.equal(databases.rowCount, 1);
const url = new URL(adminUrl);
url.pathname = '/' + databases.rows[0].datname;
const db = new pg.Client({ connectionString: url.toString() });
await db.connect();
await db.query('SET search_path TO "' + company + '"');
const api = origin + '/api/companies/' + company;
async function json(path, body, method = 'POST') {
	const response = await fetch(
		api + path,
		body
			? {
					method,
					headers: {
						Origin: origin,
						'Content-Type': 'application/json',
						'Idempotency-Key': randomUUID()
					},
					body: JSON.stringify(body)
				}
			: {}
	);
	const result = await response.json();
	assert(response.ok, JSON.stringify({ status: response.status, result }));
	return result;
}
try {
	if (mode === 'queue') {
		const created = await json('/documents', {
			title: 'Native review across daemon restart',
			kind: 'review',
			visibility: 'company',
			linked_room_id: null,
			inherit_room_visibility: false,
			reason: 'Isolated restart verification',
			content_json: {
				type: 'doc',
				content: [
					{
						type: 'paragraph',
						attrs: { block_id: randomUUID() },
						content: [
							{
								type: 'text',
								text: 'The exact saved native document remains unchanged during its Undo window.'
							}
						]
					}
				]
			}
		});
		const document = created.document_id,
			view = await json('/documents/' + document);
		await json(
			'/documents/' + document + '/participants/exec',
			{ expected_document_version: view.document.version, access: 'edit' },
			'PUT'
		);
		const shared = await json('/documents/' + document),
			review = { review: { id: randomUUID(), version: 1 } };
		// Actor-authored fixture with its final identity; the immutable review requester is never rewritten.
		await db.query('BEGIN');
		try {
			await db.query(
				"UPDATE native_documents SET status='in_review',version=version+1 WHERE id=$1",
				[document]
			);
			await db.query(
				"INSERT INTO native_document_reviews(id,document_id,requested_version_id,requested_by_actor_id,reviewer_actor_id,summary) VALUES($1,$2,$3,'exec','owner','Review the restart fixture')",
				[review.review.id, document, shared.current_version.version.id]
			);
			await db.query('COMMIT');
		} catch (error) {
			await db.query('ROLLBACK');
			throw error;
		}
		const current = await json('/documents/' + document),
			note = 'The saved review note survives the daemon restart.';
		const response = await json(
			'/documents/' + document + '/reviews/' + review.review.id + '/pending-decision',
			{
				id: randomUUID(),
				decision: {
					kind: 'accept_review',
					expected_document_version: current.document.version,
					expected_review_version: review.review.version,
					accepted_version_name: 'Reviewed restart result',
					feedback: note
				}
			}
		);
		assert.equal(response.state, 'pending');
		assert(Date.parse(response.undo_until) > Date.now());
		const comments = (
			await db.query(
				'SELECT count(*)::int AS count FROM native_document_comments WHERE document_id=$1',
				[document]
			)
		).rows[0].count;
		assert.equal(comments, 0);
		await writeFile(path, JSON.stringify({ ...response, company, document, note }), {
			mode: 0o600
		});
		console.log('Native review durably pending, no comment or acceptance before its deadline');
	} else {
		const record = JSON.parse(await readFile(path, 'utf8'));
		assert.equal(record.company, company);
		let result;
		for (let i = 0; i < (mode === 'verify' ? 100 : 1); i++) {
			result = (
				await db.query(
					'SELECT d.state,d.undo_until=$2::timestamptz AS original_deadline,n.status::text AS document_status FROM owner_document_decisions d JOIN native_documents n ON n.id=d.document_id WHERE d.id=$1',
					[record.id, record.undo_until]
				)
			).rows[0];
			if (mode === 'paused' || result.state === 'applied') break;
			await new Promise((r) => setTimeout(r, 100));
		}
		assert.equal(result.original_deadline, true);
		const notes = (
			await db.query(
				"SELECT id,mentioned_actor_ids FROM native_document_comments WHERE document_id=$1 AND position($2 in plain_text)>0 AND author_actor_id='owner'",
				[record.document, record.note]
			)
		).rows;
		if (mode === 'paused') {
			assert.equal(result.state, 'pending');
			assert.equal(result.document_status, 'in_review');
			assert.equal(notes.length, 0);
			assert(Date.parse(record.undo_until) < Date.now());
			console.log('Stopped daemon retains the expired original intent, with no note or acceptance');
		} else {
			assert.equal(result.state, 'applied');
			assert.equal(result.document_status, 'accepted');
			assert.equal(notes.length, 1);
			assert.deepEqual(notes[0].mentioned_actor_ids, ['exec']);
			const mentions = (
				await db.query(
					"SELECT count(*)::int AS count FROM native_document_mentions WHERE comment_id=$1 AND mentioned_actor_id='exec'",
					[notes[0].id]
				)
			).rows[0].count;
			assert.equal(mentions, 1);
			const accepted = (
				await db.query(
					"SELECT count(*)::int AS count FROM events WHERE document_id=$1 AND kind='document.review.accepted.v1'",
					[record.document]
				)
			).rows[0].count;
			assert.equal(accepted, 1);
			console.log(
				JSON.stringify({
					scenario:
						'native review restart keeps original deadline, accepts once and sends one note to its requester',
					...result,
					noteCount: notes.length,
					mentionCount: mentions,
					acceptanceCount: accepted
				})
			);
		}
	}
} finally {
	await db.end();
}
