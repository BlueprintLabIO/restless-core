import assert from 'node:assert/strict';
import test from 'node:test';
// @ts-expect-error Node's strip-only test runner needs the explicit TypeScript suffix.
import * as documentsModel from './documents.ts';
import type { DocumentContent, DocumentRow, EditableDocumentBlock } from './documents';

const {
	commentContent,
	checkpointMetadataInput,
	createDocument,
	documentText,
	editableBlocks,
	emptyDocument,
	editorContent,
	getDocumentCommentThreads,
	getDocumentParticipants,
	getDocuments,
	getDocumentVersions,
	isDocumentConflict,
	isRetryableDocumentFailure,
	pendingDocumentCommand,
	pendingDocumentCreation,
	pendingDocumentMutation,
	removeDocumentParticipant,
	resolveDocumentCommentThread,
	setDocumentParticipant,
	updateDocumentMetadata
} = documentsModel;

test('document reads use the bounded paired cursor contracts', async () => {
	const original = globalThis.fetch;
	const urls: string[] = [];
	globalThis.fetch = (async (input: string | URL | Request) => {
		urls.push(String(input));
		return new Response(JSON.stringify({ items: [], next_cursor: null }), {
			status: 200,
			headers: { 'content-type': 'application/json' }
		});
	}) as typeof fetch;
	try {
		await getDocuments(
			'acme group',
			{ updated_at: '2026-09-09T01:02:03Z', id: '00000000-0000-4000-8000-000000000001' },
			17,
			true
		);
		await getDocumentVersions('acme group', 'doc/id', { version_number: 7 }, 11);
		await getDocumentCommentThreads(
			'acme group',
			'doc/id',
			{ created_at: '2026-09-08T01:02:03Z', id: '00000000-0000-4000-8000-000000000002' },
			9
		);
		await getDocumentParticipants(
			'acme group',
			'doc/id',
			{ added_at: '2026-09-07T01:02:03Z', actor_id: 'lead one' },
			8
		);
		assert.deepEqual(urls, [
			'/api/companies/acme%20group/documents?limit=17&include_archived=true&before_updated_at=2026-09-09T01%3A02%3A03Z&before_document_id=00000000-0000-4000-8000-000000000001',
			'/api/companies/acme%20group/documents/doc%2Fid/versions?limit=11&before_version_number=7',
			'/api/companies/acme%20group/documents/doc%2Fid/comments?limit=9&after_created_at=2026-09-08T01%3A02%3A03Z&after_id=00000000-0000-4000-8000-000000000002',
			'/api/companies/acme%20group/documents/doc%2Fid/participants?limit=8&after_added_at=2026-09-07T01%3A02%3A03Z&after_actor_id=lead+one'
		]);
	} finally {
		globalThis.fetch = original;
	}
});

test('document mutations put the UUID command in the header and keep identity out of bodies', async () => {
	const original = globalThis.fetch;
	const requests: Array<{ url: string; init: RequestInit | undefined }> = [];
	const commandResult = {
		command_id: '00000000-0000-4000-8000-000000000003',
		document_id: '00000000-0000-4000-8000-000000000004',
		operation: 'document_create',
		result_id: '00000000-0000-4000-8000-000000000004',
		created_at: '2026-09-09T01:02:03Z'
	};
	globalThis.fetch = (async (input: string | URL | Request, init?: RequestInit) => {
		requests.push({ url: String(input), init });
		return new Response(JSON.stringify(commandResult), {
			status: 200,
			headers: { 'content-type': 'application/json' }
		});
	}) as typeof fetch;
	try {
		const commandId = '00000000-0000-4000-8000-000000000003';
		const receipt = await createDocument(
			'acme',
			{
				title: 'Launch decision',
				kind: 'decision_note',
				visibility: 'company',
				linked_room_id: null,
				inherit_room_visibility: false,
				content_json: { type: 'doc', content: [] },
				reason: 'Created by owner'
			},
			commandId
		);
		await resolveDocumentCommentThread('acme', 'document', 'thread', 4, commandId);
		assert.deepEqual(receipt, commandResult);

		assert.equal(requests[0]?.url, '/api/companies/acme/documents');
		assert.equal(requests[0]?.init?.method, 'POST');
		assert.equal(new Headers(requests[0]?.init?.headers).get('Idempotency-Key'), commandId);
		assert.equal(requests[0]?.init?.credentials, 'same-origin');
		assert.equal(requests[0]?.init?.cache, 'no-store');
		assert.deepEqual(JSON.parse(String(requests[0]?.init?.body)), {
			title: 'Launch decision',
			kind: 'decision_note',
			visibility: 'company',
			linked_room_id: null,
			inherit_room_visibility: false,
			content_json: { type: 'doc', content: [] },
			reason: 'Created by owner'
		});
		assert.deepEqual(JSON.parse(String(requests[1]?.init?.body)), {
			expected_thread_version: 4
		});
		for (const request of requests) {
			const body = String(request.init?.body);
			assert.equal(body.includes('actor_id'), false);
			assert.equal(body.includes('company_id'), false);
		}
	} finally {
		globalThis.fetch = original;
	}
});

test('metadata and participant commands use the hardened lifecycle-free write shapes', async () => {
	const original = globalThis.fetch;
	const requests: Array<{ url: string; init: RequestInit | undefined }> = [];
	globalThis.fetch = (async (input: string | URL | Request, init?: RequestInit) => {
		requests.push({ url: String(input), init });
		return new Response(
			JSON.stringify({
				command_id: '00000000-0000-4000-8000-000000000005',
				document_id: 'document',
				operation: 'document_metadata_update',
				result_id: 'document',
				created_at: '2026-09-09T01:02:03Z'
			}),
			{ status: 200, headers: { 'content-type': 'application/json' } }
		);
	}) as typeof fetch;
	try {
		const commandId = '00000000-0000-4000-8000-000000000005';
		await updateDocumentMetadata(
			'acme',
			'document',
			{
				expected_version: 5,
				title: 'New title',
				kind: 'plan',
				visibility: 'participants',
				linked_room_id: null,
				inherit_room_visibility: false
			},
			commandId
		);
		await setDocumentParticipant('acme', 'document', 'lead/one', 6, 'edit', commandId);
		await removeDocumentParticipant('acme', 'document', 'lead/one', 7, commandId);

		assert.equal(requests[0]?.init?.method, 'PATCH');
		const metadataBody = JSON.parse(String(requests[0]?.init?.body));
		assert.equal('status' in metadataBody, false);
		assert.deepEqual(metadataBody, {
			expected_version: 5,
			title: 'New title',
			kind: 'plan',
			visibility: 'participants',
			linked_room_id: null,
			inherit_room_visibility: false
		});
		assert.equal(
			requests[1]?.url,
			'/api/companies/acme/documents/document/participants/lead%2Fone'
		);
		assert.equal(requests[1]?.init?.method, 'PUT');
		assert.deepEqual(JSON.parse(String(requests[1]?.init?.body)), {
			expected_document_version: 6,
			access: 'edit'
		});
		assert.equal(requests[2]?.init?.method, 'DELETE');
		assert.deepEqual(JSON.parse(String(requests[2]?.init?.body)), {
			expected_document_version: 7
		});
	} finally {
		globalThis.fetch = original;
	}
});

test('structured editing changes one supported block and preserves unknown content byte-for-byte', () => {
	const source: DocumentContent = {
		type: 'doc',
		content: [
			{
				type: 'heading',
				attrs: { block_id: 'heading-1', level: 2, retained: 'yes' },
				content: [{ type: 'text', text: 'Old heading', marks: [{ type: 'strong' }] }]
			},
			{
				type: 'callout',
				attrs: { block_id: 'future-1', tone: 'evidence' },
				content: [{ type: 'text', text: 'A future schema node' }]
			}
		]
	};
	const blocks = editableBlocks(source);
	assert.equal(blocks[0]?.kind, 'heading');
	assert.equal(blocks[1]?.readOnly, true);
	const edited = blocks.map((block): EditableDocumentBlock =>
		block.id === 'heading-1' ? { ...block, text: 'New heading', changed: true } : block
	);
	const output = editorContent(edited);
	assert.deepEqual(output.content[0], {
		type: 'heading',
		attrs: { block_id: 'heading-1', level: 2, retained: 'yes' },
		content: [{ type: 'text', text: 'New heading' }]
	});
	assert.deepEqual(output.content[1], source.content[1]);
	assert.notEqual(output.content[1], source.content[1]);
	assert.equal(documentText(output), 'New heading\nA future schema node');
});

test('comments only create mention nodes for known, exact actor ids', () => {
	const output = commentContent(
		'Please ask @lead-one and leave @unknown as text.',
		new Set(['lead-one'])
	);
	const inline = output.content[0]?.content ?? [];
	assert.deepEqual(inline[1], {
		type: 'mention',
		attrs: { actor_id: 'lead-one', label: '@lead-one' }
	});
	assert.equal(
		inline.some((node) => node.type === 'mention' && node.attrs?.actor_id === 'unknown'),
		false
	);
	assert.equal(documentText(output), 'Please ask  and leave @unknown as text.');
});

test('HTTP 409 responses are surfaced as explicit document conflicts', async () => {
	const original = globalThis.fetch;
	globalThis.fetch = (async () =>
		new Response(JSON.stringify({ code: 'document_conflict', message: 'Document changed.' }), {
			status: 409,
			headers: { 'content-type': 'application/json' }
		})) as typeof fetch;
	try {
		await assert.rejects(
			() => getDocuments('acme'),
			(error: unknown) =>
				isDocumentConflict(error) &&
				(error as Error & { code?: string }).code === 'document_conflict' &&
				error.message === 'Document changed.'
		);
	} finally {
		globalThis.fetch = original;
	}
});

test('retryable commands retain identity only while their semantic input is unchanged', () => {
	const first = pendingDocumentCommand(null, '{"title":"Plan"}');
	const replay = pendingDocumentCommand(first, '{"title":"Plan"}');
	const changed = pendingDocumentCommand(first, '{"title":"Revised plan"}');

	assert.equal(replay, first);
	assert.equal(replay.id, first.id);
	assert.notEqual(changed.id, first.id);
	assert.equal(isRetryableDocumentFailure(new TypeError('connection lost')), true);
	assert.equal(isRetryableDocumentFailure(Object.assign(new Error('slow'), { status: 429 })), true);
	assert.equal(isRetryableDocumentFailure(Object.assign(new Error('down'), { status: 503 })), true);
	assert.equal(
		isRetryableDocumentFailure(Object.assign(new Error('conflict'), { status: 409 })),
		false
	);
	assert.equal(
		isRetryableDocumentFailure(Object.assign(new Error('invalid'), { status: 422 })),
		false
	);
});

test('document creation freezes its generated block payload across a lost-response retry', () => {
	const semanticInput = {
		title: 'Operating plan',
		kind: 'plan' as const,
		visibility: 'company' as const,
		linked_room_id: null,
		inherit_room_visibility: false,
		reason: 'Created'
	};
	const first = pendingDocumentCreation(null, semanticInput);
	const replay = pendingDocumentCreation(first, semanticInput);
	const changed = pendingDocumentCreation(first, { ...semanticInput, title: 'Revised plan' });

	assert.equal(replay, first);
	assert.equal(replay.input, first.input);
	assert.equal(replay.command, first.command);
	assert.equal(replay.command.fingerprint, JSON.stringify(first.input));
	assert.equal(
		replay.input.content_json.content[0]?.attrs?.block_id,
		first.input.content_json.content[0]?.attrs?.block_id
	);
	assert.notEqual(changed.command.id, first.command.id);
	assert.notEqual(
		changed.input.content_json.content[0]?.attrs?.block_id,
		first.input.content_json.content[0]?.attrs?.block_id
	);
	assert.notEqual(
		emptyDocument().content[0]?.attrs?.block_id,
		first.input.content_json.content[0]?.attrs?.block_id
	);
});

test('generated comment content and command identity are reused for an identical retry', () => {
	let builds = 0;
	const semanticFingerprint = JSON.stringify({
		documentId: 'document-a',
		threadId: 'thread-a',
		body: 'Check this'
	});
	const createInput = () => {
		builds += 1;
		return { content_json: commentContent('Check this', new Set()) };
	};
	const first = pendingDocumentMutation(null, semanticFingerprint, createInput);
	const replay = pendingDocumentMutation(first, semanticFingerprint, createInput);

	assert.equal(replay, first);
	assert.equal(builds, 1);
	assert.equal(replay.command.id, first.command.id);
	assert.equal(
		replay.input.content_json.content[0]?.attrs?.block_id,
		first.input.content_json.content[0]?.attrs?.block_id
	);
});

test('checkpoint metadata uses the edit base instead of borrowing a later fetched version', () => {
	const source = {
		id: 'document-a',
		title: 'Server title',
		kind: 'brief',
		status: 'draft',
		visibility: 'participants',
		linked_room_id: 'room-a',
		inherit_room_visibility: true,
		owner_actor_id: 'owner',
		current_named_version_id: 'version-a',
		created_by_actor_id: 'owner',
		created_at: '2026-09-09T00:00:00Z',
		updated_at: '2026-09-09T00:00:00Z',
		version: 7
	} satisfies DocumentRow;
	const input = checkpointMetadataInput(source, 7, 'Local title', 'plan', true, false);

	assert.deepEqual(input, {
		expected_version: 8,
		title: 'Local title',
		kind: 'brief',
		visibility: 'participants',
		linked_room_id: 'room-a',
		inherit_room_visibility: true
	});
	assert.equal(checkpointMetadataInput(source, 7, source.title, source.kind, false, false), null);
});
