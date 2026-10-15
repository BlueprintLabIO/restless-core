import assert from 'node:assert/strict';
import test from 'node:test';
import {
	inboxPreviewTarget,
	inboxPreviewHref,
	inboxPreviewBack,
	inboxPreviewContext,
	inboxDiscussionHref
} from './inbox-preview.ts';
const origin = 'http://127.0.0.1:23296',
	company = 'ui_test';
test('files, editors and specific company settings open as native Inbox context', () => {
	for (const [href, kind, value] of [
		['/ui_test/library?file=a', 'file', 'a'],
		['/ui_test/library?path=%2Fcompany%2Foutputs%2Fbrief.md', 'path', '/company/outputs/brief.md'],
		['/ui_test/library?upload=u', 'upload', 'u'],
		['/ui_test/library/documents?document=d&inspect=comments&thread=t', 'document', 'd'],
		['/ui_test/library/sheets?sheet=s&inspect=history', 'sheet', 's'],
		['/ui_test/company/resources#authority', 'limits', 'authority'],
		['/ui_test/company/identity', 'identity', ''],
		['/ui_test/company/charter', 'charter', '']
	]) {
		const target = inboxPreviewTarget(company, href, origin);
		assert.equal(target?.kind, kind);
		assert.equal(target?.value, value);
		assert.equal(target?.href, href);
	}
});
test('ordinary navigation and cross-company or external destinations keep their ordinary routes', () => {
	for (const href of [
		'/ui_test/library',
		'/ui_test/library?show=docs',
		'/ui_test/work',
		'/ui_test/company',
		'/other_test/library?file=a',
		'https://outside.test/ui_test/library?file=a',
		'javascript:alert(1)',
		'/ui_test/library?file=a&path=%2Fcompany%2Fx',
		'/ui_test/library?path=%2Fetc%2Fpasswd',
		'/documents/ui_test/%ZZ'
	])
		assert.equal(inboxPreviewTarget(company, href, origin), null, href);
});
test('a preview preserves its item and conversation; closing it restores that exact context', () => {
	const current = new URL(origin + '/ui_test?item=review-a&conversation=review-a');
	const opened = inboxPreviewHref(current, '/ui_test/library?file=a');
	assert.equal(new URL(opened, origin).searchParams.get('item'), 'review-a');
	assert.equal(new URL(opened, origin).searchParams.get('conversation'), 'review-a');
	assert.equal(inboxPreviewBack(new URL(opened, origin)), current.pathname + current.search);
	assert.equal(current.searchParams.get('preview'), null);
	const discussed = new URL(
		inboxDiscussionHref(new URL(opened, origin), company, 'review-a'),
		origin
	);
	assert.equal(discussed.searchParams.get('preview'), '/ui_test/library?file=a');
	assert.equal(discussed.searchParams.get('conversation'), 'review-a');
});
test('legacy document links normalize before the shared editor reads their identity', () => {
	const target = inboxPreviewTarget(company, '/documents/ui_test/doc-a?inspect=comments', origin);
	assert.equal(target?.kind, 'document');
	assert.equal(new URL(target!.href, origin).searchParams.get('document'), 'doc-a');
	assert.equal(new URL(target!.href, origin).searchParams.get('inspect'), 'comments');
});

test('Exec receives the exact native preview context and ignores wrappers outside Inbox', () => {
	const current = new URL(
		origin +
			inboxPreviewHref(
				new URL(origin + '/ui_test?item=review-a'),
				'/ui_test/company/resources#spend'
			)
	);
	assert.equal(inboxPreviewContext(company, current)?.href, '/ui_test/company/resources#spend');
	current.pathname = '/ui_test/work';
	assert.equal(inboxPreviewContext(company, current), null);
	current.pathname = '/ui_test';
	current.searchParams.set('preview', 'https://outside.test/ui_test/company/resources');
	assert.equal(inboxPreviewContext(company, current), null);
});
