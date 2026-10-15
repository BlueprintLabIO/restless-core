<script lang="ts">
	import type { Snippet } from 'svelte';
	import AttentionFooter from './AttentionFooter.svelte';
	import { failureSentence } from '$lib/model/failure';
	import DocumentEditor from './LazyDocumentEditor.svelte';
	import DocumentActions from './DocumentActions.svelte';
	import MessageSquare from '@lucide/svelte/icons/message-square';
	import Markdown from '$lib/primitives/Markdown.svelte';
	import DocumentInspector, { type DocumentInspectorPanel } from './DocumentInspector.svelte';
	import { documentQuery, documentVersionQuery } from '$lib/model/document-queries.svelte';
	import { collaborationBootstrapQuery, companyPrincipalQuery } from '$lib/model/queries.svelte';
	import { sameDocumentTarget, type DocumentTarget } from '$lib/model/document-cache';
	import {
		documentFileName,
		documentMarkdownDownload,
		getDocument,
		resolveDocumentCollaboration,
		type DocumentReadView
	} from '$lib/model/documents';
	import type { NativeDocumentAttention } from '$lib/model/view';
	import { resizePane } from '$lib/actions/resize-pane';
	import './document-print.css';
	import { holdQueries } from '$lib/model/hold-queries.svelte';

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
	let reviewActions = $state<Snippet>();
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
	/* What is on screen is what downloads: the version under review, or the latest saved one. */
	async function downloadMarkdown() {
		const target = { companyId, documentId: request.document_id };
		try {
			const reviewing = request.kind === 'review' && !editLive && requestedVersion.version;
			const latest = reviewing ? null : await getDocument(target.companyId, target.documentId);
			if (latest && sameDocumentTarget(target, companyId, request.document_id))
				detail.accept(target, latest);
			const title = (latest ?? detail.view)?.document.title ?? request.title;
			const markdown = reviewing
				? (requestedVersion.version?.markdown ?? '')
				: (latest?.current_version.markdown ?? '');
			const url = URL.createObjectURL(
				new Blob([documentMarkdownDownload(title, markdown)], {
					type: 'text/markdown;charset=utf-8'
				})
			);
			const link = window.document.createElement('a');
			link.href = url;
			link.download = documentFileName(title, 'md');
			link.click();
			setTimeout(() => URL.revokeObjectURL(url), 1_000);
		} catch (error) {
			failure = failureSentence(error, 'The document could not be downloaded.');
		}
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
			failure = failureSentence(error, 'Could not complete the request.');
		} finally {
			busy = false;
		}
	}
	/* What was asked, and by whom: the editor alone does not say what the agent wants changed. */
	const asker = $derived(
		collaboration.view?.people.find((person) => person.actor_id === request.requested_by_actor_id)
			?.display ??
			(request.requested_by_actor_id === 'exec'
				? 'Exec'
				: request.requested_by_actor_id
						.replaceAll('-', ' ')
						.replace(/\b\w/g, (letter) => letter.toUpperCase()))
	);
	holdQueries(() => [shell, collaboration]);
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
	{/snippet}
	{#snippet moreActions()}
		<a
			href={`/${encodeURIComponent(companyId)}/library/documents?document=${encodeURIComponent(request.document_id)}`}
			>Open in Documents</a
		>
		<button type="button" onclick={() => void downloadMarkdown()}>Download as Markdown</button>
		<button type="button" onclick={() => requestAnimationFrame(() => window.print())}
			>Print or save as PDF</button
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
	{#if request.summary.trim()}
		<p
			class="request-line print-hide"
			title={request.kind === 'review'
				? `${asker} asked you to review this version`
				: `${asker} asked you to work on this document together`}
		>
			<strong>{asker}</strong>
			<span>{request.summary}</span>
		</p>
	{/if}
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
					<article class="requested-version document-print" aria-label="Requested document version">
						<h1 class="print-title">{detail.view?.document.title ?? request.title}</h1>
						{#if requestedVersion.version}
							<h2 class="print-hide">
								Version {requestedVersion.version.version.version_number} under review
							</h2>
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
						companyUuid={collaboration.view
							? collaboration.view.company.company_id
							: collaboration.failure
								? null
								: undefined}
						view={detail.view}
						principalActorId={shell.view?.actor_id ?? ''}
						mayRename={shell.view?.membership_role === 'owner' ||
							detail.view.document.owner_actor_id === shell.view?.actor_id}
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
			{#if inspectorOpen || request.kind === 'review'}
				<div class="document-discussion" hidden={!inspectorOpen}>
					<DocumentInspector
						{companyId}
						view={detail.view}
						{panel}
						ondismiss={() => (inspectorOpen = false)}
						requestedReviewId={request.kind === 'review' ? request.id : ''}
						onreviewactionschange={(actions) => (reviewActions = actions)}
						onrequestreviewnote={() => {
							panel = 'review';
							inspectorOpen = true;
						}}
						people={collaboration.view?.people ?? []}
						viewerActorId={shell.view?.actor_id ?? ''}
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
	<AttentionFooter>
		{#if request.kind === 'review'}
			{#if reviewActions}{@render reviewActions()}
			{:else}<div class="action-row">
					<button
						class="btn small"
						type="button"
						onclick={() => {
							panel = 'review';
							inspectorOpen = true;
						}}>Review details</button
					>
				</div>{/if}
		{:else}
			<div class="action-row">
				{#if request.kind === 'collaboration'}
					<button
						class="btn small"
						type="button"
						disabled={busy || !online || !detail.view || !readyToLeave}
						onclick={done}
						title="Finish this collaboration and clear it from the Inbox. Your document stays available in Documents."
					>
						{busy ? 'Finishing…' : 'Done'}
					</button>
				{/if}
			</div>
		{/if}
	</AttentionFooter>
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
	.document-discussion[hidden] {
		display: none;
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
	.print-title {
		display: none;
	}
	.document-unavailable {
		padding: 24px;
	}
	.request-line {
		display: flex;
		gap: 8px;
		margin: 0;
		padding: 8px 16px;
		border-bottom: 1px solid var(--border);
		color: var(--text-secondary);
		font-size: var(--t-body);
	}
	.request-line strong {
		flex: none;
		color: var(--ink);
		font-weight: 600;
	}
	.request-line span {
		min-width: 0;
		overflow-wrap: anywhere;
	}
</style>
