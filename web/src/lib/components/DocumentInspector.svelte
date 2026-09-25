<script lang="ts">
	import Check from '@lucide/svelte/icons/check';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import History from '@lucide/svelte/icons/history';
	import MessageSquare from '@lucide/svelte/icons/message-square';
	import MessagesSquare from '@lucide/svelte/icons/messages-square';
	import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
	import Scale from '@lucide/svelte/icons/scale';
	import Sparkles from '@lucide/svelte/icons/sparkles';
	import X from '@lucide/svelte/icons/x';
	import { useQueryClient } from '@tanstack/svelte-query';
	import {
		failClosedDocumentRead,
		sameDocumentTarget,
		type DocumentTarget,
		type DocumentThreadTarget
	} from '$lib/model/document-cache';
	import {
		documentCommentsQuery,
		documentCommentThreadsQuery,
		documentProposalQuery,
		documentProposalsQuery,
		documentQueryKeys,
		documentReviewsQuery,
		documentVersionQuery,
		documentVersionsQuery,
		invalidateDocument
	} from '$lib/model/document-queries.svelte';
	import {
		acceptDocumentReview,
		acceptDocumentRevisionProposal,
		commentContent,
		createDocumentCommentThread,
		getDocument,
		isRetryableDocumentFailure,
		pendingDocumentCommand,
		pendingDocumentMutation,
		rejectDocumentRevisionProposal,
		replyToDocumentComment,
		requestDocumentReview,
		resolveDocumentCommentThread,
		restoreDocumentVersion,
		type DocumentReadView,
		type DocumentContent,
		type DocumentRevisionProposalSummary,
		type PendingDocumentCommand,
		type PendingDocumentMutation
	} from '$lib/model/documents';

	export type DocumentInspectorPanel = 'comments' | 'review' | 'versions';

	interface PersonReference {
		actor_id: string;
		display: string;
	}

	interface Props {
		companyId: string;
		view: DocumentReadView;
		panel: DocumentInspectorPanel;
		requestedReviewId?: string;
		people?: PersonReference[];
		online?: boolean;
		editing?: boolean;
		commentBlockId?: string | null;
		onpanelchange?: (panel: DocumentInspectorPanel) => void;
		oncommenttargetchange?: (blockId: string | null) => void;
		onaccept?: (target: DocumentTarget, view: DocumentReadView) => void;
		ondismiss?: () => void;
	}

	let {
		companyId,
		view,
		panel,
		requestedReviewId = '',
		people = [],
		online = true,
		editing = false,
		commentBlockId = null,
		onpanelchange = () => {},
		oncommenttargetchange = () => {},
		onaccept = () => {},
		ondismiss
	}: Props = $props();

	const client = useQueryClient();
	const documentId = $derived(view.document.id);
	const threads = documentCommentThreadsQuery(
		() => companyId,
		() => documentId
	);
	const reviews = documentReviewsQuery(
		() => companyId,
		() => documentId
	);
	const proposals = documentProposalsQuery(
		() => companyId,
		() => documentId
	);
	const versions = documentVersionsQuery(
		() => companyId,
		() => documentId
	);
	let selectedThreadId = $state('');
	let selectedProposalId = $state('');
	let selectionDocumentId = $state('');
	let composingThread = $state(false);
	let commentText = $state('');
	let commentBusy = $state(false);
	let commentFailure = $state('');
	let reviewSummary = $state('');
	let reviewName = $state('Accepted review');
	let reviewBusy = $state(false);
	let reviewFailure = $state('');
	let reviewNotice = $state('');
	let resolutionSummary = $state('');
	let proposalVersionName = $state('Accepted agent revision');
	let proposalBusy = $state(false);
	let proposalFailure = $state('');
	let proposalNotice = $state('');
	let restoringVersionId = $state('');
	let restoreCommand = $state<PendingDocumentCommand | null>(null);
	let selectedVersionId = $state('');
	let versionFailure = $state('');
	let versionNotice = $state('');
	let commentAttempt = $state<PendingDocumentMutation<CommentMutationInput> | null>(null);
	let resolveAttempt = $state<PendingDocumentMutation<ResolveMutationInput> | null>(null);
	let reviewRequestAttempt = $state<PendingDocumentMutation<ReviewRequestInput> | null>(null);
	let reviewAcceptAttempt = $state<PendingDocumentMutation<ReviewAcceptInput> | null>(null);
	let proposalAttempt = $state<PendingDocumentMutation<ProposalMutationInput> | null>(null);

	type CommentMutationInput =
		| {
				mode: 'thread';
				target: DocumentTarget;
				blockId: string | null;
				contentJson: DocumentContent;
		  }
		| {
				mode: 'reply';
				target: DocumentThreadTarget;
				contentJson: DocumentContent;
		  };
	interface ResolveMutationInput {
		target: DocumentThreadTarget;
		expectedThreadVersion: number;
	}
	interface ReviewRequestInput {
		target: DocumentTarget;
		expectedDocumentVersion: number;
		expectedCurrentVersionId: string;
		summary: string;
	}
	interface ReviewAcceptInput {
		target: DocumentTarget;
		reviewId: string;
		expectedDocumentVersion: number;
		expectedReviewVersion: number;
		acceptedVersionName: string;
	}
	interface ProposalMutationInput {
		target: DocumentTarget;
		proposalId: string;
		expectedProposalVersion: number;
		decision: 'accept' | 'reject';
		resolutionSummary: string;
		acceptedVersionName: string;
	}

	const threadMessages = documentCommentsQuery(
		() => companyId,
		() => documentId,
		() => selectedThreadId
	);
	const selectedThread = $derived(
		threads.threads.find((item) => item.thread.id === selectedThreadId) ?? null
	);
	const requestedReview = $derived(
		reviews.reviews.find(
			(review) =>
				review.status === 'requested' && (!requestedReviewId || review.id === requestedReviewId)
		) ?? null
	);
	const openProposals = $derived(
		proposals.proposals.filter((proposal) => proposal.status === 'proposed')
	);
	const selectedProposal = $derived(
		openProposals.find((proposal) => proposal.id === selectedProposalId) ?? openProposals[0] ?? null
	);
	const proposalDetail = documentProposalQuery(
		() => companyId,
		() => documentId,
		() => selectedProposal?.id ?? ''
	);
	const versionDetail = documentVersionQuery(
		() => companyId,
		() => documentId,
		() => selectedVersionId
	);
	const knownActorIds = $derived(new Set(people.map((person) => person.actor_id)));
	const canComment = $derived(view.access === 'comment' || view.access === 'edit');
	const canJudge = $derived(view.access === 'edit');
	const canResolveDocument = $derived(canJudge && !editing);

	$effect(() => {
		if (selectionDocumentId === documentId) return;
		selectionDocumentId = documentId;
		selectedThreadId = '';
		selectedProposalId = '';
		selectedVersionId = '';
		restoreCommand = null;
		commentAttempt = null;
		resolveAttempt = null;
		reviewRequestAttempt = null;
		reviewAcceptAttempt = null;
		proposalAttempt = null;
		commentBusy = false;
		reviewBusy = false;
		proposalBusy = false;
		restoringVersionId = '';
		commentFailure = '';
		reviewFailure = '';
		reviewNotice = '';
		proposalFailure = '';
		proposalNotice = '';
		versionFailure = '';
		versionNotice = '';
		composingThread = false;
		oncommenttargetchange(null);
	});

	$effect(() => {
		if (!selectedThreadId && threads.threads.length) {
			selectedThreadId = threads.threads[0].thread.id;
		}
	});

	$effect(() => {
		if (!selectedProposalId && openProposals.length) selectedProposalId = openProposals[0].id;
	});

	$effect(() => {
		if (!selectedVersionId && versions.versions.length) {
			selectedVersionId = versions.versions[0].id;
		}
	});

	$effect(() => {
		if (commentBlockId !== null) {
			composingThread = true;
			commentText = '';
			commentFailure = '';
			commentAttempt = null;
		}
	});

	function actorName(actorId: string): string {
		return people.find((person) => person.actor_id === actorId)?.display ?? actorId;
	}

	function isCurrentTarget(target: DocumentTarget): boolean {
		return sameDocumentTarget(target, companyId, documentId);
	}

	function shortDate(value: string): string {
		const date = new Date(value);
		if (Number.isNaN(date.getTime())) return value;
		return date.toLocaleString(undefined, {
			month: 'short',
			day: 'numeric',
			hour: 'numeric',
			minute: '2-digit'
		});
	}

	function statusLabel(value: string): string {
		return value.replaceAll('_', ' ').replace(/^\w/u, (letter) => letter.toUpperCase());
	}

	function beginDocumentComment(): void {
		oncommenttargetchange(null);
		composingThread = true;
		commentText = '';
		commentFailure = '';
		commentAttempt = null;
	}

	async function submitComment(event: SubmitEvent): Promise<void> {
		event.preventDefault();
		const body = commentText.trim();
		if (!body || !online || !canComment || commentBusy) return;
		const target = { companyId, documentId };
		const mode = composingThread ? 'thread' : 'reply';
		const semanticFingerprint = JSON.stringify({
			target,
			mode,
			blockId: mode === 'thread' ? commentBlockId : null,
			threadId: mode === 'reply' ? selectedThreadId : null,
			body
		});
		const attempt = pendingDocumentMutation(commentAttempt, semanticFingerprint, () => {
			const contentJson = commentContent(body, knownActorIds);
			return mode === 'thread'
				? { mode: 'thread' as const, target, blockId: commentBlockId, contentJson }
				: {
						mode: 'reply' as const,
						target: { ...target, threadId: selectedThreadId },
						contentJson
					};
		});
		commentAttempt = attempt;
		commentBusy = true;
		commentFailure = '';
		try {
			if (attempt.input.mode === 'thread') {
				const created = await createDocumentCommentThread(
					attempt.input.target.companyId,
					attempt.input.target.documentId,
					attempt.input.blockId,
					attempt.input.contentJson,
					attempt.command.id
				);
				threads.acceptCreated(attempt.input.target, created);
				if (isCurrentTarget(attempt.input.target)) {
					selectedThreadId = created.thread.thread.id;
					composingThread = false;
					oncommenttargetchange(null);
				}
			} else if (attempt.input.target.threadId) {
				const comment = await replyToDocumentComment(
					attempt.input.target.companyId,
					attempt.input.target.documentId,
					attempt.input.target.threadId,
					null,
					attempt.input.contentJson,
					attempt.command.id
				);
				threadMessages.accept(attempt.input.target, comment);
			}
			await client.invalidateQueries({
				queryKey: documentQueryKeys.commentThreads(target.companyId, target.documentId)
			});
			if (!isCurrentTarget(target)) return;
			commentAttempt = null;
			commentText = '';
		} catch (cause) {
			failClosedDocumentRead(client, cause, target.companyId, target.documentId);
			if (!isCurrentTarget(target)) return;
			commentFailure = cause instanceof Error ? cause.message : 'The comment was not saved.';
			if (!isRetryableDocumentFailure(cause)) commentAttempt = null;
		} finally {
			if (isCurrentTarget(target)) commentBusy = false;
		}
	}

	async function resolveThread(): Promise<void> {
		if (!selectedThread || !online || !canComment || commentBusy) return;
		const target = { companyId, documentId, threadId: selectedThread.thread.id };
		const input = {
			target,
			expectedThreadVersion: selectedThread.thread.version
		};
		const attempt = pendingDocumentMutation(resolveAttempt, JSON.stringify(input), () => input);
		resolveAttempt = attempt;
		commentBusy = true;
		commentFailure = '';
		try {
			const result = await resolveDocumentCommentThread(
				attempt.input.target.companyId,
				attempt.input.target.documentId,
				attempt.input.target.threadId,
				attempt.input.expectedThreadVersion,
				attempt.command.id
			);
			threads.acceptResolved(attempt.input.target, result);
			if (isCurrentTarget(target)) resolveAttempt = null;
		} catch (cause) {
			failClosedDocumentRead(client, cause, target.companyId, target.documentId);
			if (!isCurrentTarget(target)) return;
			commentFailure = cause instanceof Error ? cause.message : 'The thread was not resolved.';
			if (!isRetryableDocumentFailure(cause)) resolveAttempt = null;
		} finally {
			if (isCurrentTarget(target)) commentBusy = false;
		}
	}

	async function refreshDocument(target: DocumentTarget): Promise<void> {
		const latest = await getDocument(target.companyId, target.documentId);
		if (isCurrentTarget(target)) onaccept(target, latest);
		await invalidateDocument(client, target.companyId, target.documentId);
	}

	async function submitReview(event: SubmitEvent): Promise<void> {
		event.preventDefault();
		const summary = reviewSummary.trim();
		if (!summary || !online || !canResolveDocument || reviewBusy) return;
		const target = { companyId, documentId };
		const input = {
			target,
			expectedDocumentVersion: view.document.version,
			expectedCurrentVersionId: view.current_version.version.id,
			summary
		};
		const attempt = pendingDocumentMutation(
			reviewRequestAttempt,
			JSON.stringify(input),
			() => input
		);
		reviewRequestAttempt = attempt;
		reviewBusy = true;
		reviewFailure = '';
		reviewNotice = '';
		let mutationSucceeded = false;
		try {
			const review = await requestDocumentReview(
				attempt.input.target.companyId,
				attempt.input.target.documentId,
				attempt.input.expectedDocumentVersion,
				attempt.input.expectedCurrentVersionId,
				attempt.input.summary,
				attempt.command.id
			);
			mutationSucceeded = true;
			reviews.accept(attempt.input.target, review);
			if (isCurrentTarget(target)) {
				reviewRequestAttempt = null;
				reviewSummary = '';
			}
			await refreshDocument(attempt.input.target);
			if (!isCurrentTarget(target)) return;
		} catch (cause) {
			failClosedDocumentRead(client, cause, target.companyId, target.documentId);
			if (!isCurrentTarget(target)) return;
			if (mutationSucceeded) {
				reviewNotice =
					'Review requested, but this view could not fully refresh. Reload to see the latest state.';
			} else {
				reviewFailure = cause instanceof Error ? cause.message : 'Review could not be requested.';
				if (!isRetryableDocumentFailure(cause)) reviewRequestAttempt = null;
			}
		} finally {
			if (isCurrentTarget(target)) reviewBusy = false;
		}
	}

	async function acceptReview(): Promise<void> {
		if (!requestedReview || !reviewName.trim() || !online || !canResolveDocument || reviewBusy)
			return;
		const target = { companyId, documentId };
		const input = {
			target,
			reviewId: requestedReview.id,
			expectedDocumentVersion: view.document.version,
			expectedReviewVersion: requestedReview.version,
			acceptedVersionName: reviewName.trim()
		};
		const attempt = pendingDocumentMutation(
			reviewAcceptAttempt,
			JSON.stringify(input),
			() => input
		);
		reviewAcceptAttempt = attempt;
		reviewBusy = true;
		reviewFailure = '';
		reviewNotice = '';
		let mutationSucceeded = false;
		try {
			const result = await acceptDocumentReview(
				attempt.input.target.companyId,
				attempt.input.target.documentId,
				attempt.input.reviewId,
				attempt.input.expectedDocumentVersion,
				attempt.input.expectedReviewVersion,
				attempt.input.acceptedVersionName,
				attempt.command.id
			);
			mutationSucceeded = true;
			reviews.acceptResolution(attempt.input.target, result);
			if (isCurrentTarget(target)) reviewAcceptAttempt = null;
			await refreshDocument(attempt.input.target);
		} catch (cause) {
			failClosedDocumentRead(client, cause, target.companyId, target.documentId);
			if (!isCurrentTarget(target)) return;
			if (mutationSucceeded) {
				reviewNotice =
					'Review accepted, but this view could not fully refresh. Reload to see the latest state.';
			} else {
				reviewFailure = cause instanceof Error ? cause.message : 'The review was not accepted.';
				if (!isRetryableDocumentFailure(cause)) reviewAcceptAttempt = null;
			}
		} finally {
			if (isCurrentTarget(target)) reviewBusy = false;
		}
	}

	async function resolveProposal(decision: 'accept' | 'reject'): Promise<void> {
		const proposal = proposalDetail.proposal;
		if (!proposal || !resolutionSummary.trim() || !online || !canResolveDocument || proposalBusy)
			return;
		if (decision === 'accept' && !proposalVersionName.trim()) return;
		const target = { companyId, documentId };
		const input = {
			target,
			proposalId: proposal.id,
			expectedProposalVersion: proposal.version,
			decision,
			resolutionSummary: resolutionSummary.trim(),
			acceptedVersionName: proposalVersionName.trim()
		};
		const attempt = pendingDocumentMutation(proposalAttempt, JSON.stringify(input), () => input);
		proposalAttempt = attempt;
		proposalBusy = true;
		proposalFailure = '';
		proposalNotice = '';
		let mutationSucceeded = false;
		try {
			const result =
				attempt.input.decision === 'accept'
					? await acceptDocumentRevisionProposal(
							attempt.input.target.companyId,
							attempt.input.target.documentId,
							attempt.input.proposalId,
							attempt.input.expectedProposalVersion,
							attempt.input.resolutionSummary,
							attempt.input.acceptedVersionName,
							attempt.command.id
						)
					: await rejectDocumentRevisionProposal(
							attempt.input.target.companyId,
							attempt.input.target.documentId,
							attempt.input.proposalId,
							attempt.input.expectedProposalVersion,
							attempt.input.resolutionSummary,
							attempt.command.id
						);
			mutationSucceeded = true;
			proposals.acceptResolution(attempt.input.target, result);
			if (isCurrentTarget(target)) {
				proposalAttempt = null;
				resolutionSummary = '';
			}
			if (attempt.input.decision === 'accept') await refreshDocument(attempt.input.target);
			else {
				await client.invalidateQueries({
					queryKey: documentQueryKeys.proposals(
						attempt.input.target.companyId,
						attempt.input.target.documentId
					)
				});
			}
			if (!isCurrentTarget(target)) return;
		} catch (cause) {
			failClosedDocumentRead(client, cause, target.companyId, target.documentId);
			if (!isCurrentTarget(target)) return;
			if (mutationSucceeded) {
				proposalNotice = `Proposal ${attempt.input.decision === 'accept' ? 'accepted' : 'rejected'}, but this view could not fully refresh. Reload to see the latest state.`;
			} else {
				proposalFailure = cause instanceof Error ? cause.message : 'The proposal was not resolved.';
				if (!isRetryableDocumentFailure(cause)) proposalAttempt = null;
			}
		} finally {
			if (isCurrentTarget(target)) proposalBusy = false;
		}
	}

	async function restore(versionId: string): Promise<void> {
		if (!online || !canResolveDocument || restoringVersionId) return;
		if (versionDetail.version?.version.id !== versionId) {
			selectedVersionId = versionId;
			versionFailure = 'Load this version preview before restoring it.';
			versionNotice = '';
			return;
		}
		const target = { companyId, documentId };
		restoringVersionId = versionId;
		versionFailure = '';
		versionNotice = '';
		const restoreInput = {
			target,
			version_id: versionId,
			expected_current_version_id: view.current_version.version.id,
			reason: 'Restored from history'
		};
		const command = pendingDocumentCommand(restoreCommand, JSON.stringify(restoreInput));
		restoreCommand = command;
		let mutationSucceeded = false;
		try {
			const receipt = await restoreDocumentVersion(
				restoreInput.target.companyId,
				restoreInput.target.documentId,
				restoreInput.version_id,
				restoreInput.expected_current_version_id,
				restoreInput.reason,
				command.id
			);
			mutationSucceeded = true;
			const restored = await getDocument(target.companyId, receipt.document_id);
			if (restored.current_version.version.id !== receipt.result_id) {
				if (isCurrentTarget(target)) {
					versionFailure =
						'Another version completed before this restore was read. Your current view was kept.';
					restoreCommand = null;
				}
				return;
			}
			if (isCurrentTarget(target)) {
				restoreCommand = null;
				onaccept(target, restored);
			}
			await invalidateDocument(client, target.companyId, target.documentId);
		} catch (cause) {
			failClosedDocumentRead(client, cause, target.companyId, target.documentId);
			if (!isCurrentTarget(target)) return;
			if (mutationSucceeded) {
				versionNotice =
					'Version restored, but this view could not fully refresh. Reload to see the latest state.';
			} else {
				versionFailure = cause instanceof Error ? cause.message : 'The version was not restored.';
				if (!isRetryableDocumentFailure(cause)) restoreCommand = null;
			}
		} finally {
			if (isCurrentTarget(target)) restoringVersionId = '';
		}
	}

	function proposalLabel(proposal: DocumentRevisionProposalSummary): string {
		return proposal.scope === 'block' && proposal.block_id
			? `Block ${proposal.block_id}`
			: 'Whole document';
	}
</script>

<aside class="document-inspector" aria-label="Document review and discussion">
	{#if ondismiss}
		<div class="inspector-dismiss">
			<button
				type="button"
				class="flat-action"
				onclick={ondismiss}
				title="Close this tray without accepting or rejecting a review"
			>
				<X size={14} aria-hidden="true" /> Dismiss
			</button>
		</div>
	{/if}
	<nav class="inspector-tabs" aria-label="Document details">
		<button
			type="button"
			class:active={panel === 'comments'}
			aria-pressed={panel === 'comments'}
			onclick={() => onpanelchange('comments')}
		>
			<MessageSquare size={14} strokeWidth={1.8} aria-hidden="true" />
			Comments
			{#if threads.threads.filter((item) => item.thread.status === 'open').length}
				<b>{threads.threads.filter((item) => item.thread.status === 'open').length}</b>
			{/if}
		</button>
		{#if requestedReview || openProposals.length || panel === 'review'}
			<button
				type="button"
				class:active={panel === 'review'}
				aria-pressed={panel === 'review'}
				onclick={() => onpanelchange('review')}
			>
				<Scale size={14} strokeWidth={1.8} aria-hidden="true" /> Review
				{#if openProposals.length}<b>{openProposals.length}</b>{/if}
			</button>
		{/if}
		<button
			type="button"
			class:active={panel === 'versions'}
			aria-pressed={panel === 'versions'}
			onclick={() => onpanelchange('versions')}
		>
			<History size={14} strokeWidth={1.8} aria-hidden="true" /> History
		</button>
	</nav>

	{#if panel === 'comments'}
		<div class="inspector-body comments-body">
			<header class="inspector-head">
				<div>
					<h2>Discussion</h2>
					<p>Feedback stays attached to this document.</p>
				</div>
				{#if canComment}
					<button type="button" class="flat-action" onclick={beginDocumentComment}
						>New comment</button
					>
				{/if}
			</header>

			{#if threads.failure}<p class="inline-error" role="alert">{threads.failure.message}</p>{/if}

			{#if composingThread}
				<div class="comment-target">
					<span>{commentBlockId ? 'Commenting on a block' : 'Commenting on the document'}</span>
					<button
						type="button"
						aria-label="Cancel new comment"
						onclick={() => {
							composingThread = false;
							commentText = '';
							commentAttempt = null;
							oncommenttargetchange(null);
						}}><X size={14} strokeWidth={2} aria-hidden="true" /></button
					>
				</div>
			{/if}

			{#if !composingThread}
				<div class="thread-index" aria-label="Comment threads">
					{#each threads.threads as item (item.thread.id)}
						<button
							type="button"
							class:selected={selectedThreadId === item.thread.id}
							onclick={() => {
								selectedThreadId = item.thread.id;
								commentAttempt = null;
								resolveAttempt = null;
								commentFailure = '';
							}}
						>
							<span class="thread-state state-{item.thread.status}"></span>
							<span>
								<strong>{item.thread.block_id ? 'Block comment' : 'Document comment'}</strong>
								<small
									>{actorName(item.thread.created_by_actor_id)} · {shortDate(
										item.thread.created_at
									)}</small
								>
							</span>
							{#if item.anchor_state === 'orphaned'}<em>Block moved</em>{/if}
							<ChevronRight size={14} strokeWidth={1.8} aria-hidden="true" />
						</button>
					{:else}
						{#if threads.status === 'unknown'}
							<p class="quiet-state">Loading comments…</p>
						{:else}
							<div class="quiet-state">
								<MessagesSquare size={18} strokeWidth={1.5} /><strong>No comments yet.</strong><span
									>Start a thread on the document or a specific block.</span
								>
							</div>
						{/if}
					{/each}
				</div>
			{/if}

			{#if !composingThread && selectedThread}
				<div class="comment-thread">
					<header>
						<div>
							<strong
								>{selectedThread.thread.block_id
									? `Block ${selectedThread.thread.block_id}`
									: 'Whole document'}</strong
							>
							<span>{statusLabel(selectedThread.thread.status)}</span>
						</div>
						{#if selectedThread.thread.status === 'open' && canComment}
							<button
								type="button"
								class="flat-action"
								disabled={!online || commentBusy}
								onclick={() => void resolveThread()}
								><Check size={14} strokeWidth={2} /> Resolve</button
							>
						{/if}
					</header>
					{#if threadMessages?.hasMore}
						<button
							type="button"
							class="load-more"
							disabled={threadMessages.loadingMore}
							onclick={() => void threadMessages.loadMore()}
							>{threadMessages.loadingMore ? 'Loading…' : 'Earlier replies'}</button
						>
					{/if}
					{#each threadMessages?.comments ?? [] as comment (comment.id)}
						<article class="comment-row">
							<header>
								<strong>{actorName(comment.author_actor_id)}</strong><time
									datetime={comment.created_at}>{shortDate(comment.created_at)}</time
								>
							</header>
							<p>{comment.plain_text}</p>
							{#if comment.mentioned_actor_ids.length}
								<div class="mention-list">
									{#each comment.mentioned_actor_ids as actorId (actorId)}<span
											>@{actorName(actorId)}</span
										>{/each}
								</div>
							{/if}
						</article>
					{:else}
						{#if threadMessages?.status === 'unknown'}
							<p class="quiet-state">Loading the thread…</p>
						{/if}
					{/each}
				</div>
			{/if}

			{#if canComment && (composingThread || (selectedThread && selectedThread.thread.status === 'open'))}
				<form class="comment-composer" onsubmit={(event) => void submitComment(event)}>
					<label for="document-comment">{composingThread ? 'Comment' : 'Reply'}</label>
					<textarea
						id="document-comment"
						bind:value={commentText}
						rows="3"
						placeholder="Write a clear question or observation. Use @actor-id to mention someone."
						disabled={!online || commentBusy}
						oninput={() => (commentAttempt = null)}></textarea>
					{#if commentFailure}<p class="inline-error" role="alert">{commentFailure}</p>{/if}
					<button
						type="submit"
						class="btn small primary"
						disabled={!online || !commentText.trim() || commentBusy}
						>{commentBusy ? 'Saving…' : composingThread ? 'Start thread' : 'Reply'}</button
					>
				</form>
			{/if}
		</div>
	{:else if panel === 'review'}
		<div class="inspector-body review-body">
			<header class="inspector-head">
				<div>
					<h2>Review</h2>
					<p>Review the saved document and proposed changes.</p>
				</div>
			</header>
			{#if editing && canJudge}<p class="pending-draft">Waiting for edits to finish saving.</p>{/if}
			{#if requestedReview}
				<section class="review-request current">
					<header>
						<span class="review-pulse"></span><strong>Review requested</strong><time
							datetime={requestedReview.created_at}>{shortDate(requestedReview.created_at)}</time
						>
					</header>
					<p>{requestedReview.summary}</p>
					<dl>
						<div>
							<dt>Requested by</dt>
							<dd>{actorName(requestedReview.requested_by_actor_id)}</dd>
						</div>
						<div>
							<dt>Version</dt>
							<dd>
								{requestedReview.requested_version_id === view.current_version.version.id
									? `v${view.current_version.version.version_number}`
									: 'Earlier version'}
							</dd>
						</div>
					</dl>
					{#if canJudge}
						<label class="field"
							><span>Accepted version name</span><input
								bind:value={reviewName}
								disabled={!online || editing || reviewBusy}
								oninput={() => (reviewAcceptAttempt = null)}
							/></label
						>
						<button
							type="button"
							class="btn primary"
							disabled={!online || editing || reviewBusy || !reviewName.trim()}
							onclick={() => void acceptReview()}
							>{reviewBusy ? 'Accepting…' : 'Accept this version'}</button
						>
					{/if}
				</section>
			{:else if canJudge}
				<form class="review-request" onsubmit={(event) => void submitReview(event)}>
					<label class="field"
						><span>What should the reviewer judge?</span><textarea
							bind:value={reviewSummary}
							rows="4"
							placeholder="Name the decision, evidence, and any unresolved question."
							disabled={!online || editing || reviewBusy}
							oninput={() => (reviewRequestAttempt = null)}></textarea></label
					>
					<button
						type="submit"
						class="btn primary"
						disabled={!online || editing || reviewBusy || !reviewSummary.trim()}
						>{reviewBusy ? 'Requesting…' : 'Request review'}</button
					>
				</form>
			{:else}
				<p class="quiet-state">No active review.</p>
			{/if}
			{#if reviewFailure}<p class="inline-error" role="alert">{reviewFailure}</p>{/if}
			{#if reviewNotice}<p class="inline-notice" role="status">{reviewNotice}</p>{/if}

			<section class="proposal-section">
				<header>
					<div>
						<Sparkles size={15} strokeWidth={1.7} aria-hidden="true" />
						<h3>Agent proposals</h3>
					</div>
					<span>{openProposals.length} open</span>
				</header>
				{#if openProposals.length}
					<div class="proposal-index" aria-label="Agent proposals">
						{#each openProposals as proposal (proposal.id)}
							<button
								type="button"
								class:selected={selectedProposal?.id === proposal.id}
								onclick={() => {
									selectedProposalId = proposal.id;
									proposalAttempt = null;
									proposalFailure = '';
									proposalNotice = '';
								}}
								><span
									><strong>{proposal.summary}</strong><small
										>{actorName(proposal.proposed_by_actor_id)} · {proposalLabel(proposal)}</small
									></span
								><ChevronRight size={14} strokeWidth={1.8} /></button
							>
						{/each}
					</div>
					{#if proposalDetail?.proposal}
						<div class="proposal-compare">
							<div>
								<strong>Current</strong>
								<pre>{view.current_version.version.plain_text}</pre>
							</div>
							<div>
								<strong>Proposed</strong>
								<pre>{proposalDetail.proposal.proposed_plain_text}</pre>
							</div>
						</div>
						{#if canJudge}
							<label class="field"
								><span>Decision note</span><textarea
									bind:value={resolutionSummary}
									rows="3"
									placeholder="Record why this proposal should land or return."
									disabled={!online || editing || proposalBusy}
									oninput={() => (proposalAttempt = null)}></textarea></label
							>
							<label class="field"
								><span>Accepted version name</span><input
									bind:value={proposalVersionName}
									disabled={!online || editing || proposalBusy}
									oninput={() => (proposalAttempt = null)}
								/></label
							>
							<div class="proposal-actions">
								<button
									type="button"
									class="btn"
									disabled={!online || editing || proposalBusy || !resolutionSummary.trim()}
									onclick={() => void resolveProposal('reject')}>Reject</button
								><button
									type="button"
									class="btn primary"
									disabled={!online ||
										editing ||
										proposalBusy ||
										!resolutionSummary.trim() ||
										!proposalVersionName.trim()}
									onclick={() => void resolveProposal('accept')}>Accept proposal</button
								>
							</div>
						{/if}
					{/if}
				{:else if proposals.status === 'unknown'}
					<p class="quiet-state">Loading proposals…</p>
				{:else}
					<p class="quiet-state">No agent revisions are waiting.</p>
				{/if}
				{#if proposalFailure}<p class="inline-error" role="alert">{proposalFailure}</p>{/if}
				{#if proposalNotice}<p class="inline-notice" role="status">{proposalNotice}</p>{/if}
			</section>
		</div>
	{:else}
		<div class="inspector-body versions-body">
			<header class="inspector-head">
				<div>
					<h2>History</h2>
					<p>Versions are saved automatically. Restoring keeps the current draft in history.</p>
				</div>
			</header>
			{#if editing && canJudge}<p class="pending-draft">Waiting for edits to finish saving.</p>{/if}
			{#if versionFailure}<p class="inline-error" role="alert">{versionFailure}</p>{/if}
			{#if versionNotice}<p class="inline-notice" role="status">{versionNotice}</p>{/if}
			<div class="version-line" aria-label="Document version history">
				{#each versions.versions as item (item.id)}
					<article
						class:current={item.id === view.current_version.version.id}
						class:selected={item.id === selectedVersionId}
					>
						<span class="version-node"></span>
						<header>
							<strong>Version {item.version_number}</strong><span
								>{statusLabel(item.document_status)}</span
							>
						</header>
						<p>{item.reason}</p>
						<footer>
							<span>{item.created_by_actor_id === 'daemon' ? 'Restless' : actorName(item.created_by_actor_id)}</span><time datetime={item.created_at}
								>{shortDate(item.created_at)}</time
							>
						</footer>
						<button
							type="button"
							class="flat-action"
							aria-pressed={selectedVersionId === item.id}
							onclick={() => {
								selectedVersionId = item.id;
								restoreCommand = null;
								versionFailure = '';
								versionNotice = '';
							}}>Preview</button
						>
					</article>
				{:else}
					{#if versions.status === 'unknown'}<p class="quiet-state">Loading versions…</p>{:else}<p
							class="quiet-state"
						>
							No version history is available.
						</p>{/if}
				{/each}
			</div>
			{#if selectedVersionId}
				<section class="version-preview" aria-live="polite">
					{#if versionDetail.version}
						<header>
							<strong>Version {versionDetail.version.version.version_number}</strong>
							{#if versionDetail.version.version.id !== view.current_version.version.id && canJudge}
								<button
									type="button"
									class="flat-action"
									disabled={!online || editing || Boolean(restoringVersionId)}
									onclick={() => void restore(versionDetail.version?.version.id ?? '')}
									><RotateCcw size={13} strokeWidth={1.8} />
									{restoringVersionId ? 'Restoring…' : 'Restore as new version'}</button
								>
							{/if}
						</header>
						<pre>{versionDetail.version.version.plain_text || 'This version is empty.'}</pre>
					{:else if versionDetail.failure}
						<p class="inline-error" role="alert">{versionDetail.failure.message}</p>
					{:else}
						<p class="quiet-state">Loading version preview…</p>
					{/if}
				</section>
			{/if}
			{#if versions.hasMore}<button
					type="button"
					class="load-more"
					disabled={versions.loadingMore}
					onclick={() => void versions.loadMore()}
					>{versions.loadingMore ? 'Loading…' : 'Earlier versions'}</button
				>{/if}
		</div>
	{/if}
</aside>

<style>
	.inspector-dismiss {
		display: flex;
		justify-content: flex-end;
		padding: 5px 8px;
	}
	.document-inspector {
		min-width: 0;
		min-height: 0;
		display: flex;
		flex-direction: column;
		background: var(--surface-rail);
		overflow: hidden;
	}
	.inspector-tabs {
		min-height: 40px;
		display: grid;
		grid-auto-flow: column;
		grid-auto-columns: minmax(0, 1fr);
		align-items: end;
		padding: 0 6px;
		border-bottom: 1px solid var(--border);
	}
	.inspector-tabs button {
		min-width: 0;
		height: 38px;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 5px;
		padding: 0 7px;
		border: 0;
		border-bottom: 2px solid transparent;
		background: transparent;
		color: var(--text-tertiary);
		font: 600 var(--t-body) var(--font-ui);
		cursor: pointer;
	}
	.inspector-tabs button:hover {
		color: var(--ink);
		background: var(--accent-soft);
	}
	.inspector-tabs button.active {
		border-bottom-color: var(--surface-attention);
		color: var(--surface-attention);
	}
	.inspector-tabs b {
		min-width: 17px;
		padding: 1px 4px;
		border-radius: 999px;
		background: var(--surface-attention-soft);
		font: 600 var(--t-label) var(--font-mono);
	}
	.inspector-body {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
	}
	.inspector-head {
		display: flex;
		align-items: start;
		justify-content: space-between;
		gap: var(--space-3);
		padding: var(--space-4);
		border-bottom: 1px solid var(--border);
	}
	.inspector-head h2,
	.inspector-head p {
		margin: 0;
	}
	.inspector-head p {
		margin-top: 3px;
		color: var(--text-tertiary);
	}
	.flat-action {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		padding: 5px 8px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface);
		color: var(--text-secondary);
		font: 600 var(--t-body) var(--font-ui);
		cursor: pointer;
		white-space: nowrap;
	}
	.flat-action:hover:not(:disabled) {
		border-color: var(--border-strong);
		color: var(--ink);
	}
	.flat-action:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.thread-index,
	.proposal-index {
		border-bottom: 1px solid var(--border);
	}
	.thread-index > button,
	.proposal-index > button {
		width: 100%;
		min-height: 54px;
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto auto;
		align-items: center;
		gap: 9px;
		padding: 9px 12px;
		border: 0;
		border-bottom: 1px solid var(--border);
		background: transparent;
		color: inherit;
		text-align: left;
		cursor: pointer;
	}
	.proposal-index > button {
		grid-template-columns: minmax(0, 1fr) auto;
	}
	.thread-index > button:hover,
	.thread-index > button.selected,
	.proposal-index > button:hover,
	.proposal-index > button.selected {
		background: var(--accent-soft);
	}
	.thread-index strong,
	.thread-index small,
	.proposal-index strong,
	.proposal-index small {
		display: block;
	}
	.thread-index small,
	.proposal-index small {
		margin-top: 2px;
		color: var(--text-tertiary);
	}
	.thread-index em {
		color: var(--intent-authority);
		font: 500 var(--t-label) var(--font-mono);
		font-style: normal;
	}
	.thread-state {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--intent-conversation);
	}
	.thread-state.state-resolved {
		background: var(--status-offline);
	}
	.comment-target {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		padding: 9px 12px;
		border-bottom: 1px solid color-mix(in srgb, var(--intent-conversation) 18%, var(--border));
		background: var(--intent-conversation-soft);
		color: var(--intent-conversation);
		font-weight: 600;
	}
	.comment-target button {
		width: 25px;
		height: 25px;
		display: grid;
		place-items: center;
		padding: 0;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: inherit;
		cursor: pointer;
	}
	.comment-thread > header {
		display: flex;
		justify-content: space-between;
		align-items: start;
		gap: var(--space-2);
		padding: 10px 12px;
		border-bottom: 1px solid var(--border);
		background: var(--surface-alt);
	}
	.comment-thread > header div {
		display: flex;
		flex-direction: column;
	}
	.comment-thread > header span {
		color: var(--text-tertiary);
	}
	.comment-row {
		padding: 11px 13px;
		border-bottom: 1px solid var(--border);
	}
	.comment-row header {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: var(--space-2);
	}
	.comment-row time {
		color: var(--text-tertiary);
		font: var(--t-label) var(--font-mono);
	}
	.comment-row p {
		margin: 5px 0 0;
		white-space: pre-wrap;
		color: var(--text-secondary);
		line-height: 1.55;
	}
	.mention-list {
		display: flex;
		flex-wrap: wrap;
		gap: 4px;
		margin-top: 7px;
	}
	.mention-list span {
		padding: 2px 5px;
		border-radius: var(--radius-control);
		background: var(--intent-conversation-soft);
		color: var(--intent-conversation);
	}
	.comment-composer,
	.review-request {
		display: grid;
		gap: var(--space-2);
		padding: var(--space-3);
		border-top: 1px solid var(--border);
	}
	.comment-composer label,
	.field span {
		font-weight: 600;
		color: var(--text-secondary);
	}
	.comment-composer textarea,
	.field textarea,
	.field input {
		width: 100%;
		padding: 8px 9px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: #fff;
		color: var(--ink);
		font: var(--t-body) / 1.5 var(--font-ui);
		resize: vertical;
	}
	.comment-composer .btn,
	.review-request .btn {
		justify-self: end;
	}
	.inline-error {
		margin: 0;
		padding: 8px 12px;
		background: var(--state-danger-soft);
		color: var(--state-danger);
	}
	.inline-notice {
		margin: 0;
		padding: 8px 12px;
		background: color-mix(in srgb, var(--surface-work) 6%, var(--surface));
		color: var(--text-secondary);
	}
	.pending-draft {
		margin: 0;
		padding: 8px 12px;
		border-bottom: 1px solid color-mix(in srgb, var(--surface-work) 20%, var(--border));
		background: color-mix(in srgb, var(--surface-work) 6%, var(--surface));
		color: var(--text-secondary);
	}
	.quiet-state {
		display: grid;
		justify-items: start;
		gap: 5px;
		margin: 0;
		padding: var(--space-4);
		color: var(--text-tertiary);
	}
	.review-request.current {
		border-top: 0;
		border-bottom: 1px solid var(--border);
		background: color-mix(in srgb, var(--surface-attention-soft) 42%, var(--surface));
	}
	.review-request.current > header {
		display: flex;
		align-items: center;
		gap: 7px;
	}
	.review-request.current > header time {
		margin-left: auto;
		color: var(--text-tertiary);
		font: var(--t-label) var(--font-mono);
	}
	.review-request.current > p {
		margin: 4px 0;
		color: var(--text-secondary);
	}
	.review-pulse {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--surface-attention);
		box-shadow: 0 0 0 4px var(--surface-attention-soft);
	}
	dl {
		margin: 0;
	}
	dl div {
		display: flex;
		justify-content: space-between;
		gap: var(--space-3);
		padding: 5px 0;
		border-top: 1px solid var(--border);
	}
	dt {
		color: var(--text-tertiary);
	}
	dd {
		margin: 0;
		font-weight: 600;
	}
	.field {
		display: grid;
		gap: 5px;
	}
	.proposal-section {
		padding-bottom: var(--space-4);
	}
	.proposal-section > header {
		min-height: 46px;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		padding: 0 12px;
		border-bottom: 1px solid var(--border);
	}
	.proposal-section > header div {
		display: flex;
		align-items: center;
		gap: 7px;
		color: var(--surface-attention);
	}
	.proposal-section > header h3 {
		margin: 0;
	}
	.proposal-section > header > span {
		color: var(--text-tertiary);
	}
	.proposal-compare {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1px;
		margin: var(--space-3);
		border: 1px solid var(--border);
		background: var(--border);
	}
	.proposal-compare > div {
		min-width: 0;
		padding: 9px;
		background: var(--surface);
	}
	.proposal-compare pre {
		max-height: 190px;
		margin: 6px 0 0;
		overflow: auto;
		white-space: pre-wrap;
		color: var(--text-secondary);
		font: var(--t-body) / 1.45 var(--font-ui);
	}
	.proposal-section > .field {
		margin: 0 var(--space-3) var(--space-2);
	}
	.proposal-actions {
		display: flex;
		justify-content: flex-end;
		gap: var(--space-2);
		margin: var(--space-3);
	}
	.version-line {
		position: relative;
		padding: var(--space-3) var(--space-3) var(--space-4) 30px;
	}
	.version-line::before {
		content: '';
		position: absolute;
		top: var(--space-4);
		bottom: var(--space-4);
		left: 16px;
		width: 1px;
		background: var(--border-strong);
	}
	.version-line article {
		position: relative;
		padding: 8px 0 14px;
	}
	.version-line article.selected {
		color: var(--ink);
	}
	.version-node {
		position: absolute;
		left: -18px;
		top: 13px;
		width: 8px;
		height: 8px;
		border: 2px solid var(--surface-rail);
		border-radius: 50%;
		background: var(--status-offline);
		box-shadow: 0 0 0 1px var(--border-strong);
	}
	.version-line article.current .version-node {
		background: var(--surface-work);
		box-shadow: 0 0 0 3px var(--intent-feedback-soft);
	}
	.version-line article > header,
	.version-line article > footer {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-2);
	}
	.version-line article > header span {
		color: var(--text-tertiary);
	}
	.version-line article p {
		margin: 4px 0;
		color: var(--text-secondary);
	}
	.version-line article footer {
		color: var(--text-tertiary);
	}
	.version-line .flat-action {
		margin-top: 7px;
	}
	.version-preview {
		margin: 0 var(--space-3) var(--space-3);
		border: 1px solid var(--border);
		background: var(--surface);
	}
	.version-preview > header {
		min-height: 42px;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-2);
		padding: 6px 9px;
		border-bottom: 1px solid var(--border);
	}
	.version-preview pre {
		max-height: 260px;
		margin: 0;
		padding: 10px;
		overflow: auto;
		white-space: pre-wrap;
		color: var(--text-secondary);
		font: var(--t-body) / 1.55 var(--font-ui);
	}
	.load-more {
		width: calc(100% - 24px);
		margin: 8px 12px;
		padding: 7px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface);
		color: var(--text-secondary);
		font: inherit;
		cursor: pointer;
	}
	@media (max-width: 760px) {
		.proposal-compare {
			grid-template-columns: 1fr;
		}
		.inspector-tabs {
			position: sticky;
			top: 0;
			z-index: 2;
			background: var(--surface-rail);
		}
	}
</style>
