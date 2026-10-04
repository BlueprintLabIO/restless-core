<script lang="ts">
	/* One document, full width, opened from the Library. Its discussion and
	 * review open beside it on wide screens and in its place on narrow ones. */
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import BookOpen from '@lucide/svelte/icons/book-open';
	import MessageSquare from '@lucide/svelte/icons/message-square';
	import DocumentEditor from '$lib/components/DocumentEditor.svelte';
	import LibraryCrumb from '$lib/components/LibraryCrumb.svelte';
	import DocumentInspector, {
		type DocumentInspectorPanel
	} from '$lib/components/DocumentInspector.svelte';
	import { documentQuery } from '$lib/model/document-queries.svelte';
	import { sameDocumentTarget, type DocumentTarget } from '$lib/model/document-cache';
	import type { DocumentReadView } from '$lib/model/documents';
	import {
		collaborationBootstrapQuery,
		companyPrincipalQuery,
		cockpitQuery
	} from '$lib/model/queries.svelte';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const shellPrincipal = $derived(companyPrincipalQuery(companyId));
	const ownerAccess = $derived(shellPrincipal.view?.membership_role === 'owner');
	const collaboration = $derived(collaborationBootstrapQuery(companyId, () => shellPrincipal.view));
	const cockpit = $derived(cockpitQuery(companyId, () => ownerAccess));
	const documentId = $derived(page.url.searchParams.get('document') ?? '');
	const detail = documentQuery(
		() => companyId,
		() => documentId
	);
	const documentView = $derived(detail.view);
	const inspectorValue = $derived(page.url.searchParams.get('inspect'));
	const inspectorOpen = $derived(inspectorValue !== null);
	const inspectorPanel = $derived<DocumentInspectorPanel>(
		inspectorValue === 'review' || inspectorValue === 'versions' ? inspectorValue : 'comments'
	);
	let online = $state(true);
	let commentBlockId = $state<string | null>(null);
	let documentDirty = $state(false);
	const failure = $derived(shellPrincipal.failure ?? detail.failure ?? null);

	/* Documents are found in the Library; this address without one goes there. */
	$effect(() => {
		if (!documentId)
			void goto(`/${encodeURIComponent(companyId)}/library?show=docs`, { replaceState: true });
	});

	function documentHref(query: Record<string, string> = {}): string {
		const search = new URLSearchParams({ document: documentId, ...query });
		return `/${encodeURIComponent(companyId)}/library/documents?${search}`;
	}
	function changePanel(panel: DocumentInspectorPanel): void {
		void goto(documentHref({ inspect: panel }), { keepFocus: true, noScroll: true });
	}
	function closeInspector(): void {
		void goto(documentHref(), { keepFocus: true, noScroll: true });
	}
	function openBlockComment(blockId: string | null): void {
		commentBlockId = blockId;
		changePanel('comments');
	}
	function acceptDocument(target: DocumentTarget, view: DocumentReadView): void {
		if (!sameDocumentTarget(target, companyId, documentId)) return;
		detail.accept(target, view);
	}
</script>

<svelte:window bind:online />
<svelte:head
	><title
		>{documentView?.document.title ?? 'Document'} — {ownerAccess
			? (cockpit.view?.company.name ?? companyId)
			: (collaboration.view?.company.name ?? companyId)}</title
	></svelte:head
>

<div class="documents-screen" class:inspector-open={inspectorOpen}>
	<main class="document-stage cockpit-pane" tabindex="-1">
		{#if failure && !documentView}
			<div class="stage-message">
				<FailureNotice
					error={failure}
					subject="this document"
					variant="page"
					onretry={() => void detail.refresh()}
				/>
			</div>
		{:else if documentView}
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
				{#snippet leading()}<LibraryCrumb {companyId} show="docs" />{/snippet}
				{#snippet actions()}
					<a
						class="btn small discuss"
						href={`/${encodeURIComponent(companyId)}/people?${documentView.document.linked_room_id ? `room=${encodeURIComponent(documentView.document.linked_room_id)}` : 'person=exec'}&document=${encodeURIComponent(documentView.document.id)}`}
						title="Talk it through with Exec, with this document alongside">Discuss</a
					>
					<button
						type="button"
						class="btn small icon"
						aria-label="Comments and history"
						title="Comments and history"
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
		{:else if documentId && detail.status === 'unknown'}
			<div class="document-stage-loading" aria-label="Loading document">
				<i></i><i></i><i></i><i></i>
			</div>
		{:else if documentId}
			<div class="stage-message">
				<BookOpen size={22} strokeWidth={1.5} aria-hidden="true" />
				<h2>Document unavailable</h2>
				<p>It may have moved, or you may no longer have access.</p>
				<div class="stage-actions">
					<a class="btn small" href={`/${encodeURIComponent(companyId)}/library?show=docs`}
						>Back to the Library</a
					>
					<button type="button" class="btn small" onclick={() => void detail.refresh()}
						>Try again</button
					>
				</div>
			</div>
		{/if}
	</main>

	{#if documentView && inspectorOpen}
		<section class="inspector-pane cockpit-pane">
			<header class="inspector-bar">
				<button
					type="button"
					aria-label="Back to the document"
					title="Back to the document"
					onclick={closeInspector}><ArrowLeft size={16} strokeWidth={2} /></button
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
		</section>
	{/if}
</div>

<style>
	.documents-screen {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: var(--pane-gap);
		width: 100%;
		min-height: 0;
	}
	.documents-screen.inspector-open {
		grid-template-columns: minmax(0, 1fr) minmax(300px, 360px);
	}
	.document-stage,
	.inspector-pane {
		display: flex;
		flex-direction: column;
		min-height: 0;
		overflow: hidden;
	}
	.document-stage {
		background: var(--surface-pane);
	}
	.inspector-pane {
		background: var(--surface-rail);
	}
	.btn.icon {
		width: 30px;
		padding: 0;
		justify-content: center;
	}
	.inspector-bar {
		display: none;
		grid-template-columns: auto minmax(0, 1fr);
		align-items: center;
		gap: var(--space-2);
		min-height: 44px;
		padding: 7px 9px;
		border-bottom: 1px solid var(--border);
	}
	.inspector-bar button {
		display: grid;
		place-items: center;
		width: 28px;
		height: 28px;
		padding: 0;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-secondary);
		cursor: pointer;
	}
	.inspector-bar button:hover {
		background: var(--wash-hover);
		color: var(--ink);
	}
	.inspector-bar strong {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.document-stage-loading {
		display: grid;
		gap: 9px;
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
	.stage-message {
		display: grid;
		justify-items: center;
		gap: var(--space-2);
		max-width: 420px;
		margin: auto;
		padding: var(--space-6);
		color: var(--text-tertiary);
		text-align: center;
	}
	.stage-message h2,
	.stage-message p {
		margin: 0;
	}
	.stage-message h2 {
		color: var(--ink);
		font-size: var(--t-head);
		font-weight: 600;
	}
	.stage-message p {
		color: var(--text-secondary);
	}
	.stage-actions {
		display: flex;
		gap: 8px;
		margin-top: var(--space-2);
	}
	/* Below the width that fits both, the discussion takes the document's place. */
	@media (max-width: 1180px) {
		.documents-screen.inspector-open {
			grid-template-columns: minmax(0, 1fr);
		}
		.documents-screen.inspector-open .document-stage {
			display: none;
		}
		.inspector-bar {
			display: grid;
		}
	}
	@media (max-width: 560px) {
		.discuss {
			display: none;
		}
	}
</style>
