<script lang="ts">
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { untrack } from 'svelte';
	import ArtifactSidebar from '$lib/ui/navigation/ArtifactSidebar.svelte';
	import { documentSearch } from '$lib/model/document-search.svelte';
	import { isAuthoritativeDocumentFailure } from '$lib/model/document-cache';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import BookOpen from '@lucide/svelte/icons/book-open';
	import FilePlus2 from '@lucide/svelte/icons/file-plus-2';
	import MessageSquare from '@lucide/svelte/icons/message-square';
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
	let creating = $state(false);
	let createTitle = $state('');
	let createBusy = $state(false);
	let createFailure = $state('');
	let createAttempt = $state<PendingDocumentCreation | null>(null);
	let createAttemptCompanyId = $state('');
	let commentBlockId = $state<string | null>(null);
	let documentDirty = $state(false);
	let mobileFocusPending = $state(false);
	let documentStage: HTMLElement;
	let searchAccessDenied = $state(false);
	function companyDenied(cause: unknown): boolean {
		const status =
			cause && typeof cause === 'object' && 'status' in cause ? cause.status : undefined;
		return status === 401 || status === 403;
	}

	const partition = $derived(shellPrincipal.view?.cache_partition ?? '');
	const authorized = $derived(
		Boolean(partition) &&
			!isAuthoritativeDocumentFailure(shellPrincipal.failure) &&
			!searchAccessDenied &&
			!companyDenied(list.failure) &&
			!companyDenied(detail.failure)
	);
	const search = documentSearch({
		company: () => companyId,
		partition: () => partition,
		authorized: () => authorized
	});
	const sidebarItems = $derived(
		!authorized
			? []
			: search.query.trim()
				? search.hits.map((item) => ({
						id: item.document_id,
						title: item.title,
						href: documentHref(item.document_id),
						updatedAt: item.updated_at,
						hint: item.snippet
					}))
				: list.summaries.map((item) => ({
						id: item.document.id,
						title: item.document.title,
						href: documentHref(item.document.id),
						updatedAt: item.document.updated_at,
						hint: summaryLine(item.document)
					}))
	);
	$effect(() => {
		companyId;
		partition;
		searchAccessDenied = false;
		creating = false;
		createTitle = '';
		createAttempt = null;
		createAttemptCompanyId = '';
		createFailure = '';
		createBusy = false;
		mobileFocusPending = false;
		untrack(() => search.setQuery(''));
	});
	$effect(() => {
		if ([shellPrincipal.failure, list.failure, detail.failure].some(companyDenied))
			searchAccessDenied = true;
	});
	$effect(() => {
		const id = selectedDocumentId;
		if (isAuthoritativeDocumentFailure(detail.failure)) untrack(() => search.denyDocument(id));
		else if (documentView?.document.id === id) untrack(() => search.admitDocument(id));
	});
	/* The Library's "New document" lands here with ?new=1 and opens the one
	 * create flow, which keeps its own retry and duplicate protection. */
	let newRequested = false;
	$effect(() => {
		if (newRequested || page.url.searchParams.get('new') !== '1' || creating) return;
		newRequested = true;
		untrack(() => toggleCreate());
	});
	function toggleCreate() {
		if (createBusy) return;
		if (creating) {
			createAttempt = null;
			createAttemptCompanyId = '';
			createTitle = '';
		}
		creating = !creating;
		createFailure = '';
	}
	async function retrySidebar() {
		const company = companyId,
			expectedPartition = partition;
		await shellPrincipal.refresh();
		if (companyId !== company || partition !== expectedPartition || shellPrincipal.failure) return;
		await Promise.allSettled([list.refresh(), ...(selectedDocumentId ? [detail.refresh()] : [])]);
		if (companyId !== company || partition !== expectedPartition) return;
		if (![shellPrincipal.failure, list.failure, detail.failure].some(companyDenied)) {
			searchAccessDenied = false;
			search.refresh();
		}
	}

	const sourceFailure = $derived(shellPrincipal.failure ?? list.failure ?? detail.failure ?? null);

	function documentHref(documentId: string): string {
		return `/${encodeURIComponent(companyId)}/library/documents?document=${encodeURIComponent(documentId)}`;
	}

	function documentHrefFor(company: string, documentId: string): string {
		return `/${encodeURIComponent(company)}/library/documents?document=${encodeURIComponent(documentId)}`;
	}

	function inspectorHref(panel: DocumentInspectorPanel): string {
		const query = new URLSearchParams({ document: selectedDocumentId, inspect: panel });
		return `/${encodeURIComponent(companyId)}/library/documents?${query}`;
	}

	async function openDocument(documentId: string): Promise<void> {
		const previousDocumentId = selectedDocumentId;
		const targetCompanyId = companyId;
		const targetPartition = partition;
		if (previousDocumentId && previousDocumentId !== documentId) {
			await client.cancelQueries({
				predicate: (query) =>
					query.queryKey[1] === targetCompanyId && query.queryKey[2] === previousDocumentId
			});
		}
		if (companyId !== targetCompanyId || partition !== targetPartition || !authorized) return;
		commentBlockId = null;
		documentDirty = false;
		const desktop = window.matchMedia('(min-width: 821px)').matches;
		await goto(documentHrefFor(targetCompanyId, documentId), {
			keepFocus: desktop,
			noScroll: true
		});
		if (
			companyId === targetCompanyId &&
			partition === targetPartition &&
			selectedDocumentId === documentId &&
			!desktop
		) {
			mobileFocusPending = true;
			documentStage?.focus();
		}
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
		const targetPartition = partition;
		const currentCreation = () =>
			companyId === targetCompanyId && partition === targetPartition && authorized;
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
			if (!currentCreation()) return;
			createdDocumentId = receipt.document_id;
			const target = { companyId: targetCompanyId, documentId: receipt.document_id };
			const created = await getDocument(target.companyId, target.documentId);
			if (!currentCreation()) return;
			client.setQueryData(documentQueryKeys.detail(target.companyId, target.documentId), created);
			await client.invalidateQueries({ queryKey: documentQueryKeys.list(target.companyId) });
			if (!currentCreation()) return;
			creating = false;
			createTitle = '';
			createAttempt = null;
			createAttemptCompanyId = '';
			await openDocument(created.document.id);
		} catch (cause) {
			if (!currentCreation()) return;
			failClosedDocumentRead(client, cause, targetCompanyId, createdDocumentId);
			createFailure = failureSentence(cause, 'The document was not created.');
			if (!isRetryableDocumentFailure(cause)) createAttempt = null;
		} finally {
			if (currentCreation()) createBusy = false;
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
	{#if sourceFailure}
		<div class="cockpit-error">
			<FailureNotice
				error={sourceFailure}
				subject="documents"
				stale={list.status === 'stale'}
				onretry={() => void retrySidebar()}
			/>
		</div>
	{/if}

	<div class="document-index">
		{#key `${companyId}/${partition}`}
			<ArtifactSidebar
				kind="docs"
				items={sidebarItems}
				selected={selectedDocumentId}
				workHref={`/${encodeURIComponent(companyId)}/library`}
				docsHref={`/${encodeURIComponent(companyId)}/library/documents`}
				sheetsHref={`/${encodeURIComponent(companyId)}/library/sheets`}
				loading={search.query.trim() ? search.loading : list.status === 'unknown' && !list.failure}
				searching={search.loading}
				filterLocally={false}
				queryByteLimit={256}
				onquery={(value) => search.setQuery(value)}
				failure={search.query.trim()
					? !authorized
						? 'Documents are unavailable.'
						: search.failure
					: list.failure || (!authorized && shellPrincipal.failure)
						? 'Documents are unavailable.'
						: ''}
				{creating}
				createDisabled={!online || createBusy || !authorized}
				oncreate={toggleCreate}
				onopen={(item) => openDocument(item.id)}
				onretry={() => void retrySidebar()}
				hasMore={search.query.trim() ? search.hasMore : list.hasMore}
				loadingMore={search.query.trim() ? search.loadingMore : list.loadingMore}
				onloadmore={() => {
					if (search.query.trim()) search.loadMore();
					else void list.loadMore();
				}}
			>
				{#snippet creation()}
					<form class="create-document" onsubmit={(event) => void submitDocument(event)}>
						<label
							><span>Title</span><input
								bind:value={createTitle}
								maxlength="200"
								placeholder="Name the document"
								{@attach (input) => input.focus()}
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
				{/snippet}
			</ArtifactSidebar>
		{/key}
	</div>

	<main class="document-stage cockpit-pane" tabindex="-1" bind:this={documentStage}>
		{#if documentView}
			<header class="mobile-document-bar">
				<button
					type="button"
					aria-label="Back to Documents"
					{@attach (button) => {
						if (mobileFocusPending && button.getClientRects().length) {
							button.focus();
							mobileFocusPending = false;
						}
					}}
					onclick={() => void goto(`/${encodeURIComponent(companyId)}/library/documents`)}
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
				companyUuid={collaboration.view
					? collaboration.view.company.company_id
					: collaboration.failure
						? null
						: undefined}
				view={documentView}
				principalActorId={shellPrincipal.view?.actor_id ?? ''}
				{online}
				onaccept={acceptDocument}
				oncomment={openBlockComment}
				ondirtychange={(value) => (documentDirty = value)}
			>
				{#snippet actions()}
					<a
						class="btn small"
						href={`/${encodeURIComponent(companyId)}/people?${documentView.document.linked_room_id ? `room=${encodeURIComponent(documentView.document.linked_room_id)}` : 'person=exec'}&document=${encodeURIComponent(documentView.document.id)}`}
						>Discuss alongside</a
					>
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
				<h2>Start the company notebook</h2>
				<p>Briefs, plans, decisions and reports you write with the team, readable any time.</p>
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
	.mobile-document-bar button:hover,
	.mobile-inspector-bar button:hover {
		border-color: var(--border-strong);
		color: var(--ink);
	}
	.create-document {
		display: grid;
		gap: 8px;
		padding: 11px;
		border-bottom: 1px solid var(--border);
	}
	.create-document label {
		min-width: 0;
		display: grid;
		gap: 4px;
	}
	.create-document label > span {
		color: var(--text-secondary);
		font-weight: 500;
	}
	.create-document input {
		min-width: 0;
		width: 100%;
		height: 36px;
		padding: 5px 7px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		color: var(--ink);
		font: var(--t-body) var(--font-ui);
	}
	.create-document p {
		margin: 0;
		color: var(--state-danger);
	}
	.create-document .btn {
		justify-self: end;
	}
	.document-stage-loading {
		display: grid;
		gap: 9px;
		padding: var(--space-4);
		width: min(680px, 80%);
		margin: 64px auto;
	}
	.document-stage-loading i {
		display: block;
		height: 42px;
		border-radius: var(--radius-control);
		background: var(--surface-alt);
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
	.document-stage-empty .btn {
		margin-top: var(--space-2);
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
		/* The editor's own toolbar already names the document and opens
		 * comments; only the inspector, which replaces it here, needs a bar. */
		.documents-screen.inspector-open .mobile-inspector-bar {
			min-height: 44px;
			display: grid;
			grid-template-columns: minmax(0, 1fr) auto;
			align-items: center;
			gap: var(--space-2);
			padding: 7px 9px;
			border-bottom: 1px solid var(--border);
		}
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
		.document-stage-loading i {
			animation: none;
		}
	}
</style>
