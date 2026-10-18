import assert from 'node:assert/strict';
import test from 'node:test';
import { attachmentReadKind, distinctKept, type RetainedAttachment } from './attachments.ts';

test('the same file kept twice lists once, newest first', () => {
	const row = (id: string, name: string, sha: string | undefined, sender = 'owner') =>
		({
			attachment_id: id,
			sender_actor_id: sender,
			target_actor_id: 'exec',
			canonical_name: name,
			canonical_media_type: 'text/markdown',
			size_bytes: 10,
			message_id: null,
			content_sha256: sha,
			created_at: '2026-10-18T00:00:00Z',
			purge_requested_at: null,
			purged_at: null
		}) as RetainedAttachment;
	const ids = distinctKept([
		row('new', 'draft.md', 'a'),
		row('old', 'draft.md', 'a'),
		row('edited', 'draft.md', 'b'),
		row('renamed', 'other.md', 'a'),
		row('theirs', 'draft.md', 'a', 'baloo'),
		row('legacy1', 'x.md', undefined),
		row('legacy2', 'x.md', undefined)
	]).map((r) => r.attachment_id);
	assert.deepEqual(ids, ['new', 'edited', 'renamed', 'theirs', 'legacy1', 'legacy2']);
});

/* A kept file is read into the cockpit's own origin, so only passive types may open in place:
 * anything a browser renders as a document would run with the owner's session. */
test('only passive kinds of kept file open in place', () => {
	assert.equal(attachmentReadKind('image/png', 'logo.png'), 'image');
	assert.equal(attachmentReadKind('image/jpeg', 'receipt.jpg'), 'image');
	assert.equal(attachmentReadKind('application/pdf', 'contract.pdf'), 'pdf');
	assert.equal(attachmentReadKind('application/octet-stream', 'contract.pdf'), 'pdf');
	assert.equal(attachmentReadKind('video/mp4', 'walkthrough.mp4'), 'media');

	for (const [type, name] of [
		['image/svg+xml', 'logo.svg'],
		['text/html', 'page.html'],
		['application/xhtml+xml', 'page.xhtml'],
		['text/xml', 'feed.xml'],
		['audio/x-fake+xml', 'clip.xml'],
		['video/mp4+xml', 'clip.mp4'],
		['application/octet-stream', 'page.html'],
		['text/plain', 'notes.txt']
	])
		assert.equal(attachmentReadKind(type, name), null, `${type} must download`);
});
