<script lang="ts">
	import { browser } from '$app/environment';
	import { onDestroy, tick } from 'svelte';
	import { useQueryClient } from '@tanstack/svelte-query';
	import AlignLeft from '@lucide/svelte/icons/align-left';
	import Braces from '@lucide/svelte/icons/braces';
	import Check from '@lucide/svelte/icons/check';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import GripVertical from '@lucide/svelte/icons/grip-vertical';
	import Heading2 from '@lucide/svelte/icons/heading-2';
	import List from '@lucide/svelte/icons/list';
	import ListChecks from '@lucide/svelte/icons/list-checks';
	import ListOrdered from '@lucide/svelte/icons/list-ordered';
	import MessageSquarePlus from '@lucide/svelte/icons/message-square-plus';
	import Minus from '@lucide/svelte/icons/minus';
	import Plus from '@lucide/svelte/icons/plus';
	import Quote from '@lucide/svelte/icons/quote';
	import RefreshCw from '@lucide/svelte/icons/refresh-cw';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import {
		canPersistDocumentDraft,
		documentDraftStorageKey,
		failClosedDocumentRead,
		sameDocumentTarget,
		type DocumentTarget
	} from '$lib/model/document-cache';
	import {
		checkpointMetadataInput,
		createDocumentVersion,
		documentText,
		DOCUMENT_KINDS,
		editableBlocks,
		editorContent,
		getDocument,
		isDocumentConflict,
		isRetryableDocumentFailure,
		newEditableBlock,
		pendingDocumentCommand,
		updateDocumentMetadata,
		type DocumentReadView,
		type DocumentKind,
		type EditableBlockKind,
		type EditableDocumentBlock,
		type PendingDocumentCommand
	} from '$lib/model/documents';

	interface Props {
		companyId: string;
		view: DocumentReadView;
		principalActorId: string;
		cachePartition: string;
		online?: boolean;
		onaccept?: (target: DocumentTarget, view: DocumentReadView) => void;
		oncomment?: (blockId: string | null) => void;
		ondirtychange?: (dirty: boolean) => void;
	}

	let {
		companyId,
		view,
		principalActorId,
		cachePartition,
		online = true,
		onaccept = () => {},
		oncomment = () => {},
		ondirtychange = () => {}
	}: Props = $props();
	const client = useQueryClient();

	let blocks = $state<EditableDocumentBlock[]>([]);
	let title = $state('');
	let kind = $state<DocumentKind>('freeform');
	let serverView = $state<DocumentReadView | null>(null);
	let loadedDocumentId = $state('');
	let loadedVersionId = $state('');
	let loadedDraftScope = $state('');
	let loadedDraftKey = $state('');
	let baseDocumentVersion = $state(0);
	let dirty = $state(false);
	let titleDirty = $state(false);
	let kindDirty = $state(false);
	let editRevision = $state(0);
	let saving = $state(false);
	let saveState = $state<'saved' | 'pending' | 'local' | 'saving' | 'offline' | 'failed'>('saved');
	let saveFailure = $state('');
	let conflict = $state<DocumentReadView | null>(null);
	let checkpointOpen = $state(false);
	let checkpointReason = $state('');
	let checkpointCommand = $state<PendingDocumentCommand | null>(null);
	let metadataCommand = $state<PendingDocumentCommand | null>(null);
	let draftTimer: ReturnType<typeof setTimeout> | null = null;

	const draftScope = $derived(
		principalActorId && cachePartition ? `${principalActorId}:${cachePartition}` : ''
	);
	const canEdit = $derived(view.access === 'edit' && Boolean(draftScope));
	const localContent = $derived(editorContent(blocks));
	const localPlainText = $derived(documentText(localContent));

	interface StoredDocumentDraft {
		base_version_id: string;
		base_document_version: number;
		title: string;
		title_dirty: boolean;
		kind: DocumentKind;
		kind_dirty: boolean;
		content_json: ReturnType<typeof editorContent>;
		updated_at: string;
	}

	function draftKey(documentId: string): string | null {
		if (!principalActorId || !cachePartition) return null;
		return documentDraftStorageKey(companyId, principalActorId, cachePartition, documentId);
	}

	function readDraft(documentId: string): StoredDocumentDraft | null {
		if (!browser) return null;
		try {
			const key = draftKey(documentId);
			if (!key) return null;
			const value = localStorage.getItem(key);
			if (!value) return null;
			const draft = JSON.parse(value) as Partial<StoredDocumentDraft>;
			return typeof draft.base_document_version === 'number' &&
				typeof draft.title_dirty === 'boolean' &&
				typeof draft.kind_dirty === 'boolean' &&
				draft.content_json
				? (draft as StoredDocumentDraft)
				: null;
		} catch {
			return null;
		}
	}

	function removeDraft(documentId: string): void {
		if (!browser) return;
		const key = draftKey(documentId);
		if (key) localStorage.removeItem(key);
	}

	function load(source: DocumentReadView): void {
		serverView = source;
		loadedDocumentId = source.document.id;
		loadedVersionId = source.current_version.version.id;
		loadedDraftScope = draftScope;
		loadedDraftKey = draftKey(source.document.id) ?? '';
		const draft = readDraft(source.document.id);
		if (draft) {
			blocks = editableBlocks(draft.content_json);
			titleDirty = draft.title_dirty;
			kindDirty = draft.kind_dirty;
			title = titleDirty ? draft.title : source.document.title;
			kind = kindDirty ? draft.kind : source.document.kind;
			baseDocumentVersion =
				titleDirty || kindDirty ? draft.base_document_version : source.document.version;
			dirty =
				JSON.stringify(draft.content_json) !==
					JSON.stringify(source.current_version.version.content_json) ||
				titleDirty ||
				kindDirty;
			conflict =
				dirty &&
				(draft.base_version_id !== loadedVersionId ||
					((titleDirty || kindDirty) && draft.base_document_version !== source.document.version))
					? source
					: null;
		} else {
			blocks = editableBlocks(source.current_version.version.content_json);
			title = source.document.title;
			kind = source.document.kind;
			baseDocumentVersion = source.document.version;
			titleDirty = false;
			kindDirty = false;
			dirty = false;
			conflict = null;
		}
		saveFailure = '';
		saving = false;
		checkpointOpen = false;
		checkpointReason = '';
		checkpointCommand = null;
		metadataCommand = null;
		saveState = dirty ? (online ? 'local' : 'offline') : 'saved';
		ondirtychange(dirty);
		if (!dirty && draft) removeDraft(source.document.id);
	}

	$effect(() => {
		const incoming = view;
		const nextDraftScope = draftScope;
		if (
			!loadedDocumentId ||
			incoming.document.id !== loadedDocumentId ||
			nextDraftScope !== loadedDraftScope
		) {
			if (loadedDocumentId && dirty) persistDraft();
			load(incoming);
			return;
		}
		if (incoming.current_version.version.id === loadedVersionId) {
			if (incoming.document.version !== baseDocumentVersion) {
				if (titleDirty || kindDirty) {
					conflict = incoming;
					return;
				}
				title = incoming.document.title;
				kind = incoming.document.kind;
				baseDocumentVersion = incoming.document.version;
			}
			serverView = incoming;
			if (dirty) scheduleDraft();
			return;
		}
		if (saving) return;
		if (!dirty) load(incoming);
		else if (!conflict) conflict = incoming;
	});

	$effect(() => {
		if (!dirty) return;
		if (saveState !== 'pending' && saveState !== 'failed' && saveState !== 'saving') {
			saveState = online ? 'local' : 'offline';
		}
	});

	onDestroy(() => {
		persistDraft();
	});

	function markChanged(): void {
		if (!canEdit) return;
		const source = serverView;
		if (!source) return;
		titleDirty = title.trim() !== source.document.title;
		kindDirty = kind !== source.document.kind;
		dirty =
			JSON.stringify(editorContent(blocks)) !==
				JSON.stringify(source.current_version.version.content_json) ||
			titleDirty ||
			kindDirty;
		editRevision += 1;
		saveFailure = '';
		checkpointCommand = null;
		metadataCommand = null;
		ondirtychange(dirty);
		if (dirty) scheduleDraft();
		else {
			if (draftTimer) clearTimeout(draftTimer);
			draftTimer = null;
			removeDraft(source.document.id);
			saveState = 'saved';
		}
	}

	function scheduleDraft(): void {
		if (draftTimer) clearTimeout(draftTimer);
		saveState = 'pending';
		draftTimer = setTimeout(persistDraft, 180);
	}

	function persistDraft(): void {
		if (draftTimer) clearTimeout(draftTimer);
		draftTimer = null;
		if (!browser || !dirty || !serverView) return;
		if (!canPersistDocumentDraft(companyId, serverView.document.id)) return;
		const draft: StoredDocumentDraft = {
			base_version_id: serverView.current_version.version.id,
			base_document_version: baseDocumentVersion,
			title,
			title_dirty: titleDirty,
			kind,
			kind_dirty: kindDirty,
			content_json: editorContent(blocks),
			updated_at: new Date().toISOString()
		};
		try {
			const key = loadedDraftKey;
			if (!key) return;
			localStorage.setItem(key, JSON.stringify(draft));
			saveState = online ? 'local' : 'offline';
		} catch {
			saveFailure = 'This browser could not keep the local draft.';
			saveState = 'failed';
		}
	}

	function persistWhenHidden(): void {
		if (document.visibilityState === 'hidden') persistDraft();
	}

	async function flush(): Promise<void> {
		if (!dirty || !canEdit || conflict) return;
		const reason = checkpointReason.trim();
		if (!reason) {
			checkpointOpen = true;
			return;
		}
		if (!online) {
			saveState = 'offline';
			return;
		}
		if (saving) return;
		const source = serverView;
		if (!source) return;
		const target = { companyId, documentId: source.document.id };
		const titleDirtyAtStart = titleDirty;
		const kindDirtyAtStart = kindDirty;
		const metadataDirtyAtStart = titleDirtyAtStart || kindDirtyAtStart;
		const baseDocumentVersionAtStart = baseDocumentVersion;
		if (metadataDirtyAtStart && source.document.version !== baseDocumentVersionAtStart) {
			conflict = source;
			saveState = 'failed';
			return;
		}
		persistDraft();
		saving = true;
		saveState = 'saving';
		saveFailure = '';
		const startedAtRevision = editRevision;
		const contentAtStart = editorContent(blocks);
		const titleAtStart = title.trim();
		const kindAtStart = kind;
		const checkpointInput = {
			expected_current_version_id: source.current_version.version.id,
			content_json: contentAtStart,
			reason
		};
		const versionCommand = pendingDocumentCommand(
			checkpointCommand,
			JSON.stringify(checkpointInput)
		);
		const metadataInput = checkpointMetadataInput(
			source.document,
			baseDocumentVersionAtStart,
			titleAtStart,
			kindAtStart,
			titleDirtyAtStart,
			kindDirtyAtStart
		);
		const metadataCommandAtStart = metadataInput
			? pendingDocumentCommand(metadataCommand, JSON.stringify(metadataInput))
			: null;
		checkpointCommand = versionCommand;
		metadataCommand = metadataCommandAtStart;
		try {
			const versionReceipt = await createDocumentVersion(
				target.companyId,
				target.documentId,
				checkpointInput,
				versionCommand.id
			);
			let accepted = await getDocument(target.companyId, versionReceipt.document_id);
			if (accepted.current_version.version.id !== versionReceipt.result_id) {
				if (sameDocumentTarget(target, companyId, view.document.id)) {
					conflict = accepted;
					checkpointCommand = null;
					metadataCommand = null;
					saveFailure = 'Another saved version completed before this checkpoint was read.';
					saveState = 'failed';
				}
				return;
			}
			if (metadataInput && metadataCommandAtStart && titleAtStart) {
				await updateDocumentMetadata(
					target.companyId,
					target.documentId,
					metadataInput,
					metadataCommandAtStart.id
				);
				accepted = await getDocument(target.companyId, target.documentId);
			}
			if (!sameDocumentTarget(target, companyId, view.document.id)) return;
			serverView = accepted;
			loadedVersionId = accepted.current_version.version.id;
			baseDocumentVersion = accepted.document.version;
			onaccept(target, accepted);
			if (editRevision === startedAtRevision) {
				blocks = editableBlocks(accepted.current_version.version.content_json);
				title = accepted.document.title;
				kind = accepted.document.kind;
				dirty = false;
				titleDirty = false;
				kindDirty = false;
				saveState = 'saved';
				checkpointOpen = false;
				checkpointReason = '';
				checkpointCommand = null;
				metadataCommand = null;
				removeDraft(accepted.document.id);
				ondirtychange(false);
			} else {
				titleDirty = title.trim() !== accepted.document.title;
				kindDirty = kind !== accepted.document.kind;
				persistDraft();
				saveState = online ? 'local' : 'offline';
			}
		} catch (cause) {
			failClosedDocumentRead(client, cause, target.companyId, target.documentId);
			if (!sameDocumentTarget(target, companyId, view.document.id)) return;
			if (!isRetryableDocumentFailure(cause)) {
				checkpointCommand = null;
				metadataCommand = null;
			}
			if (isDocumentConflict(cause)) {
				try {
					const latest = await getDocument(target.companyId, target.documentId);
					if (!sameDocumentTarget(target, companyId, view.document.id)) return;
					conflict = latest;
					saveState = 'failed';
				} catch (refreshCause) {
					failClosedDocumentRead(client, refreshCause, target.companyId, target.documentId);
					saveFailure =
						refreshCause instanceof Error
							? refreshCause.message
							: 'The latest document could not be loaded.';
					saveState = 'failed';
				}
			} else {
				saveFailure = cause instanceof Error ? cause.message : 'Changes could not be saved.';
				saveState = 'failed';
			}
		} finally {
			if (sameDocumentTarget(target, companyId, view.document.id)) saving = false;
		}
	}

	function updateBlock(id: string, patch: Partial<EditableDocumentBlock>): void {
		blocks = blocks.map((block) =>
			block.id === id ? { ...block, ...patch, changed: true } : block
		);
		markChanged();
	}

	function changeBlockKind(block: EditableDocumentBlock, next: string): void {
		const kind = next as EditableBlockKind;
		updateBlock(block.id, { kind, readOnly: kind === 'table' });
	}

	function addBlock(kind: EditableBlockKind): void {
		blocks = [...blocks, newEditableBlock(kind)];
		markChanged();
		void tick().then(() => {
			document.querySelector<HTMLElement>('[data-new-document-block="true"]')?.focus();
		});
	}

	function removeBlock(id: string): void {
		if (blocks.length === 1) {
			updateBlock(id, { text: '', kind: 'paragraph' });
			return;
		}
		blocks = blocks.filter((block) => block.id !== id);
		markChanged();
	}

	function moveBlock(id: string, offset: -1 | 1): void {
		const index = blocks.findIndex((block) => block.id === id);
		const target = index + offset;
		if (index < 0 || target < 0 || target >= blocks.length) return;
		const next = [...blocks];
		[next[index], next[target]] = [next[target], next[index]];
		blocks = next;
		markChanged();
	}

	function blockKeydown(event: KeyboardEvent, block: EditableDocumentBlock): void {
		if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
			event.preventDefault();
			checkpointOpen = true;
			return;
		}
		if (event.altKey && (event.key === 'ArrowUp' || event.key === 'ArrowDown')) {
			event.preventDefault();
			moveBlock(block.id, event.key === 'ArrowUp' ? -1 : 1);
		}
	}

	function useLatest(): void {
		if (!conflict) return;
		const latest = conflict;
		removeDraft(latest.document.id);
		load(latest);
		onaccept({ companyId, documentId: latest.document.id }, latest);
	}

	function keepDraft(): void {
		if (!conflict) return;
		serverView = conflict;
		loadedVersionId = conflict.current_version.version.id;
		baseDocumentVersion = conflict.document.version;
		titleDirty = title.trim() !== conflict.document.title;
		kindDirty = kind !== conflict.document.kind;
		conflict = null;
		checkpointCommand = null;
		metadataCommand = null;
		dirty = true;
		checkpointOpen = true;
		ondirtychange(true);
		scheduleDraft();
	}

	function saveLabel(): string {
		if (saveState === 'saving') return 'Saving version…';
		if (saveState === 'pending') return 'Saving draft on this device…';
		if (saveState === 'local') return 'Draft saved on this device';
		if (saveState === 'offline') return 'Offline · draft saved here';
		if (saveState === 'failed') return 'Not saved';
		return `Saved · version ${serverView?.current_version.version.version_number ?? view.current_version.version.version_number}`;
	}

	function rowsFor(text: string): number {
		return Math.max(1, Math.min(16, text.split('\n').length + Math.ceil(text.length / 78)));
	}

	const blockTypes: ReadonlyArray<{
		kind: EditableBlockKind;
		label: string;
		icon: typeof AlignLeft;
	}> = [
		{ kind: 'paragraph', label: 'Text', icon: AlignLeft },
		{ kind: 'heading', label: 'Heading', icon: Heading2 },
		{ kind: 'bulletList', label: 'Bulleted list', icon: List },
		{ kind: 'orderedList', label: 'Numbered list', icon: ListOrdered },
		{ kind: 'taskList', label: 'Checklist', icon: ListChecks },
		{ kind: 'blockquote', label: 'Quote', icon: Quote },
		{ kind: 'codeBlock', label: 'Code', icon: Braces },
		{ kind: 'horizontalRule', label: 'Divider', icon: Minus }
	];
</script>

<svelte:window onpagehide={persistDraft} />
<svelte:document onvisibilitychange={persistWhenHidden} />

<section class="document-editor" aria-label="Document editor">
	<header class="editor-head">
		<div class="document-title-field">
			<label for="document-title">Document title</label>
			<input
				id="document-title"
				value={title}
				disabled={!canEdit}
				oninput={(event) => {
					title = event.currentTarget.value;
					markChanged();
				}}
				onkeydown={(event) => {
					if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
						event.preventDefault();
						checkpointOpen = true;
					}
				}}
			/>
		</div>
		<label class="kind-control">
			<span>Type</span>
			<select
				value={kind}
				disabled={!canEdit}
				onchange={(event) => {
					kind = event.currentTarget.value as typeof kind;
					markChanged();
				}}
			>
				{#each DOCUMENT_KINDS as option (option.value)}
					<option value={option.value}>{option.label}</option>
				{/each}
			</select>
			<ChevronDown size={14} strokeWidth={1.8} aria-hidden="true" />
		</label>
		<div class="save-state" class:failed={saveState === 'failed'} aria-live="polite">
			{#if saveState === 'saved'}
				<Check size={14} strokeWidth={2} aria-hidden="true" />
			{:else if saveState === 'saving'}
				<RefreshCw class="spinning" size={14} strokeWidth={1.8} aria-hidden="true" />
			{/if}
			<span>{canEdit ? saveLabel() : draftScope ? 'Read only' : 'Verifying access…'}</span>
		</div>
		{#if canEdit}
			<button
				type="button"
				class="save-version-button"
				aria-expanded={checkpointOpen}
				disabled={!dirty || saving}
				onclick={() => (checkpointOpen = !checkpointOpen)}
			>
				Save version
			</button>
		{/if}
	</header>

	{#if checkpointOpen && dirty && !conflict}
		<form
			class="checkpoint-band"
			onsubmit={(event) => {
				event.preventDefault();
				void flush();
			}}
		>
			<label for="checkpoint-reason">
				<span>Version name</span>
				<input
					id="checkpoint-reason"
					bind:value={checkpointReason}
					maxlength="160"
					placeholder="Ready for review"
					disabled={!online || saving}
					oninput={() => {
						checkpointCommand = null;
						metadataCommand = null;
					}}
				/>
			</label>
			<div>
				<button
					type="button"
					class="btn small"
					disabled={saving}
					onclick={() => (checkpointOpen = false)}>Cancel</button
				>
				<button
					type="submit"
					class="btn small primary"
					disabled={!online || saving || !checkpointReason.trim()}
					>{saving ? 'Saving…' : 'Save checkpoint'}</button
				>
			</div>
		</form>
	{/if}

	{#if conflict}
		<section class="conflict-band" aria-labelledby="document-conflict-title">
			<div>
				<h2 id="document-conflict-title">This document changed elsewhere.</h2>
				<p>Compare the latest saved copy with the draft in this browser before choosing.</p>
			</div>
			<div class="conflict-compare">
				<div>
					<strong>Latest saved</strong>
					<pre>{conflict.current_version.version.plain_text}</pre>
				</div>
				<div>
					<strong>Your draft</strong>
					<pre>{localPlainText}</pre>
				</div>
			</div>
			<div class="conflict-actions">
				<button type="button" class="btn small" onclick={useLatest}>Use latest</button>
				<button type="button" class="btn small primary" onclick={keepDraft}>Keep my draft</button>
			</div>
		</section>
	{/if}

	{#if saveFailure}
		<div class="editor-error" role="alert">
			<span>{saveFailure}</span>
			<button type="button" class="btn small" disabled={!online} onclick={() => void flush()}
				>Retry</button
			>
		</div>
	{/if}

	<div class="paper" class:read-only={!canEdit}>
		<div class="paper-rule" aria-hidden="true"></div>
		{#each blocks as block, index (block.id)}
			<section
				class="editor-block block-{block.kind}"
				class:last={index === blocks.length - 1}
				data-block-id={block.id}
			>
				<div class="block-rail">
					<button
						type="button"
						class="grip"
						title="Move block. Alt plus arrow keys also works."
						aria-label="Move block"
						disabled={!canEdit}
						onclick={() => moveBlock(block.id, index === 0 ? 1 : -1)}
					>
						<GripVertical size={14} strokeWidth={1.7} aria-hidden="true" />
					</button>
					<button
						type="button"
						class="block-comment"
						title="Comment on this block"
						aria-label="Comment on this block"
						onclick={() => oncomment(block.id)}
					>
						<MessageSquarePlus size={14} strokeWidth={1.8} aria-hidden="true" />
					</button>
				</div>

				{#if canEdit && !block.readOnly}
					<label class="block-type-control" title="Change block type">
						<span class="sr-only">Block type</span>
						<select
							value={block.kind}
							onchange={(event) => changeBlockKind(block, event.currentTarget.value)}
						>
							{#each blockTypes as option (option.kind)}
								<option value={option.kind}>{option.label}</option>
							{/each}
						</select>
					</label>
				{/if}

				{#if block.kind === 'horizontalRule'}
					<hr />
				{:else if block.readOnly}
					<div class="unsupported-block">
						<strong>Structured table</strong>
						<p>{block.text || 'This table has no text content.'}</p>
					</div>
				{:else if canEdit}
					<textarea
						class:heading={block.kind === 'heading'}
						class:quote={block.kind === 'blockquote'}
						class:code={block.kind === 'codeBlock'}
						class:list={['bulletList', 'orderedList', 'taskList'].includes(block.kind)}
						rows={rowsFor(block.text)}
						value={block.text}
						placeholder={block.kind === 'heading'
							? 'Heading'
							: block.kind === 'taskList'
								? '[ ] One item per line'
								: 'Write here…'}
						data-new-document-block={index === blocks.length - 1 && !block.text
							? 'true'
							: undefined}
						oninput={(event) => updateBlock(block.id, { text: event.currentTarget.value })}
						onkeydown={(event) => blockKeydown(event, block)}></textarea>
				{:else}
					<div
						class="read-block"
						class:heading={block.kind === 'heading'}
						class:quote={block.kind === 'blockquote'}
						class:code={block.kind === 'codeBlock'}
					>
						{block.text || 'Empty block'}
					</div>
				{/if}

				{#if canEdit}
					<button
						type="button"
						class="remove-block"
						title="Remove block"
						aria-label="Remove block"
						onclick={() => removeBlock(block.id)}
					>
						<Trash2 size={14} strokeWidth={1.8} aria-hidden="true" />
					</button>
				{/if}
			</section>
		{:else}
			<p class="blank-document">This document is empty.</p>
		{/each}

		{#if canEdit}
			<details class="add-block">
				<summary><Plus size={14} strokeWidth={2} aria-hidden="true" /> Add a block</summary>
				<div class="add-block-menu">
					{#each blockTypes as option (option.kind)}
						{@const Icon = option.icon}
						<button type="button" onclick={() => addBlock(option.kind)}>
							<Icon size={15} strokeWidth={1.8} aria-hidden="true" />
							{option.label}
						</button>
					{/each}
				</div>
			</details>
		{/if}
	</div>
</section>

<style>
	.document-editor {
		min-width: 0;
		min-height: 0;
		display: flex;
		flex-direction: column;
		background: var(--surface-pane);
		overflow: hidden;
	}

	.editor-head {
		min-height: var(--pane-head-h);
		display: grid;
		grid-template-columns: minmax(180px, 1fr) auto auto auto;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-2) var(--space-4);
		border-bottom: 1px solid var(--border);
	}

	.document-title-field label,
	.kind-control > span {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip: rect(0 0 0 0);
		white-space: nowrap;
	}

	.document-title-field input {
		width: 100%;
		padding: 3px 1px;
		border: 0;
		border-bottom: 1px solid transparent;
		background: transparent;
		color: var(--ink);
		font: 600 var(--t-head) / 1.25 var(--font-ui);
	}

	.document-title-field input:hover:not(:disabled),
	.document-title-field input:focus {
		border-bottom-color: var(--border-strong);
	}

	.document-title-field input:focus {
		outline: none;
	}

	.document-title-field input:disabled {
		opacity: 1;
	}

	.kind-control {
		position: relative;
		display: flex;
		align-items: center;
	}

	.kind-control select,
	.block-type-control select {
		appearance: none;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: var(--surface);
		color: var(--text-secondary);
		font: 500 var(--t-body) var(--font-ui);
	}

	.kind-control select {
		padding: 5px 27px 5px 9px;
	}

	.kind-control :global(svg) {
		position: absolute;
		right: 8px;
		pointer-events: none;
	}

	.save-state {
		display: flex;
		align-items: center;
		gap: 6px;
		color: var(--text-tertiary);
		font: 500 var(--t-label) var(--font-mono);
		white-space: nowrap;
	}

	.save-state.failed {
		color: var(--state-danger);
	}

	.save-version-button {
		min-height: 30px;
		padding: 5px 10px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: var(--surface);
		box-shadow: var(--control-depth);
		color: var(--ink);
		font: 600 var(--t-body) var(--font-ui);
		cursor: pointer;
	}

	.save-version-button:disabled {
		opacity: 0.46;
		box-shadow: none;
		cursor: not-allowed;
	}

	.checkpoint-band {
		display: flex;
		align-items: end;
		justify-content: flex-end;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-4);
		border-bottom: 1px solid color-mix(in srgb, var(--surface-work) 22%, var(--border));
		background: color-mix(in srgb, var(--surface-work) 5%, var(--surface));
	}

	.checkpoint-band label {
		width: min(430px, 100%);
		display: grid;
		gap: 4px;
	}

	.checkpoint-band label span {
		color: var(--text-secondary);
		font-weight: 600;
	}

	.checkpoint-band input {
		width: 100%;
		height: 32px;
		padding: 5px 8px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: #fff;
		color: var(--ink);
		font: var(--t-body) var(--font-ui);
	}

	.checkpoint-band > div {
		display: flex;
		gap: var(--space-2);
	}

	.spinning {
		animation: spin var(--motion-working) linear infinite;
	}

	.conflict-band {
		position: relative;
		z-index: 2;
		display: grid;
		gap: var(--space-3);
		padding: var(--space-4);
		border-bottom: 1px solid color-mix(in srgb, var(--intent-authority) 28%, var(--border));
		background: color-mix(in srgb, var(--intent-authority-soft) 70%, var(--surface));
	}

	.conflict-band h2,
	.conflict-band p {
		margin: 0;
	}

	.conflict-band p {
		color: var(--text-secondary);
	}

	.conflict-compare {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-2);
	}

	.conflict-compare > div {
		min-width: 0;
		padding: var(--space-3);
		border-left: 2px solid var(--intent-authority);
		background: rgba(255, 255, 255, 0.64);
	}

	.conflict-compare pre {
		max-height: 120px;
		margin: 6px 0 0;
		overflow: auto;
		white-space: pre-wrap;
		font: var(--t-body) / 1.5 var(--font-ui);
		color: var(--text-secondary);
	}

	.conflict-actions,
	.editor-error {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: var(--space-2);
	}

	.editor-error {
		justify-content: space-between;
		padding: var(--space-2) var(--space-4);
		border-bottom: 1px solid color-mix(in srgb, var(--state-danger) 25%, var(--border));
		background: var(--state-danger-soft);
		color: var(--state-danger);
	}

	.paper {
		position: relative;
		flex: 1;
		min-height: 0;
		overflow: auto;
		padding: clamp(28px, 5vw, 68px) clamp(24px, 7vw, 96px) 112px;
		background:
			linear-gradient(
				90deg,
				transparent 0 54px,
				rgba(35, 117, 99, 0.08) 54px 55px,
				transparent 55px
			),
			#fff;
	}

	.paper-rule {
		position: absolute;
		inset: 0 auto 0 54px;
		width: 1px;
		background: rgba(35, 117, 99, 0.08);
		pointer-events: none;
	}

	.editor-block {
		position: relative;
		max-width: 840px;
		margin: 0 auto var(--space-3);
		padding: 5px 34px 5px 42px;
		border: 1px solid transparent;
		border-radius: var(--radius-control);
		transition:
			border-color var(--motion-state) var(--ease-standard),
			background-color var(--motion-state) var(--ease-standard);
	}

	.editor-block:hover,
	.editor-block:focus-within {
		border-color: color-mix(in srgb, var(--surface-work) 18%, var(--border));
		background: color-mix(in srgb, var(--surface-work) 2%, white);
	}

	.block-rail {
		position: absolute;
		left: 5px;
		top: 8px;
		display: flex;
		align-items: center;
		gap: 1px;
		opacity: 0;
		transition: opacity var(--motion-state) var(--ease-standard);
	}

	.editor-block:hover .block-rail,
	.editor-block:focus-within .block-rail {
		opacity: 1;
	}

	.grip,
	.block-comment,
	.remove-block {
		width: 26px;
		height: 26px;
		display: grid;
		place-items: center;
		padding: 0;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-tertiary);
		cursor: pointer;
	}

	.grip:hover,
	.block-comment:hover,
	.remove-block:hover {
		background: var(--accent-soft);
		color: var(--ink);
	}

	.block-comment:hover {
		color: var(--intent-conversation);
	}

	.block-type-control {
		position: absolute;
		right: calc(100% + 6px);
		top: 7px;
		opacity: 0;
		transition: opacity var(--motion-state) var(--ease-standard);
	}

	.editor-block:focus-within .block-type-control,
	.editor-block:hover .block-type-control {
		opacity: 1;
	}

	.block-type-control select {
		width: 32px;
		padding: 4px;
		color: transparent;
		cursor: pointer;
	}

	.editor-block textarea {
		width: 100%;
		min-height: 31px;
		padding: 4px 2px;
		resize: none;
		overflow: hidden;
		border: 0;
		background: transparent;
		color: var(--ink);
		font: 400 var(--t-body) / 1.72 var(--font-ui);
	}

	.editor-block textarea:focus {
		outline: none;
	}

	.editor-block textarea.heading,
	.read-block.heading {
		font-size: var(--t-title);
		font-weight: 600;
		line-height: 1.25;
		letter-spacing: -0.02em;
	}

	.editor-block textarea.quote,
	.read-block.quote {
		padding-left: var(--space-4);
		border-left: 2px solid var(--surface-attention);
		color: var(--text-secondary);
		font-style: italic;
	}

	.editor-block textarea.code,
	.read-block.code {
		padding: var(--space-3);
		border-radius: var(--radius-control);
		background: #171b24;
		color: #eef2f7;
		font-family: var(--font-mono);
		white-space: pre-wrap;
	}

	.editor-block textarea.list,
	.read-block.list {
		padding-left: var(--space-5);
	}

	.read-block {
		min-height: 31px;
		padding: 4px 2px;
		white-space: pre-wrap;
		line-height: 1.72;
	}

	.unsupported-block {
		padding: var(--space-3) var(--space-4);
		border: 1px solid var(--border);
		background: var(--surface-alt);
	}

	.unsupported-block p {
		margin: 4px 0 0;
		white-space: pre-wrap;
		color: var(--text-secondary);
	}

	.remove-block {
		position: absolute;
		right: 5px;
		top: 8px;
		opacity: 0;
	}

	.editor-block:hover .remove-block,
	.editor-block:focus-within .remove-block {
		opacity: 1;
	}

	.editor-block hr {
		margin: var(--space-4) 0;
		border: 0;
		border-top: 1px solid var(--border-strong);
	}

	.blank-document {
		max-width: 840px;
		margin: 0 auto;
		color: var(--text-tertiary);
	}

	.add-block {
		position: relative;
		max-width: 840px;
		margin: var(--space-5) auto 0;
	}

	.add-block summary {
		width: max-content;
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 6px 10px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface);
		color: var(--text-secondary);
		cursor: pointer;
		list-style: none;
	}

	.add-block summary::-webkit-details-marker {
		display: none;
	}

	.add-block-menu {
		position: absolute;
		z-index: 3;
		left: 0;
		bottom: calc(100% + 6px);
		width: 220px;
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 2px;
		padding: 6px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-pane);
		background: var(--surface);
		box-shadow: var(--shadow-lift);
		animation: bridge-popover-in var(--motion-disclosure) var(--ease-spring) both;
	}

	.add-block-menu button {
		display: flex;
		align-items: center;
		gap: 7px;
		padding: 7px 8px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-secondary);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}

	.add-block-menu button:hover {
		background: var(--accent-soft);
		color: var(--ink);
	}

	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		white-space: nowrap;
		border: 0;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	@media (max-width: 760px) {
		.editor-head {
			grid-template-columns: minmax(0, 1fr) auto auto;
			gap: var(--space-2);
			padding-inline: var(--space-3);
		}

		.save-state {
			grid-column: 1 / -1;
			min-height: 18px;
		}

		.paper {
			padding: 28px 10px 96px;
			background: #fff;
		}

		.paper-rule {
			display: none;
		}

		.editor-block {
			padding-inline: 34px;
		}

		.block-rail,
		.block-type-control,
		.remove-block {
			opacity: 1;
		}

		.block-type-control {
			right: auto;
			left: 5px;
			top: 36px;
		}

		.conflict-compare {
			grid-template-columns: 1fr;
		}

		.checkpoint-band {
			align-items: stretch;
			flex-direction: column;
		}

		.checkpoint-band label {
			width: 100%;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.spinning,
		.add-block-menu {
			animation: none;
		}
	}
</style>
