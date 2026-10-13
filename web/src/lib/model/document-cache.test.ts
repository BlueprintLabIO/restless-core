import assert from 'node:assert/strict';
import test from 'node:test';
import { QueryClient, QueryObserver } from '@tanstack/query-core';
import * as documentCache from './document-cache.ts';

const { failClosedDocumentRead, sameDocumentTarget } = documentCache;

test('an authoritative document denial clears a previously successful view and list item', () => {
	const client = new QueryClient();
	const companyId = 'acme';
	const documentId = 'document-a';
	const detailKey = ['document', companyId, documentId];
	client.setQueryData(detailKey, { secret: 'previously readable' });
	client.setQueryData(['documents', companyId], {
		pages: [
			{
				items: [{ document: { id: documentId } }, { document: { id: 'document-b' } }],
				next_cursor: null
			}
		],
		pageParams: [null]
	});
	assert.equal(
		failClosedDocumentRead(
			client,
			Object.assign(new Error('gone'), { status: 404 }),
			companyId,
			documentId
		),
		true
	);
	assert.equal(client.getQueryData(detailKey), undefined);
	assert.deepEqual(
		(
			client.getQueryData<{ pages: Array<{ items: Array<{ document: { id: string } }> }> }>([
				'documents',
				companyId
			])?.pages[0]?.items ?? []
		).map((item) => item.document.id),
		['document-b']
	);
});

test('a company denial clears every document while a transient failure preserves stale evidence', () => {
	const client = new QueryClient();
	const companyId = 'acme';
	client.setQueryData(['document', companyId, 'document-a'], { secret: 'a' });
	client.setQueryData(['document', companyId, 'document-b'], { secret: 'b' });

	assert.equal(
		failClosedDocumentRead(
			client,
			Object.assign(new Error('down'), { status: 503 }),
			companyId,
			'document-a'
		),
		false
	);
	assert.deepEqual(client.getQueryData(['document', companyId, 'document-a']), { secret: 'a' });

	assert.equal(
		failClosedDocumentRead(
			client,
			Object.assign(new Error('signed out'), { status: 401 }),
			companyId,
			'document-a'
		),
		true
	);
	assert.equal(client.getQueryData(['document', companyId, 'document-a']), undefined);
	assert.equal(client.getQueryData(['document', companyId, 'document-b']), undefined);

	client.setQueryData(['document', companyId, 'document-a'], { secret: 'a-again' });
	client.setQueryData(['document', companyId, 'document-b'], { secret: 'b-again' });
	client.setQueryData(['document-principal', companyId], {
		actor_id: 'alice',
		cache_partition: 'stale'
	});
	assert.equal(
		failClosedDocumentRead(
			client,
			Object.assign(new Error('company forbidden'), { status: 403 }),
			companyId,
			'document-a'
		),
		true
	);
	assert.equal(client.getQueryData(['document', companyId, 'document-a']), undefined);
	assert.equal(client.getQueryData(['document', companyId, 'document-b']), undefined);
	assert.equal(client.getQueryData(['document-principal', companyId]), undefined);
});

test('late completions only match the document target that started them', () => {
	const target = { companyId: 'acme', documentId: 'document-a' };
	assert.equal(sameDocumentTarget(target, 'acme', 'document-a'), true);
	assert.equal(sameDocumentTarget(target, 'acme', 'document-b'), false);
	assert.equal(sameDocumentTarget(target, 'other', 'document-a'), false);
});

test('a read on screen is kept when refused, so its refusal shows instead of a skeleton', async () => {
	const client = new QueryClient();
	const companyId = 'acme';
	const denied = Object.assign(new Error('gone'), { status: 404 });
	// The page watches the document; a sibling read for it (its comments) is refused first.
	const detail = new QueryObserver(client, {
		queryKey: ['document', companyId, 'document-a'],
		retry: false,
		queryFn: async () => {
			await new Promise((resolve) => setTimeout(resolve, 5));
			throw denied;
		}
	});
	const unsubscribe = detail.subscribe(() => undefined);
	client.setQueryData(['document-comments', companyId, 'document-a'], { stale: true });
	failClosedDocumentRead(client, denied, companyId, 'document-a');
	await new Promise((resolve) => setTimeout(resolve, 30));
	assert.equal(detail.getCurrentResult().error, denied, 'the page sees the refusal');
	assert.equal(
		client.getQueryData(['document-comments', companyId, 'document-a']),
		undefined,
		'what nothing watches is cleared'
	);
	unsubscribe();
});
