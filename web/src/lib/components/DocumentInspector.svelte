<script lang="ts" module>
	/** Scroll the live document to one block and mark it for a moment. False when it is not on
	 * screen (moved, deleted, or the document is not showing). */
	export function revealDocumentBlock(blockId: string): boolean {
		const element = window.document.querySelector<HTMLElement>(
			`.native-document-body [data-block-id="${CSS.escape(blockId)}"]`
		);
		if (!element || !element.offsetParent) return false;
		element.scrollIntoView({ block: 'center', behavior: 'smooth' });
		/* The mark sits over the block, outside the editor's own DOM, which it must not touch. */
		const page = element.closest<HTMLElement>('.document-print');
		if (page) {
			const box = element.getBoundingClientRect();
			const frame = page.getBoundingClientRect();
			const mark = window.document.createElement('div');
			mark.className = 'block-flash';
			mark.setAttribute('aria-hidden', 'true');
			mark.style.top = `${box.top - frame.top + page.scrollTop - 4}px`;
			mark.style.left = `${box.left - frame.left + page.scrollLeft - 8}px`;
			mark.style.width = `${box.width + 16}px`;
			mark.style.height = `${box.height + 8}px`;
			page.append(mark);
			setTimeout(() => mark.remove(), 1_800);
		}
		return true;
	}
</script>

<script lang="ts">
	import { composing } from '$lib/primitives/composer-keys';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import MessageReactions from '$lib/primitives/MessageReactions.svelte';
	import ReactionAdd from '$lib/primitives/ReactionAdd.svelte';
	import { documentCommentReactionsQuery } from '$lib/model/document-comment-reactions.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import Check from '@lucide/svelte/icons/check';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import History from '@lucide/svelte/icons/history';
	import MessageSquare from '@lucide/svelte/icons/message-square';
	import MessagesSquare from '@lucide/svelte/icons/messages-square';
	import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
	import Scale from '@lucide/svelte/icons/scale';
	import Sparkles from '@lucide/svelte/icons/sparkles';
	import X from '@lucide/svelte/icons/x';
	import { tick, untrack } from 'svelte';
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
		deleteDocumentComment,
		documentBlockText,
		documentFileName,
		documentMarkdownDownload,
		editDocumentComment,
		getDocument,
		reopenDocumentCommentThread,
		isRetryableDocumentFailure,
		pendingDocumentCommand,
		pendingDocumentMutation,
		quoteExcerpt,
		rejectDocumentRevisionProposal,
		replyToDocumentComment,
		requestDocumentReview,
		requestDocumentReviewChanges,
		resolveDocumentCommentThread,
		restoreDocumentVersion,
		saveDocumentVersion,
		type CommentMention,
		type DocumentCommentRow,
		type DocumentNode,
		type DocumentReadView,
		type DocumentContent,
		type DocumentVersionSummary,
		type DocumentRevisionProposalSummary,
		type PendingDocumentCommand,
		type PendingDocumentMutation
	} from '$lib/model/documents';
	import { composerTrigger } from '$lib/primitives/composer-menu';
	import { proposalChanges, versionChanges, type BlockChange } from '$lib/model/document-diff';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import { withDocumentImages } from '$lib/model/document-editor-schema';

	export type DocumentInspectorPanel = 'comments' | 'review' | 'versions';

	interface PersonReference {
		actor_id: string;
		display: string;
		kind?: string;
	}

	interface Props {
		companyId: string;
		view: DocumentReadView;
		panel: DocumentInspectorPanel;
		requestedReviewId?: string;
		/** A thread to open on arrival (from For you or a margin mark), and a comment to point at. */
		requestedThreadId?: string;
		requestedCommentId?: string;
		/** Asking again for the thread already in the address opens it again. */
		threadRequest?: number;
		/** Each new value opens History ready to name a version, from the document's … menu. */
		nameVersionRequest?: number;
		/** History opens on what changed after this moment: when the person last looked. */
		changesSince?: string;
		people?: PersonReference[];
		/** Who is looking: a Work-bound review can be sent back only by the person it asks. */
		viewerActorId?: string;
		online?: boolean;
		editing?: boolean;
		/** The others typing in the document now, whom a restore would interrupt. */
		othersEditing?: string[];
		commentBlockId?: string | null;
		/** The words selected for a new block comment, when a passage was chosen. */
		commentQuote?: string | null;
		onpanelchange?: (panel: DocumentInspectorPanel) => void;
		oncommenttargetchange?: (blockId: string | null) => void;
		onaccept?: (target: DocumentTarget, view: DocumentReadView) => void;
		/** Show a block in the document; without it the inspector scrolls the document itself. */
		onshowblock?: (blockId: string) => void;
		ondismiss?: () => void;
		/** A Work-bound document: a comment can go to that Work as a change request. The answer
		 * says where it went. */
		onchangerequest?: (request: { quote: string | null; text: string }) => Promise<string>;
	}

	let {
		companyId,
		view,
		panel,
		requestedReviewId = '',
		requestedThreadId = '',
		requestedCommentId = '',
		threadRequest = 0,
		nameVersionRequest = 0,
		changesSince = '',
		people = [],
		viewerActorId = '',
		online = true,
		editing = false,
		othersEditing = [],
		commentBlockId = null,
		commentQuote = null,
		onpanelchange = () => {},
		oncommenttargetchange = () => {},
		onaccept = () => {},
		onshowblock,
		ondismiss,
		onchangerequest
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
	/* Reactions on comments, as on chat messages: a signal kept with the comment, never a reply. */
	const commentReactions = documentCommentReactionsQuery(
		() => companyId,
		() => documentId,
		() => viewerActorId
	);
	let reactionFailure = $state('');
	async function reactTo(commentId: string, emoji: string, on: boolean): Promise<void> {
		if (!canComment || !online) return;
		reactionFailure = '';
		try {
			await commentReactions.react(commentId, emoji, on);
		} catch (cause) {
			reactionFailure = failureSentence(cause, 'The reaction was not saved.');
		}
	}
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
	/* Starts at the open document: a reset on mount would drop the comment target the
	 * editor just chose, and the selected words with it. */
	let selectionDocumentId = $state(untrack(() => documentId));
	let composingThread = $state(false);
	let commentText = $state('');
	let commentBusy = $state(false);
	let commentFailure = $state('');
	let reviewSummary = $state('');
	let reviewName = $state(
		`Reviewed ${new Date().toLocaleDateString(undefined, { day: 'numeric', month: 'short' })}`
	);
	/* Whom the requester asks to judge it; empty lets anyone who can edit it judge it. */
	let reviewerId = $state('');
	const reviewerChoices = $derived(
		people.filter(
			(person) =>
				(person.kind === 'human' || person.kind === 'owner') && person.actor_id !== viewerActorId
		)
	);
	let reviewNote = $state('');
	let reviewBusy = $state(false);
	let reviewFailure = $state('');
	let reviewNotice = $state('');
	let reviewChangesAttempt = $state<PendingDocumentMutation<ReviewChangesInput> | null>(null);
	let resolutionSummary = $state('');
	let proposalVersionName = $state('Accepted agent revision');
	let proposalBusy = $state(false);
	let proposalFailure = $state('');
	let proposalNotice = $state('');
	let restoringVersionId = $state('');
	let restoreCommand = $state<PendingDocumentCommand | null>(null);
	let selectedVersionId = $state('');
	/* Naming a version marks a moment the owner will want back, such as "the version I sent". */
	let namingVersion = $state(false);
	let versionName = $state('');
	let savingVersion = $state(false);
	let saveCommand = $state<PendingDocumentCommand | null>(null);
	/* A previewed version reads as the page it was; what it changed is one click away. */
	let previewMode = $state<'page' | 'changes'>('page');
	async function downloadVersion(format: 'docx' | 'md'): Promise<void> {
		const detail = versionDetail.version;
		if (!detail) return;
		const title = `${view.document.title || 'Document'} (version ${detail.version.version_number})`;
		try {
			const blob =
				format === 'md'
					? new Blob([documentMarkdownDownload(title, detail.markdown)], {
							type: 'text/markdown;charset=utf-8'
						})
					: (await import('$lib/model/document-docx')).documentDocx(
							view.document.title,
							detail.version.content_json
						);
			const url = URL.createObjectURL(blob);
			const link = document.createElement('a');
			link.href = url;
			link.download = documentFileName(title, format);
			link.click();
			setTimeout(() => URL.revokeObjectURL(url), 1_000);
		} catch (cause) {
			versionFailure = failureSentence(cause, 'This version could not be downloaded.');
		}
	}
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
				quote: string | null;
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
		reviewer: string | null;
	}
	interface ReviewAcceptInput {
		target: DocumentTarget;
		reviewId: string;
		expectedDocumentVersion: number;
		expectedReviewVersion: number;
		acceptedVersionName: string;
		feedback: string;
	}
	interface ReviewChangesInput {
		target: DocumentTarget;
		reviewId: string;
		expectedDocumentVersion: number;
		expectedReviewVersion: number;
		feedback: string;
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
	/* A proposal reads as what it would change in the document now, word by word. */
	const proposalDiff = $derived(
		proposalDetail.proposal
			? proposalChanges(
					view.current_version.version.content_json,
					proposalDetail.proposal.proposed_content_json,
					proposalDetail.proposal.scope === 'block' ? proposalDetail.proposal.block_id : null
				)
			: null
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
	/* A review a piece of Work asked for can go back to it with what should change. */
	const requestedDependency = $derived(
		requestedReview ? reviews.dependencyOf(requestedReview.id) : null
	);
	const canRequestChanges = $derived(
		requestedDependency?.status === 'pending' &&
			(!viewerActorId || requestedDependency.reviewer_actor_id === viewerActorId)
	);
	const canCommentChanges = $derived(
		Boolean(requestedReview) &&
			!requestedDependency &&
			canComment &&
			requestedReview?.requested_by_actor_id !== viewerActorId
	);
	const mayRequestChanges = $derived(canRequestChanges || canCommentChanges);

	/* What a block comment is about, quoted: the live page first (newest), then the saved version. */
	function blockQuote(blockId: string | null): string | null {
		if (!blockId) return null;
		const live =
			typeof window === 'undefined'
				? null
				: window.document.querySelector(
						`.native-document-body [data-block-id="${CSS.escape(blockId)}"]`
					)?.textContent;
		const text = live ?? documentBlockText(view.current_version.version.content_json, blockId);
		const quote = text === null || text === undefined ? '' : quoteExcerpt(text);
		return quote || null;
	}

	function showBlock(blockId: string): void {
		if (onshowblock) onshowblock(blockId);
		else revealDocumentBlock(blockId);
	}

	/* Mentions: typing @ offers people by name. The name goes into the text, the person is
	 * remembered, and the comment carries a mention of them, not the letters of their name. */
	let mentionQuery = $state<{ query: string; start: number; end: number } | null>(null);
	let mentionIndex = $state(0);
	let mentioned = $state<CommentMention[]>([]);
	let commentInput = $state<HTMLTextAreaElement>();
	const mentionOptions = $derived.by(() => {
		if (!mentionQuery) return [];
		const query = mentionQuery.query.toLowerCase();
		return people
			.filter((person) => person.kind !== 'system')
			.filter(
				(person) =>
					!query ||
					person.display.toLowerCase().includes(query) ||
					person.actor_id.toLowerCase().includes(query)
			)
			.toSorted(
				(left, right) =>
					Number(!left.display.toLowerCase().startsWith(query)) -
						Number(!right.display.toLowerCase().startsWith(query)) ||
					left.display.localeCompare(right.display)
			)
			.slice(0, 6);
	});

	function readMentionQuery(input: HTMLTextAreaElement): void {
		const trigger = composerTrigger(input.value, input.selectionStart ?? input.value.length);
		mentionQuery =
			trigger?.trigger === '@'
				? { query: trigger.query, start: trigger.start, end: trigger.end }
				: null;
		mentionIndex = 0;
	}

	function pickMention(person: PersonReference): void {
		const at = mentionQuery;
		if (!at) return;
		const inserted = `@${person.display} `;
		commentText = `${commentText.slice(0, at.start)}${inserted}${commentText.slice(at.end)}`;
		if (!mentioned.some((mention) => mention.actor_id === person.actor_id))
			mentioned = [...mentioned, { actor_id: person.actor_id, label: person.display }];
		mentionQuery = null;
		commentAttempt = null;
		const caret = at.start + inserted.length;
		void tick().then(() => {
			commentInput?.focus();
			commentInput?.setSelectionRange(caret, caret);
		});
	}

	function mentionKeys(event: KeyboardEvent): void {
		if (!mentionQuery || !mentionOptions.length || composing(event)) return;
		if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
			event.preventDefault();
			const step = event.key === 'ArrowDown' ? 1 : -1;
			mentionIndex = (mentionIndex + step + mentionOptions.length) % mentionOptions.length;
		} else if (event.key === 'Enter' || event.key === 'Tab') {
			event.preventDefault();
			pickMention(mentionOptions[mentionIndex] ?? mentionOptions[0]);
		} else if (event.key === 'Escape') {
			event.preventDefault();
			mentionQuery = null;
		}
	}

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
		reviewChangesAttempt = null;
		reviewNote = '';
		mentioned = [];
		mentionQuery = null;
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

	/* Open threads first; resolved ones wait behind their own tab, with Re-open. */
	let threadFilter = $state<'open' | 'resolved'>('open');
	const shownThreads = $derived(
		threads.threads.filter((item) => item.thread.status === threadFilter)
	);
	const resolvedCount = $derived(
		threads.threads.filter((item) => item.thread.status === 'resolved').length
	);
	/* Arriving for one thread (from For you, or a mark in the margin) opens that thread, shows
	 * the passage it is about, and points at the comment that brought the person here. */
	let appliedThread = $state('');
	const requestedKey = $derived(requestedThreadId ? `${requestedThreadId}#${threadRequest}` : '');
	let flashCommentId = $state('');
	$effect(() => {
		/* Until a thread asked for is found, nothing else is chosen in its place. */
		if (requestedKey && appliedThread !== requestedKey) return;
		if (!shownThreads.some((item) => item.thread.id === selectedThreadId))
			selectedThreadId = shownThreads.at(-1)?.thread.id ?? '';
	});
	$effect(() => {
		const wanted = requestedThreadId;
		const key = requestedKey;
		if (!wanted || key === untrack(() => appliedThread)) return;
		const found = threads.threads.find((item) => item.thread.id === wanted);
		if (!found) {
			/* Gone (taken back, or no longer readable), or not among those loaded: show the list
			 * as usual rather than nothing. */
			if (threads.status !== 'unknown') appliedThread = key;
			return;
		}
		appliedThread = key;
		threadFilter = found.thread.status;
		selectedThreadId = wanted;
		composingThread = false;
		flashCommentId = requestedCommentId || 'newest';
		setTimeout(() => (flashCommentId = ''), 2_400);
		if (found.thread.block_id && found.anchor_state === 'active')
			void tick().then(() => revealDocumentBlock(found.thread.block_id ?? ''));
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
		const picked = mentioned.filter((mention) => body.includes(`@${mention.label}`));
		const semanticFingerprint = JSON.stringify({
			target,
			mode,
			blockId: mode === 'thread' ? commentBlockId : null,
			quote: mode === 'thread' ? commentQuote : null,
			threadId: mode === 'reply' ? selectedThreadId : null,
			body,
			picked
		});
		const attempt = pendingDocumentMutation(commentAttempt, semanticFingerprint, () => {
			const contentJson = commentContent(body, knownActorIds, picked);
			return mode === 'thread'
				? {
						mode: 'thread' as const,
						target,
						blockId: commentBlockId,
						quote: commentBlockId ? commentQuote : null,
						contentJson
					}
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
					attempt.command.id,
					attempt.input.quote
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
			mentioned = [];
			mentionQuery = null;
		} catch (cause) {
			failClosedDocumentRead(client, cause, target.companyId, target.documentId);
			if (!isCurrentTarget(target)) return;
			commentFailure = failureSentence(cause, 'The comment was not saved.');
			if (!isRetryableDocumentFailure(cause)) commentAttempt = null;
		} finally {
			if (isCurrentTarget(target)) commentBusy = false;
		}
	}

	/* "Please change this" in a comment becomes a revision of the Work, not just a reply. */
	let sendingChange = $state(false);
	async function sendChangeRequest(quote: string | null): Promise<void> {
		if (!onchangerequest || sendingChange) return;
		const text = (threadMessages?.comments ?? [])
			.map((comment) => `${actorName(comment.author_actor_id)}: ${comment.plain_text}`)
			.join('\n');
		if (!text.trim()) return;
		sendingChange = true;
		try {
			toast(await onchangerequest({ quote, text }));
		} catch (error) {
			toast(failureSentence(error, 'The change request was not sent.'), { tone: 'failure' });
		} finally {
			sendingChange = false;
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
			commentFailure = failureSentence(cause, 'The thread was not resolved.');
			if (!isRetryableDocumentFailure(cause)) resolveAttempt = null;
		} finally {
			if (isCurrentTarget(target)) commentBusy = false;
		}
	}

	async function reopenThread(): Promise<void> {
		if (!selectedThread || !online || !canComment || commentBusy) return;
		const target = { companyId, documentId, threadId: selectedThread.thread.id };
		commentBusy = true;
		commentFailure = '';
		try {
			const result = await reopenDocumentCommentThread(
				target.companyId,
				target.documentId,
				target.threadId,
				selectedThread.thread.version
			);
			threads.acceptResolved(target, result);
			if (isCurrentTarget(target)) threadFilter = 'open';
		} catch (cause) {
			failClosedDocumentRead(client, cause, target.companyId, target.documentId);
			if (isCurrentTarget(target))
				commentFailure = failureSentence(cause, 'The thread was not reopened.');
		} finally {
			if (isCurrentTarget(target)) commentBusy = false;
		}
	}

	/* A comment is its author's to change or take back; the company's owner may take back any. */
	let editingCommentId = $state('');
	let editText = $state('');
	let confirmingDelete = $state('');
	function mentionsIn(node: DocumentContent | DocumentNode): CommentMention[] {
		const found: CommentMention[] = [];
		const walk = (value: DocumentNode) => {
			const actor = value.attrs?.actor_id;
			const label = value.attrs?.label;
			if (value.type === 'mention' && typeof actor === 'string' && typeof label === 'string')
				found.push({ actor_id: actor, label });
			value.content?.forEach(walk);
		};
		walk(node);
		return found;
	}
	function startEdit(comment: DocumentCommentRow): void {
		confirmingDelete = '';
		editingCommentId = comment.id;
		editText = comment.plain_text.trimEnd();
	}
	async function refreshThread(target: DocumentThreadTarget): Promise<void> {
		await client.invalidateQueries({
			queryKey: documentQueryKeys.comments(target.companyId, target.documentId, target.threadId)
		});
		await client.invalidateQueries({
			queryKey: documentQueryKeys.commentThreads(target.companyId, target.documentId)
		});
	}
	async function saveEdit(comment: DocumentCommentRow): Promise<void> {
		const body = editText.trim();
		if (!body || commentBusy) return;
		const target = { companyId, documentId, threadId: comment.thread_id };
		commentBusy = true;
		commentFailure = '';
		try {
			await editDocumentComment(
				target.companyId,
				target.documentId,
				target.threadId,
				comment.id,
				commentContent(body, knownActorIds, mentionsIn(comment.content_json))
			);
			if (isCurrentTarget(target)) editingCommentId = '';
			await refreshThread(target);
		} catch (cause) {
			if (isCurrentTarget(target))
				commentFailure = failureSentence(cause, 'Your change was not saved.');
		} finally {
			if (isCurrentTarget(target)) commentBusy = false;
		}
	}
	async function deleteComment(comment: DocumentCommentRow): Promise<void> {
		if (commentBusy) return;
		const target = { companyId, documentId, threadId: comment.thread_id };
		commentBusy = true;
		commentFailure = '';
		try {
			await deleteDocumentComment(target.companyId, target.documentId, target.threadId, comment.id);
			confirmingDelete = '';
			await refreshThread(target);
		} catch (cause) {
			if (isCurrentTarget(target))
				commentFailure = failureSentence(cause, 'The comment was not deleted.');
		} finally {
			if (isCurrentTarget(target)) commentBusy = false;
		}
	}

	const isMac = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/u.test(navigator.platform);
	/* Enter picks a mention; Cmd or Ctrl+Enter sends, as in any comment box. */
	function composerKeys(event: KeyboardEvent): void {
		if (
			event.key === 'Enter' &&
			(event.metaKey || event.ctrlKey) &&
			!(mentionQuery && mentionOptions.length)
		) {
			event.preventDefault();
			(event.currentTarget as HTMLTextAreaElement).form?.requestSubmit();
			return;
		}
		mentionKeys(event);
	}

	/* History: what each version changed against the one before it, or, arriving from "changed
	 * since you looked", everything after the last version this person had seen. */
	/* A person may compare the previewed version with any older one; by default, the one before. */
	let compareWithId = $state('');
	let comparedFor = '';
	$effect(() => {
		if (selectedVersionId !== comparedFor) {
			comparedFor = selectedVersionId;
			compareWithId = '';
		}
	});
	const olderVersions = $derived.by(() => {
		const selected = versions.versions.find((item) => item.id === selectedVersionId);
		return selected
			? versions.versions.filter((item) => item.version_number < selected.version_number)
			: [];
	});
	const baseVersionId = $derived.by(() => {
		const list = versions.versions;
		const selected = list.find((item) => item.id === selectedVersionId);
		if (!selected) return '';
		if (compareWithId && olderVersions.some((item) => item.id === compareWithId))
			return compareWithId;
		if (changesSince && selected.id === list[0]?.id) {
			const looked = Date.parse(changesSince);
			const seen = list.find((item) => Date.parse(item.created_at) <= looked);
			if (seen && seen.version_number < selected.version_number) return seen.id;
		}
		return list.find((item) => item.version_number < selected.version_number)?.id ?? '';
	});
	const baseDetail = documentVersionQuery(
		() => companyId,
		() => documentId,
		() => baseVersionId
	);
	const changes = $derived(
		versionDetail.version && (!baseVersionId || baseDetail.version)
			? versionChanges(
					baseDetail.version?.version.content_json ?? null,
					versionDetail.version.version.content_json
				)
			: null
	);
	/* Arriving from "changed since you looked", the newest version is the one to read. */
	let appliedSince = $state('');
	$effect(() => {
		const since = changesSince;
		if (!since || since === untrack(() => appliedSince) || !versions.versions.length) return;
		appliedSince = since;
		selectedVersionId = versions.versions[0].id;
	});
	const changedSince = (item: DocumentVersionSummary) =>
		Boolean(changesSince) && Date.parse(item.created_at) > Date.parse(changesSince);
	function editorsOf(item: DocumentVersionSummary): string {
		const ids = item.editor_actor_ids?.length
			? item.editor_actor_ids
			: item.created_by_actor_id === 'daemon'
				? []
				: [item.created_by_actor_id];
		const names = ids.map(actorName);
		if (!names.length) return 'Saved automatically';
		return names.length <= 2
			? names.join(' and ')
			: `${names.slice(0, -1).join(', ')} and ${names.at(-1)}`;
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
			summary,
			reviewer: reviewerId || null
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
				attempt.command.id,
				attempt.input.reviewer
			);
			mutationSucceeded = true;
			reviews.accept(attempt.input.target, review);
			if (isCurrentTarget(target)) {
				reviewRequestAttempt = null;
				reviewSummary = '';
				reviewerId = '';
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
				reviewFailure = failureSentence(cause, 'Review could not be requested.');
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
			acceptedVersionName: reviewName.trim(),
			feedback: canRequestChanges ? reviewNote.trim() : ''
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
			/* Not bound to Work, a note has nowhere to go with the decision: it is posted first,
			 * and the version is not accepted if the note could not be. */
			if (canCommentChanges && reviewNote.trim() && !(await postReviewNote(''))) return;
			const result = await acceptDocumentReview(
				attempt.input.target.companyId,
				attempt.input.target.documentId,
				attempt.input.reviewId,
				attempt.input.expectedDocumentVersion,
				attempt.input.expectedReviewVersion,
				attempt.input.acceptedVersionName,
				attempt.command.id,
				attempt.input.feedback
			);
			mutationSucceeded = true;
			reviews.acceptResolution(attempt.input.target, result);
			if (isCurrentTarget(target)) {
				reviewAcceptAttempt = null;
				reviewNote = '';
			}
			await refreshDocument(attempt.input.target);
		} catch (cause) {
			failClosedDocumentRead(client, cause, target.companyId, target.documentId);
			if (!isCurrentTarget(target)) return;
			if (mutationSucceeded) {
				reviewNotice =
					'Review accepted, but this view could not fully refresh. Reload to see the latest state.';
			} else {
				reviewFailure = failureSentence(cause, 'The review was not accepted.');
				if (!isRetryableDocumentFailure(cause)) reviewAcceptAttempt = null;
			}
		} finally {
			if (isCurrentTarget(target)) reviewBusy = false;
		}
	}

	/* Sending the version back: the Work that asked resumes with the note, and the review leaves
	 * the Inbox. It refreshes the document as accepting does, so the Inbox closes it the same way. */
	async function requestChanges(): Promise<void> {
		const feedback = reviewNote.trim();
		if (!requestedReview || !feedback || !online || !canRequestChanges || reviewBusy) return;
		const target = { companyId, documentId };
		const input = {
			target,
			reviewId: requestedReview.id,
			expectedDocumentVersion: view.document.version,
			expectedReviewVersion: requestedReview.version,
			feedback
		};
		const attempt = pendingDocumentMutation(
			reviewChangesAttempt,
			JSON.stringify(input),
			() => input
		);
		reviewChangesAttempt = attempt;
		reviewBusy = true;
		reviewFailure = '';
		reviewNotice = '';
		let mutationSucceeded = false;
		try {
			const result = await requestDocumentReviewChanges(
				attempt.input.target.companyId,
				attempt.input.target.documentId,
				attempt.input.reviewId,
				attempt.input.expectedDocumentVersion,
				attempt.input.expectedReviewVersion,
				attempt.input.feedback,
				attempt.command.id
			);
			mutationSucceeded = true;
			reviews.acceptResolution(attempt.input.target, result);
			if (isCurrentTarget(target)) {
				reviewChangesAttempt = null;
				reviewNote = '';
				reviewNotice =
					result.review.status === 'stale'
						? 'A newer version replaced the one under review, so it went back to be checked again.'
						: `Sent back to ${actorName(result.review.requested_by_actor_id)} with your note.`;
			}
			await refreshDocument(attempt.input.target);
		} catch (cause) {
			failClosedDocumentRead(client, cause, target.companyId, target.documentId);
			if (!isCurrentTarget(target)) return;
			if (mutationSucceeded) {
				reviewNotice =
					'Changes requested, but this view could not fully refresh. Reload to see the latest state.';
			} else {
				reviewFailure = failureSentence(cause, 'Changes were not requested.');
				if (!isRetryableDocumentFailure(cause)) reviewChangesAttempt = null;
			}
		} finally {
			if (isCurrentTarget(target)) reviewBusy = false;
		}
	}

	/* A review nobody's Work is waiting on has nothing to send back to: the note goes to whoever
	 * asked, as a comment that mentions them, and the review stays open for their next version. */
	let reviewCommentAttempt = $state<PendingDocumentMutation<{
		target: DocumentTarget;
		contentJson: DocumentContent;
	}> | null>(null);
	async function commentChanges(): Promise<void> {
		if (!reviewNote.trim() || !online || !canCommentChanges || reviewBusy) return;
		reviewBusy = true;
		reviewFailure = '';
		reviewNotice = '';
		try {
			const name = await postReviewNote('Changes requested: ');
			if (name)
				reviewNotice = `Your note is in the comments for ${name}. The review stays open for their next version.`;
		} finally {
			reviewBusy = false;
		}
	}

	/** Posts the review note as a comment to whoever asked; their name once it is saved. */
	async function postReviewNote(prefix: string): Promise<string | null> {
		const feedback = reviewNote.trim();
		if (!requestedReview || !feedback) return null;
		const target = { companyId, documentId };
		const requester = requestedReview.requested_by_actor_id;
		const name = actorName(requester);
		const attempt = pendingDocumentMutation(
			reviewCommentAttempt,
			JSON.stringify({ target, requester, prefix, feedback }),
			() => ({
				target,
				contentJson: commentContent(`@${name} ${prefix}${feedback}`, new Set([requester]), [
					{ actor_id: requester, label: name }
				])
			})
		);
		reviewCommentAttempt = attempt;
		try {
			const created = await createDocumentCommentThread(
				attempt.input.target.companyId,
				attempt.input.target.documentId,
				null,
				attempt.input.contentJson,
				attempt.command.id
			);
			threads.acceptCreated(attempt.input.target, created);
			await client.invalidateQueries({
				queryKey: documentQueryKeys.commentThreads(target.companyId, target.documentId)
			});
			if (!isCurrentTarget(target)) return null;
			reviewCommentAttempt = null;
			reviewNote = '';
			return name;
		} catch (cause) {
			failClosedDocumentRead(client, cause, target.companyId, target.documentId);
			if (!isCurrentTarget(target)) return null;
			reviewFailure = failureSentence(cause, 'Your note was not posted.');
			if (!isRetryableDocumentFailure(cause)) reviewCommentAttempt = null;
			return null;
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
				proposalFailure = failureSentence(cause, 'The proposal was not resolved.');
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
				versionFailure = failureSentence(cause, 'The version was not restored.');
				if (!isRetryableDocumentFailure(cause)) restoreCommand = null;
			}
		} finally {
			if (isCurrentTarget(target)) restoringVersionId = '';
		}
	}

	let answeredNameRequest = 0;
	$effect(() => {
		const request = nameVersionRequest;
		if (!request || request === answeredNameRequest || panel !== 'versions') return;
		answeredNameRequest = request;
		untrack(() => {
			namingVersion = true;
			versionFailure = '';
			versionNotice = '';
		});
		void tick().then(() => window.document.getElementById('document-version-name')?.focus());
	});

	async function saveVersion(): Promise<void> {
		if (!online || !canResolveDocument || savingVersion) return;
		const target = { companyId, documentId };
		const reason = versionName.trim() || 'Saved version';
		const command = pendingDocumentCommand(saveCommand, JSON.stringify({ target, reason }));
		saveCommand = command;
		savingVersion = true;
		versionFailure = '';
		versionNotice = '';
		try {
			await saveDocumentVersion(target.companyId, target.documentId, reason, command.id);
			saveCommand = null;
			const saved = await getDocument(target.companyId, target.documentId);
			if (isCurrentTarget(target)) {
				namingVersion = false;
				versionName = '';
				versionNotice = `Saved as “${reason}”.`;
				onaccept(target, saved);
			}
			await invalidateDocument(client, target.companyId, target.documentId);
		} catch (cause) {
			if (!isCurrentTarget(target)) return;
			versionFailure = failureSentence(cause, 'The version was not saved.');
			if (!isRetryableDocumentFailure(cause)) saveCommand = null;
		} finally {
			if (isCurrentTarget(target)) savingVersion = false;
		}
	}

	function proposalLabel(proposal: DocumentRevisionProposalSummary): string {
		if (proposal.scope !== 'block' || !proposal.block_id) return 'Whole document';
		const quote = blockQuote(proposal.block_id);
		return quote ? `“${quote}”` : 'A block that has since moved';
	}
</script>

{#snippet changeList(list: BlockChange[], label: string)}
	<ol class="version-diff" aria-label={label}>
		{#each list as change (change.id)}
			<li class="diff-{change.kind}">
				{#if change.kind === 'changed'}
					{#each change.words as part, index (index)}{#if part.op === 'added'}<ins>{part.text}</ins
							>{:else if part.op === 'removed'}<del>{part.text}</del>{:else}{part.text}{/if}{/each}
				{:else if change.kind === 'added'}
					<ins>{change.text}</ins>
				{:else}
					<del>{change.text}</del>
				{/if}
			</li>
		{/each}
	</ol>
{/snippet}

<aside class="document-inspector" aria-label="Document review and discussion">
	{#if ondismiss}
		<div class="inspector-dismiss">
			<button
				type="button"
				class="flat-action icon-only"
				onclick={ondismiss}
				aria-label="Close"
				title="Close this panel without accepting or rejecting a review"
			>
				<X size={15} aria-hidden="true" />
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
				{#if resolvedCount}
					<div class="thread-filter" role="group" aria-label="Show threads">
						<button
							type="button"
							aria-pressed={threadFilter === 'open'}
							onclick={() => (threadFilter = 'open')}>Open</button
						>
						<button
							type="button"
							aria-pressed={threadFilter === 'resolved'}
							title="Resolved threads can be reopened"
							onclick={() => (threadFilter = 'resolved')}>Resolved {resolvedCount}</button
						>
					</div>
				{:else}
					<h2 title="Comments stay attached to this document">Discussion</h2>
				{/if}
				{#if canComment}
					<button type="button" class="flat-action" onclick={beginDocumentComment}
						>New comment</button
					>
				{/if}
			</header>

			{#if threads.failure}
				<FailureNotice error={threads.failure} subject="comments" onretry={threads.refresh} />
			{/if}

			{#if composingThread}
				{@const quote = commentQuote ? quoteExcerpt(commentQuote) : blockQuote(commentBlockId)}
				<div class="comment-target">
					<span
						>{#if commentBlockId && quote}<q>{quote}</q>{:else if commentBlockId}Commenting on a
							block{:else}Commenting on the document{/if}</span
					>
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
					{#each shownThreads as item (item.thread.id)}
						{@const quote =
							item.anchor_state === 'orphaned'
								? null
								: item.thread.anchor_quote
									? quoteExcerpt(item.thread.anchor_quote)
									: blockQuote(item.thread.block_id)}
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
								<strong
									>{#if item.thread.block_id && quote}<q>{quote}</q
										>{:else if item.thread.block_id}Block comment{:else}Document comment{/if}</strong
								>
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
							<Skeleton label="Loading comments" variant="messages" count={2} />
						{:else if threads.threads.length}
							<p class="quiet-state">
								{threadFilter === 'open' ? 'Every thread is resolved.' : 'Nothing resolved yet.'}
							</p>
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
				{@const blockId = selectedThread.thread.block_id}
				{@const quote =
					selectedThread.anchor_state === 'orphaned'
						? null
						: selectedThread.thread.anchor_quote
							? quoteExcerpt(selectedThread.thread.anchor_quote)
							: blockQuote(blockId)}
				<div class="comment-thread">
					<header>
						<div>
							<strong
								>{#if blockId && quote}<q>{quote}</q>{:else if blockId}A block that has since moved{:else}Whole
									document{/if}</strong
							>
							<span>{statusLabel(selectedThread.thread.status)}</span>
						</div>
						<div class="thread-actions">
							{#if blockId && quote}
								<button
									type="button"
									class="flat-action"
									title="Scroll the document to this passage"
									onclick={() => showBlock(blockId)}>Show</button
								>
							{/if}
							{#if selectedThread.thread.status === 'open' && onchangerequest}
								<button
									type="button"
									class="flat-action"
									title="Send this thread to the Work as a change request, so a revision follows rather than a reply"
									disabled={!online || sendingChange}
									onclick={() => void sendChangeRequest(quote)}
									>{sendingChange ? 'Sending…' : 'Send as change request'}</button
								>
							{/if}
							{#if selectedThread.thread.status === 'open' && canComment}
								<button
									type="button"
									class="flat-action"
									disabled={!online || commentBusy}
									onclick={() => void resolveThread()}
									><Check size={14} strokeWidth={2} /> Resolve</button
								>
							{:else if selectedThread.thread.status === 'resolved' && canComment && (canJudge || selectedThread.thread.created_by_actor_id === viewerActorId)}
								<button
									type="button"
									class="flat-action"
									title="Open the thread again to reply"
									disabled={!online || commentBusy}
									onclick={() => void reopenThread()}
									><RotateCcw size={13} strokeWidth={1.8} /> Re-open</button
								>
							{/if}
						</div>
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
					{#each threadMessages?.comments ?? [] as comment, index (comment.id)}
						{@const reactions = commentReactions.summaryFor(comment.id)}
						{@const own = comment.author_actor_id === viewerActorId && canComment && online}
						<article
							class="comment-row"
							class:flash={flashCommentId === comment.id ||
								(flashCommentId === 'newest' &&
									index === (threadMessages?.comments.length ?? 0) - 1)}
						>
							<header>
								<strong>{actorName(comment.author_actor_id)}</strong><span class="comment-meta"
									><time
										datetime={comment.created_at}
										title={comment.edited_at ? `Edited ${shortDate(comment.edited_at)}` : undefined}
										>{shortDate(comment.created_at)}{comment.edited_at ? ' · edited' : ''}</time
									>{#if canComment && online}<ReactionAdd
											{reactions}
											onreact={(emoji, on) => void reactTo(comment.id, emoji, on)}
										/>{/if}{#if own && editingCommentId !== comment.id}<ActionMenu
											label="Comment actions"
											><button type="button" onclick={() => startEdit(comment)}>Edit</button><button
												type="button"
												class="danger"
												onclick={() => {
													editingCommentId = '';
													confirmingDelete = comment.id;
												}}>Delete…</button
											></ActionMenu
										>{/if}</span
								>
							</header>
							{#if editingCommentId === comment.id}
								<form
									class="comment-edit"
									onsubmit={(event) => {
										event.preventDefault();
										void saveEdit(comment);
									}}
								>
									<textarea
										bind:value={editText}
										rows="3"
										aria-label="Edit your comment"
										disabled={commentBusy}
										onkeydown={(event) => {
											if (event.key === 'Escape') editingCommentId = '';
											else if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
												event.preventDefault();
												event.currentTarget.form?.requestSubmit();
											}
										}}></textarea>
									<div class="edit-actions">
										<button
											type="button"
											class="btn small ghost"
											onclick={() => (editingCommentId = '')}>Cancel</button
										>
										<button
											type="submit"
											class="btn small primary"
											disabled={!editText.trim() || commentBusy}>Save</button
										>
									</div>
								</form>
							{:else}
								<p>{comment.plain_text}</p>
							{/if}
							{#if confirmingDelete === comment.id}
								<div class="confirm-delete" role="alert">
									<span>Delete this comment? It goes for everyone.</span>
									<button
										type="button"
										class="btn small ghost"
										onclick={() => (confirmingDelete = '')}>Keep</button
									>
									<button
										type="button"
										class="btn small danger"
										disabled={commentBusy}
										onclick={() => void deleteComment(comment)}>Delete</button
									>
								</div>
							{/if}
							{#if comment.mentioned_actor_ids.length}
								<div class="mention-list">
									{#each comment.mentioned_actor_ids as actorId (actorId)}<span
											>@{actorName(actorId)}</span
										>{/each}
								</div>
							{/if}
							<MessageReactions
								{reactions}
								readonly={!canComment || !online}
								onreact={(emoji, on) => void reactTo(comment.id, emoji, on)}
							/>
						</article>
					{:else}
						{#if threadMessages?.status === 'unknown'}
							<Skeleton label="Loading the thread" variant="messages" count={2} />
						{/if}
					{/each}
					{#if reactionFailure}<p class="inline-error" role="alert">{reactionFailure}</p>{/if}
				</div>
			{/if}

			{#if canComment && (composingThread || (selectedThread && selectedThread.thread.status === 'open'))}
				<form class="comment-composer" onsubmit={(event) => void submitComment(event)}>
					<label for="document-comment">{composingThread ? 'Comment' : 'Reply'}</label>
					<div class="mention-field">
						<textarea
							id="document-comment"
							bind:this={commentInput}
							bind:value={commentText}
							rows="3"
							placeholder="Reply, or type @ to mention someone"
							title={`${isMac ? '⌘' : 'Ctrl'}+Enter sends`}
							disabled={!online || commentBusy}
							role="combobox"
							aria-autocomplete="list"
							aria-expanded={Boolean(mentionQuery && mentionOptions.length)}
							aria-controls="document-comment-mentions"
							aria-activedescendant={mentionQuery && mentionOptions.length
								? `document-comment-mention-${mentionIndex}`
								: undefined}
							oninput={(event) => {
								commentAttempt = null;
								readMentionQuery(event.currentTarget);
							}}
							onkeydown={composerKeys}
							onblur={() => setTimeout(() => (mentionQuery = null), 120)}></textarea>
						{#if mentionQuery && mentionOptions.length}
							<ul class="mention-menu" id="document-comment-mentions" role="listbox">
								{#each mentionOptions as person, index (person.actor_id)}
									<li
										id={`document-comment-mention-${index}`}
										role="option"
										aria-selected={index === mentionIndex}
										class:active={index === mentionIndex}
									>
										<button
											type="button"
											tabindex="-1"
											onmousedown={(event) => event.preventDefault()}
											onclick={() => pickMention(person)}>{person.display}</button
										>
									</li>
								{/each}
							</ul>
						{/if}
					</div>
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
				<h2 title="Ask someone to judge the saved document, and decide on changes agents propose">
					Review
				</h2>
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
						{#if requestedReview.reviewer_actor_id}
							<div>
								<dt>Asked of</dt>
								<dd>{actorName(requestedReview.reviewer_actor_id)}</dd>
							</div>
						{/if}
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
							><span title="History lists the accepted version under this name">Version name</span
							><input
								bind:value={reviewName}
								disabled={!online || editing || reviewBusy}
								oninput={() => (reviewAcceptAttempt = null)}
							/></label
						>
						{#if mayRequestChanges}
							<label class="field"
								><span
									title={canRequestChanges
										? `Goes back to ${actorName(requestedReview.requested_by_actor_id)} with your decision. Needed to request changes.`
										: `Posted as a comment to ${actorName(requestedReview.requested_by_actor_id)} with your decision. Needed to request changes.`}
									>Note</span
								><textarea
									bind:value={reviewNote}
									rows="3"
									maxlength="4000"
									placeholder="What should change, or anything to carry forward"
									disabled={!online || editing || reviewBusy}
									oninput={() => {
										reviewAcceptAttempt = null;
										reviewChangesAttempt = null;
										reviewCommentAttempt = null;
									}}></textarea></label
							>
						{/if}
						<div class="review-actions">
							{#if mayRequestChanges}
								<button
									type="button"
									class="btn"
									title={!reviewNote.trim()
										? 'Write what should change first'
										: canRequestChanges
											? 'Send this version back with your note'
											: 'Post your note as a comment to whoever asked'}
									disabled={!online || editing || reviewBusy || !reviewNote.trim()}
									onclick={() => void (canRequestChanges ? requestChanges() : commentChanges())}
									>Request changes</button
								>
							{/if}
							<button
								type="button"
								class="btn primary"
								disabled={!online || editing || reviewBusy || !reviewName.trim()}
								onclick={() => void acceptReview()}
								>{reviewBusy ? 'Saving…' : 'Accept this version'}</button
							>
						</div>
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
					{#if reviewerChoices.length}
						<label class="field"
							><span
								title="They hear about it in For you; anyone who can edit it may still judge it"
								>Who should review it?</span
							><select
								bind:value={reviewerId}
								disabled={!online || editing || reviewBusy}
								onchange={() => (reviewRequestAttempt = null)}
							>
								<option value="">Anyone who can edit it</option>
								{#each reviewerChoices as person (person.actor_id)}
									<option value={person.actor_id}>{person.display}</option>
								{/each}
							</select></label
						>
					{/if}
					<button
						type="submit"
						class="btn primary"
						disabled={!online || editing || reviewBusy || !reviewSummary.trim()}
						>{reviewBusy ? 'Requesting…' : 'Request review'}</button
					>
				</form>
			{:else}
				<p class="quiet-state">
					No review in progress. People who can edit this document can ask for one.
				</p>
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
						{#if proposalDetail.proposal.base_version_id !== view.current_version.version.id}
							<p
								class="diff-scope"
								title="The document changed after this was proposed, so the comparison also shows those later edits."
							>
								Proposed on an earlier version
							</p>
						{/if}
						{#if proposalDiff?.length}
							{@render changeList(proposalDiff, 'What this proposal changes')}
						{:else}
							<p class="diff-scope">It changes no words.</p>
						{/if}
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
								><span title="History lists the accepted version under this name">Version name</span
								><input
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
					<Skeleton label="Loading proposals" count={3} />
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
				<h2
					title="Versions are saved automatically, named for who was editing. Restoring keeps the current draft in history."
				>
					History
				</h2>
				{#if canJudge && !namingVersion}<button
						type="button"
						class="flat-action"
						disabled={!online || editing}
						title={editing
							? 'Wait for edits to finish saving'
							: 'Keep what the document holds now under a name, such as the version you sent'}
						onclick={() => {
							namingVersion = true;
							versionFailure = '';
							versionNotice = '';
						}}>Save version…</button
					>{/if}
			</header>
			{#if namingVersion}
				<form
					class="name-version"
					onsubmit={(event) => {
						event.preventDefault();
						void saveVersion();
					}}
				>
					<label for="document-version-name" class="sr-only">Version name</label>
					<input
						id="document-version-name"
						type="text"
						maxlength="500"
						placeholder="Version name (optional)"
						bind:value={versionName}
						disabled={savingVersion}
					/>
					<button
						type="submit"
						class="flat-action"
						disabled={savingVersion || !online || !canResolveDocument}
						>{savingVersion ? 'Saving…' : 'Save'}</button
					>
					<button
						type="button"
						class="flat-action"
						disabled={savingVersion}
						onclick={() => {
							namingVersion = false;
							versionName = '';
						}}>Cancel</button
					>
				</form>
			{/if}
			{#if editing && canJudge}<p class="pending-draft">Waiting for edits to finish saving.</p>{/if}
			{#if versionFailure}<p class="inline-error" role="alert">{versionFailure}</p>{/if}
			{#if versionNotice}<p class="inline-notice" role="status">{versionNotice}</p>{/if}
			<div class="version-line" aria-label="Document version history">
				{#each versions.versions as item (item.id)}
					<article
						class:current={item.id === view.current_version.version.id}
						class:selected={item.id === selectedVersionId}
						class:fresh={changedSince(item)}
						title={changedSince(item) ? 'Changed since you last looked' : undefined}
					>
						<span class="version-node"></span>
						<header>
							<strong>Version {item.version_number}</strong><span
								>{statusLabel(item.document_status)}</span
							>
						</header>
						<p>{item.reason}</p>
						<footer>
							<span>{editorsOf(item)}</span><time datetime={item.created_at}
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
					{#if versions.status === 'unknown'}<Skeleton
							label="Loading versions"
							variant="list"
							count={3}
						/>{:else}<p class="quiet-state">No version history is available.</p>{/if}
				{/each}
			</div>
			{#if selectedVersionId}
				<section class="version-preview" aria-live="polite">
					{#if versionDetail.version}
						<header>
							<strong>Version {versionDetail.version.version.version_number}</strong>
							<div class="thread-filter" role="group" aria-label="Show">
								<button
									type="button"
									aria-pressed={previewMode === 'page'}
									title="The document as it read in this version"
									onclick={() => (previewMode = 'page')}>Page</button
								><button
									type="button"
									aria-pressed={previewMode === 'changes'}
									title="What this version changed"
									onclick={() => (previewMode = 'changes')}>Changes</button
								>
							</div>
							<ActionMenu label="Download this version">
								<button type="button" onclick={() => void downloadVersion('docx')}
									>Download as Word</button
								>
								<button type="button" onclick={() => void downloadVersion('md')}
									>Download as Markdown</button
								>
							</ActionMenu>
							{#if versionDetail.version.version.id !== view.current_version.version.id && canJudge}
								<button
									type="button"
									class="flat-action"
									title={othersEditing.length
										? `${othersEditing.join(', ')} ${othersEditing.length === 1 ? 'is' : 'are'} typing in it now. Their editors switch to the restored version, with their unsaved changes put back on top.`
										: 'Make this version the current one; the version it replaces stays in history.'}
									disabled={!online || editing || Boolean(restoringVersionId)}
									onclick={() => void restore(versionDetail.version?.version.id ?? '')}
									><RotateCcw size={13} strokeWidth={1.8} />
									{restoringVersionId
										? 'Restoring…'
										: 'Restore as new version'}{#if othersEditing.length}<span
											class="others-editing"
											>· {othersEditing.length === 1
												? `${othersEditing[0]} is typing`
												: `${othersEditing.length} people typing`}</span
										>{/if}</button
								>
							{/if}
						</header>
						{#if previewMode === 'page'}
							<div class="version-page rendered-document">
								{@html withDocumentImages(versionDetail.version.rendered_html, companyId)}
							</div>
						{:else if olderVersions.length > 1}
							<label class="compare-with"
								>Compare with
								<select
									value={baseVersionId}
									onchange={(event) => (compareWithId = event.currentTarget.value)}
								>
									{#each olderVersions as older (older.id)}
										<option value={older.id}
											>Version {older.version_number} · {shortDate(older.created_at)}</option
										>
									{/each}
								</select></label
							>
						{/if}
						{#if previewMode === 'page'}
							<!-- The page reads above. -->
						{:else if changes === null}
							<Skeleton label="Comparing with the version before" count={3} />
						{:else if changes.length}
							<p class="diff-scope">
								{baseVersionId && baseDetail.version
									? `What changed since version ${baseDetail.version.version.version_number}`
									: versionDetail.version.version.version_number === 1
										? 'The first version'
										: 'Everything in this version'}
							</p>
							{@render changeList(changes, 'What changed')}
						{:else}
							<p class="diff-scope">No words changed in this version.</p>
						{/if}
					{:else if versionDetail.failure}
						<FailureNotice
							error={versionDetail.failure}
							subject="this version"
							onretry={versionDetail.refresh}
						/>
					{:else}
						<Skeleton label="Loading version preview" count={4} />
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
		font: 600 var(--t-label) var(--font-ui);
		font-variant-numeric: tabular-nums;
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
	.inspector-head h2 {
		margin: 0;
	}
	.name-version {
		display: flex;
		gap: var(--space-2);
		padding: var(--space-3) var(--space-4) 0;
	}
	.name-version input {
		flex: 1;
		min-width: 0;
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
	.others-editing {
		color: var(--ink);
		font-weight: 500;
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
		font: 500 var(--t-label) var(--font-ui);
		font-variant-numeric: tabular-nums;
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
	.comment-meta {
		display: inline-flex;
		align-items: center;
		gap: 2px;
	}
	/* The React control waits beside the time until the comment is reached for. */
	.comment-meta :global(.reaction-add summary) {
		width: 24px;
		height: 24px;
		margin-block: -4px;
	}
	.comment-meta :global(.reaction-add) {
		opacity: 0;
		transition: opacity var(--motion-state) var(--ease-standard);
	}
	.comment-row:hover .comment-meta :global(.reaction-add),
	.comment-row:focus-within .comment-meta :global(.reaction-add),
	.comment-meta :global(.reaction-add:has(details[open])) {
		opacity: 1;
	}
	@media (hover: none) {
		.comment-meta :global(.reaction-add) {
			opacity: 1;
		}
	}
	.comment-row time {
		color: var(--text-tertiary);
		font: var(--t-label) var(--font-ui);
		font-variant-numeric: tabular-nums;
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
	.field select,
	.field input {
		width: 100%;
		padding: 8px 9px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		color: var(--ink);
		font: var(--t-body) / 1.5 var(--font-ui);
		resize: vertical;
	}
	.comment-composer .btn,
	.review-request .btn {
		justify-self: end;
	}
	/* A quoted passage reads as the passage: the document's own type, quieter. */
	q {
		display: -webkit-box;
		overflow: hidden;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		font-weight: 500;
		font-style: italic;
		quotes: '“' '”';
	}
	.thread-index q {
		-webkit-line-clamp: 1;
		line-clamp: 1;
	}
	.thread-actions {
		display: flex;
		flex: none;
		gap: 6px;
	}
	.mention-field {
		position: relative;
	}
	.mention-menu {
		position: absolute;
		bottom: calc(100% + 4px);
		left: 0;
		z-index: 4;
		min-width: 200px;
		max-width: 100%;
		margin: 0;
		padding: 4px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		box-shadow: 0 8px 24px #15243b20;
		list-style: none;
	}
	.mention-menu button {
		display: block;
		width: 100%;
		padding: 7px 9px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--ink);
		font: var(--t-body) var(--font-ui);
		text-align: left;
		cursor: pointer;
	}
	.mention-menu li.active button,
	.mention-menu button:hover {
		background: var(--surface-alt);
	}
	.review-actions {
		display: flex;
		flex-wrap: wrap;
		justify-content: flex-end;
		gap: var(--space-2);
	}
	.review-request .review-actions .btn {
		justify-self: auto;
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
		font: var(--t-label) var(--font-ui);
		font-variant-numeric: tabular-nums;
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
	/* The version as a page: the saved body, read only, in the panel's width. */
	.version-page {
		max-height: 420px;
		padding: 10px 12px;
		overflow: auto;
		color: var(--ink);
		font: var(--t-body) / 1.55 var(--font-ui);
		overflow-wrap: anywhere;
	}
	.version-page :global(:is(h1, h2, h3, h4, h5, h6)) {
		margin: 0.9em 0 0.35em;
		font-size: var(--t-body);
		font-weight: 600;
	}
	.version-page :global(> :first-child) {
		margin-top: 0;
	}
	.version-page :global(p) {
		margin: 0 0 0.6em;
	}
	.version-page :global(:is(ul, ol)) {
		margin: 0 0 0.6em;
		padding-left: 1.3em;
	}
	.version-page :global(img) {
		max-width: 100%;
		height: auto;
	}
	.version-page :global(table) {
		border-collapse: collapse;
	}
	.version-page :global(:is(th, td)) {
		padding: 3px 6px;
		border: 1px solid var(--border);
	}
	/* What a version changed: added words in a quiet green, removed ones struck through. */
	.compare-with {
		display: flex;
		align-items: center;
		gap: 6px;
		margin: 0;
		padding: 8px 10px 0;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.compare-with select {
		min-width: 0;
		max-width: 100%;
		padding: 2px 6px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		color: var(--ink);
		font: inherit;
	}
	.diff-scope {
		margin: 0;
		padding: 8px 10px 0;
		color: var(--text-tertiary);
		font: var(--t-label) var(--font-ui);
	}
	.version-diff {
		display: grid;
		gap: 6px;
		max-height: 320px;
		margin: 0;
		padding: 8px 10px 10px;
		overflow: auto;
		list-style: none;
		color: var(--text-secondary);
		font: var(--t-body) / 1.55 var(--font-ui);
		white-space: pre-wrap;
	}
	.version-diff li {
		padding-left: 8px;
		border-left: 2px solid var(--border);
	}
	.version-diff li.diff-added {
		border-left-color: var(--state-success, var(--surface-work));
	}
	.version-diff li.diff-removed {
		border-left-color: var(--state-danger);
	}
	.version-diff ins {
		background: color-mix(in srgb, var(--surface-work) 16%, transparent);
		color: var(--ink);
		text-decoration: none;
	}
	.version-diff del {
		color: var(--text-tertiary);
		text-decoration: line-through;
		text-decoration-color: var(--state-danger);
	}
	.version-line article.fresh .version-node {
		background: var(--surface-attention);
	}
	/* Open or Resolved, once anything is resolved. */
	.thread-filter {
		display: inline-flex;
		gap: 2px;
	}
	.thread-filter button {
		padding: 3px 8px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-tertiary);
		font: 600 var(--t-body) var(--font-ui);
		cursor: pointer;
	}
	.thread-filter button[aria-pressed='true'] {
		background: var(--wash-hover);
		color: var(--ink);
	}
	/* The comment someone came here for, marked for a moment. */
	.comment-row.flash {
		animation: comment-flash 2.4s var(--ease-standard);
	}
	@keyframes comment-flash {
		0%,
		50% {
			background: color-mix(in srgb, var(--highlight) 55%, transparent);
		}
		100% {
			background: transparent;
		}
	}
	.comment-meta :global(.action-menu > summary) {
		width: 24px;
		height: 24px;
		margin-block: -4px;
	}
	.comment-edit {
		display: grid;
		gap: 6px;
		margin-top: 6px;
	}
	.comment-edit textarea {
		width: 100%;
		padding: 7px 9px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		color: var(--ink);
		font: var(--t-body) / 1.5 var(--font-ui);
		resize: vertical;
	}
	.edit-actions,
	.confirm-delete {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 6px;
	}
	.confirm-delete {
		margin-top: 8px;
		color: var(--text-secondary);
	}
	.confirm-delete span {
		margin-right: auto;
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
	.inspector-dismiss .icon-only {
		padding: 4px;
	}
</style>
