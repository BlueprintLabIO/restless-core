<script lang="ts">
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import { namePageContext } from '$lib/model/page-context.svelte';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	/* One document, full width, opened from the Library. Its discussion and
	 * review open beside it on wide screens and in its place on narrow ones. */
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import BookOpen from '@lucide/svelte/icons/book-open';
	import MessageSquare from '@lucide/svelte/icons/message-square';
	import Lock from '@lucide/svelte/icons/lock';
	import { PRIVATE_HINT } from '$lib/model/general-access';
	import DocumentEditor from '$lib/components/DocumentEditor.svelte';
	import DocumentShare from '$lib/components/DocumentShare.svelte';
	import DocumentAccessRequests from '$lib/components/DocumentAccessRequests.svelte';
	import LibraryCrumb from '$lib/components/LibraryCrumb.svelte';
	import DocumentInspector, {
		revealDocumentBlock,
		type DocumentInspectorPanel
	} from '$lib/components/DocumentInspector.svelte';
	import { tick } from 'svelte';
	import { Notice } from '$lib/ui/page';
	import { askExec } from '$lib/model/ask-exec';
	import { reviewAction, sendActorMessage } from '$lib/model/attention';
	import { failureKind, failureSentence } from '$lib/model/failure';
	import {
		documentCommentThreadsQuery,
		documentQuery,
		documentQueryKeys,
		documentReviewsQuery,
		documentsQuery
	} from '$lib/model/document-queries.svelte';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { sameDocumentTarget, type DocumentTarget } from '$lib/model/document-cache';
	import {
		deleteArchivedDocument,
		documentFileName,
		documentMarkdownDownload,
		duplicateDocument,
		getDocument,
		getDocumentLinks,
		getDocumentParticipants,
		requestDocumentAccess,
		setDocumentArchived,
		setDocumentParticipant,
		type DocumentAccess,
		type DocumentReadView
	} from '$lib/model/documents';
	import {
		collaborationBootstrapQuery,
		attentionQuery,
		companyPrincipalQuery,
		cockpitQuery
	} from '$lib/model/queries.svelte';
	import type { DocumentSuggestion } from '$lib/model/document-editor-schema';
	import { holdQueries } from '$lib/model/hold-queries.svelte';
	import { markForYouRead, setDocumentFollow } from '$lib/model/for-you';
	import { useForYou } from '$lib/model/for-you.svelte';
	import { formatMoment, formatRelative } from '$lib/ui/time';

	const companyId = $derived(page.params.companyId ?? '');
	const shellPrincipal = $derived(companyPrincipalQuery(companyId));
	const ownerAccess = $derived(shellPrincipal.view?.membership_role === 'owner');
	/* A guest (UXG-070) reads, discusses and, where they may, edits what was shared with them:
	 * nothing that shares it on, copies, files or reviews it for the company. */
	const guest = $derived(shellPrincipal.view?.membership_role === 'guest');
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
	/* The words selected when the comment began, so the thread points at them, not the block. */
	let commentQuote = $state<string | null>(null);
	let documentDirty = $state(false);
	let othersEditing = $state<string[]>([]);
	const failure = $derived(shellPrincipal.failure ?? detail.failure ?? null);
	const principalId = $derived(shellPrincipal.view?.actor_id ?? '');
	const people = $derived((ownerAccess ? cockpit.view?.people : collaboration.view?.people) ?? []);
	/* Whom and what "@" offers in the text: the company's people and Staff by the names they go
	 * by, recent documents and Work by title. A pasted link to one of them becomes its chip. */
	const recentDocuments = documentsQuery(() => companyId);
	const attentionView = $derived(attentionQuery(companyId, () => ownerAccess));
	const suggestions = $derived<DocumentSuggestion[]>([
		...people
			.filter((person) => person.kind !== 'system' && person.display)
			.map((person) => ({ kind: 'actor' as const, id: person.actor_id, label: person.display })),
		...recentDocuments.documents
			.filter((item) => item.id !== documentId && item.title.trim())
			.map((item) => ({ kind: 'document' as const, id: item.id, label: item.title })),
		...(
			(ownerAccess ? attentionView.view?.workGraph?.work : collaboration.view?.work_graph.work) ??
			[]
		)
			.filter((work) => work.title.trim())
			.map((work) => ({ kind: 'work' as const, id: work.id, label: work.title }))
	]);
	/* The Work this document was produced for: one its review was bound to, or one whose outputs
	 * name it. The header links to it, so the job and the document are one click apart. */
	const documentReviews = documentReviewsQuery(
		() => companyId,
		() => (guest ? '' : documentId)
	);
	const graph = $derived(
		ownerAccess ? attentionView.view?.workGraph : collaboration.view?.work_graph
	);
	const producedFor = $derived.by(() => {
		if (!documentId || !graph) return null;
		const ids = [
			...documentReviews.workIds,
			...graph.artifacts
				.filter((artifact) => artifact.work_id && artifact.uri?.includes(documentId))
				.map((artifact) => artifact.work_id as string)
		];
		return ids.map((id) => graph.work.find((work) => work.id === id)).find(Boolean) ?? null;
	});
	/* A comment asking for a change goes to the Work as a revision: the open outcome review is
	 * sent back with it, or, with none open, its lead gets it in the Work's conversation. */
	async function requestChange(request: { quote: string | null; text: string }): Promise<string> {
		const work = producedFor;
		if (!work || !documentView) throw new Error('This document is not tied to a Work any more.');
		const feedback = [
			`On “${documentView.document.title || 'Untitled'}”${request.quote ? `, at “${request.quote}”` : ''}:`,
			request.text
		].join('\n');
		const handoffs = ownerAccess ? (attentionView.view?.workGraph?.handoffs ?? []) : [];
		const review = handoffs.find(
			(handoff) =>
				handoff.work_id === work.id &&
				handoff.state === 'pending' &&
				handoff.category === 'owner_judgement'
		);
		if (review && ownerAccess) {
			await reviewAction(companyId, review.id, 'request_changes', feedback);
			return 'Sent back for changes';
		}
		await sendActorMessage(companyId, work.owner_id, `Please revise: ${feedback}`, work.id);
		return 'Change request sent to the Work’s lead';
	}
	/* Anyone who can open it sees who else can; its creator, the company's owner and the Exec
	 * change that. Anyone who may edit it retitles, archives and restores it; the company's owner
	 * may do that to anything they can read. */
	const ownsDocument = $derived(
		Boolean(documentView && principalId && documentView.document.owner_actor_id === principalId)
	);
	const sharing = $derived(inspectorValue === 'share');
	const mayRename = $derived(
		ownerAccess || ownsDocument || (!guest && documentView?.access === 'edit')
	);
	const mayArchive = $derived(ownerAccess || (!guest && documentView?.access === 'edit'));
	const archived = $derived(documentView?.document.status === 'archived');
	/* An archived document's creator, or the company's owner, deletes it for good. */
	const mayDelete = $derived(archived && (ownerAccess || ownsDocument));

	/* A menu action is answered on the shared toast, with Undo when it can be taken back. */
	const say = (message: string) => toast(message);
	const sayFailure = (message: string) => toast(message, { tone: 'failure' });

	/* Open comment threads, by the block they are on: the text marks them, the header counts
	 * them. The panel reads the same cached list. */
	const threads = documentCommentThreadsQuery(
		() => companyId,
		() => (documentView ? documentId : '')
	);
	const openThreads = $derived(threads.threads.filter((item) => item.thread.status === 'open'));
	const commentedBlocks = $derived.by(() => {
		const blocks = new Map<string, { threadId: string; count: number; quotes: string[] }>();
		for (const item of openThreads) {
			const block = item.thread.block_id;
			if (!block || item.anchor_state !== 'active') continue;
			/* The newest thread on a block is the one its mark opens; each quote is marked. */
			const quotes = blocks.get(block)?.quotes ?? [];
			blocks.set(block, {
				threadId: item.thread.id,
				count: (blocks.get(block)?.count ?? 0) + 1,
				/* An empty quote is a thread on the whole block. */
				quotes: [...quotes, item.thread.anchor_quote ?? '']
			});
		}
		return blocks;
	});
	/* A margin mark opens its thread, even when that thread is already in the address and the
	 * person has since looked at another. */
	let threadRequest = $state(0);
	let nameVersionRequest = $state(0);
	function openThread(threadId: string): void {
		const url = page.url.searchParams;
		if (url.get('inspect') === 'comments' && url.get('thread') === threadId) {
			threadRequest += 1;
			return;
		}
		void goto(documentHref({ inspect: 'comments', thread: threadId }), {
			keepFocus: true,
			noScroll: true
		});
	}

	/* Someone who may not open it, or may only read it, asks the people who share it. */
	let asked = $state('');
	async function askForAccess(access: 'read' | 'edit'): Promise<void> {
		const target = currentTarget();
		try {
			await requestDocumentAccess(target.companyId, target.documentId, access);
			asked = target.documentId;
			say(
				access === 'edit'
					? 'Asked for edit access. The people who share it will see it in For you.'
					: 'Asked. The people who share it will see it in For you.'
			);
		} catch (cause) {
			sayFailure(failureSentence(cause, 'The ask was not sent.'));
		}
	}
	const unreadable = $derived(
		!documentView &&
			!shellPrincipal.failure &&
			Boolean(detail.failure) &&
			['access', 'missing'].includes(failureKind(detail.failure))
	);
	/* Inside the company the answer is "not found" for private and gone alike, and asking is
	 * possible; from outside it there is nobody to ask. */
	const mayAsk = $derived(unreadable && failureKind(detail.failure) === 'missing');
	const ownerName = $derived(
		documentView
			? (people.find((person) => person.actor_id === documentView.document.owner_actor_id)
					?.display ?? 'its owner')
			: 'its owner'
	);

	/* A link to one block opens the document there. */
	async function copyBlockLink(blockId: string): Promise<void> {
		try {
			await navigator.clipboard.writeText(
				new URL(documentHref({ block: blockId }), location.origin).href
			);
			say('Link to the block copied');
		} catch {
			sayFailure('Could not copy the link');
		}
	}
	const blockParam = $derived(page.url.searchParams.get('block'));
	$effect(() => {
		const block = blockParam;
		if (!block || !documentView) return;
		/* The live body mounts once it syncs; look for the block for a few seconds. */
		let tries = 0;
		const timer = setInterval(() => {
			tries += 1;
			if (revealDocumentBlock(block) || tries > 24) clearInterval(timer);
		}, 250);
		return () => clearInterval(timer);
	});

	function currentTarget(): DocumentTarget {
		return { companyId, documentId };
	}

	async function copyLink(): Promise<void> {
		try {
			await navigator.clipboard.writeText(
				new URL(
					`/${encodeURIComponent(companyId)}/library/documents?document=${encodeURIComponent(documentId)}`,
					location.origin
				).href
			);
			say('Link copied');
		} catch {
			sayFailure('Could not copy the link');
		}
	}

	/* A copy you own opens with its title selected, ready to rename. */
	async function duplicate(): Promise<void> {
		try {
			const copy = await duplicateDocument(companyId, documentId, crypto.randomUUID());
			await goto(
				`/${encodeURIComponent(companyId)}/library/documents?document=${encodeURIComponent(copy.document_id)}&rename=1`
			);
		} catch (cause) {
			sayFailure(failureSentence(cause, 'It was not duplicated.'));
		}
	}

	/* The latest saved version, read fresh: edits save as versions within seconds of a pause. */
	async function download(format: 'md' | 'docx'): Promise<void> {
		const target = currentTarget();
		try {
			const latest = await getDocument(target.companyId, target.documentId);
			if (sameDocumentTarget(target, companyId, documentId)) detail.accept(target, latest);
			const blob =
				format === 'md'
					? new Blob(
							[documentMarkdownDownload(latest.document.title, latest.current_version.markdown)],
							{ type: 'text/markdown;charset=utf-8' }
						)
					: (await import('$lib/model/document-docx')).documentDocx(
							latest.document.title,
							latest.current_version.version.content_json
						);
			const url = URL.createObjectURL(blob);
			const link = window.document.createElement('a');
			link.href = url;
			link.download = documentFileName(latest.document.title, format);
			link.click();
			setTimeout(() => URL.revokeObjectURL(url), 1_000);
		} catch (cause) {
			sayFailure(failureSentence(cause, 'The document could not be downloaded.'));
		}
	}

	function printDocument(): void {
		/* Let the menu close first, so the page prints as it reads. */
		requestAnimationFrame(() => window.print());
	}

	async function setArchived(next: boolean, target = currentTarget()): Promise<void> {
		try {
			await setDocumentArchived(target.companyId, target.documentId, next);
			const latest = await getDocument(target.companyId, target.documentId);
			if (sameDocumentTarget(target, companyId, documentId)) detail.accept(target, latest);
			/* Archiving can be taken back from the toast, as from a Library row. */
			toast(next ? 'Archived' : 'Restored to the Library', {
				undo: next ? () => setArchived(false, target) : undefined
			});
		} catch (cause) {
			sayFailure(failureSentence(cause, next ? 'It was not archived.' : 'It was not restored.'));
		}
	}

	/* Deleting cannot be taken back, so it asks first; the Library's Archived list is next. */
	async function deleteForever(): Promise<void> {
		const target = currentTarget();
		const title = documentView?.document.title ?? 'this document';
		const yes = await confirmAction({
			title: `Delete “${title}” permanently?`,
			body: 'Its versions, comments and reviews go with it. This cannot be undone.',
			action: 'Delete',
			danger: true
		});
		if (!yes) return;
		try {
			await deleteArchivedDocument(target.companyId, target.documentId);
			say(`“${title}” was deleted`);
			await goto(`/${encodeURIComponent(target.companyId)}/library?show=archived`, {
				replaceState: true
			});
			queries.removeQueries({
				queryKey: documentQueryKeys.detail(target.companyId, target.documentId)
			});
			void queries.invalidateQueries({ queryKey: documentQueryKeys.list(target.companyId) });
		} catch (cause) {
			sayFailure(failureSentence(cause, `“${title}” was not deleted.`));
		}
	}

	/* Talking a document through with Exec happens beside it, in the Exec rail, with the document
	 * linked to the message. Exec sees a document only once it is shared with it, so whoever may
	 * share it shares it with Exec to comment on first, and is told. Exec applies a bounded edit
	 * the person asks for only under edit access (ARCHITECTURE §4.3), so the notice offers that
	 * one deliberate step; it can be taken back. */
	async function discuss(): Promise<void> {
		const view = documentView;
		const exec = people.find((person) => person.kind === 'exec');
		const target = currentTarget();
		if (view && exec && view.document.owner_actor_id !== exec.actor_id) {
			try {
				const listed = await getDocumentParticipants(
					target.companyId,
					target.documentId,
					null,
					200
				);
				const current = listed.items.find((row) => row.actor_id === exec.actor_id)?.access;
				if (listed.may_manage && current !== 'edit') {
					const letEdit = {
						label: `Let ${exec.display} edit`,
						run: () => void setExecAccess(target, exec, 'edit', current ?? 'comment')
					};
					if (!current) {
						await setExecAccess(target, exec, 'comment', null, false);
						toast(`${exec.display} can now read and comment on this document`, {
							action: letEdit
						});
					} else
						toast(
							`${exec.display} can ${current === 'read' ? 'read' : 'comment on'} this document`,
							{
								action: letEdit
							}
						);
				}
			} catch (cause) {
				sayFailure(
					failureSentence(cause, `${exec.display} could not be given access to this document.`)
				);
			}
		}
		/* The draft carries the document as the composer's own @ reference does. */
		askExec(
			view
				? `[${view.document.title.replace(/[[\]]/gu, '')}](/${encodeURIComponent(target.companyId)}/library/documents?document=${encodeURIComponent(target.documentId)}) `
				: undefined
		);
	}
	/* Exec's level on this document, set against its latest version; a change says so, with Undo
	 * back to the level it had. */
	async function setExecAccess(
		target: DocumentTarget,
		exec: { actor_id: string; display: string },
		access: DocumentAccess,
		previous: DocumentAccess | null,
		announce = true
	): Promise<void> {
		try {
			const before = await getDocument(target.companyId, target.documentId);
			await setDocumentParticipant(
				target.companyId,
				target.documentId,
				exec.actor_id,
				before.document.version,
				access,
				crypto.randomUUID()
			);
			const latest = await getDocument(target.companyId, target.documentId);
			if (sameDocumentTarget(target, companyId, documentId)) detail.accept(target, latest);
			shareEpoch += 1;
			if (!announce) return;
			toast(
				access === 'edit'
					? `${exec.display} can now edit this document, and applies the changes you ask for`
					: `${exec.display} can ${access === 'read' ? 'read' : 'comment on'} this document again`,
				{
					undo:
						previous && previous !== access
							? () => setExecAccess(target, exec, previous, access)
							: undefined
				}
			);
		} catch (cause) {
			if (!announce) throw cause;
			sayFailure(failureSentence(cause, `${exec.display}’s access did not change.`));
		}
	}

	/* What links here: the documents this person can open that link to this one, once each. */
	const links = createQuery(() => ({
		queryKey: ['document-links', companyId, documentId, principalId],
		queryFn: ({ signal }) => getDocumentLinks(companyId, documentId, 50, signal),
		enabled: Boolean(documentView && principalId),
		staleTime: 30_000
	}));
	const backlinks = $derived.by(() => {
		const seen = new Map<string, string>();
		for (const link of links.data?.incoming_document_references ?? [])
			if (!seen.has(link.source_document_id))
				seen.set(link.source_document_id, link.source_document_title);
		return [...seen].map(([id, title]) => ({ id, title }));
	});

	/* Following, and what others changed since this person last opened it. The notice says what
	 * was true on arrival; opening the document, and leaving it, reads its changes. */
	const forYou = useForYou();
	const queries = useQueryClient();
	let follow = $state<{ id: string; following: boolean } | null>(null);
	/* The kept copy of the document answers the next visit at once, so it learns what this one
	 * changed: the follow choice, and that the changes were seen. */
	function keep(target: DocumentTarget, patch: Partial<DocumentReadView>): void {
		queries.setQueryData<DocumentReadView>(
			documentQueryKeys.detail(target.companyId, target.documentId),
			(view) => (view ? { ...view, ...patch } : view)
		);
	}
	let changed = $state<{
		id: string;
		actor_name: string;
		at: string;
		since: string | null;
	} | null>(null);
	let arrivedAt = '';
	function readChanges(target: DocumentTarget): void {
		const keys = [`changes:${target.documentId}`];
		void (forYou ? forYou.read(keys) : markForYouRead(target.companyId, keys)).catch(
			() => undefined
		);
	}
	$effect(() => {
		const view = documentView;
		if (!view || view.document.id !== documentId || typeof view.following !== 'boolean') return;
		if (follow?.id !== documentId) follow = { id: documentId, following: view.following };
		if (arrivedAt === documentId) return;
		arrivedAt = documentId;
		const change = view.changed_since_you_looked;
		changed = change
			? {
					id: documentId,
					actor_name: change.actor_name,
					at: change.at,
					since: change.since ?? null
				}
			: null;
		if (change) {
			const target = currentTarget();
			readChanges(target);
			keep(target, { changed_since_you_looked: null });
		}
	});
	$effect(() => {
		const target = currentTarget();
		return () => {
			if (target.documentId && follow?.id === target.documentId && follow.following)
				readChanges(target);
		};
	});
	async function toggleFollow(): Promise<void> {
		if (!follow) return;
		const target = currentTarget();
		const next = !follow.following;
		follow = { id: target.documentId, following: next };
		try {
			const now = await setDocumentFollow(target.companyId, target.documentId, next);
			if (follow?.id === target.documentId) follow = { id: target.documentId, following: now };
			keep(target, { following: now });
			say(now ? 'Following. Comments and changes reach you in For you.' : 'Unfollowed');
		} catch (cause) {
			if (follow?.id === target.documentId) follow = { id: target.documentId, following: !next };
			sayFailure(failureSentence(cause, 'That did not change. Try again.'));
		}
	}
	/* History opens on what changed since this person last looked. */
	function seeChanges(): void {
		const since = changed?.since;
		changed = null;
		void goto(documentHref({ inspect: 'versions', ...(since ? { since } : {}) }), {
			keepFocus: true,
			noScroll: true
		});
	}
	const changedWhen = (at: string) => {
		const relative = formatRelative(at, '');
		/* A clock a little ahead of the server's still reads as now. */
		return relative === 'Just now' || relative === 'Soon' || relative.startsWith('in ')
			? 'just now'
			: relative;
	};

	/* Documents are found in the Library; this address without one goes there. */
	$effect(() => {
		if (!documentId)
			void goto(`/${encodeURIComponent(companyId)}/library?show=docs`, { replaceState: true });
	});

	function documentHref(query: Record<string, string> = {}): string {
		const search = new URLSearchParams({ document: documentId, ...query });
		return `/${encodeURIComponent(companyId)}/library/documents?${search}`;
	}
	function changePanel(panel: DocumentInspectorPanel | 'share'): void {
		void goto(documentHref({ inspect: panel }), { keepFocus: true, noScroll: true });
	}
	/* Below the width that fits both, the discussion covers the document; close it first. */
	async function showBlock(blockId: string): Promise<void> {
		if (window.matchMedia('(max-width: 900px)').matches) {
			await goto(documentHref(), { keepFocus: true, noScroll: true });
			await tick();
		}
		revealDocumentBlock(blockId);
	}
	function closeInspector(): void {
		void goto(documentHref(), { keepFocus: true, noScroll: true });
	}
	function openBlockComment(blockId: string | null, quote: string | null = null): void {
		commentBlockId = blockId;
		commentQuote = blockId ? quote : null;
		changePanel('comments');
	}
	function acceptDocument(target: DocumentTarget, view: DocumentReadView): void {
		if (!sameDocumentTarget(target, companyId, documentId)) return;
		detail.accept(target, view);
	}
	/* An ask granted above the sharing list redraws the list. */
	let shareEpoch = $state(0);
	function acceptGrant(target: DocumentTarget, view: DocumentReadView): void {
		acceptDocument(target, view);
		shareEpoch += 1;
	}
	namePageContext(() => documentView?.document.title ?? '');
	holdQueries(() => [shellPrincipal, collaboration, cockpit]);
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
	<section class="document-stage cockpit-pane" tabindex="-1">
		{#if unreadable}
			<!-- Private to others, moved or gone: the answer is the same, so asking reveals nothing. -->
			<div class="stage-message">
				<Lock size={22} strokeWidth={1.5} aria-hidden="true" />
				<h2>You can’t open this document</h2>
				<p>It may be private to others, or it was moved or removed.</p>
				<div class="stage-actions">
					{#if mayAsk}
						<button
							type="button"
							class="btn small primary"
							disabled={asked === documentId}
							title="Whoever shares it sees your ask in For you"
							onclick={() => void askForAccess('read')}
							>{asked === documentId ? 'Asked' : 'Ask for access'}</button
						>
					{/if}
					<a class="btn small" href={`/${encodeURIComponent(companyId)}/library?show=docs`}
						>Back to the Library</a
					>
				</div>
			</div>
		{:else if failure && !documentView}
			<div class="stage-message">
				<FailureNotice
					error={failure}
					subject="this document"
					variant="page"
					onretry={() => void detail.refresh()}
				/>
				<a class="btn small" href={`/${encodeURIComponent(companyId)}/library?show=docs`}
					>Back to the Library</a
				>
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
				principalActorId={principalId}
				nameOf={(actorId) =>
					people.find((person) => person.actor_id === actorId)?.display ?? 'Someone'}
				{online}
				{mayRename}
				onaccept={acceptDocument}
				oncomment={openBlockComment}
				{commentedBlocks}
				onopenthread={openThread}
				oncopyblocklink={(blockId) => void copyBlockLink(blockId)}
				{suggestions}
				ondirtychange={(value) => (documentDirty = value)}
				oneditorschange={(names) => (othersEditing = names)}
			>
				{#snippet leading()}<LibraryCrumb
						{companyId}
						show="docs"
					/>{#if documentView.document.general_access === 'private'}<span
							class="private"
							title={PRIVATE_HINT}><Lock size={13} strokeWidth={1.8} aria-label="Private" /></span
						>{/if}{/snippet}
				{#snippet actions()}
					{#if producedFor}<a
							class="btn small ghost for-work"
							href={`/${encodeURIComponent(companyId)}/work/${encodeURIComponent(producedFor.id)}`}
							title={`Produced for this Work: ${producedFor.title}. Opens its lead, attempts and review.`}
							>{producedFor.title}</a
						>{/if}
					{#if !guest}
						{#if !documentView.document.linked_room_id}
							<button
								type="button"
								class="btn small discuss"
								title="Talk it through with Exec beside this document. Where you may share it, Exec can then comment, and you can let Exec edit."
								onclick={() => void discuss()}>Discuss</button
							>
						{:else}
							<a
								class="btn small discuss"
								href={`/${encodeURIComponent(companyId)}/people?${documentView.document.linked_room_id ? `room=${encodeURIComponent(documentView.document.linked_room_id)}` : 'person=exec'}&document=${encodeURIComponent(documentView.document.id)}`}
								title="Talk it through with Exec, with this document alongside">Discuss</a
							>
						{/if}
						<button
							type="button"
							class="btn small share"
							class:active={sharing}
							aria-expanded={sharing}
							title="Who can open it, and at what level, Exec and Staff included"
							onclick={() => (sharing ? closeInspector() : changePanel('share'))}>Share</button
						>
					{/if}
					{#if follow?.id === documentView.document.id && !guest}
						<button
							type="button"
							class="btn small follow"
							aria-pressed={follow.following}
							title={follow.following
								? 'Comments and changes reach you in For you. Select to stop.'
								: 'Hear about comments and changes in For you'}
							onclick={() => void toggleFollow()}
							>{follow.following ? 'Following' : 'Follow'}</button
						>
					{/if}
					{#if documentView.access !== 'edit' && !ownerAccess && !archived && !guest}
						<button
							type="button"
							class="btn small ask"
							disabled={asked === documentView.document.id}
							title={`Ask ${ownerName} and whoever shares it for edit access`}
							onclick={() => void askForAccess('edit')}
							>{asked === documentView.document.id ? 'Asked' : 'Ask to edit'}</button
						>
					{/if}
					<button
						type="button"
						class="btn small icon"
						class:counted={openThreads.length > 0}
						aria-label={openThreads.length
							? `Comments (${openThreads.length} open) and history`
							: 'Comments and history'}
						title={openThreads.length
							? `${openThreads.length} open ${openThreads.length === 1 ? 'thread' : 'threads'}, and history`
							: 'Comments and history'}
						aria-expanded={inspectorOpen && !sharing}
						onclick={() => (inspectorOpen && !sharing ? closeInspector() : changePanel('comments'))}
					>
						<MessageSquare size={15} aria-hidden="true" />{#if openThreads.length}<span
								class="count">{openThreads.length}</span
							>{/if}
					</button>
				{/snippet}
				{#snippet moreActions()}
					{#if documentView}{@const editor = documentView.last_editor_actor_id
							? people.find((person) => person.actor_id === documentView.last_editor_actor_id)
									?.display
							: null}
						<p class="menu-note">
							Edited{editor ? ` by ${editor}` : ''}
							<RelativeTime value={documentView.current_version.version.created_at} />
						</p>{/if}
					<button
						type="button"
						title="The latest saved version, as a Word file"
						onclick={() => void download('docx')}>Download as Word</button
					>
					<button
						type="button"
						title="The latest saved version, as a Markdown file"
						onclick={() => void download('md')}>Download as Markdown</button
					>
					<button type="button" onclick={printDocument}>Print or save as PDF</button>
					<button type="button" onclick={() => void copyLink()}>Copy link</button>
					{#if !guest}
						<button
							type="button"
							title="A copy you own, with the same access"
							onclick={() => void duplicate()}>Duplicate</button
						>
						{#if !documentView.document.linked_room_id}
							<button type="button" class="narrow-only" onclick={() => void discuss()}
								>Discuss with Exec</button
							>
						{/if}
						<button type="button" class="narrow-only" onclick={() => changePanel('share')}
							>Share…</button
						>
					{/if}
					{#if follow?.id === documentView.document.id && !guest}
						<button type="button" class="narrow-only" onclick={() => void toggleFollow()}
							>{follow.following ? 'Unfollow' : 'Follow'}</button
						>
					{/if}
					{#if documentView.access !== 'edit' && !ownerAccess && !archived && !guest && asked !== documentView.document.id}
						<button type="button" class="narrow-only" onclick={() => void askForAccess('edit')}
							>Ask to edit</button
						>
					{/if}
					{#if guest}
						<!-- Versions, reviews and proposals are the company's to keep and decide. -->
					{:else if documentView.access === 'edit'}
						<button
							type="button"
							title="Keep what the document holds now under a name, such as the version you sent"
							onclick={() => {
								nameVersionRequest += 1;
								changePanel('versions');
							}}>Save version…</button
						>
						<button type="button" onclick={() => changePanel('review')}>Request review…</button>
					{:else}
						<button
							type="button"
							title="See a review in progress and changes agents propose. People who can edit it request reviews."
							onclick={() => changePanel('review')}>Review</button
						>
					{/if}
					{#if mayArchive}
						<button
							type="button"
							title={archived
								? 'Return it to the Library as a draft'
								: 'Take it out of the Library lists; restore it any time'}
							onclick={() => void setArchived(!archived)}>{archived ? 'Restore' : 'Archive'}</button
						>
					{/if}
					{#if backlinks.length}
						<p class="menu-label" title="Documents you can open that link to this one">
							Linked from
						</p>
						{#each backlinks.slice(0, 6) as link (link.id)}
							<a
								href={`/${encodeURIComponent(companyId)}/library/documents?document=${encodeURIComponent(link.id)}`}
								>{link.title}</a
							>
						{/each}
					{/if}
					{#if mayDelete}
						<button type="button" class="danger" onclick={() => void deleteForever()}
							>Delete permanently…</button
						>
					{/if}
				{/snippet}
				{#snippet banner()}
					{#if changed?.id === documentView.document.id && !archived}
						<div class="document-banner">
							<Notice title={`${changed.actor_name} changed this since you looked`}>
								<time datetime={changed.at} title={formatMoment(changed.at)}
									>{changedWhen(changed.at)}</time
								>
								{#snippet actions()}
									<button type="button" class="btn small" onclick={seeChanges}>See changes</button>
									<button
										type="button"
										class="btn small ghost"
										aria-label="Dismiss"
										title="Dismiss"
										onclick={() => (changed = null)}>×</button
									>
								{/snippet}
							</Notice>
						</div>
					{/if}
					{#if archived}
						<div class="document-banner">
							<Notice title="Archived">
								It is out of the Library lists. Anyone who can edit it still can.
								{#snippet actions()}
									{#if mayArchive}
										<button
											type="button"
											class="btn small"
											title="Return it to the Library as a draft"
											onclick={() => void setArchived(false)}>Restore</button
										>
									{/if}
								{/snippet}
							</Notice>
						</div>
					{/if}
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
	</section>

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
			{#if sharing}
				<div class="share-stack">
					<DocumentAccessRequests
						{companyId}
						view={documentView}
						nameOf={(actorId) =>
							people.find((person) => person.actor_id === actorId)?.display ?? 'Someone'}
						ongranted={acceptGrant}
					/>
					{#key shareEpoch}
						<DocumentShare
							{companyId}
							view={documentView}
							{people}
							onchanged={acceptDocument}
							onclose={closeInspector}
						/>
					{/key}
				</div>
			{:else}
				<DocumentInspector
					{companyId}
					view={documentView}
					panel={inspectorPanel}
					requestedThreadId={page.url.searchParams.get('thread') ?? ''}
					requestedCommentId={page.url.searchParams.get('comment') ?? ''}
					{threadRequest}
					{nameVersionRequest}
					changesSince={page.url.searchParams.get('since') ?? ''}
					{people}
					viewerActorId={principalId}
					{online}
					editing={documentDirty}
					{othersEditing}
					{commentBlockId}
					{commentQuote}
					ondismiss={closeInspector}
					onpanelchange={changePanel}
					oncommenttargetchange={(value: string | null) => {
						commentBlockId = value;
						if (!value) commentQuote = null;
					}}
					onaccept={acceptDocument}
					onshowblock={showBlock}
					onchangerequest={producedFor ? requestChange : undefined}
				/>
			{/if}
		</section>
	{/if}
</div>

<style>
	.danger {
		color: var(--state-danger);
	}
	.documents-screen {
		position: relative;
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: var(--pane-gap);
		width: 100%;
		min-height: 0;
	}
	.btn.active {
		background: var(--surface-hover);
	}
	:global(.document-actions) .narrow-only {
		display: none;
	}
	.document-banner {
		padding: 10px var(--space-4) 0;
	}
	.documents-screen.inspector-open {
		grid-template-columns: minmax(0, 1fr) minmax(300px, 360px);
	}
	.share-stack {
		display: flex;
		flex-direction: column;
		min-height: 0;
		overflow: auto;
	}
	.btn.icon.counted {
		width: auto;
		gap: 4px;
		padding: 0 8px;
	}
	.count {
		font-size: var(--t-label);
		font-variant-numeric: tabular-nums;
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
	.private {
		display: inline-grid;
		place-items: center;
		color: var(--text-tertiary);
	}
	/* A small laptop still fits the discussion beside the text, a little narrower. */
	@media (max-width: 1180px) {
		.documents-screen.inspector-open {
			grid-template-columns: minmax(0, 1fr) 300px;
		}
	}
	/* Below the width that fits both, the discussion takes the document's place, under one bar:
	 * the panel's own header would only repeat it. */
	@media (max-width: 900px) {
		.documents-screen.inspector-open {
			grid-template-columns: minmax(0, 1fr);
		}
		.documents-screen.inspector-open .document-stage {
			display: none;
		}
		.inspector-bar {
			display: grid;
		}
		.inspector-pane :global(.share-head),
		.inspector-pane :global(.inspector-dismiss) {
			display: none;
		}
	}
	@media (max-width: 560px) {
		.discuss,
		.share,
		.follow,
		.ask {
			display: none;
		}
		:global(.document-actions) .narrow-only {
			display: block;
		}
		/* On a phone the discussion is a sheet over the page, which stays in view above it. */
		.documents-screen.inspector-open .document-stage {
			display: flex;
		}
		.inspector-pane {
			position: fixed;
			right: 0;
			bottom: 0;
			left: 0;
			z-index: 20;
			height: min(72dvh, 640px);
			border-top: 1px solid var(--border-strong);
			border-radius: 14px 14px 0 0;
			box-shadow: 0 -10px 32px #18243a24;
			animation: bridge-popover-in var(--motion-disclosure) var(--ease-spring) both;
		}
	}
	.for-work {
		max-width: 220px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
