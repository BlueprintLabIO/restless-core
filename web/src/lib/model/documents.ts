import { responseFailure } from './failure.ts';
import type { GeneralAccess } from './general-access.ts';
export type DocumentKind =
	| 'brief'
	| 'plan'
	| 'decision_note'
	| 'report'
	| 'review'
	| 'handbook'
	| 'operating_note'
	| 'freeform';

export type DocumentStatus = 'draft' | 'in_review' | 'accepted' | 'archived';
export type DocumentVisibility = 'company' | 'participants';
export type DocumentAccess = 'read' | 'comment' | 'edit';
export type DocumentCommentStatus = 'open' | 'resolved';
export type DocumentCommentAnchorState = 'document' | 'active' | 'orphaned';
export type DocumentReviewStatus = 'requested' | 'accepted' | 'changes_requested' | 'stale';
export type DocumentRevisionScope = 'whole_document' | 'block';
export type DocumentRevisionStatus = 'proposed' | 'accepted' | 'rejected' | 'stale';

export interface DocumentMark {
	type: string;
	attrs?: Record<string, unknown>;
}

export interface DocumentNode {
	type: string;
	attrs?: Record<string, unknown>;
	content?: DocumentNode[];
	text?: string;
	marks?: DocumentMark[];
}

export interface DocumentContent extends DocumentNode {
	type: 'doc';
	content: DocumentNode[];
}

export interface DocumentRow {
	id: string;
	title: string;
	kind: DocumentKind;
	status: DocumentStatus;
	visibility: DocumentVisibility;
	/** What every person in the company may do without being added. */
	general_access: GeneralAccess;
	linked_room_id: string | null;
	inherit_room_visibility: boolean;
	owner_actor_id: string;
	current_named_version_id: string;
	created_by_actor_id: string;
	created_at: string;
	updated_at: string;
	version: number;
}

export interface DocumentVersionRow {
	id: string;
	document_id: string;
	version_number: number;
	schema_version: number;
	content_json: DocumentContent;
	plain_text: string;
	content_hash: string;
	document_status: DocumentStatus;
	restored_from_version_id: string | null;
	created_by_actor_id: string;
	reason: string;
	created_at: string;
}

export interface DocumentVersionView {
	version: DocumentVersionRow;
	rendered_html: string;
	markdown: string;
}

export interface DocumentVersionSummary {
	id: string;
	document_id: string;
	version_number: number;
	schema_version: number;
	content_hash: string;
	document_status: DocumentStatus;
	restored_from_version_id: string | null;
	created_by_actor_id: string;
	reason: string;
	created_at: string;
}

export interface DocumentReadView {
	document: DocumentRow;
	access: DocumentAccess;
	current_version: DocumentVersionView;
	/** This person's follow state; only the document read itself carries it. */
	following?: boolean;
	/** Others changed it since this person last opened it. */
	changed_since_you_looked?: { actor_ids: string[]; actor_name: string; at: string } | null;
}

export interface DocumentSummary {
	document: DocumentRow;
	access: DocumentAccess;
}

export interface DocumentListCursor {
	updated_at: string;
	id: string;
}

export interface DocumentListPage {
	items: DocumentSummary[];
	next_cursor: DocumentListCursor | null;
}

export interface DocumentVersionCursor {
	version_number: number;
}

export interface DocumentVersionPage {
	items: DocumentVersionSummary[];
	next_cursor: DocumentVersionCursor | null;
}

export type DocumentCommandOperation =
	| 'document_create'
	| 'document_metadata_update'
	| 'named_version_create'
	| 'named_version_restore'
	| 'participant_set'
	| 'participant_remove';

export interface DocumentCommandResult {
	command_id: string;
	document_id: string;
	operation: DocumentCommandOperation;
	result_id: string;
	created_at: string;
}

export interface DocumentPageCursor {
	created_at: string;
	id: string;
}

export interface DocumentParticipantRow {
	document_id: string;
	actor_id: string;
	access: DocumentAccess;
	added_by_actor_id: string;
	added_at: string;
}

export interface DocumentParticipantCursor {
	added_at: string;
	actor_id: string;
}

export interface DocumentParticipantPage {
	items: DocumentParticipantRow[];
	next_cursor: DocumentParticipantCursor | null;
	/** Whether the reader may change who has access; anyone who can open it sees the list. */
	may_manage: boolean;
}

export interface DocumentCommentThreadRow {
	id: string;
	document_id: string;
	anchored_version_id: string;
	block_id: string | null;
	status: DocumentCommentStatus;
	created_by_actor_id: string;
	created_at: string;
	resolved_by_actor_id: string | null;
	resolved_at: string | null;
	version: number;
}

export interface DocumentCommentThreadView {
	thread: DocumentCommentThreadRow;
	anchor_state: DocumentCommentAnchorState;
}

export interface DocumentCommentThreadPage {
	items: DocumentCommentThreadView[];
	next_cursor: DocumentPageCursor | null;
}

export interface DocumentCommentRow {
	id: string;
	document_id: string;
	thread_id: string;
	reply_to_comment_id: string | null;
	author_actor_id: string;
	content_json: DocumentContent;
	plain_text: string;
	content_hash: string;
	mentioned_actor_ids: string[];
	created_at: string;
}

export interface DocumentCommentPage {
	items: DocumentCommentRow[];
	next_cursor: DocumentPageCursor | null;
}

export interface DocumentCommentThreadCreated {
	thread: DocumentCommentThreadView;
	first_comment: DocumentCommentRow;
}

export interface DocumentReviewRow {
	id: string;
	document_id: string;
	requested_version_id: string;
	requested_by_actor_id: string;
	summary: string;
	status: DocumentReviewStatus;
	accepted_version_id: string | null;
	accepted_by_actor_id: string | null;
	accepted_at: string | null;
	material_unresolved_thread_ids: string[] | null;
	stale_against_version_id: string | null;
	stale_detected_by_actor_id: string | null;
	stale_at: string | null;
	created_at: string;
	version: number;
}

export type DocumentWorkReviewStatus = 'pending' | 'accepted' | 'changes_requested' | 'stale';

export interface DocumentWorkReviewDependency {
	review_id: string;
	document_id: string;
	requested_version_id: string;
	work_id: string;
	reviewer_actor_id: string;
	status: DocumentWorkReviewStatus;
	feedback: string | null;
	resolved_by_actor_id: string | null;
	resolved_at: string | null;
	resumed_at: string | null;
	created_at: string;
}

export interface DocumentReviewView {
	review: DocumentReviewRow;
	work_dependency: DocumentWorkReviewDependency | null;
}

export interface DocumentReviewPage {
	items: DocumentReviewView[];
	next_cursor: DocumentPageCursor | null;
}

export interface DocumentReviewResolution {
	review: DocumentReviewRow;
	accepted_version: DocumentVersionView | null;
	work_dependency: DocumentWorkReviewDependency | null;
}

export interface DocumentRevisionProposalSummary {
	id: string;
	document_id: string;
	base_version_id: string;
	scope: DocumentRevisionScope;
	block_id: string | null;
	proposed_content_hash: string;
	summary: string;
	proposed_by_actor_id: string;
	status: DocumentRevisionStatus;
	accepted_version_id: string | null;
	resolved_by_actor_id: string | null;
	resolution_summary: string | null;
	resolved_at: string | null;
	created_at: string;
	version: number;
}

export interface DocumentRevisionProposalRow extends DocumentRevisionProposalSummary {
	proposed_content_json: DocumentContent | DocumentNode;
	proposed_plain_text: string;
}

export interface DocumentRevisionProposalPage {
	items: DocumentRevisionProposalSummary[];
	next_cursor: DocumentPageCursor | null;
}

export interface DocumentRevisionResolution {
	proposal: DocumentRevisionProposalRow;
	accepted_version: DocumentVersionView | null;
}

export interface CreateDocumentInput {
	title: string;
	kind: DocumentKind;
	visibility: DocumentVisibility;
	linked_room_id: string | null;
	inherit_room_visibility: boolean;
	content_json: DocumentContent;
	reason: string;
}

export interface UpdateDocumentMetadataInput {
	expected_version: number;
	title: string;
	kind: DocumentKind;
	visibility: DocumentVisibility;
	linked_room_id: string | null;
	inherit_room_visibility: boolean;
}

export interface CreateDocumentVersionInput {
	expected_current_version_id: string;
	content_json: DocumentContent;
	reason: string;
}

export interface DocumentError extends Error {
	status: number;
	code: string | null;
}

export interface PendingDocumentCommand {
	id: string;
	fingerprint: string;
}

export interface PendingDocumentCreation {
	semanticFingerprint: string;
	command: PendingDocumentCommand;
	input: CreateDocumentInput;
}

export interface PendingDocumentMutation<T> {
	semanticFingerprint: string;
	command: PendingDocumentCommand;
	input: T;
}

const documentsPath = (company: string, document?: string): string => {
	const base = `/api/companies/${encodeURIComponent(company)}/documents`;
	return document ? `${base}/${encodeURIComponent(document)}` : base;
};

async function responseError(response: Response): Promise<DocumentError> {
	return (await responseFailure(response)) as DocumentError;
}

async function documentJson<T>(url: string, init?: RequestInit): Promise<T> {
	const response = await fetch(url, {
		credentials: 'same-origin',
		cache: 'no-store',
		...init
	});
	if (!response.ok) throw await responseError(response);
	return (await response.json()) as T;
}

function mutation(body: unknown, commandId: string, method = 'POST'): RequestInit {
	return {
		method,
		headers: {
			'content-type': 'application/json',
			'Idempotency-Key': commandId
		},
		body: JSON.stringify(body)
	};
}

function appendPageCursor(query: URLSearchParams, cursor: DocumentPageCursor | null): void {
	if (!cursor) return;
	query.set('after_created_at', cursor.created_at);
	query.set('after_id', cursor.id);
}

export function getDocuments(
	company: string,
	cursor: DocumentListCursor | null = null,
	limit = 30,
	includeArchived = false,
	signal?: AbortSignal
): Promise<DocumentListPage> {
	const query = new URLSearchParams({
		limit: String(limit),
		include_archived: String(includeArchived)
	});
	if (cursor) {
		query.set('before_updated_at', cursor.updated_at);
		query.set('before_document_id', cursor.id);
	}
	return documentJson(`${documentsPath(company)}?${query}`, { signal });
}

/** Only archived documents, newest first, a page at a time. */
export function getArchivedDocuments(
	company: string,
	cursor: DocumentListCursor | null = null,
	limit = 50,
	signal?: AbortSignal
): Promise<DocumentListPage> {
	const query = new URLSearchParams({ limit: String(limit), archived_only: 'true' });
	if (cursor) {
		query.set('before_updated_at', cursor.updated_at);
		query.set('before_document_id', cursor.id);
	}
	return documentJson(`${documentsPath(company)}?${query}`, { signal });
}

/** Delete an archived document for good. */
export async function deleteArchivedDocument(company: string, document: string): Promise<void> {
	await documentJson(documentsPath(company, document), { method: 'DELETE' });
}

export function createDocument(
	company: string,
	input: CreateDocumentInput,
	commandId: string
): Promise<DocumentCommandResult> {
	return documentJson(documentsPath(company), mutation(input, commandId));
}

export interface DocumentSearchHit {
	document_id: string;
	named_version_id: string;
	title: string;
	kind: string;
	snippet: string;
	updated_at: string;
	relevance: number;
}

export interface DocumentSearchPage {
	items: DocumentSearchHit[];
	next_offset: number | null;
}

export function searchDocuments(company: string, q: string, offset = 0, signal?: AbortSignal): Promise<DocumentSearchPage> {
	const query = new URLSearchParams({ q, offset: String(offset), limit: '30', include_archived: 'false' });
	return documentJson(`${documentsPath(company)}/search?${query}`, { signal });
}

export function getDocument(
	company: string,
	document: string,
	signal?: AbortSignal
): Promise<DocumentReadView> {
	return documentJson(documentsPath(company, document), { signal });
}

export function updateDocumentMetadata(
	company: string,
	document: string,
	input: UpdateDocumentMetadataInput,
	commandId: string
): Promise<DocumentCommandResult> {
	return documentJson(documentsPath(company, document), mutation(input, commandId, 'PATCH'));
}

/** Set what every person in the company may do without being added. Its creator, the company's
 * owner and the Exec may; anyone else is told it is unavailable. */
export function setDocumentGeneralAccess(
	company: string,
	document: string,
	expectedVersion: number,
	generalAccess: GeneralAccess,
	commandId: string
): Promise<DocumentCommandResult> {
	return documentJson(
		documentsPath(company, document),
		mutation({ expected_version: expectedVersion, general_access: generalAccess }, commandId, 'PATCH')
	);
}

export function getDocumentVersions(
	company: string,
	document: string,
	cursor: DocumentVersionCursor | null = null,
	limit = 25,
	signal?: AbortSignal
): Promise<DocumentVersionPage> {
	const query = new URLSearchParams({ limit: String(limit) });
	if (cursor) query.set('before_version_number', String(cursor.version_number));
	return documentJson(`${documentsPath(company, document)}/versions?${query}`, { signal });
}

export function getDocumentVersion(
	company: string,
	document: string,
	version: string,
	signal?: AbortSignal
): Promise<DocumentVersionView> {
	return documentJson(
		`${documentsPath(company, document)}/versions/${encodeURIComponent(version)}`,
		{ signal }
	);
}

export function restoreDocumentVersion(
	company: string,
	document: string,
	version: string,
	expectedCurrentVersionId: string,
	reason: string,
	commandId: string
): Promise<DocumentCommandResult> {
	return documentJson(
		`${documentsPath(company, document)}/versions/${encodeURIComponent(version)}/restore`,
		mutation({ expected_current_version_id: expectedCurrentVersionId, reason }, commandId)
	);
}

export function getDocumentParticipants(
	company: string,
	document: string,
	cursor: DocumentParticipantCursor | null = null,
	limit = 50,
	signal?: AbortSignal
): Promise<DocumentParticipantPage> {
	const query = new URLSearchParams({ limit: String(limit) });
	if (cursor) {
		query.set('after_added_at', cursor.added_at);
		query.set('after_actor_id', cursor.actor_id);
	}
	return documentJson(`${documentsPath(company, document)}/participants?${query}`, { signal });
}

export function setDocumentParticipant(
	company: string,
	document: string,
	participant: string,
	expectedDocumentVersion: number,
	access: DocumentAccess,
	commandId: string
): Promise<DocumentCommandResult> {
	return documentJson(
		`${documentsPath(company, document)}/participants/${encodeURIComponent(participant)}`,
		mutation({ expected_document_version: expectedDocumentVersion, access }, commandId, 'PUT')
	);
}

export function removeDocumentParticipant(
	company: string,
	document: string,
	participant: string,
	expectedDocumentVersion: number,
	commandId: string
): Promise<DocumentCommandResult> {
	return documentJson(
		`${documentsPath(company, document)}/participants/${encodeURIComponent(participant)}`,
		mutation({ expected_document_version: expectedDocumentVersion }, commandId, 'DELETE')
	);
}

export function getDocumentCommentThreads(
	company: string,
	document: string,
	cursor: DocumentPageCursor | null = null,
	limit = 50,
	signal?: AbortSignal
): Promise<DocumentCommentThreadPage> {
	const query = new URLSearchParams({ limit: String(limit) });
	appendPageCursor(query, cursor);
	return documentJson(`${documentsPath(company, document)}/comments?${query}`, { signal });
}

export function createDocumentCommentThread(
	company: string,
	document: string,
	blockId: string | null,
	contentJson: DocumentContent,
	commandId: string
): Promise<DocumentCommentThreadCreated> {
	return documentJson(
		`${documentsPath(company, document)}/comments`,
		mutation({ block_id: blockId, content_json: contentJson }, commandId)
	);
}

export function getDocumentComments(
	company: string,
	document: string,
	thread: string,
	cursor: DocumentPageCursor | null = null,
	limit = 100,
	signal?: AbortSignal
): Promise<DocumentCommentPage> {
	const query = new URLSearchParams({ limit: String(limit) });
	appendPageCursor(query, cursor);
	return documentJson(
		`${documentsPath(company, document)}/comments/${encodeURIComponent(thread)}/replies?${query}`,
		{ signal }
	);
}

export function replyToDocumentComment(
	company: string,
	document: string,
	thread: string,
	replyToCommentId: string | null,
	contentJson: DocumentContent,
	commandId: string
): Promise<DocumentCommentRow> {
	return documentJson(
		`${documentsPath(company, document)}/comments/${encodeURIComponent(thread)}/replies`,
		mutation({ reply_to_comment_id: replyToCommentId, content_json: contentJson }, commandId)
	);
}

export function resolveDocumentCommentThread(
	company: string,
	document: string,
	thread: string,
	expectedThreadVersion: number,
	commandId: string
): Promise<DocumentCommentThreadRow> {
	return documentJson(
		`${documentsPath(company, document)}/comments/${encodeURIComponent(thread)}/resolve`,
		mutation({ expected_thread_version: expectedThreadVersion }, commandId)
	);
}

export function getDocumentReviews(
	company: string,
	document: string,
	cursor: DocumentPageCursor | null = null,
	limit = 50,
	signal?: AbortSignal
): Promise<DocumentReviewPage> {
	const query = new URLSearchParams({ limit: String(limit) });
	appendPageCursor(query, cursor);
	return documentJson(`${documentsPath(company, document)}/reviews?${query}`, { signal });
}

export function requestDocumentReview(
	company: string,
	document: string,
	expectedDocumentVersion: number,
	expectedCurrentVersionId: string,
	summary: string,
	commandId: string
): Promise<DocumentReviewView> {
	return documentJson(
		`${documentsPath(company, document)}/reviews`,
		mutation(
			{
				expected_document_version: expectedDocumentVersion,
				expected_current_version_id: expectedCurrentVersionId,
				summary
			},
			commandId
		)
	);
}

export function acceptDocumentReview(
	company: string,
	document: string,
	review: string,
	expectedDocumentVersion: number,
	expectedReviewVersion: number,
	acceptedVersionName: string,
	commandId: string,
	feedback = ''
): Promise<DocumentReviewResolution> {
	return documentJson(
		`${documentsPath(company, document)}/reviews/${encodeURIComponent(review)}/accept`,
		mutation(
			{
				expected_document_version: expectedDocumentVersion,
				expected_review_version: expectedReviewVersion,
				accepted_version_name: acceptedVersionName,
				...(feedback.trim() ? { feedback: feedback.trim() } : {})
			},
			commandId
		)
	);
}

/** Send a Work-bound review back to the Work that asked for it, with what should change. */
export function requestDocumentReviewChanges(
	company: string,
	document: string,
	review: string,
	expectedDocumentVersion: number,
	expectedReviewVersion: number,
	feedback: string,
	commandId: string
): Promise<DocumentReviewResolution> {
	return documentJson(
		`${documentsPath(company, document)}/reviews/${encodeURIComponent(review)}/request-changes`,
		mutation(
			{
				expected_document_version: expectedDocumentVersion,
				expected_review_version: expectedReviewVersion,
				feedback
			},
			commandId
		)
	);
}

/** Archive a document out of the Library, or restore it (as a draft). */
export function setDocumentArchived(
	company: string,
	document: string,
	archived: boolean
): Promise<{ document: string; archived: boolean }> {
	return documentJson(`${documentsPath(company, document)}/archive`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ archived })
	});
}

export function getDocumentRevisionProposals(
	company: string,
	document: string,
	cursor: DocumentPageCursor | null = null,
	limit = 50,
	signal?: AbortSignal
): Promise<DocumentRevisionProposalPage> {
	const query = new URLSearchParams({ limit: String(limit) });
	appendPageCursor(query, cursor);
	return documentJson(`${documentsPath(company, document)}/proposals?${query}`, { signal });
}

export function getDocumentRevisionProposal(
	company: string,
	document: string,
	proposal: string,
	signal?: AbortSignal
): Promise<DocumentRevisionProposalRow> {
	return documentJson(
		`${documentsPath(company, document)}/proposals/${encodeURIComponent(proposal)}`,
		{ signal }
	);
}

export function acceptDocumentRevisionProposal(
	company: string,
	document: string,
	proposal: string,
	expectedProposalVersion: number,
	resolutionSummary: string,
	acceptedVersionName: string,
	commandId: string
): Promise<DocumentRevisionResolution> {
	return documentJson(
		`${documentsPath(company, document)}/proposals/${encodeURIComponent(proposal)}/accept`,
		mutation(
			{
				expected_proposal_version: expectedProposalVersion,
				resolution_summary: resolutionSummary,
				accepted_version_name: acceptedVersionName
			},
			commandId
		)
	);
}

export function rejectDocumentRevisionProposal(
	company: string,
	document: string,
	proposal: string,
	expectedProposalVersion: number,
	resolutionSummary: string,
	commandId: string
): Promise<DocumentRevisionResolution> {
	return documentJson(
		`${documentsPath(company, document)}/proposals/${encodeURIComponent(proposal)}/reject`,
		mutation(
			{
				expected_proposal_version: expectedProposalVersion,
				resolution_summary: resolutionSummary
			},
			commandId
		)
	);
}

export type EditableBlockKind =
	| 'paragraph'
	| 'heading'
	| 'bulletList'
	| 'orderedList'
	| 'taskList'
	| 'blockquote'
	| 'codeBlock'
	| 'horizontalRule'
	| 'table';

export interface EditableDocumentBlock {
	id: string;
	kind: EditableBlockKind;
	text: string;
	original: DocumentNode;
	changed: boolean;
	readOnly: boolean;
}

const EDITABLE_BLOCK_TYPES = new Set<EditableBlockKind>([
	'paragraph',
	'heading',
	'bulletList',
	'orderedList',
	'taskList',
	'blockquote',
	'codeBlock',
	'horizontalRule'
]);

function blockId(node: DocumentNode): string {
	const value = node.attrs?.block_id;
	return typeof value === 'string' && value ? value : `block-${crypto.randomUUID()}`;
}

export function documentText(node: DocumentNode | null | undefined): string {
	if (!node) return '';
	if (typeof node.text === 'string') return node.text;
	const separator = [
		'doc',
		'blockquote',
		'bulletList',
		'orderedList',
		'taskList',
		'table'
	].includes(node.type)
		? '\n'
		: '';
	return (node.content ?? []).map(documentText).filter(Boolean).join(separator);
}

function listBlockText(node: DocumentNode): string {
	return (node.content ?? [])
		.map((item) => {
			const text = documentText(item).trim();
			if (node.type !== 'taskList') return text;
			return `${item.attrs?.checked === true ? '[x]' : '[ ]'} ${text}`;
		})
		.join('\n');
}

export function editableBlocks(content: DocumentContent): EditableDocumentBlock[] {
	return content.content.map((node) => {
		const kind = node.type as EditableBlockKind;
		const supported = EDITABLE_BLOCK_TYPES.has(kind);
		return {
			id: blockId(node),
			kind: supported ? kind : 'table',
			text: ['bulletList', 'orderedList', 'taskList'].includes(node.type)
				? listBlockText(node)
				: documentText(node),
			original: structuredClone(node),
			changed: false,
			readOnly: !supported || node.type === 'table'
		};
	});
}

function textNode(text: string): DocumentNode[] | undefined {
	return text ? [{ type: 'text', text }] : undefined;
}

function listItem(text: string, id: string, checked?: boolean): DocumentNode {
	const item: DocumentNode = {
		type: checked === undefined ? 'listItem' : 'taskItem',
		attrs: checked === undefined ? { block_id: id } : { block_id: id, checked },
		content: [
			{
				type: 'paragraph',
				attrs: { block_id: `${id}-text` },
				content: textNode(text)
			}
		]
	};
	return item;
}

function changedNode(block: EditableDocumentBlock): DocumentNode {
	const attrs = { ...(block.original.attrs ?? {}), block_id: block.id };
	switch (block.kind) {
		case 'heading':
			return {
				type: 'heading',
				attrs: { ...attrs, level: Number(block.original.attrs?.level ?? 2) },
				content: textNode(block.text)
			};
		case 'blockquote':
			return {
				type: 'blockquote',
				attrs,
				content: [
					{
						type: 'paragraph',
						attrs: { block_id: `${block.id}-quote` },
						content: textNode(block.text)
					}
				]
			};
		case 'codeBlock':
			return { type: 'codeBlock', attrs, content: textNode(block.text) };
		case 'bulletList':
		case 'orderedList': {
			const lines = block.text
				.split('\n')
				.map((line) => line.trim())
				.filter(Boolean);
			return {
				type: block.kind,
				attrs,
				content: lines.map((line, index) => listItem(line, `${block.id}-${index + 1}`))
			};
		}
		case 'taskList': {
			const lines = block.text
				.split('\n')
				.map((line) => line.trim())
				.filter(Boolean);
			return {
				type: 'taskList',
				attrs,
				content: lines.map((line, index) => {
					const checked = /^\[x\]\s*/iu.test(line);
					return listItem(
						line.replace(/^\[(?:x| )\]\s*/iu, ''),
						`${block.id}-${index + 1}`,
						checked
					);
				})
			};
		}
		case 'horizontalRule':
			return { type: 'horizontalRule', attrs };
		case 'paragraph':
		default:
			return { type: 'paragraph', attrs, content: textNode(block.text) };
	}
}

export function editorContent(blocks: EditableDocumentBlock[]): DocumentContent {
	return {
		type: 'doc',
		content: blocks.map((block) => (block.changed ? changedNode(block) : block.original))
	};
}

export function newEditableBlock(kind: EditableBlockKind = 'paragraph'): EditableDocumentBlock {
	const id = `block-${crypto.randomUUID()}`;
	const block: EditableDocumentBlock = {
		id,
		kind,
		text: '',
		original: { type: kind, attrs: { block_id: id } },
		changed: true,
		readOnly: kind === 'table'
	};
	return block;
}

export function emptyDocument(): DocumentContent {
	return editorContent([newEditableBlock('paragraph')]);
}

/** Someone picked from the mention menu: shown in the text as `@label`. */
export interface CommentMention {
	actor_id: string;
	label: string;
}

function escapeRegExp(value: string): string {
	return value.replace(/[.*+?^${}()|[\]\\]/gu, '\\$&');
}

export function commentContent(
	text: string,
	knownActorIds: ReadonlySet<string>,
	mentions: readonly CommentMention[] = []
): DocumentContent {
	const content: DocumentNode[] = [];
	/* People picked by name first (longest name first, so "@Ann Lee" wins over "@Ann"), then any
	 * `@actor-id` typed by hand. The label is the name alone; the `@` is how it is shown. */
	const picked = mentions
		.filter((mention) => mention.label.trim() && knownActorIds.has(mention.actor_id))
		.toSorted((left, right) => right.label.length - left.label.length);
	const byLabel = new Map(picked.map((mention) => [`@${mention.label}`, mention]));
	const pattern = new RegExp(
		[
			...picked.map((mention) => `${escapeRegExp(`@${mention.label}`)}(?![\\p{L}\\p{N}])`),
			'@[A-Za-z0-9_.:-]+'
		].join('|'),
		'gu'
	);
	const pieces = text.split(new RegExp(`(${pattern.source})`, 'gu'));
	for (const piece of pieces) {
		if (!piece) continue;
		const named = byLabel.get(piece);
		const actorId = named?.actor_id ?? (piece.startsWith('@') ? piece.slice(1) : '');
		if (actorId && knownActorIds.has(actorId)) {
			content.push({
				type: 'mention',
				attrs: { actor_id: actorId, label: named?.label ?? actorId }
			});
		} else {
			content.push({ type: 'text', text: piece });
		}
	}
	return {
		type: 'doc',
		content: [
			{
				type: 'paragraph',
				attrs: { block_id: `comment-${crypto.randomUUID()}` },
				content
			}
		]
	};
}

export function isDocumentConflict(error: unknown): error is DocumentError {
	return error instanceof Error && (error as Partial<DocumentError>).status === 409;
}

export function pendingDocumentCommand(
	current: PendingDocumentCommand | null,
	fingerprint: string
): PendingDocumentCommand {
	if (current?.fingerprint === fingerprint) return current;
	return { id: crypto.randomUUID(), fingerprint };
}

export function pendingDocumentCreation(
	current: PendingDocumentCreation | null,
	semanticInput: Omit<CreateDocumentInput, 'content_json'>
): PendingDocumentCreation {
	return pendingDocumentMutation(current, JSON.stringify(semanticInput), () => ({
		...semanticInput,
		content_json: emptyDocument()
	}));
}

export function pendingDocumentMutation<T>(
	current: PendingDocumentMutation<T> | null,
	semanticFingerprint: string,
	createInput: () => T
): PendingDocumentMutation<T> {
	if (current?.semanticFingerprint === semanticFingerprint) return current;
	const input = createInput();
	return {
		semanticFingerprint,
		input,
		command: pendingDocumentCommand(null, JSON.stringify(input))
	};
}

export function checkpointMetadataInput(
	source: DocumentRow,
	baseDocumentVersion: number,
	title: string,
	kind: DocumentKind,
	titleDirty: boolean,
	kindDirty: boolean
): UpdateDocumentMetadataInput | null {
	if (!titleDirty && !kindDirty) return null;
	return {
		expected_version: baseDocumentVersion + 1,
		title: titleDirty ? title : source.title,
		kind: kindDirty ? kind : source.kind,
		visibility: source.visibility,
		linked_room_id: source.linked_room_id,
		inherit_room_visibility: source.inherit_room_visibility
	};
}

export function isRetryableDocumentFailure(error: unknown): boolean {
	const status =
		typeof error === 'object' && error !== null && 'status' in error
			? (error as { status?: unknown }).status
			: undefined;
	if (typeof status !== 'number') return true;
	return status === 408 || status === 425 || status === 429 || status >= 500;
}

/** The text of one block in a saved version, for quoting what a comment is about. */
export function documentBlockText(
	content: DocumentNode | null | undefined,
	blockId: string
): string | null {
	if (!content || !blockId) return null;
	if (content.attrs?.block_id === blockId) return documentText(content);
	for (const child of content.content ?? []) {
		const found = documentBlockText(child, blockId);
		if (found !== null) return found;
	}
	return null;
}

/** A short, single-line quotation of a block's text. */
export function quoteExcerpt(text: string, maximum = 90): string {
	const line = text.replace(/\s+/gu, ' ').trim();
	return line.length > maximum ? `${line.slice(0, maximum - 1).trimEnd()}…` : line;
}

/** A file name a document can be saved under on any system. */
export function documentFileName(title: string, extension: string): string {
	const base = title
		.normalize('NFKC')
		.replace(/[\\/:*?"<>|\p{Cc}]+/gu, ' ')
		.replace(/\s+/gu, ' ')
		.replace(/^[.\s]+|[.\s]+$/gu, '')
		.slice(0, 120)
		.trim();
	return `${base || 'Document'}.${extension}`;
}

/** The Markdown a person downloads: the title as its heading, then the saved body. */
export function documentMarkdownDownload(title: string, markdown: string): string {
	const body = markdown.replace(/^\s+/u, '');
	const heading = title.trim();
	const first = body
		.split('\n', 1)[0]
		?.replace(/^#+\s*/u, '')
		.trim();
	if (!heading || (body.startsWith('#') && first === heading)) return `${body.trimEnd()}\n`;
	return `# ${heading}\n\n${body.trimEnd()}\n`;
}

export function resolveDocumentCollaboration(
	company: string,
	document: string,
	request: string
): Promise<unknown> {
	return documentJson(
		`${documentsPath(company, document)}/collaboration-requests/${encodeURIComponent(request)}/resolve`,
		{ method: 'POST' }
	);
}
