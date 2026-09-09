import assert from 'node:assert/strict';
import test from 'node:test';
import { QueryClient } from '@tanstack/query-core';
// @ts-expect-error Node's strip-only test runner needs the explicit TypeScript suffix.
import * as documentCache from './document-cache.ts';

const {
	allowDocumentDraftPersistence,
	canPersistDocumentDraft,
	documentAccessToken,
	documentDraftStorageKey,
	failClosedDocumentRead,
	sameDocumentTarget
} = documentCache;

class MemoryStorage {
	#values = new Map<string, string>();

	get length(): number {
		return this.#values.size;
	}

	getItem(key: string): string | null {
		return this.#values.get(key) ?? null;
	}

	key(index: number): string | null {
		return [...this.#values.keys()][index] ?? null;
	}

	removeItem(key: string): void {
		this.#values.delete(key);
	}

	setItem(key: string, value: string): void {
		this.#values.set(key, value);
	}
}

test('an authoritative document denial clears a previously successful view, list item, and draft', () => {
	const client = new QueryClient();
	const storage = new MemoryStorage();
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
	const draftKey = documentDraftStorageKey(companyId, 'alice', 'opaque-a', documentId);
	storage.setItem(draftKey, 'local secret');
	const requestStartedBeforeDenial = documentAccessToken(companyId, documentId);

	assert.equal(
		failClosedDocumentRead(
			client,
			Object.assign(new Error('gone'), { status: 404 }),
			companyId,
			documentId,
			storage
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
	assert.equal(storage.getItem(draftKey), null);
	assert.equal(canPersistDocumentDraft(companyId, documentId), false);
	assert.equal(
		allowDocumentDraftPersistence(companyId, documentId, requestStartedBeforeDenial),
		false
	);
	assert.equal(canPersistDocumentDraft(companyId, documentId), false);
	allowDocumentDraftPersistence(companyId, documentId, documentAccessToken(companyId, documentId));
	assert.equal(canPersistDocumentDraft(companyId, documentId), true);
});

test('a company denial clears every document while a transient failure preserves stale evidence', () => {
	const client = new QueryClient();
	const storage = new MemoryStorage();
	const companyId = 'acme';
	const draftA = documentDraftStorageKey(companyId, 'alice', 'opaque-a', 'document-a');
	const draftB = documentDraftStorageKey(companyId, 'alice', 'opaque-a', 'document-b');
	client.setQueryData(['document', companyId, 'document-a'], { secret: 'a' });
	client.setQueryData(['document', companyId, 'document-b'], { secret: 'b' });
	storage.setItem(draftA, 'a');
	storage.setItem(draftB, 'b');

	assert.equal(
		failClosedDocumentRead(
			client,
			Object.assign(new Error('down'), { status: 503 }),
			companyId,
			'document-a',
			storage
		),
		false
	);
	assert.deepEqual(client.getQueryData(['document', companyId, 'document-a']), { secret: 'a' });
	assert.equal(storage.getItem(draftA), 'a');

	assert.equal(
		failClosedDocumentRead(
			client,
			Object.assign(new Error('signed out'), { status: 401 }),
			companyId,
			'document-a',
			storage
		),
		true
	);
	assert.equal(client.getQueryData(['document', companyId, 'document-a']), undefined);
	assert.equal(client.getQueryData(['document', companyId, 'document-b']), undefined);
	assert.equal(storage.getItem(draftA), null);
	assert.equal(storage.getItem(draftB), null);
	assert.equal(canPersistDocumentDraft(companyId, 'document-b'), false);
	allowDocumentDraftPersistence(companyId, null, documentAccessToken(companyId));
	assert.equal(canPersistDocumentDraft(companyId, 'document-b'), true);

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
			'document-a',
			storage
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
