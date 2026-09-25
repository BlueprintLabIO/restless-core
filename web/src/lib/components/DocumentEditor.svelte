<script lang="ts">
	import { browser } from '$app/environment';
	import { tick, untrack, type Snippet } from 'svelte';
	import DocumentActions from './DocumentActions.svelte';
	import { Editor } from '@tiptap/core';
	import { HocuspocusProvider } from '@hocuspocus/provider';
	import * as Y from 'yjs';
	import AlignLeft from '@lucide/svelte/icons/align-left';
	import BoldIcon from '@lucide/svelte/icons/bold';
	import Braces from '@lucide/svelte/icons/braces';
	import Check from '@lucide/svelte/icons/check';
	import CodeIcon from '@lucide/svelte/icons/code';
	import Heading2 from '@lucide/svelte/icons/heading-2';
	import ItalicIcon from '@lucide/svelte/icons/italic';
	import List from '@lucide/svelte/icons/list';
	import ListChecks from '@lucide/svelte/icons/list-checks';
	import ListOrdered from '@lucide/svelte/icons/list-ordered';
	import MessageSquarePlus from '@lucide/svelte/icons/message-square-plus';
	import Minus from '@lucide/svelte/icons/minus';
	import Quote from '@lucide/svelte/icons/quote';
	import Redo2 from '@lucide/svelte/icons/redo-2';
	import RefreshCw from '@lucide/svelte/icons/refresh-cw';
	import Strikethrough from '@lucide/svelte/icons/strikethrough';
	import Table2 from '@lucide/svelte/icons/table-2';
	import Undo2 from '@lucide/svelte/icons/undo-2';
	import { useQueryClient } from '@tanstack/svelte-query';
	import {
		documentCollaborationName,
		documentCollaborationStateLabel,
		documentCollaborationWebSocketUrl,
		requestDocumentCollaborationToken,
		transitionDocumentCollaborationState,
		type DocumentCollaborationAccess,
		type DocumentCollaborationState
	} from '$lib/model/document-collaboration';
	import {
		createDocumentEditorExtensions,
		selectedDocumentBlockId
	} from '$lib/model/document-editor-schema';
	import {
		failClosedDocumentRead,
		sameDocumentTarget,
		type DocumentTarget
	} from '$lib/model/document-cache';
	import {
		documentText,
		getDocument,
		isDocumentConflict,
		isRetryableDocumentFailure,
		pendingDocumentCommand,
		updateDocumentMetadata,
		type DocumentKind,
		type DocumentReadView,
		type PendingDocumentCommand
	} from '$lib/model/documents';

	interface Props {
		companyId: string;
		companyUuid: string | null;
		view: DocumentReadView;
		principalActorId: string;
		online?: boolean;
		actions?: Snippet;
		moreActions?: Snippet;
		onaccept?: (target: DocumentTarget, view: DocumentReadView) => void;
		oncomment?: (blockId: string | null) => void;
		ondirtychange?: (dirty: boolean) => void;
		onleavechange?: (ready: boolean) => void;
	}

	let {
		companyId,
		companyUuid,
		view,
		principalActorId,
		online = true,
		actions,
		moreActions,
		onaccept = () => {},
		oncomment = () => {},
		ondirtychange = () => {},
		onleavechange = () => {}
	}: Props = $props();
	const client = useQueryClient();

	let editorElement = $state<HTMLDivElement>();
	let editorState = $state<{ editor: Editor | null }>({ editor: null });
	let collaborationState = $state<DocumentCollaborationState>('connecting');
	let collaborationAccess = $state<DocumentCollaborationAccess | null>(null);
	let collaborationFailure = $state('');
	let documentRestored = $state(false);
	let retryGeneration = $state(0);
	let activeProvider = $state.raw<HocuspocusProvider | null>(null);
	const offlineProviders = new WeakSet<HocuspocusProvider>();
	let title = $state('');
	let kind = $state<DocumentKind>('freeform');
	let serverView = $state<DocumentReadView | null>(null);
	let loadedDocumentId = $state('');
	let loadedVersionId = $state('');
	let baseDocumentVersion = $state(0);
	let titleDirty = $state(false);
	let kindDirty = $state(false);
	let saving = $state(false);
	let saveFailure = $state('');
	let conflict = $state<DocumentReadView | null>(null);
	let metadataCommand = $state<PendingDocumentCommand | null>(null);
	let lastReportedDirty = false;
	let unsyncedChanges = $state(0);

	const editor = $derived(editorState.editor);
	// Metadata refreshes replace view without changing the live editing session.
	// Track primitive identity/access so polling cannot tear down the editor.
	const liveDocumentId = $derived(view.document.id);
	const liveReadAccess = $derived(view.access);
	const canWriteLive = $derived(
		collaborationState === 'synced' && collaborationAccess === 'write' && view.access === 'edit'
	);
	const dirty = $derived(
		titleDirty ||
			kindDirty ||
			saving ||
			unsyncedChanges > 0 ||
			(collaborationState !== 'synced' && collaborationState !== 'read-only')
	);
	$effect(() => {
		onleavechange(
			!titleDirty &&
				!kindDirty &&
				!saving &&
				unsyncedChanges === 0 &&
				(collaborationState === 'synced' || collaborationState === 'read-only')
		);
	});
	const statusLabel = $derived(documentCollaborationStateLabel(collaborationState));

	function canonical(value: unknown): string {
		if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`;
		if (value && typeof value === 'object') {
			const source = value as Record<string, unknown>;
			return `{${Object.keys(source)
				.sort()
				.map((key) => `${JSON.stringify(key)}:${canonical(source[key])}`)
				.join(',')}}`;
		}
		return JSON.stringify(value);
	}

	function updateMetadataDirty(): void {
		const source = serverView;
		if (!source) return;
		titleDirty = title.trim() !== source.document.title;
		kindDirty = kind !== source.document.kind;
		metadataCommand = null;
		saveFailure = '';
	}

	function load(source: DocumentReadView): void {
		serverView = source;
		loadedDocumentId = source.document.id;
		loadedVersionId = source.current_version.version.id;
		baseDocumentVersion = source.document.version;
		title = source.document.title;
		kind = source.document.kind;
		titleDirty = false;
		kindDirty = false;
		conflict = null;
		metadataCommand = null;
		saveFailure = '';
	}

	$effect(() => {
		const incoming = view;
		if (!loadedDocumentId || incoming.document.id !== loadedDocumentId) {
			load(incoming);
			return;
		}
		if (
			incoming.current_version.version.id === loadedVersionId &&
			incoming.document.version === baseDocumentVersion
		) {
			serverView = incoming;
			return;
		}
		if (
			(titleDirty && incoming.document.title !== serverView?.document.title) ||
			(kindDirty && incoming.document.kind !== serverView?.document.kind)
		) {
			conflict = incoming;
			return;
		}
		serverView = incoming;
		loadedVersionId = incoming.current_version.version.id;
		baseDocumentVersion = incoming.document.version;
		if (!titleDirty) title = incoming.document.title;
		if (!kindDirty) kind = incoming.document.kind;
	});

	$effect(() => {
		const value = dirty;
		if (value === lastReportedDirty) return;
		lastReportedDirty = value;
		ondirtychange(value);
	});

	$effect(() => {
		const instance = editorState.editor;
		if (instance && instance.isEditable !== canWriteLive) instance.setEditable(canWriteLive);
	});

	$effect(() => {
		const identity = companyUuid ?? '';
		const documentId = liveDocumentId;
		const routeCompany = companyId;
		const readAccess = liveReadAccess;
		const actorId = principalActorId;
		void retryGeneration;

		// Provider constructors invoke status callbacks synchronously. Those
		// reads must not turn connection status into a lifecycle dependency.
		return untrack(() => {
			if (!browser) return;
			editorState = { editor: null };
			collaborationAccess = null;
			collaborationFailure = '';
			documentRestored = false;
			unsyncedChanges = 0;
			if (!identity) {
				collaborationState = 'degraded';
				collaborationFailure =
					'This company has no verified collaboration identity. The latest named version is read only.';
				return;
			}

			let url: string;
			let name: string;
			try {
				url = documentCollaborationWebSocketUrl(window.location.origin, identity, documentId);
				name = documentCollaborationName(identity, documentId);
			} catch (cause) {
				collaborationState = 'degraded';
				collaborationFailure =
					cause instanceof Error ? cause.message : 'The collaboration target is invalid.';
				return;
			}

			let disposed = false;
			let mountedEditor: Editor | null = null;
			let hadConnection = false;
			const controller = new AbortController();
			const yDocument = new Y.Doc();
			collaborationState = 'connecting';

			const provider = new HocuspocusProvider({
				url,
				name,
				document: yDocument,
				flushDelay: 80,
				token: async () => {
					try {
						const grant = await requestDocumentCollaborationToken(
							routeCompany,
							documentId,
							controller.signal
						);
						if (disposed) throw new Error('Document collaboration stopped.');
						collaborationAccess = grant.access;
						return grant.token;
					} catch (cause) {
						if (!disposed) {
							collaborationState = transitionDocumentCollaborationState(
								collaborationState,
								'failed'
							);
							collaborationFailure =
								cause instanceof Error
									? cause.message
									: 'A collaboration token could not be issued.';
						}
						throw new Error('A fresh collaboration token could not be issued.');
					}
				},
				onOpen: () => {
					hadConnection = true;
				},
				onStatus: ({ status }) => {
					if (disposed || collaborationState === 'degraded') return;
					if (status === 'connecting') {
						collaborationState = transitionDocumentCollaborationState(
							collaborationState,
							'socket-connecting'
						);
					} else if (status === 'disconnected') {
						collaborationState = hadConnection
							? 'reconnecting'
							: transitionDocumentCollaborationState(collaborationState, 'socket-disconnected');
					}
				},
				onAuthenticationFailed: () => {
					if (disposed) return;
					collaborationState = transitionDocumentCollaborationState(collaborationState, 'failed');
					collaborationFailure =
						'Collaboration access was rejected. The latest named version remains read only.';
				},
				onClose: ({ event }) => {
					if (disposed || collaborationState === 'degraded') return;
					// Hocuspocus carries logical close reasons with code 1000.
					if (event.reason === 'Document restored') {
						documentRestored = true;
						collaborationState = 'degraded';
						collaborationFailure =
							'Someone restored an earlier version. Copy any recent changes you want to keep from this read-only draft, then load the restored version.';
						provider.disconnect();
						return;
					}
					if (event.code === 4401 || event.code === 4403) {
						collaborationState = 'degraded';
						collaborationFailure =
							'Collaboration access ended. Refresh access before making more changes.';
						return;
					}
					collaborationState = hadConnection ? 'reconnecting' : collaborationState;
				},
				onUnsyncedChanges: ({ number }) => {
					if (!disposed) unsyncedChanges = number;
				},
				onSynced: ({ state: synced }) => {
					if (disposed || !synced) return;
					const writable = collaborationAccess === 'write' && readAccess === 'edit';
					collaborationState = transitionDocumentCollaborationState(
						collaborationState,
						writable ? 'synced-write' : 'synced-read'
					);
					collaborationFailure = '';
					void tick().then(() => {
						if (disposed || mountedEditor || !editorElement) return;
						mountedEditor = new Editor({
							element: editorElement,
							extensions: createDocumentEditorExtensions(yDocument, provider, writable),
							editable: writable,
							editorProps: {
								attributes: {
									class: 'tiptap native-document-body',
									'aria-label': writable ? 'Collaborative document body' : 'Document body'
								}
							},
							onCreate: ({ editor: instance }) => {
								editorState = { editor: instance };
							},
							onTransaction: ({ editor: instance }) => {
								editorState = { editor: instance };
							}
						});
						editorState = { editor: mountedEditor };
						provider.awareness?.setLocalStateField('user', {
							id: actorId,
							name: actorId
						});
					});
				}
			});

			activeProvider = provider;

			const connectDeadline = setTimeout(() => {
				if (
					disposed ||
					collaborationState === 'synced' ||
					collaborationState === 'read-only' ||
					collaborationState === 'degraded'
				)
					return;
				collaborationState = 'degraded';
				collaborationFailure =
					'Live collaboration did not become ready. The latest named version remains read only.';
			}, 10_000);

			return () => {
				disposed = true;
				clearTimeout(connectDeadline);
				controller.abort();
				mountedEditor?.destroy();
				if (activeProvider === provider) activeProvider = null;
				provider.destroy();
				yDocument.destroy();
				if (editorState.editor === mountedEditor) editorState = { editor: null };
			};
		});
	});

	$effect(() => {
		const provider = activeProvider;
		const connected = online;
		untrack(() => {
			if (!provider) return;
			if (!connected) {
				offlineProviders.add(provider);
				collaborationState = 'degraded';
				collaborationFailure =
					'You are offline. Unsynced live changes remain in this tab and will retry when the connection returns.';
				provider.disconnect();
			} else if (offlineProviders.delete(provider)) {
				collaborationState = 'reconnecting';
				collaborationFailure = '';
				// Reauthenticate on the same Y.Doc so pending local changes survive.
				void provider.connect();
			}
		});
	});

	function run(command: (instance: Editor) => boolean): void {
		const instance = editorState.editor;
		if (!instance || !canWriteLive) return;
		command(instance);
	}

	function addTable(): void {
		run((instance) =>
			instance.commands.insertContent({
				type: 'table',
				content: [
					{
						type: 'tableRow',
						content: [
							{ type: 'tableHeader', content: [{ type: 'paragraph' }] },
							{ type: 'tableHeader', content: [{ type: 'paragraph' }] }
						]
					},
					{
						type: 'tableRow',
						content: [
							{ type: 'tableCell', content: [{ type: 'paragraph' }] },
							{ type: 'tableCell', content: [{ type: 'paragraph' }] }
						]
					}
				]
			})
		);
	}

	function commentOnSelection(): void {
		const instance = editorState.editor;
		oncomment(instance ? selectedDocumentBlockId(instance) : null);
	}

	async function saveTitle(retryHistoryAdvance = true): Promise<void> {
		if (
			!titleDirty ||
			!canWriteLive ||
			view.document.owner_actor_id !== principalActorId ||
			saving ||
			conflict ||
			!serverView
		)
			return;
		if (!title.trim()) {
			saveFailure = 'Give the document a title.';
			return;
		}
		const target = { companyId, documentId: serverView.document.id };
		const source = serverView.document;
		const input = {
			expected_version: baseDocumentVersion,
			title: title.trim(),
			kind: source.kind,
			visibility: source.visibility,
			linked_room_id: source.linked_room_id,
			inherit_room_visibility: source.inherit_room_visibility
		};
		const command = pendingDocumentCommand(metadataCommand, canonical(input));
		metadataCommand = command;
		saving = true;
		saveFailure = '';
		try {
			await updateDocumentMetadata(target.companyId, target.documentId, input, command.id);
			const accepted = await getDocument(target.companyId, target.documentId);
			if (!sameDocumentTarget(target, companyId, view.document.id)) return;
			serverView = accepted;
			baseDocumentVersion = accepted.document.version;
			titleDirty = title.trim() !== accepted.document.title;
			metadataCommand = null;
			onaccept(target, accepted);
		} catch (cause) {
			failClosedDocumentRead(client, cause, target.companyId, target.documentId);
			if (!sameDocumentTarget(target, companyId, view.document.id)) return;
			if (!isRetryableDocumentFailure(cause)) metadataCommand = null;
			if (isDocumentConflict(cause)) {
				try {
					const latest = await getDocument(target.companyId, target.documentId);
					if (!sameDocumentTarget(target, companyId, view.document.id)) return;
					if (
						retryHistoryAdvance &&
						latest.document.title === source.title &&
						latest.document.kind === source.kind &&
						latest.document.visibility === source.visibility &&
						latest.document.linked_room_id === source.linked_room_id &&
						latest.document.inherit_room_visibility === source.inherit_room_visibility
					) {
						// A history snapshot advanced the revision, but nobody changed this metadata.
						serverView = latest;
						baseDocumentVersion = latest.document.version;
						loadedVersionId = latest.current_version.version.id;
						saving = false;
						await saveTitle(false);
					} else conflict = latest;
				} catch {
					saveFailure = 'Could not reload the document title.';
				}
			} else saveFailure = cause instanceof Error ? cause.message : 'The title was not saved.';
		} finally {
			if (sameDocumentTarget(target, companyId, view.document.id)) saving = false;
		}
	}

	function useLatest(): void {
		if (!conflict) return;
		const latest = conflict;
		serverView = latest;
		loadedVersionId = latest.current_version.version.id;
		baseDocumentVersion = latest.document.version;
		title = latest.document.title;
		kind = latest.document.kind;
		titleDirty = false;
		kindDirty = false;
		conflict = null;
		metadataCommand = null;
		onaccept({ companyId, documentId: latest.document.id }, latest);
	}

	function keepMetadata(): void {
		if (!conflict) return;
		const latest = conflict;
		serverView = latest;
		loadedVersionId = latest.current_version.version.id;
		baseDocumentVersion = latest.document.version;
		titleDirty = title.trim() !== latest.document.title;
		kindDirty = kind !== latest.document.kind;
		conflict = null;
		metadataCommand = null;
		void saveTitle();
	}
</script>

<section class="document-editor" aria-label="Document editor">
	<header class="editor-head">
		<div class="document-title-field">
			<label for="document-title">Document title</label>
			<input
				id="document-title"
				onblur={() => void saveTitle()}
				onkeydown={(event) => {
					if (event.key === 'Enter') event.currentTarget.blur();
				}}
				value={title}
				disabled={!canWriteLive || view.document.owner_actor_id !== principalActorId}
				oninput={(event) => {
					title = event.currentTarget.value;
					updateMetadataDirty();
				}}
			/>
		</div>
		<div
			class="collaboration-state state-{collaborationState}"
			aria-live="polite"
			title={canWriteLive ? 'Edits and version history save automatically.' : statusLabel}
		>
			{#if collaborationState === 'synced'}
				<Check size={14} strokeWidth={2} aria-hidden="true" />
			{:else if collaborationState === 'connecting' || collaborationState === 'reconnecting'}
				<RefreshCw class="spinning" size={14} strokeWidth={1.8} aria-hidden="true" />
			{/if}
			<span
				>{unsyncedChanges > 0 || saving
					? 'Saving…'
					: titleDirty
						? 'Unsaved title'
						: canWriteLive
							? 'Saved'
							: statusLabel}</span
			>
		</div>
		<div class="editor-actions">
			{@render actions?.()}
			{#if moreActions}
				<DocumentActions>{@render moreActions()}</DocumentActions>
			{/if}
		</div>
	</header>

	{#if editor}
		<div class="format-toolbar" aria-label="Document formatting">
			<button
				type="button"
				class:active={editor.isActive('paragraph')}
				disabled={!canWriteLive}
				aria-label="Paragraph"
				title="Paragraph"
				onclick={() => run((instance) => instance.chain().focus().setParagraph().run())}
				><AlignLeft size={15} strokeWidth={1.8} /></button
			>
			<button
				type="button"
				class:active={editor.isActive('heading', { level: 2 })}
				disabled={!canWriteLive}
				aria-label="Heading"
				title="Heading"
				onclick={() =>
					run((instance) => instance.chain().focus().toggleHeading({ level: 2 }).run())}
				><Heading2 size={15} strokeWidth={1.8} /></button
			>
			<span class="toolbar-rule"></span>
			<button
				type="button"
				class:active={editor.isActive('bold')}
				disabled={!canWriteLive}
				aria-label="Bold"
				title="Bold"
				onclick={() => run((instance) => instance.chain().focus().toggleBold().run())}
				><BoldIcon size={15} strokeWidth={1.8} /></button
			>
			<button
				type="button"
				class:active={editor.isActive('italic')}
				disabled={!canWriteLive}
				aria-label="Italic"
				title="Italic"
				onclick={() => run((instance) => instance.chain().focus().toggleItalic().run())}
				><ItalicIcon size={15} strokeWidth={1.8} /></button
			>
			<button
				type="button"
				class:active={editor.isActive('strike')}
				disabled={!canWriteLive}
				aria-label="Strike"
				title="Strike"
				onclick={() => run((instance) => instance.chain().focus().toggleStrike().run())}
				><Strikethrough size={15} strokeWidth={1.8} /></button
			>
			<button
				type="button"
				class:active={editor.isActive('code')}
				disabled={!canWriteLive}
				aria-label="Inline code"
				title="Inline code"
				onclick={() => run((instance) => instance.chain().focus().toggleCode().run())}
				><CodeIcon size={15} strokeWidth={1.8} /></button
			>
			<span class="toolbar-rule"></span>
			<button
				type="button"
				class:active={editor.isActive('bulletList')}
				disabled={!canWriteLive}
				aria-label="Bulleted list"
				title="Bulleted list"
				onclick={() => run((instance) => instance.chain().focus().toggleBulletList().run())}
				><List size={15} strokeWidth={1.8} /></button
			>
			<button
				type="button"
				class:active={editor.isActive('orderedList')}
				disabled={!canWriteLive}
				aria-label="Numbered list"
				title="Numbered list"
				onclick={() => run((instance) => instance.chain().focus().toggleOrderedList().run())}
				><ListOrdered size={15} strokeWidth={1.8} /></button
			>
			<button
				type="button"
				class:active={editor.isActive('taskList')}
				disabled={!canWriteLive}
				aria-label="Checklist"
				title="Checklist"
				onclick={() => run((instance) => instance.chain().focus().toggleTaskList().run())}
				><ListChecks size={15} strokeWidth={1.8} /></button
			>
			<button
				type="button"
				class:active={editor.isActive('blockquote')}
				disabled={!canWriteLive}
				aria-label="Quote"
				title="Quote"
				onclick={() => run((instance) => instance.chain().focus().toggleBlockquote().run())}
				><Quote size={15} strokeWidth={1.8} /></button
			>
			<button
				type="button"
				class:active={editor.isActive('codeBlock')}
				disabled={!canWriteLive}
				aria-label="Code block"
				title="Code block"
				onclick={() => run((instance) => instance.chain().focus().toggleCodeBlock().run())}
				><Braces size={15} strokeWidth={1.8} /></button
			>
			<button
				type="button"
				disabled={!canWriteLive}
				aria-label="Divider"
				title="Divider"
				onclick={() => run((instance) => instance.chain().focus().setHorizontalRule().run())}
				><Minus size={15} strokeWidth={1.8} /></button
			>
			<button
				type="button"
				disabled={!canWriteLive}
				aria-label="Table"
				title="Insert a two-column table"
				onclick={addTable}><Table2 size={15} strokeWidth={1.8} /></button
			>
			<span class="toolbar-rule"></span>
			<button
				type="button"
				disabled={!canWriteLive || !editor.can().undo()}
				aria-label="Undo"
				title="Undo"
				onclick={() => run((instance) => instance.chain().focus().undo().run())}
				><Undo2 size={15} strokeWidth={1.8} /></button
			>
			<button
				type="button"
				disabled={!canWriteLive || !editor.can().redo()}
				aria-label="Redo"
				title="Redo"
				onclick={() => run((instance) => instance.chain().focus().redo().run())}
				><Redo2 size={15} strokeWidth={1.8} /></button
			>
			<button
				type="button"
				class="comment-button"
				aria-label="Comment on selected block"
				title="Comment on selected block"
				onclick={commentOnSelection}
				><MessageSquarePlus size={15} strokeWidth={1.8} /> Comment</button
			>
		</div>
	{/if}

	{#if conflict}
		<section class="conflict-band" aria-labelledby="document-conflict-title">
			<div>
				<h2 id="document-conflict-title">The title changed elsewhere.</h2>
				<p>The live body is already merged. Choose which title to carry forward.</p>
			</div>
			<div class="conflict-compare">
				<div>
					<strong>Latest named version</strong>
					<pre>{conflict.current_version.version.plain_text}</pre>
				</div>
				<div>
					<strong>Live document</strong>
					<pre>{editor?.getText() ?? documentText(view.current_version.version.content_json)}</pre>
				</div>
			</div>
			<div class="conflict-actions">
				<button type="button" class="btn small" onclick={useLatest}>Use latest metadata</button
				><button type="button" class="btn small primary" onclick={keepMetadata}
					>Keep my metadata</button
				>
			</div>
		</section>
	{/if}

	{#if collaborationFailure}
		<div class="collaboration-error" role="alert">
			<div><strong>Live editing is unavailable.</strong><span>{collaborationFailure}</span></div>
			<button
				type="button"
				class="btn small"
				disabled={!online}
				onclick={() => (retryGeneration += 1)}>{documentRestored ? 'Load restored version' : 'Retry'}</button
			>
		</div>
	{/if}

	{#if saveFailure}
		<div class="editor-error" role="alert">
			<span>{saveFailure}</span>{#if titleDirty}<button
					type="button"
					class="btn small"
					disabled={!canWriteLive || saving}
					onclick={() => void saveTitle()}>Retry title</button
				>{/if}
		</div>
	{/if}

	<div class="paper" class:read-only={!canWriteLive}>
		<div class="paper-rule" aria-hidden="true"></div>
		{#if collaborationState === 'synced' || collaborationState === 'read-only' || editor}
			<div class="editor-mount" bind:this={editorElement}></div>
		{:else}
			<div class="named-version-projection">
				<div class="projection-note">
					<strong>Latest named version</strong><span
						>{collaborationState === 'connecting'
							? 'Live document is connecting.'
							: 'Read only.'}</span
					>
				</div>
				<div class="rendered-document">{@html view.current_version.rendered_html}</div>
			</div>
		{/if}
	</div>
</section>

<style>
	.document-editor {
		container: document-editor / inline-size;
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
		grid-template-columns: minmax(0, 1fr) auto auto;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-2) var(--space-4);
		border-bottom: 1px solid var(--border);
	}
	.document-title-field label {
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
	.editor-actions {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.document-title-field {
		min-width: 0;
	}
	.collaboration-state {
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 6px;
		color: var(--text-tertiary);
		font: 500 var(--t-label) var(--font-ui);
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}
	.collaboration-state > span {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.collaboration-state :global(svg) {
		flex-shrink: 0;
	}

	.collaboration-state.state-synced {
		color: var(--surface-work);
	}
	.collaboration-state.state-degraded {
		color: var(--state-danger);
	}
	.format-toolbar button:disabled {
		opacity: 0.42;
		cursor: not-allowed;
	}
	.format-toolbar {
		display: flex;
		align-items: center;
		gap: 2px;
		padding: 6px var(--space-4);
		border-bottom: 1px solid var(--border);
		background: color-mix(in srgb, var(--surface-alt) 55%, var(--surface));
		overflow-x: auto;
	}
	.format-toolbar button {
		min-width: 28px;
		height: 28px;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 5px;
		padding: 0 6px;
		border: 1px solid transparent;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-secondary);
		font: 600 var(--t-label) var(--font-ui);
		cursor: pointer;
	}
	.format-toolbar button:hover:not(:disabled),
	.format-toolbar button.active {
		border-color: var(--border);
		background: var(--surface);
		color: var(--ink);
	}
	.format-toolbar .comment-button {
		margin-left: auto;
		padding-inline: 9px;
		white-space: nowrap;
	}
	.toolbar-rule {
		width: 1px;
		height: 18px;
		margin: 0 3px;
		background: var(--border);
		flex: none;
	}
	.conflict-actions {
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
	.conflict-actions {
		justify-content: flex-end;
	}
	.collaboration-error,
	.editor-error {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
		padding: var(--space-2) var(--space-4);
		border-bottom: 1px solid color-mix(in srgb, var(--state-danger) 25%, var(--border));
		background: var(--state-danger-soft);
		color: var(--state-danger);
	}
	.collaboration-error > div {
		display: flex;
		align-items: baseline;
		gap: var(--space-2);
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
	.editor-mount,
	.named-version-projection {
		max-width: 840px;
		margin: 0 auto;
	}
	.projection-note {
		display: flex;
		align-items: baseline;
		gap: var(--space-2);
		margin-bottom: var(--space-5);
		padding-bottom: var(--space-2);
		border-bottom: 1px solid var(--border);
		color: var(--text-tertiary);
		font: var(--t-label) var(--font-ui);
		font-variant-numeric: tabular-nums;
	}
	.projection-note strong {
		color: var(--text-secondary);
	}
	:global(.native-document-body) {
		min-height: 55vh;
		padding: 0 2px 64px;
		color: var(--ink);
		font: 400 var(--t-body) / 1.72 var(--font-ui);
		outline: none;
	}
	:global(.native-document-body > *) {
		position: relative;
		margin: 0 0 var(--space-3);
	}
	:global(.native-document-body h1),
	:global(.native-document-body h2),
	:global(.native-document-body h3),
	:global(.native-document-body h4),
	:global(.native-document-body h5),
	:global(.native-document-body h6) {
		margin-top: var(--space-6);
		margin-bottom: var(--space-3);
		font-size: var(--t-title);
		font-weight: 600;
		line-height: 1.25;
		letter-spacing: -0.02em;
	}
	:global(.native-document-body blockquote) {
		padding-left: var(--space-4);
		border-left: 2px solid var(--surface-attention);
		color: var(--text-secondary);
		font-style: italic;
	}
	:global(.native-document-body pre) {
		padding: var(--space-3);
		border-radius: var(--radius-control);
		background: #171b24;
		color: #eef2f7;
		font: var(--t-body) / 1.55 var(--font-mono);
		white-space: pre-wrap;
	}
	:global(.native-document-body ul),
	:global(.native-document-body ol) {
		padding-left: var(--space-6);
	}
	:global(.native-document-body ul[data-type='taskList']) {
		padding-left: 0;
		list-style: none;
	}
	:global(.native-document-body ul[data-type='taskList'] li) {
		display: flex;
		align-items: flex-start;
		gap: var(--space-2);
	}
	:global(.native-document-body table) {
		width: 100%;
		margin: var(--space-5) 0;
		border-collapse: collapse;
	}
	:global(.native-document-body th),
	:global(.native-document-body td) {
		min-width: 120px;
		padding: var(--space-2) var(--space-3);
		border: 1px solid var(--border-strong);
		text-align: left;
		vertical-align: top;
	}
	:global(.native-document-body th) {
		background: var(--surface-alt);
		font-weight: 600;
	}
	:global(.native-document-body hr) {
		margin: var(--space-6) 0;
		border: 0;
		border-top: 1px solid var(--border-strong);
	}
	:global(.native-document-body a) {
		color: var(--intent-conversation);
		text-decoration: underline;
		text-underline-offset: 2px;
	}
	:global(.native-document-body .native-mention),
	:global(.native-document-body .native-reference) {
		padding: 1px 4px;
		border-radius: var(--radius-control);
		background: var(--accent-soft);
		color: var(--ink);
		font-weight: 600;
	}
	:global(.native-document-body.ProseMirror-focused > [data-block-id]) {
		transition: background-color var(--motion-state) var(--ease-standard);
	}
	:global(.native-document-body.ProseMirror-focused > [data-block-id]:hover) {
		background: color-mix(in srgb, var(--surface-work) 3%, transparent);
	}
	:global(.rendered-document) {
		color: var(--ink);
		font: 400 var(--t-body) / 1.72 var(--font-ui);
	}
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
	@container document-editor (max-width: 760px) {
		.editor-head {
			grid-template-columns: minmax(0, 1fr) auto;
			gap: var(--space-2);
			padding-inline: var(--space-3);
		}
		.collaboration-state {
			grid-column: 1;
			grid-row: 2;
		}
		.editor-actions {
			grid-column: 2;
			grid-row: 1 / 3;
		}
		.format-toolbar {
			padding-inline: var(--space-3);
		}
		.paper {
			padding: 28px 18px 96px;
			background: #fff;
		}
		.paper-rule {
			display: none;
		}
		.conflict-compare {
			grid-template-columns: 1fr;
		}
		.collaboration-error > div {
			align-items: flex-start;
			flex-direction: column;
			gap: 2px;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.spinning {
			animation: none;
		}
	}
</style>
