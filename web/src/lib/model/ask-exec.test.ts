import assert from 'node:assert/strict';
import test from 'node:test';
import { ASK_EXEC_EVENT, askExecAboutItem, linkDraft, replyInExec } from './ask-exec.ts';

function captured(run: () => void): { draft: string; reply?: { prefix: string } } {
	const target = new EventTarget();
	let detail: { draft: string; reply?: { prefix: string } } | null = null;
	target.addEventListener(ASK_EXEC_EVENT, (event) => {
		detail = (event as CustomEvent).detail;
	});
	const previous = (globalThis as { window?: unknown }).window;
	(globalThis as { window?: unknown }).window = target;
	try {
		run();
	} finally {
		(globalThis as { window?: unknown }).window = previous;
	}
	assert.ok(detail, 'the layout is asked');
	return detail;
}

test('a linked draft names the thing and keeps Markdown link brackets out of its title', () => {
	assert.equal(linkDraft('Fix [DNS]', '/c/work/w1'), 'About [Fix DNS](/c/work/w1): ');
});

test('Ask Exec links the item into the one conversation, with nothing to record', () => {
	const detail = captured(() => askExecAboutItem('co 1', { id: 'i/1', title: 'Pick a name' }));
	assert.equal(detail.draft, 'About [Pick a name](/co%201?item=i%2F1): ');
	assert.equal(detail.reply, undefined);
});

test('Reply opens the same conversation and carries the link its answer follows', () => {
	const detail = captured(() =>
		replyInExec('co', { id: 'i1', title: 'Which domain?' }, async () => {})
	);
	assert.equal(detail.draft, 'Reply to [Which domain?](/co?item=i1): ');
	assert.equal(detail.reply?.prefix, detail.draft);
});
