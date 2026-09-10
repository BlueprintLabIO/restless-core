import assert from 'node:assert/strict';
import test from 'node:test';
import { QueryClient } from '@tanstack/query-core';
// @ts-expect-error Node's strip-only test runner needs the explicit TypeScript suffix.
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
