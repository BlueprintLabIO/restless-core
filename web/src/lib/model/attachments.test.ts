import assert from 'node:assert/strict';
import test from 'node:test';
import { attachmentReadKind } from './attachments.ts';

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
