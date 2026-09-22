<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import BookOpen from '@lucide/svelte/icons/book-open';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import FilePlus2 from '@lucide/svelte/icons/file-plus-2';
	import Files from '@lucide/svelte/icons/files';
	import MessageSquare from '@lucide/svelte/icons/message-square';
	import Plus from '@lucide/svelte/icons/plus';
	import Search from '@lucide/svelte/icons/search';
	import X from '@lucide/svelte/icons/x';
	import { useQueryClient } from '@tanstack/svelte-query';
	import DocumentEditor from '$lib/components/DocumentEditor.svelte';
	import DocumentInspector, {
		type DocumentInspectorPanel
	} from '$lib/components/DocumentInspector.svelte';
	import {
		documentQuery,
		documentQueryKeys,
		documentsQuery
	} from '$lib/model/document-queries.svelte';
	import {
		failClosedDocumentRead,
		sameDocumentTarget,
		type DocumentTarget
	} from '$lib/model/document-cache';
	import {
		createDocument,
		getDocument,
		isRetryableDocumentFailure,
		pendingDocumentCreation,
		type CreateDocumentInput,
		type DocumentReadView,
		type DocumentRow,
		type PendingDocumentCreation
	} from '$lib/model/documents';
	import {
		collaborationBootstrapQuery,
		companyPrincipalQuery,
		cockpitQuery
	} from '$lib/model/queries.svelte';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const client = useQueryClient();
	const list = documentsQuery(() => companyId);
	const shellPrincipal = $derived(companyPrincipalQuery(companyId));
	const ownerAccess = $derived(shellPrincipal.view?.membership_role === 'owner');
	const collaboration = $derived(collaborationBootstrapQuery(companyId, () => shellPrincipal.view));
	const cockpit = $derived(cockpitQuery(companyId, () => ownerAccess));
	const requestedDocumentId = $derived(page.url.searchParams.get('document') ?? '');
	const selectedDocumentId = $derived(requestedDocumentId || list.summaries[0]?.document.id || '');
	const detail = documentQuery(
		() => companyId,
		() => selectedDocumentId
	);
	const documentView = $derived(detail.view);
	const inspectorValue = $derived(page.url.searchParams.get('inspect'));
	const inspectorOpen = $derived(inspectorValue !== null);
	const inspectorPanel = $derived<DocumentInspectorPanel>(
		inspectorValue === 'review' || inspectorValue === 'versions' ? inspectorValue : 'comments'
	);
	let online = $state(true);
	let search = $state('');
	let creating = $state(false);
	let createTitle = $state('');
	let createBusy = $state(false);
	let createFailure = $state('');
	let createAttempt = $state<PendingDocumentCreation | null>(null);
	let createAttemptCompanyId = $state('');
	let commentBlockId = $state<string | null>(null);
	let documentDirty = $state(false);

	const visibleDocuments = $derived.by(() => {
		const needle = search.trim().toLocaleLowerCase();
		if (!needle) return list.summaries;
		return list.summaries.filter((item) => {
			const haystack = `${item.document.title} ${item.document.kind} ${item.document.status}`;
			return haystack.toLocaleLowerCase().includes(needle);
		});
	});

	const sourceFailure = $derived(
		shellPrincipal.failure?.message ?? list.failure?.message ?? detail.failure?.message ?? ''
	);

	function documentHref(documentId: string): string {
		return `/${encodeURIComponent(companyId)}/work/documents?document=${encodeURIComponent(documentId)}`;
	}

	function documentHrefFor(company: string, documentId: string): string {
		return `/${encodeURIComponent(company)}/work/documents?document=${encodeURIComponent(documentId)}`;
	}

	function inspectorHref(panel: DocumentInspectorPanel): string {
		const query = new URLSearchParams({ document: selectedDocumentId, inspect: panel });
		return `/${encodeURIComponent(companyId)}/work/documents?${query}`;
	}

	async function openDocument(documentId: string): Promise<void> {
		const previousDocumentId = selectedDocumentId;
		const targetCompanyId = companyId;
		if (previousDocumentId && previousDocumentId !== documentId) {
			await client.cancelQueries({
				predicate: (query) =>
					query.queryKey[1] === targetCompanyId && query.queryKey[2] === previousDocumentId
			});
		}
		commentBlockId = null;
		documentDirty = false;
		await goto(documentHrefFor(targetCompanyId, documentId), { keepFocus: true, noScroll: true });
	}

	function changePanel(panel: DocumentInspectorPanel): void {
		void goto(inspectorHref(panel), { keepFocus: true, noScroll: true });
	}

	function closeInspector(): void {
		void goto(documentHref(selectedDocumentId), { keepFocus: true, noScroll: true });
	}

	function openBlockComment(blockId: string | null): void {
		commentBlockId = blockId;
		changePanel('comments');
	}

	function acceptDocument(target: DocumentTarget, view: DocumentReadView): void {
		if (!sameDocumentTarget(target, companyId, selectedDocumentId)) return;
		detail.accept(target, view);
	}

	async function submitDocument(event: SubmitEvent): Promise<void> {
		event.preventDefault();
		const title = createTitle.trim();
		if (!title || !online || createBusy) return;
		const targetCompanyId = companyId;
		if (createAttemptCompanyId !== targetCompanyId) createAttempt = null;
		createAttemptCompanyId = targetCompanyId;
		const semanticInput: Omit<CreateDocumentInput, 'content_json'> = {
			title,
			kind: 'freeform',
			visibility: 'company' as const,
			linked_room_id: null,
			inherit_room_visibility: false,
			reason: 'Created'
		};
		const attempt = pendingDocumentCreation(createAttempt, semanticInput);
		createAttempt = attempt;
		createBusy = true;
		createFailure = '';
		let createdDocumentId: string | null = null;
		try {
			const receipt = await createDocument(targetCompanyId, attempt.input, attempt.command.id);
			createdDocumentId = receipt.document_id;
			const target = { companyId: targetCompanyId, documentId: receipt.document_id };
			const created = await getDocument(target.companyId, target.documentId);
			client.setQueryData(documentQueryKeys.detail(target.companyId, target.documentId), created);
			await client.invalidateQueries({ queryKey: documentQueryKeys.list(target.companyId) });
			if (companyId !== target.companyId) return;
			creating = false;
			createTitle = '';
			createAttempt = null;
			createAttemptCompanyId = '';
			await openDocument(created.document.id);
		} catch (cause) {
			failClosedDocumentRead(client, cause, targetCompanyId, createdDocumentId);
			if (companyId !== targetCompanyId) return;
			createFailure = cause instanceof Error ? cause.message : 'The document was not created.';
			if (!isRetryableDocumentFailure(cause)) createAttempt = null;
		} finally {
			createBusy = false;
		}
	}

	function shortDate(value: string): string {
		const date = new Date(value);
		if (Number.isNaN(date.getTime())) return value;
		return date.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
	}

	function statusLabel(value: string): string {
		return value.replaceAll('_', ' ').replace(/^\w/u, (letter) => letter.toUpperCase());
	}

	function summaryLine(document: DocumentRow): string {
		return `${statusLabel(document.status)} · ${shortDate(document.updated_at)}`;
	}
</script>

<svelte:window bind:online />
<svelte:head
	><title
		>Documents — {ownerAccess
			? (cockpit.view?.company.name ?? companyId)
			: (collaboration.view?.company.name ?? companyId)}</title
	></svelte:head
>

<div
	class="cockpit-screen documents-screen"
	class:document-requested={requestedDocumentId !== ''}
	class:inspector-open={inspectorOpen}
>
	{#if sourceFailure}<div class="cockpit-error" role="alert">{sourceFailure}</div>{/if}

	<aside class="document-index cockpit-pane" aria-label="Company documents">
		<header class="cockpit-pane-head document-index-head">
			<div>
				<a
					href={`/${encodeURIComponent(companyId)}/work`}
					aria-label="Back to Work"
					title="Back to Work"
				>
					<ArrowLeft size={15} strokeWidth={2} aria-hidden="true" />
				</a>
				<h1>Documents</h1>
			</div>
			<button
				type="button"
				class="new-document-button"
				aria-expanded={creating}
				title="Create document"
				onclick={() => {
					if (creating) {
						createAttempt = null;
						createAttemptCompanyId = '';
						createTitle = '';
					}
					creating = !creating;
					createFailure = '';
				}}
			>
				{#if creating}<X size={15} strokeWidth={2} aria-hidden="true" />{:else}<Plus
						size={15}
						strokeWidth={2}
						aria-hidden="true"
					/>{/if}
				<span>{creating ? 'Cancel' : 'New'}</span>
			</button>
		</header>

		{#if creating}
			<form class="create-document" onsubmit={(event) => void submitDocument(event)}>
				<label
					><span>Title</span><input
						bind:value={createTitle}
						maxlength="200"
						placeholder="Name the document"
						disabled={!online || createBusy}
						oninput={() => (createAttempt = null)}
					/></label
				>

				{#if createFailure}<p role="alert">{createFailure}</p>{/if}
				<button
					type="submit"
					class="btn small primary"
					disabled={!online || createBusy || !createTitle.trim()}
					>{createBusy ? 'Creating…' : 'Create document'}</button
				>
			</form>
		{/if}

		<label class="document-search">
			<Search class="search-icon" size={14} strokeWidth={1.8} aria-hidden="true" />
			<span class="sr-only">Search documents</span>
			<input bind:value={search} type="search" placeholder="Find a document" />
		</label>

		<div class="document-list">
			{#each visibleDocuments as item (item.document.id)}
				<a
					class="document-row status-{item.document.status}"
					class:selected={item.document.id === selectedDocumentId}
					href={documentHref(item.document.id)}
					aria-current={item.document.id === selectedDocumentId ? 'page' : undefined}
					onclick={(event) => {
						event.preventDefault();
						void openDocument(item.document.id);
					}}
				>
					<span class="document-state" aria-hidden="true"></span>
					<span
						><strong>{item.document.title}</strong><small>{summaryLine(item.document)}</small></span
					>
					<ChevronRight size={14} strokeWidth={1.8} aria-hidden="true" />
				</a>
			{:else}
				{#if list.status === 'unknown'}
					<div class="document-list-loading" aria-label="Loading documents">
						<i></i><i></i><i></i>
					</div>
				{:else if search}
					<p class="document-list-empty">No documents match “{search}”.</p>
				{:else}
					<div class="document-list-empty">
						<Files size={19} strokeWidth={1.5} aria-hidden="true" /><strong
							>No documents yet.</strong
						>
						<p>Create a shared brief, plan, decision, or report.</p>
					</div>
				{/if}
			{/each}
		</div>
		{#if list.hasMore}
			<button
				type="button"
				class="load-document-page"
				disabled={list.loadingMore}
				onclick={() => void list.loadMore()}
				>{list.loadingMore ? 'Loading…' : 'Load older documents'}</button
			>
		{/if}
	</aside>

	<main class="document-stage cockpit-pane">
		{#if documentView}
			<header class="mobile-document-bar">
				<button
					type="button"
					aria-label="Back to Documents"
					onclick={() => void goto(`/${encodeURIComponent(companyId)}/work/documents`)}
					><ArrowLeft size={16} strokeWidth={2} /></button
				>
				<strong>{documentView.document.title}</strong>
				<button
					type="button"
					aria-label="Open comments and review"
					onclick={() => changePanel('comments')}
					><MessageSquare size={16} strokeWidth={1.8} /></button
				>
			</header>
			<DocumentEditor
				{companyId}
				companyUuid={collaboration.view?.company.company_id ?? null}
				view={documentView}
				principalActorId={shellPrincipal.view?.actor_id ?? ''}
				{online}
				onaccept={acceptDocument}
				oncomment={openBlockComment}
				ondirtychange={(value) => (documentDirty = value)}
			>
				{#snippet actions()}
                    <a class="btn small" href={`/${encodeURIComponent(companyId)}/people?${documentView.document.linked_room_id ? `room=${encodeURIComponent(documentView.document.linked_room_id)}` : 'person=exec'}&document=${encodeURIComponent(documentView.document.id)}`}>Discuss alongside</a>
					<button
						type="button"
						class="btn small"
						aria-label="Comments and history"
						aria-expanded={inspectorOpen}
						onclick={() => (inspectorOpen ? closeInspector() : changePanel('comments'))}
					>
						<MessageSquare size={15} aria-hidden="true" />
					</button>
				{/snippet}
				{#snippet moreActions()}
					<button type="button" onclick={() => changePanel('review')}>Request review…</button>
				{/snippet}
			</DocumentEditor>
		{:else if selectedDocumentId && detail.status === 'unknown'}
			<div class="document-stage-loading" aria-label="Loading document">
				<i></i><i></i><i></i><i></i>
			</div>
		{:else if selectedDocumentId}
			<div class="document-stage-empty">
				<BookOpen class="stage-empty-icon" size={25} strokeWidth={1.4} aria-hidden="true" />
				<h2>Document unavailable</h2>
				<p>This document may have moved, or you may no longer have access.</p>
				<button type="button" class="btn small" onclick={() => void detail.refresh()}
					>Try again</button
				>
			</div>
		{:else}
			<div class="document-stage-empty">
				<FilePlus2 class="stage-empty-icon" size={25} strokeWidth={1.4} aria-hidden="true" />
				<h2>Start the company notebook.</h2>
				<p>
					Briefs, plans, decisions, reports, and reviews stay readable while the Company Runtime
					sleeps.
				</p>
				<button type="button" class="btn primary" onclick={() => (creating = true)}
					>Create the first document</button
				>
			</div>
		{/if}
	</main>

	<section class="inspector-pane cockpit-pane">
		{#if documentView}
			<header class="mobile-inspector-bar">
				<button type="button" aria-label="Back to document" onclick={closeInspector}
					><ArrowLeft size={16} strokeWidth={2} /></button
				>
				<strong>{documentView.document.title}</strong>
			</header>
			<DocumentInspector
				{companyId}
				view={documentView}
				panel={inspectorPanel}
				people={ownerAccess ? (cockpit.view?.people ?? []) : (collaboration.view?.people ?? [])}
				{online}
				editing={documentDirty}
				{commentBlockId}
				ondismiss={closeInspector}
				onpanelchange={changePanel}
				oncommenttargetchange={(value: string | null) => (commentBlockId = value)}
				onaccept={acceptDocument}
			/>
		{:else}
			<div class="inspector-empty">
				<MessageSquare size={18} strokeWidth={1.5} /><span
					>Open a document to see its discussion and review.</span
				>
			</div>
		{/if}
	</section>
</div>

<style>
	.documents-screen {
		display: grid;
		grid-template-columns: minmax(220px, 270px) minmax(0, 1fr);
		gap: var(--pane-gap);
	}
	@media (min-width: 1451px) {
		.documents-screen.inspector-open {
			grid-template-columns: minmax(220px, 270px) minmax(0, 1fr) minmax(300px, 360px);
		}
		.documents-screen.inspector-open .inspector-pane {
			display: flex;
		}
	}
	.document-index,
	.document-stage,
	.inspector-pane {
		overflow: hidden;
	}
	.document-index {
		display: flex;
		flex-direction: column;
	}
	.document-index-head > div {
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 9px;
	}
	.document-index-head a,
	.mobile-document-bar button,
	.mobile-inspector-bar button {
		width: 28px;
		height: 28px;
		display: grid;
		place-items: center;
		flex: none;
		padding: 0;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface);
		color: var(--text-secondary);
		cursor: pointer;
	}
	.document-index-head a:hover,
	.mobile-document-bar button:hover,
	.mobile-inspector-bar button:hover {
		border-color: var(--border-strong);
		color: var(--ink);
	}
	.new-document-button {
		min-height: 28px;
		display: flex;
		align-items: center;
		gap: 5px;
		padding: 4px 8px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: var(--surface);
		box-shadow: var(--control-depth);
		color: var(--ink);
		font: 600 var(--t-body) var(--font-ui);
		cursor: pointer;
	}
	.new-document-button:active {
		transform: translateY(1px);
		box-shadow: var(--control-depth-pressed);
	}
	.create-document {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 108px;
		gap: 8px;
		padding: 11px;
		border-bottom: 1px solid var(--border);
		background: color-mix(in srgb, var(--surface-work) 5%, var(--surface));
		animation: bridge-popover-in var(--motion-disclosure) var(--ease-spring) both;
	}
	.create-document label {
		min-width: 0;
		display: grid;
		gap: 4px;
	}
	.create-document label > span {
		color: var(--text-secondary);
		font-weight: 600;
	}
	.create-document input {
		min-width: 0;
		width: 100%;
		height: 32px;
		padding: 5px 7px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: #fff;
		color: var(--ink);
		font: var(--t-body) var(--font-ui);
	}
	.create-document p {
		grid-column: 1 / -1;
		margin: 0;
		color: var(--state-danger);
	}
	.create-document .btn {
		grid-column: 1 / -1;
		justify-self: end;
	}
	.document-search {
		position: relative;
		display: flex;
		align-items: center;
		padding: 9px 11px;
		border-bottom: 1px solid var(--border);
		color: var(--text-tertiary);
	}
	.document-search :global(.search-icon) {
		position: absolute;
		left: 19px;
		pointer-events: none;
	}
	.document-search input {
		width: 100%;
		height: 31px;
		padding: 5px 8px 5px 29px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: rgba(255, 255, 255, 0.72);
		color: var(--ink);
		font: var(--t-body) var(--font-ui);
	}
	.document-search input:focus {
		border-color: color-mix(in srgb, var(--intent-conversation) 36%, var(--border));
		outline: 2px solid color-mix(in srgb, var(--intent-conversation) 16%, transparent);
	}
	.document-list {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
	}
	.document-row {
		min-height: 65px;
		display: grid;
		grid-template-columns: 7px minmax(0, 1fr) auto;
		align-items: center;
		gap: 9px;
		padding: 9px 11px;
		border-bottom: 1px solid var(--border);
		color: inherit;
		text-decoration: none;
		transition: background-color var(--motion-state) var(--ease-standard);
	}
	.document-row:hover {
		background: var(--surface-alt);
	}
	.document-row.selected {
		background: color-mix(in srgb, var(--surface-work) 8%, var(--surface));
		box-shadow: inset 3px 0 0 var(--surface-work);
	}
	.document-row > span:nth-child(2) {
		min-width: 0;
	}
	.document-row strong,
	.document-row small {
		display: block;
	}
	.document-row strong {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.document-row small {
		margin-top: 4px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--text-tertiary);
	}
	.document-state {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--status-offline);
	}
	.status-draft .document-state {
		background: var(--intent-conversation);
	}
	.status-in_review .document-state {
		background: var(--surface-attention);
		box-shadow: 0 0 0 3px var(--surface-attention-soft);
	}
	.status-accepted .document-state {
		background: var(--state-success);
	}
	.document-list-empty {
		display: grid;
		justify-items: start;
		gap: 5px;
		margin: 0;
		padding: var(--space-4);
		color: var(--text-tertiary);
	}
	.document-list-empty p {
		margin: 0;
	}
	.document-list-loading,
	.document-stage-loading {
		display: grid;
		gap: 9px;
		padding: var(--space-4);
	}
	.document-list-loading i,
	.document-stage-loading i {
		display: block;
		height: 42px;
		border-radius: var(--radius-control);
		background: linear-gradient(
			100deg,
			rgba(48, 57, 74, 0.06),
			rgba(48, 57, 74, 0.025) 42%,
			rgba(48, 57, 74, 0.06)
		);
		animation: document-loading var(--motion-working) var(--ease-standard) infinite;
	}
	.document-stage-loading {
		width: min(680px, 80%);
		margin: 64px auto;
	}
	.document-stage-loading i:nth-child(1) {
		width: 56%;
		height: 27px;
	}
	.document-stage-loading i:nth-child(2) {
		width: 92%;
	}
	.document-stage-loading i:nth-child(3) {
		width: 84%;
	}
	.document-stage-loading i:nth-child(4) {
		width: 67%;
	}
	.load-document-page {
		margin: 8px;
		padding: 7px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface);
		color: var(--text-secondary);
		font: inherit;
		cursor: pointer;
	}
	.document-stage {
		display: flex;
		flex-direction: column;
		background: var(--surface-pane);
	}
	.document-stage-empty {
		max-width: 460px;
		display: grid;
		justify-items: start;
		gap: var(--space-2);
		margin: auto;
		padding: var(--space-6);
	}
	.document-stage-empty :global(.stage-empty-icon) {
		color: var(--surface-work);
	}
	.document-stage-empty h2,
	.document-stage-empty p {
		margin: 0;
	}
	.document-stage-empty p {
		color: var(--text-secondary);
		line-height: 1.6;
	}
	.inspector-pane {
		display: none;
		flex-direction: column;
		background: var(--surface-rail);
	}
	.inspector-empty {
		margin: auto;
		display: grid;
		justify-items: center;
		gap: 7px;
		max-width: 220px;
		padding: var(--space-4);
		text-align: center;
		color: var(--text-tertiary);
	}
	.mobile-document-bar,
	.mobile-inspector-bar {
		display: none;
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
	@keyframes document-loading {
		0% {
			background-position: -280px 0;
		}
		100% {
			background-position: 280px 0;
		}
	}

	@media (min-width: 821px) and (max-width: 1450px) {
		.documents-screen {
			grid-template-columns: minmax(220px, 270px) minmax(0, 1fr);
		}
		.inspector-pane {
			display: none;
		}
		.documents-screen.inspector-open .document-stage {
			display: none;
		}
		.documents-screen.inspector-open .inspector-pane {
			grid-column: 2;
			display: flex;
		}
		.mobile-document-bar,
		.documents-screen.inspector-open .mobile-inspector-bar {
			min-height: 44px;
			display: grid;
			grid-template-columns: minmax(0, 1fr) auto;
			align-items: center;
			gap: var(--space-2);
			padding: 7px 9px;
			border-bottom: 1px solid var(--border);
		}
		.mobile-document-bar > button:first-child {
			display: none;
		}
		.mobile-document-bar strong,
		.mobile-inspector-bar strong {
			overflow: hidden;
			text-overflow: ellipsis;
			white-space: nowrap;
		}
		.mobile-inspector-bar strong {
			text-align: left;
		}
	}

	@media (max-width: 820px) {
		.documents-screen {
			display: block;
		}
		.document-index,
		.document-stage,
		.inspector-pane {
			width: 100%;
			height: 100%;
			border-radius: var(--radius-pane);
		}
		.document-stage,
		.inspector-pane {
			display: none;
		}
		.documents-screen.document-requested:not(.inspector-open) .document-index {
			display: none;
		}
		.documents-screen.document-requested:not(.inspector-open) .document-stage {
			display: flex;
		}
		.documents-screen.inspector-open .document-index,
		.documents-screen.inspector-open .document-stage {
			display: none;
		}
		.documents-screen.inspector-open .inspector-pane {
			display: flex;
		}
		.mobile-document-bar,
		.mobile-inspector-bar {
			min-height: 44px;
			display: grid;
			grid-template-columns: auto minmax(0, 1fr) auto;
			align-items: center;
			gap: var(--space-2);
			padding: 7px 9px;
			border-bottom: 1px solid var(--border);
		}
		.mobile-inspector-bar {
			grid-template-columns: auto minmax(0, 1fr);
		}
		.mobile-document-bar strong,
		.mobile-inspector-bar strong {
			overflow: hidden;
			text-overflow: ellipsis;
			white-space: nowrap;
			text-align: center;
		}
		.mobile-inspector-bar strong {
			text-align: left;
		}
	}

	@media (max-width: 560px) {
		.create-document {
			grid-template-columns: 1fr;
		}
		.create-document p,
		.create-document .btn {
			grid-column: 1;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.create-document,
		.document-list-loading i,
		.document-stage-loading i {
			animation: none;
		}
	}
</style>
