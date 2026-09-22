<script lang="ts">
	import { beforeNavigate, goto } from '$app/navigation';
	import { page } from '$app/state';
	import X from '@lucide/svelte/icons/x';
	import MessageSquare from '@lucide/svelte/icons/message-square';
	import DocumentEditor from './DocumentEditor.svelte';
	import DocumentInspector, { type DocumentInspectorPanel } from './DocumentInspector.svelte';
	import { sameDocumentTarget, type DocumentTarget } from '$lib/model/document-cache';
	import { documentQuery, documentsQuery } from '$lib/model/document-queries.svelte';
	import {
		updateDocumentMetadata,
		pendingDocumentCommand,
		type DocumentReadView,
		type PendingDocumentCommand
	} from '$lib/model/documents';
	let {
		companyId,
		companyUuid,
		actorId,
		roomId = '',
		onclose
	}: {
		companyId: string;
		companyUuid: string | null;
		actorId: string;
		roomId?: string;
		onclose: () => void;
	} = $props();
	const list = documentsQuery(() => companyId);
	const id = $derived(page.url.searchParams.get('document') ?? '');
	const detail = documentQuery(
		() => companyId,
		() => id
	);
	let online = $state(true);
	let ready = $state(true);
	let busy = $state(false);
	let failure = $state('');
	let command: PendingDocumentCommand | null = null;
	let inspectorOpen = $state(false);
	let panel = $state<DocumentInspectorPanel>('comments');
	let commentBlockId = $state<string | null>(null);
	let dirty = $state(false);
	let initializedDocument = $state('');
	$effect(() => {
		const document = `${companyId}:${id}`;
		if (initializedDocument === document) return;
		initializedDocument = document;
		inspectorOpen = false;
		panel = 'comments';
		commentBlockId = null;
		dirty = false;
	});
	beforeNavigate((navigation) => {
		if (navigation.to?.url.href !== page.url.href && (busy || !ready)) navigation.cancel();
	});
	function select(value: string) {
		if (!ready) return;
		const url = new URL(page.url);
		url.searchParams.set('document', value);
		void goto(url, { noScroll: true, keepFocus: true });
	}
	function accept(target: DocumentTarget, view: DocumentReadView) {
		if (sameDocumentTarget(target, companyId, id)) detail.accept(target, view);
	}
	async function link() {
		if (!detail.view || !roomId || busy || !ready) return;
		const source = detail.view.document;
		const target = { companyId, documentId: source.id };
		const input = {
			expected_version: source.version,
			title: source.title,
			kind: source.kind,
			visibility: source.visibility,
			linked_room_id: roomId,
			inherit_room_visibility: false
		};
		const attempt = pendingDocumentCommand(command, JSON.stringify(input));
		command = attempt;
		busy = true;
		failure = '';
		try {
			await updateDocumentMetadata(companyId, source.id, input, attempt.id);
			if (sameDocumentTarget(target, companyId, id) && command?.id === attempt.id) {
				await detail.refresh();
				await list.refresh();
				command = null;
			}
		} catch (cause) {
			if (sameDocumentTarget(target, companyId, id) && command?.id === attempt.id)
				failure = cause instanceof Error ? cause.message : 'Could not link document.';
		} finally {
			busy = false;
		}
	}
</script>

<svelte:window bind:online />
<aside class="document-sidecar cockpit-pane" aria-label="Conversation document">
	<header>
		<select
			aria-label="Open document alongside conversation"
			value={id}
			disabled={!ready || busy}
			onchange={(e) => select(e.currentTarget.value)}
			><option value="">Choose a document…</option>{#each list.documents as doc (doc.id)}<option
					value={doc.id}>{doc.title}</option
				>{/each}</select
		><button
			aria-label="Close document"
			title={ready ? 'Close document' : 'Waiting for document changes to save'}
			disabled={!ready || busy}
			onclick={onclose}><X size={16} /></button
		>
	</header>
	{#if list.hasMore}<button class="more" onclick={() => list.loadMore()}>More documents</button
		>{/if}
	{#if detail.view}
		<div class="document-context">
			{#if detail.view.document.linked_room_id}<a
					href={`/${companyId}/people?room=${detail.view.document.linked_room_id}&document=${id}`}
					>Open linked conversation</a
				>
			{:else if roomId && detail.view.document.owner_actor_id === actorId}<button
					disabled={busy || !ready || !online}
					title="Associate this document with the conversation. Existing document access stays the same."
					onclick={link}>Link to this conversation</button
				>{/if}
		</div>
		<div class="document-panes" class:with-inspector={inspectorOpen}>
			<div class="document-body">
				{#key `${companyId}:${id}`}<DocumentEditor
						{companyId}
						{companyUuid}
						view={detail.view}
						principalActorId={actorId}
						{online}
						onaccept={accept}
						onleavechange={(value) => (ready = value)}
						ondirtychange={(value) => (dirty = value)}
						oncomment={(block) => {
							commentBlockId = block;
							panel = 'comments';
							inspectorOpen = true;
						}}
					>
						{#snippet actions()}<button
								class="discussion-toggle"
								type="button"
								aria-label={inspectorOpen ? 'Hide comments and history' : 'Comments and history'}
								title="Document comments, review requests, and saved versions"
								aria-expanded={inspectorOpen}
								onclick={() => (inspectorOpen = !inspectorOpen)}><MessageSquare size={15} /></button
							>{/snippet}
					</DocumentEditor>{/key}
			</div>
			{#if inspectorOpen}<div class="document-discussion">
					<DocumentInspector
						{companyId}
						view={detail.view}
						{panel}
						{online}
						editing={dirty}
						{commentBlockId}
						ondismiss={() => (inspectorOpen = false)}
						onpanelchange={(value) => (panel = value)}
						oncommenttargetchange={(value) => (commentBlockId = value)}
						onaccept={accept}
					/>
				</div>{/if}
		</div>
	{:else}<p class="empty">
			{detail.failure?.message ??
				list.failure?.message ??
				(id ? 'Loading document…' : 'Choose a document to work on beside the conversation.')}
		</p>{/if}
	{#if failure}<p class="empty" role="alert">{failure}</p>{/if}
	{#if !ready}<p class="saving" role="status">Saving changes before closing…</p>{/if}
</aside>

<style>
	.document-sidecar {
		min-height: 0;
		min-width: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
	header {
		display: flex;
		gap: 8px;
		align-items: center;
		padding: 9px;
		border-bottom: 1px solid var(--border);
	}
	select {
		flex: 1;
		min-width: 0;
		padding: 7px;
		background: var(--surface);
		color: var(--ink);
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		font: inherit;
	}
	button,
	a {
		font: inherit;
		font-size: var(--t-label);
		color: var(--intent-conversation);
	}
	button {
		border: 0;
		background: transparent;
		padding: 8px;
		cursor: pointer;
	}
	button:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.document-context {
		display: flex;
		justify-content: flex-end;
		padding: 0 8px;
	}
	.document-context a {
		padding: 8px;
	}
	.document-panes {
		min-height: 0;
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
	.document-body,
	.document-discussion {
		min-height: 0;
		display: flex;
		flex-direction: column;
	}
	.document-body {
		flex: 1;
	}
	.document-discussion {
		min-height: 280px;
		border-top: 1px solid var(--border);
	}
	.document-body :global(.document-editor),
	.document-discussion :global(.document-inspector) {
		flex: 1;
		min-height: 0;
		overflow: hidden;
	}
	.discussion-toggle {
		display: grid;
		place-items: center;
		color: var(--text-secondary);
	}
	.empty,
	.saving {
		padding: 12px;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
</style>
