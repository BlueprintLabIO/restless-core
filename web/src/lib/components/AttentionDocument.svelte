<script lang="ts">
	import DocumentEditor from './DocumentEditor.svelte';
	import DocumentActions from './DocumentActions.svelte';
	import MessageSquare from '@lucide/svelte/icons/message-square';
	import Markdown from '$lib/primitives/Markdown.svelte';
	import DocumentInspector, { type DocumentInspectorPanel } from './DocumentInspector.svelte';
	import { documentQuery, documentVersionQuery } from '$lib/model/document-queries.svelte';
	import { collaborationBootstrapQuery, companyPrincipalQuery } from '$lib/model/queries.svelte';
	import { sameDocumentTarget, type DocumentTarget } from '$lib/model/document-cache';
	import { resolveDocumentCollaboration, type DocumentReadView } from '$lib/model/documents';
	import type { NativeDocumentAttention } from '$lib/model/view';
	import { resizePane } from '$lib/actions/resize-pane';

	let {
		companyId,
		request,
		onresolved = () => {}
	}: {
		companyId: string;
		request: NativeDocumentAttention;
		onresolved?: () => void;
	} = $props();
	const detail = documentQuery(
		() => companyId,
		() => request.document_id
	);
	const requestedVersion = documentVersionQuery(
		() => companyId,
		() => request.document_id,
		() => request.named_version_id ?? ''
	);
	let editLive = $state(false);
	const shell = $derived(companyPrincipalQuery(companyId));
	const collaboration = $derived(collaborationBootstrapQuery(companyId, () => shell.view));
	let online = $state(true);
	let dirty = $state(false);
	let readyToLeave = $state(false);
	let busy = $state(false);
	let failure = $state('');
	let panel = $state<DocumentInspectorPanel>('comments');
	let inspectorOpen = $state(false);
	let commentBlockId = $state<string | null>(null);
	let width = $state(0);
	let initializedRequest = $state('');
	$effect(() => {
		if (initializedRequest === request.id) return;
		initializedRequest = request.id;
		inspectorOpen = false;
		panel = 'comments';
		if (request.kind === 'review') {
			panel = 'review';
			inspectorOpen = true;
		}
	});
	function accept(target: DocumentTarget, view: DocumentReadView) {
		if (!sameDocumentTarget(target, companyId, request.document_id)) return;
		detail.accept(target, view);
		onresolved();
	}
	async function done() {
		if (busy || !online || !detail.view || !readyToLeave) return;
		const target = { companyId, documentId: request.document_id };
		const requestId = request.id;
		busy = true;
		failure = '';
		try {
			await resolveDocumentCollaboration(target.companyId, target.documentId, requestId);
			if (sameDocumentTarget(target, companyId, request.document_id) && request.id === requestId)
				onresolved();
		} catch (error) {
			failure = error instanceof Error ? error.message : 'Could not complete the request.';
		} finally {
			busy = false;
		}
	}
</script>

<svelte:window bind:online />
<section class="attention-document" aria-label="Shared document workspace" bind:clientWidth={width}>
	{#snippet actions()}
		<button
			class="discussion-toggle"
			type="button"
			aria-label={inspectorOpen ? 'Hide comments and history' : 'Comments and history'}
			title="Document comments, review requests, and saved versions"
			aria-expanded={inspectorOpen}
			onclick={() => (inspectorOpen = !inspectorOpen)}
		>
			<MessageSquare size={16} />
		</button>
		{#if request.kind === 'collaboration'}
			<button
				class="btn small"
				type="button"
				disabled={busy || !online || !detail.view || !readyToLeave}
				onclick={done}
				title="Finish this collaboration and clear it from Attention. Your document stays available in Documents."
			>
				{busy ? 'Finishing…' : 'Done'}
			</button>
		{/if}
	{/snippet}
	{#snippet moreActions()}
		<a
			href={`/${encodeURIComponent(companyId)}/work/documents?document=${encodeURIComponent(request.document_id)}`}
			>Open in Documents</a
		>
		{#if request.kind === 'review'}
			<button
				type="button"
				disabled={editLive && !readyToLeave}
				onclick={() => (editLive = !editLive)}
			>
				{editLive ? 'View requested version' : 'Edit live document'}
			</button>
		{/if}
	{/snippet}
	{#if request.kind === 'review' && !editLive}
		<header>
			<h1>{detail.view?.document.title ?? request.title}</h1>
			<div class="request-actions">
				{@render actions()}<DocumentActions>{@render moreActions()}</DocumentActions>
			</div>
		</header>
	{/if}
	{#if failure}<p class="cockpit-error" role="alert">{failure}</p>{/if}
	{#if detail.view}
		<div
			class="document-panes"
			class:with-inspector={inspectorOpen}
			class:wide={width >= 850}
			use:resizePane={{
				key: `${companyId}:attention-document`,
				label: 'Resize document discussion',
				target: '.document-discussion',
				variable: '--discussion-width',
				side: 'start',
				min: 260,
				minOther: 400,
				defaultSize: 320,
				enabled: inspectorOpen && width >= 850,
				breakpoint: 0
			}}
		>
			<div class="document-body">
				{#if request.kind === 'review' && !editLive}
					<article class="requested-version" aria-label="Requested document version">
						{#if requestedVersion.version}
							<h2>Version {requestedVersion.version.version.version_number} under review</h2>
							<Markdown text={requestedVersion.version.markdown} />
						{:else}<p>
								{requestedVersion.failure
									? 'The requested version is unavailable.'
									: 'Loading requested version…'}
							</p>{/if}
					</article>
				{:else}
					<DocumentEditor
						{actions}
						{moreActions}
						{companyId}
						companyUuid={collaboration.view?.company.company_id ?? null}
						view={detail.view}
						principalActorId={shell.view?.actor_id ?? ''}
						{online}
						onaccept={accept}
						onleavechange={(value) => (readyToLeave = value)}
						ondirtychange={(value) => (dirty = value)}
						oncomment={(block) => {
							commentBlockId = block;
							panel = 'comments';
							inspectorOpen = true;
						}}
					/>
				{/if}
			</div>
			{#if inspectorOpen}
				<div class="document-discussion">
					<DocumentInspector
						{companyId}
						view={detail.view}
						{panel}
						ondismiss={() => (inspectorOpen = false)}
						requestedReviewId={request.kind === 'review' ? request.id : ''}
						people={collaboration.view?.people ?? []}
						{online}
						editing={dirty}
						{commentBlockId}
						onpanelchange={(value) => (panel = value)}
						oncommenttargetchange={(value) => (commentBlockId = value)}
						onaccept={accept}
					/>
				</div>
			{/if}
		</div>
	{:else}
		<div class="document-unavailable" role="status">
			<h2>{detail.status === 'unknown' ? 'Loading document…' : 'Document unavailable'}</h2>
			{#if detail.status !== 'unknown'}<p>Your access may have changed.</p>
				<button class="btn small" onclick={() => void detail.refresh()}>Try again</button>{/if}
		</div>
	{/if}
</section>

<style>
	.attention-document {
		min-width: 0;
		min-height: 0;
		height: 100%;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		background: var(--surface);
	}
	header {
		display: flex;
		flex-wrap: wrap;
		gap: 12px;
		align-items: center;
		justify-content: space-between;
		padding: 10px 16px;
		border-bottom: 1px solid var(--border);
	}
	h1 {
		margin: 0;
		min-width: 0;
		overflow-wrap: anywhere;
		font-size: var(--t-head);
		flex: 1;
	}
	.discussion-toggle {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		border: 1px solid transparent;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-secondary);
		cursor: pointer;
	}
	.discussion-toggle:hover,
	.discussion-toggle[aria-expanded='true'] {
		background: var(--surface-alt);
		border-color: var(--border);
	}
	p {
		margin: 4px 0 0;
		color: var(--text-secondary);
	}
	.request-actions {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}
	.document-panes {
		min-width: 0;
		min-height: 0;
		flex: 1;
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		overflow: auto;
	}
	.document-panes.with-inspector {
		grid-template-rows: minmax(440px, 1fr) minmax(320px, auto);
	}
	.document-panes.wide.with-inspector {
		grid-template-columns: minmax(0, 1fr) var(--discussion-width, 320px);
		grid-template-rows: minmax(0, 1fr);
		overflow: hidden;
	}
	.document-body,
	.document-discussion {
		min-width: 0;
		min-height: 0;
		display: flex;
		flex-direction: column;
	}
	.document-discussion {
		border-left: 1px solid var(--border);
		border-top: 1px solid var(--border);
	}
	.document-body :global(.document-editor),
	.document-discussion :global(.document-inspector) {
		flex: 1;
	}
	.requested-version {
		padding: 24px;
		overflow: auto;
		overflow-wrap: anywhere;
	}
	.requested-version h2 {
		font-size: var(--t-head);
	}
	.document-unavailable {
		padding: 24px;
	}
</style>
