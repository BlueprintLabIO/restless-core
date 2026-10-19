<script lang="ts">
	/* One Inbox item, read as a message from whoever is asking and answered
	 * from one action bar at the bottom.
	 *
	 * A reply goes to the Work's owner as a message in the owner's conversation
	 * with them (the handoff decision endpoint posts it), so the owner is told
	 * where it went, even when a lead asked. Skip is one click with Undo: the
	 * company carries on without it. Approvals keep hold-to-confirm, with the exact
	 * draft shown under the ask; declining one, or asking for changes, can carry
	 * a note that reaches the company with the decision. Files the item points
	 * at open alongside the decision in Inbox. Context and the rest of the evidence are folded
	 * away until asked for. */
	import { page } from '$app/state';
	import { inboxDiscussionHref } from '$lib/model/inbox-preview';
	import { decideEmailMandateProposal } from '$lib/model/company-actions';
	import { decidePayee } from '$lib/model/company';
	import { authorizePublication, stopPublication } from '$lib/model/publications';
	import { failureSentence } from '$lib/model/failure';
	import { tick, untrack, type Snippet } from 'svelte';
	import AttentionFooter from './AttentionFooter.svelte';
	import AccountUseNote from './AccountUseNote.svelte';
	import { grantSummary, kindPhrase } from '$lib/model/owner-accounts';
	import { answerAccountCode, ownerAccounts } from '$lib/model/owner-accounts-client';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import type { AttentionItem } from '$lib/model/view';
	import {
		reviewAction,
		approvalAction,
		delegateHandoff,
		passHandoffToOwner,
		resolveHandoffDecision
	} from '$lib/model/attention';
	import {
		cockpitQuery,
		refreshAttention,
		removeConfirmedAttention
	} from '$lib/model/queries.svelte';
	import {
		attentionAsker,
		attentionKindLabel,
		attentionTitle,
		deadlineAt
	} from '$lib/model/attention-presentation';
	import { getInboxMarks } from '$lib/model/for-you';
	import SnoozeChoices from './SnoozeChoices.svelte';
	import InlineAppRequest from './inbox/InlineAppRequest.svelte';
	import InlineAccountRequest from './inbox/InlineAccountRequest.svelte';
	import InlineConversation from './inbox/InlineConversation.svelte';
	import InlineModel from './inbox/InlineModel.svelte';
	import InlineSpendLimit from './inbox/InlineSpendLimit.svelte';
	import InlineDesktop from './inbox/InlineDesktop.svelte';
	import { provideCardActions } from './inbox/card-actions.svelte';
	import InlineBrandDrift from './inbox/InlineBrandDrift.svelte';
	import InlineMachine from './inbox/InlineMachine.svelte';
	import InlineReconnect from './inbox/InlineReconnect.svelte';
	import InlineStartComputer from './inbox/InlineStartComputer.svelte';
	import InlineSettingProposal from './inbox/InlineSettingProposal.svelte';
	import {
		discussInbox,
		appRequestHandoffs,
		copyInboxLink,
		inboxMarksKey,
		markDoneSoon,
		setAsideInboxItem,
		skipHandoffSoon,
		skipsByHandoff,
		snoozeInboxItem,
		wakeInboxItem
	} from './inbox-actions';
	import HoldApprove from '$lib/ui/controls/HoldApprove.svelte';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import Markdown from '$lib/primitives/Markdown.svelte';
	import { Fold, Notice } from '$lib/ui/page';
	import { formatMoment } from '$lib/ui/time';
	import { initials } from '$lib/model/initials';
	import { companyFileLinks, type CompanyFileLink } from '$lib/model/inbox-files';
	import type { LibraryArtifact } from '$lib/model/library-files';

	let {
		companyId,
		item,
		artifacts = [],
		content,
		preview,
		declining: startDeclining = false,
		workOwnerId,
		canSetAside = true,
		canPassToOwner = false,
		onopenDocument,
		onresolved,
		onleave
	}: {
		companyId: string;
		item: AttentionItem;
		content?: Snippet;
		preview?: Snippet;
		/** The Work graph's artifact references, so a file the item names opens in the Library. */
		artifacts?: readonly LibraryArtifact[];
		/** Open an approval straight into its decline, as the list's quick action does. */
		declining?: boolean;
		/** Who owns the item's Work: a reply or decline is delivered to them. */
		workOwnerId?: string;
		onopenDocument?: () => Promise<unknown>;
		/** Where the answer went, for the page to confirm after the item leaves. */
		onresolved?: (sent: { text: string; href?: string }) => void;
		/** Skipping what is not a request owed to this person is the owner's: it answers for the
		 * whole company queue. */
		canSetAside?: boolean;
		/** A member's own item: a judgement they should not make can go up to the owner. */
		canPassToOwner?: boolean;
		/** The item was snoozed or skipped here: the page moves on to the next one. */
		onleave?: () => void;
	} = $props();

	const client = useQueryClient();
	const discussionHref = $derived(inboxDiscussionHref(page.url, companyId, item.id));
	/* Working something through with the agent happens in the card, not on another page (S101). */
	let discussing = $state(false);
	const asker = $derived(attentionAsker(item));
	const askerId = $derived(item.responsibleActor?.id ?? item.briefAuthor?.id ?? 'exec');
	const conversationHref = $derived(
		`/${encodeURIComponent(companyId)}/people?person=${encodeURIComponent(askerId)}&view=people`
	);
	/* A decision answer reaches the Work's owner, who may not be whoever wrote the brief. */
	const cockpit = cockpitQuery(() => companyId);
	const recipientId = $derived(workOwnerId || askerId);
	const recipient = $derived(
		recipientId === askerId
			? asker
			: (cockpit.view?.people.find((person) => person.actor_id === recipientId)?.display ??
					recipientId.replaceAll('-', ' ').replace(/\b\w/g, (letter) => letter.toUpperCase()))
	);
	const recipientHref = $derived(
		`/${encodeURIComponent(companyId)}/people?person=${encodeURIComponent(recipientId)}&view=people`
	);
	/* An approval answer is announced to Exec, whoever prepared the draft. */
	const execHref = $derived(`/${encodeURIComponent(companyId)}/people?person=exec&view=people`);
	/* What the card's inline panel chose, done from the footer like every other answer. */
	const cardSlot = provideCardActions();
	const approval = $derived(item.category === 'approval');
	/* The exact thing an approval asks about (a command's draft, the build and limits put online,
	 * a payment's amount and payee), shown under the ask rather than folded away. */
	const EXACT_KINDS = ['command', 'authority-publication', 'authority-payment'];
	const drafts = $derived(
		approval || item.evidence.some((entry) => entry.kind === 'authority-payment')
			? item.evidence.filter((entry) => EXACT_KINDS.includes(entry.kind) && entry.content)
			: []
	);
	/* Files the item points at (a receipt, an invoice, a draft's attachment), each opening in the
	 * Library viewer. Other evidence stays in the fold. */
	const fileLink = $derived(companyFileLinks(companyId, artifacts));
	const files = $derived(
		item.evidence
			.filter((entry) => entry.kind === 'artifact' && entry.content)
			.map((entry) => fileLink(entry.content ?? '', entry.label))
			.filter(
				(link, index, all): link is CompanyFileLink =>
					!!link && all.findIndex((other) => other?.href === link.href) === index
			)
	);
	const folded = $derived(
		item.evidence.filter(
			(entry) =>
				!drafts.includes(entry) &&
				!(entry.kind === 'artifact' && entry.content && fileLink(entry.content))
		)
	);
	const base = $derived(`/${encodeURIComponent(companyId)}?item=${encodeURIComponent(item.id)}`);
	const action = (id: string) => item.actions.find((candidate) => candidate.id === id);
	const grant = $derived(action('grant'));
	/* Permission at first use: approve this call and let the tool act from now on. The owner's
	 * alone: an away deputy answers one call at a time. */
	const grantAlways = $derived(canSetAside ? action('grant-always') : undefined);
	const decline = $derived(action('decline'));
	const approveMandate = $derived(action('approve-email-mandate'));
	const declineMandate = $derived(action('decline-email-mandate'));
	const approvePayee = $derived(action('approve-payee'));
	const declinePayee = $derived(action('decline-payee'));
	const authorizeOnline = $derived(action('authorize-publication'));
	const declineOnline = $derived(action('decline-publication'));
	const record = $derived(action('record-decision'));
	/* Accounts (S98-09/10): a notice is answered Fine or Freeze, an admin's request Approve, Approve
	 * with limits (Settings, with the grant open) or Deny. The reference is the notice or request. */
	const accountNotice = $derived(item.source.kind === 'account_notice' && !!item.accountUse);
	const noticeFine = $derived(accountNotice ? action('fine') : undefined);
	const noticeFreeze = $derived(accountNotice ? action('freeze') : undefined);
	const accountAsk = $derived(item.source.kind === 'account_request');
	const requestApprove = $derived(accountAsk ? action('approve') : undefined);
	const requestLimits = $derived(accountAsk ? action('approve-with-limits') : undefined);
	const requestDeny = $derived(accountAsk ? action('deny') : undefined);
	const open = $derived(action('open-outcome'));
	const review = $derived(item.actions.some((candidate) => candidate.id === 'accept-review'));
	let requestingReviewChanges = $state(false),
		reviewFeedback = $state('');
	let reviewNote = $state<HTMLTextAreaElement>();
	async function showReviewChanges() {
		requestingReviewChanges = true;
		await tick();
		reviewNote?.focus();
	}
	function decideReview(decision: 'accept' | 'request_changes') {
		if (decision === 'request_changes' && !reviewFeedback.trim()) return;
		return run(
			async () => {
				await reviewAction(companyId, item.source.reference, decision, reviewFeedback.trim());
				await refreshAttention(client, companyId);
			},
			decision === 'accept' ? 'Accepted' : 'Changes sent to ' + recipient,
			'The review decision was not recorded.',
			recipientHref
		);
	}
	const humanStep = $derived(action('complete-human-step'));
	const discuss = $derived(item.actions.some((candidate) => candidate.id === 'chat-lead'));
	const documentRequest = $derived(
		item.source.kind === 'document_collaboration' || item.source.kind === 'document_review'
	);
	/* Cards whose task the card itself does (S101): their links out are not shown. */
	const SITE_ASK =
		/company browser|shared desktop|https?:\/\/|\b[a-z0-9-]+\.(?:com|net|org|io|co|au|nz|uk)\b|\b(?:sign|log) in\b|\bregister\b|\bpay\b|\bcheckout\b/i;
	const MODEL_KINDS = ['model_cooldown', 'account_headroom', 'exec_startup_blocked', 'agent_model_unavailable'];
	const SPEND_KINDS = ['spend_warning', 'spend_stopped'];
	const modelSurface = $derived(
		MODEL_KINDS.includes(item.source.kind) || !!action('open-intelligence-provider')
	);
	const spendSurface = $derived(SPEND_KINDS.includes(item.source.kind));
	const driftSurface = $derived(item.source.kind === 'identity_drift');
	const machineSurface = $derived(item.source.kind === 'machine_offline');
	const reconnectSurface = $derived(item.source.kind === 'connection_sign_in');
	const startSurface = $derived(
		item.source.kind === 'overdue_dispatch_blocked' && !!action('open-company-health')
	);
	/* An outside action that failed or went unanswered: the answer goes to whoever ran it. */
	const outcomeSurface = $derived(
		item.source.kind === 'effect_outcome' || item.source.kind === 'payment_outcome'
	);
	const outcomeAnswers = $derived(
		item.uncertainty
			? ['It happened, carry on', 'It did not happen, retry it', 'Drop it']
			: ['Fix it and retry', 'Drop it']
	);
	const navigation = $derived(
		modelSurface || spendSurface || driftSurface || machineSurface || reconnectSurface || startSurface
			? []
			: item.actions.filter(
					(candidate) =>
						candidate.href &&
						candidate.role !== 'conversation' &&
						!(candidate.id === 'open-external-human-step' && canSetAside && !item.preparing)
				)
	);
	/* A refused model offers two ways out (Top up, or another account the owner already holds);
	 * both are buttons, since the free one is often the right one. */
	const inlineNavigation = $derived(item.source.kind === 'model_cooldown' ? 2 : 1);
	/* A provider approval opens in the owner's own browser: the company browser would leave
	 * its approver session with the agents that drive it. */
	const ownBrowser = (candidate: { role: string }) =>
		candidate.role === 'owner_browser' ? '' : undefined;
	/* An app request (Sprint 63): a sign-in handoff that names the app to add.
	 * Adding and allowing it is the resume condition, so there is no "Mark done".
	 * Apps are the owner's, so only the owner's detail reads them; a failed read is said rather
	 * than falling back to a Mark done that would not resume anything. */
	const appRequests = appRequestHandoffs(
		() => companyId,
		() => canSetAside && item.source.kind === 'owner_handoff'
	);
	const appRequest = $derived(
		item.source.kind === 'owner_handoff'
			? (appRequests.data?.find((row) => row.handoff_id === item.source.reference) ?? null)
			: null
	);
	const appRequestsFailed = $derived(
		canSetAside && !!humanStep && !appRequests.data && !!appRequests.error
	);
	/* Older daemons put a verification link only in the instructions. */
	const instructionLink = $derived.by(() => {
		if (item.preparing || item.category !== 'human_step' || item.actions.some((a) => a.href))
			return undefined;
		const match = item.requestedAction.match(/https?:\/\/[^\s<>"']+/);
		if (!match) return undefined;
		try {
			const url = new URL(match[0].replace(/[).,;!?]+$/, ''));
			if (['localhost', '127.0.0.1', '[::1]'].includes(url.hostname)) return undefined;
			return { href: url.href, label: `Open ${url.hostname}` };
		} catch {
			return undefined;
		}
	});
	/* A step done on the company computer happens on the live desktop inside the card (S101):
	 * a prepared computer connects by itself, a provider page opens there when asked. */
	const stepUrl = $derived(
		action('open-external-human-step')?.href ?? (humanStep ? (instructionLink?.href ?? null) : null)
	);
	const desktopSurface = $derived(
		canSetAside &&
			!appRequest &&
			!item.preparing &&
			((!!item.runtimeAttach && item.category !== 'review') || !!stepUrl)
	);
	/* An agent's question about doing something on a site (register a domain, sign in, pay) shows
	 * the company computer too, at the page the agent left open, so it is done in the card. */
	const siteAsk = $derived(
		item.source.kind === 'conversation_owner_need' &&
			canSetAside &&
			SITE_ASK.test(`${item.requestedAction} ${item.whatHappened}`)
	);
	/* A live outcome to review renders in the card's content beside the decision. */
	const outcomeInline = $derived(item.category === 'review' && !!item.reviewTarget);
	const showRecommendation = $derived(
		!!item.recommendation.trim() &&
			item.recommendation.trim() !== item.whatHappened.trim() &&
			item.recommendation.trim() !== item.whyItMatters.trim() &&
			item.recommendation.trim() !== item.requestedAction.trim()
	);
	const fields = $derived(item.requestFields ?? []);
	const optionsFor = (field: { type: string; options?: string[] }) =>
		field.type === 'boolean' ? ['Yes', 'No'] : (field.options ?? []);

	let acting = $state(false);
	let error = $state('');
	let reply = $state('');
	let reason = $state('');
	/* An approval answered with words: a plain decline (the note is optional) or a request for a
	 * changed draft (the note says what should change). */
	let approvalNote = $state<'' | 'decline' | 'changes'>('');
	let fieldValues = $state<Record<string, string>>({});
	let attempt = $state(0);
	/* Only the arrival opens straight into a decline: a poll that refreshes the item must not
	 * reopen a closed box, or turn a request for changes back into a plain decline. */
	$effect(() => {
		const start = startDeclining;
		void item.id;
		untrack(() => {
			if (start && grant && decline) approvalNote = 'decline';
		});
	});

	const replyReady = $derived(
		fields.length
			? fields.every((field) => !field.required || (fieldValues[field.id] ?? '').trim())
			: !!reply.trim()
	);
	const answer = () =>
		fields.length
			? [
					...fields.map((field) => `${field.label}: ${fieldValues[field.id] ?? ''}`),
					...(reply.trim() ? [reply.trim()] : [])
				].join('\n')
			: reply.trim();

	async function run(
		work: () => Promise<unknown>,
		sent: string,
		fallback: string,
		href = conversationHref
	) {
		if (acting) return;
		acting = true;
		error = '';
		try {
			await work();
			onresolved?.({ text: sent, href });
		} catch (cause) {
			error = failureSentence(cause, fallback);
		} finally {
			acting = false;
			attempt += 1;
		}
	}

	/* Skip, as the list's row menu does. A request the company waits on closes after a few
	 * seconds of Undo and wakes whoever owns the Work, who judges whether it changes anything.
	 * Anything else stays off the Inbox under Skipped, restorable, until it is raised again. */
	const byHandoff = $derived(skipsByHandoff(item));
	const skipWho = $derived(byHandoff ? recipient : asker);
	async function skip() {
		if (acting) return;
		if (byHandoff) {
			onleave?.();
			await skipHandoffSoon(client, companyId, item.source.reference, skipWho);
			return;
		}
		acting = true;
		try {
			if (await setAsideInboxItem(client, companyId, item.id, skipWho)) onleave?.();
		} finally {
			acting = false;
		}
	}
	/* Snooze and wake from the item itself, as the row menu does: on a phone the list is hidden. */
	const marks = createQuery(() => ({
		queryKey: inboxMarksKey(companyId),
		queryFn: () => getInboxMarks(companyId),
		enabled: Boolean(companyId),
		staleTime: 10_000
	}));
	const snoozed = $derived.by(() => {
		const until = marks.data?.find((mark) => mark.item_id === item.id)?.snoozed_until;
		return !!until && Date.parse(until) > Date.now();
	});
	function snooze(until: Date) {
		snoozeInboxItem(client, companyId, item.id, until);
		onleave?.();
	}
	const due = $derived(deadlineAt(item));
	/* Hand to…: an owner handoff can become a member's or admin's own Inbox item. */
	const delegates = $derived(
		canSetAside && item.source.kind === 'owner_handoff' && !item.preparing
			? (cockpit.view?.people ?? []).filter(
					(person) => person.kind === 'human' && person.role !== 'owner'
				)
			: []
	);
	let handingTo = $state<{ actor_id: string; display: string } | null>(null);
	let handNote = $state('');
	$effect(() => {
		void item.id;
		untrack(() => {
			handingTo = null;
			handNote = '';
		});
	});
	/* Pass to owner: a member's item goes up with the reason, which the owner reads with it. */
	const passable = $derived(
		canPassToOwner && item.source.kind === 'owner_handoff' && !item.preparing
	);
	let passing = $state(false);
	$effect(() => {
		void item.id;
		untrack(() => (passing = false));
	});
	const passUp = () =>
		reason.trim() &&
		run(
			async () => {
				await passHandoffToOwner(companyId, item.source.reference, reason.trim());
				await refreshAttention(client, companyId);
			},
			'Passed on with your reason. It has left your Inbox.',
			'That was not passed on. Try again.'
		);
	const handOn = () => {
		const person = handingTo;
		if (!person) return;
		return run(
			async () => {
				await delegateHandoff(companyId, item.source.reference, person.actor_id, handNote);
				await refreshAttention(client, companyId);
			},
			`Handed to ${person.display}. It is in their Inbox now.`,
			'That was not handed on. Try again.',
			`/${encodeURIComponent(companyId)}/people?person=${encodeURIComponent(person.actor_id)}&view=people`
		);
	};
	const sendReply = () =>
		replyReady &&
		run(
			async () => {
				/* A sign-in code goes straight to the waiting page and is never stored, so it does not
				 * travel as an ordinary reply (S98-11). */
				if (item.source.kind === 'account_code')
					await answerAccountCode(companyId, item.source.reference, fieldValues.code ?? '');
				else await resolveHandoffDecision(companyId, item.source.reference, answer());
				await refreshAttention(client, companyId);
			},
			`Sent to ${recipient}`,
			'Your answer was not sent. Try again.',
			recipientHref
		);
	/* Mark done resumes the Work, so it waits out a few seconds of Undo first. */
	let markingDone = $state(false);
	/* A task finished inside the card (S101): the item leaves once the server agrees, and the page
	 * shows the receipt and moves on. */
	async function completedInline(receipt: string) {
		await refreshAttention(client, companyId);
		onresolved?.({ text: receipt, href: conversationHref });
	}

	async function markDone() {
		if (acting || markingDone) return;
		markingDone = true;
		try {
			if (await markDoneSoon(client, companyId, item.source.reference))
				onresolved?.({ text: 'Marked done. The work resumes.', href: conversationHref });
		} finally {
			markingDone = false;
		}
	}
	const approvalTarget = (always = false) => {
		const target = item.source.call_key
			? { call_key: item.source.call_key, ...(always ? { always: true } : {}) }
			: item.source.party;
		if (!target) throw new Error('This request has no approval target. Refresh and try again.');
		return target;
	};
	const approve = (always = false) =>
		run(
			async () => {
				await approvalAction(companyId, 'grant', approvalTarget(always));
				await removeConfirmedAttention(client, companyId, item.id);
			},
			always ? 'Approved. It won’t ask again.' : 'Approved',
			'The approval was not recorded. Try again.',
			execHref
		);
	/* Declined either way: a request for changes tells the company to prepare a changed draft,
	 * which comes back here; a plain decline tells it not to go ahead. */
	const sendApprovalNote = () => {
		if (declineMandate && !grant) return decideMandate('decline', reason);
		if (declinePayee) return answerPayee(false, reason);
		const changes = approvalNote === 'changes';
		if (changes && !reason.trim()) return;
		return run(
			async () => {
				await approvalAction(companyId, 'decline', approvalTarget(), {
					note: reason,
					changesRequested: changes
				});
				await removeConfirmedAttention(client, companyId, item.id);
			},
			changes
				? 'Changes sent to Exec'
				: reason.trim()
					? 'Declined. Exec has your note.'
					: 'Declined',
			'That was not recorded. Try again.',
			execHref
		);
	};
	const decideMandate = (decision: 'approve' | 'decline', note = '') =>
		run(
			async () => {
				await decideEmailMandateProposal(companyId, item.source.reference, decision, note);
				await refreshAttention(client, companyId);
			},
			decision === 'approve'
				? 'Mandate approved'
				: note.trim()
					? 'Mandate declined. Exec has your note.'
					: 'Mandate declined',
			'The mandate decision was not recorded. Try again.'
		);
	const answerPayee = (approve: boolean, note = '') =>
		run(
			async () => {
				await decidePayee(companyId, item.source.reference, approve, note);
				await refreshAttention(client, companyId);
			},
			approve ? 'Payee approved' : note.trim() ? 'Declined. Exec has your note.' : 'Declined',
			'The payee decision was not recorded. Try again.',
			execHref
		);
	const answerNotice = (decision: 'fine' | 'freeze') =>
		run(
			async () => {
				await ownerAccounts.decideNotice(
					item.accountUse!.account.id,
					item.source.reference,
					decision
				);
				await refreshAttention(client, companyId);
			},
			decision === 'fine'
				? 'Fine. The company keeps using it.'
				: 'Frozen. This company can’t use the account until you unfreeze it.',
			'That was not recorded. Try again.'
		);
	const answerRequest = (decision: 'approved' | 'denied') =>
		run(
			async () => {
				await ownerAccounts.settleRequest(item.source.reference, { decision });
				await refreshAttention(client, companyId);
			},
			decision === 'approved'
				? 'Approved. The company can use it now.'
				: 'Denied. The company is told.',
			'That was not recorded. Try again.'
		);
	/* A prepared publication: the owner's yes puts the exact build online; no stops it. */
	const decidePublication = (authorize: boolean) =>
		run(
			async () => {
				if (authorize) await authorizePublication(companyId, item.source.reference);
				else
					await stopPublication(
						companyId,
						item.source.reference,
						'Declined by the owner from the Inbox'
					);
				await refreshAttention(client, companyId);
			},
			authorize ? 'Authorised. It goes online shortly.' : 'Declined. Nothing went online.',
			'That was not recorded. Try again.'
		);
	async function openDocument() {
		if (acting || !onopenDocument) return;
		acting = true;
		error = '';
		try {
			await onopenDocument();
		} catch {
			error = 'Could not open the document. Try again.';
		} finally {
			acting = false;
		}
	}
	async function copyReference() {
		try {
			await navigator.clipboard.writeText(
				`${item.source.plane}:${item.source.kind}:${item.source.reference}`
			);
		} catch {
			/* The reference stays visible in Evidence. */
		}
	}
	function keys(event: KeyboardEvent, submit: () => unknown) {
		if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
			event.preventDefault();
			void submit();
		}
		if (event.key === 'Escape') (event.currentTarget as HTMLElement).blur();
	}
	const initial = (name: string) => initials(name) || '?';
</script>

{#snippet secondaryActions()}
	{#if byHandoff || canSetAside}<button
			type="button"
			class="btn ghost small dismiss"
			disabled={acting || markingDone}
			title={`${skipWho} carries on without it. You can undo.`}
			onclick={() => void skip()}>Skip</button
		>{/if}
	<ActionMenu label="More">
		{#if discuss}<a href={discussionHref} onclick={(event) => void discussInbox(event)}>Discuss</a
			>{/if}
		<a href={conversationHref}>Open conversation with {asker}</a>
		{#if item.workId}<a
				href={`/${encodeURIComponent(companyId)}/work/${encodeURIComponent(item.workId)}`}
				>Open the work</a
			>{/if}
		{#each navigation.slice(cardSlot.primary || cardSlot.secondary.length ? 0 : inlineNavigation) as extra (extra.id)}<a
				href={extra.href}
				target={extra.href?.startsWith('/') ? undefined : '_blank'}
				rel="noreferrer"
				data-open-externally={ownBrowser(extra)}
				title={`${extra.consequence} ${extra.nextState}`}>{extra.label}</a
			>{/each}
		{#if snoozed}<button onclick={() => void wakeInboxItem(client, companyId, item.id)}
				>Wake now</button
			>{:else}<SnoozeChoices onpick={snooze} />{/if}
		{#each delegates as person (person.actor_id)}<button
				title={`Hand this to ${person.display}: it leaves your Inbox and becomes theirs. They can pass it back.`}
				onclick={() => (handingTo = person)}>Hand to {person.display}…</button
			>{/each}
		{#if passable}<button
				title="Not yours to decide: it goes up with your reason and leaves your Inbox. One handed to you goes back to the owner."
				onclick={() => (passing = true)}>Pass to owner…</button
			>{/if}
		<button onclick={() => void copyInboxLink(companyId, item.id)}>Copy link</button>
		<button onclick={copyReference}>Copy reference</button>
	</ActionMenu>
{/snippet}

<article class="inbox-detail" aria-label={attentionTitle(item)}>
	<div class="detail-content" class:has-preview={!!(preview || content)}>
		<header class="head">
			<div class="title">
				<h1>{attentionTitle(item)}</h1>
				<p class="meta">
					<span title={attentionKindLabel(item)}>{attentionKindLabel(item)}</span>
					<span aria-hidden="true">·</span>
					<span>{asker}</span>
					<span aria-hidden="true">·</span>
					<RelativeTime value={item.createdAt} />
					{#if item.deadline}<span aria-hidden="true">·</span><span class="deadline"
							>Needed {#if due !== null}<RelativeTime value={due} />{:else}by {item.deadline}{/if}</span
						>{/if}
					{#if !item.preparing}<span aria-hidden="true">·</span><span
							class="continues"
							class:held={!item.canContinue}
							title={item.ifNoAction ? `If nothing happens: ${item.ifNoAction}` : undefined}
							>{item.canContinue
								? 'Work keeps going meanwhile'
								: 'Work is paused until you answer'}</span
						>{/if}
				</p>
			</div>
		</header>

		{#if preview}<div class="detail-preview">{@render preview()}</div>{/if}
		{#if content}<div class="detail-preview" hidden={!!preview}>{@render content()}</div>
		{:else}
			<div class="body" hidden={!!preview}>
				<div class="message">
					<span class="avatar" aria-hidden="true">{initial(asker)}</span>
					<div class="message-body">
						<div class="message-head">
							<strong>{asker}</strong>
							{#if item.responsibleActor?.role}<span>{item.responsibleActor.role}</span>{/if}
						</div>
						{#if item.preparing}
							<p class="waiting">
								Nothing to do yet. This step is being prepared; instructions and any sign-in link
								will appear here when ready.
							</p>
						{:else}
							<div class="ask"><Markdown text={item.requestedAction} /></div>
						{/if}
						{#if item.accountUse}<AccountUseNote
								use={item.accountUse}
								owner={!canPassToOwner}
							/>{/if}
						{#if item.accountRequest && !canPassToOwner && !item.preparing}
							<!-- The owner's form below carries the kind and abilities; only the why stays. -->
							{@const why = item.accountRequest.reason
								.replace(/\s*\bsite:\s*\S+\s*/i, ' ')
								.trim()}
							{#if why}<p class="account-request"><q>{why}</q></p>{/if}
						{:else if item.accountRequest}
							<p class="account-request">
								<strong
									>{item.accountRequest.requested_by_display} asks for {item.accountRequest
										.account_id
										? 'more on an account'
										: kindPhrase(item.accountRequest.kind)}</strong
								>
								<span
									>{grantSummary({
										abilities: item.accountRequest.abilities,
										spend_cap: null
									})}</span
								>
								{#if item.accountRequest.reason}<q>{item.accountRequest.reason}</q>{/if}
							</p>
						{/if}
						{#each drafts as draft, index (index)}
							<pre
								class="draft"
								aria-label={draft.label}
								title={draft.kind === 'command'
									? 'The exact draft. Allowing it lets exactly this run.'
									: `${draft.label}: exactly what your answer covers.`}>{draft.content}</pre>
						{/each}
						{#if files.length}
							<div class="attach-list files">
								{#each item.workId ? files.slice(0, 6) : files as file (file.href)}
									<a
										class="attach-file"
										href={file.href}
										title={`Open in the Library · ${file.path}`}
									>
										<span class="a-kind">{file.ext}</span>
										<span class="af-main"><span class="a-name">{file.label}</span></span>
									</a>
								{/each}
								{#if files.length > 6 && item.workId}<a
										class="link"
										href={`/${encodeURIComponent(companyId)}/work/${encodeURIComponent(item.workId)}`}
										>All {files.length} files</a
									>{/if}
							</div>
						{/if}
						{#if item.uncertainty}<p class="uncertain" title="What the team is not sure about">
								<span>Not certain</span>{item.uncertainty}
							</p>{/if}
						<!-- One panel answers the card (S101); the footer acts on it. -->
						{#if appRequest && humanStep && !item.preparing}<InlineAppRequest
								{companyId}
								request={appRequest}
								owner={!canPassToOwner}
								ondone={(receipt) => void completedInline(receipt)}
							/>{/if}
						{#if item.source.kind === 'conversation_owner_need' && item.responsibleActor}<InlineConversation
								{companyId}
								actorId={item.responsibleActor.id}
								actorName={asker}
								question={item.requestedAction}
								workId={item.workId}
								ondone={(receipt) => void completedInline(receipt)}
							/>{/if}
						<!-- Discussing keeps the card's own decision in the footer; the talk is the People thread. -->
						{#if discussing && item.source.kind !== 'conversation_owner_need'}<InlineConversation
								{companyId}
								actorId={askerId}
								actorName={asker}
								workId={item.workId}
							/>{/if}
						{#if modelSurface}<InlineModel
								{companyId}
								owner={!canPassToOwner}
								topUpHref={action('top-up-ai-credit')?.href}
								ondone={(receipt) => void completedInline(receipt)}
							/>{/if}
						{#if driftSurface}<InlineBrandDrift
								{companyId}
								finding={item.source.reference}
								owner={!canPassToOwner}
								ondone={(receipt) => void completedInline(receipt)}
							/>{/if}
						{#if item.settingProposal}<InlineSettingProposal
								{companyId}
								proposal={item.settingProposal}
								asker={asker}
								owner={!canPassToOwner}
								ondone={(receipt) => void completedInline(receipt)}
							/>{/if}
						{#if startSurface}<InlineStartComputer
								{companyId}
								owner={!canPassToOwner}
								ondone={(receipt) => void completedInline(receipt)}
							/>{/if}
						{#if reconnectSurface}<InlineReconnect
								{companyId}
								name={item.source.reference}
								owner={!canPassToOwner}
								ondone={(receipt) => void completedInline(receipt)}
							/>{/if}
						{#if machineSurface}<InlineMachine
								{companyId}
								machineId={item.source.reference}
								owner={!canPassToOwner}
								ondone={(receipt) => void completedInline(receipt)}
							/>{/if}
						{#if outcomeSurface && canSetAside}<InlineConversation
								{companyId}
								actorId={item.responsibleActor?.id ?? 'exec'}
								actorName={item.responsibleActor?.display ?? 'Exec'}
								question={item.title}
								answers={outcomeAnswers}
								workId={item.workId}
								ondone={(receipt) =>
									void setAsideInboxItem(client, companyId, item.id).then(() =>
										completedInline(receipt)
									)}
							/>{/if}
						{#if siteAsk && !desktopSurface}{#key item.id}<InlineDesktop
									{companyId}
									itemId="runtime-rescue"
									attached={false}
								/>{/key}{/if}
						{#if desktopSurface}{#key item.id}<InlineDesktop
									{companyId}
									itemId={item.id}
									attached={!!item.runtimeAttach}
									url={stepUrl}
								/>{/key}{/if}
						{#if spendSurface}<InlineSpendLimit
								{companyId}
								owner={!canPassToOwner}
								ondone={(receipt) => void completedInline(receipt)}
							/>{/if}
						{#if item.accountRequest && !item.preparing}<InlineAccountRequest
								ask={item.accountRequest}
								owner={!canPassToOwner}
								ondone={(receipt) => void completedInline(receipt)}
							/>{/if}
					</div>
				</div>

				<div class="folds">
					{#if item.whatHappened || item.whyItMatters || showRecommendation}
						<Fold label="Context">
							<div class="prose">
								{#if item.whatHappened}<Markdown text={item.whatHappened} />{/if}
								{#if item.whyItMatters && item.whyItMatters !== item.whatHappened}<Markdown
										text={item.whyItMatters}
									/>{/if}
								{#if showRecommendation}<div class="recommended">
										<strong>Recommended</strong>
										<Markdown text={item.recommendation} />
									</div>{/if}
							</div>
						</Fold>
					{/if}
					<Fold label="Evidence" count={folded.length || null}>
						<div class="prose evidence">
							<p class="credit">
								Prepared by <strong>{item.briefAuthor?.display ?? asker}</strong
								>{#if item.briefedAt}
									· <time title={formatMoment(item.briefedAt)}
										><RelativeTime value={item.briefedAt} /></time
									>{/if}
							</p>
							{#each folded as evidence, index (`${evidence.kind}:${evidence.label}:${index}`)}
								{#if evidence.content}
									<div class="quote">
										<span>{evidence.label}</span>
										<blockquote>{evidence.content}</blockquote>
									</div>
								{:else if evidence.uri}
									<a class="link" href={evidence.uri} target="_blank" rel="noreferrer"
										>{evidence.label} <span aria-hidden="true">↗</span></a
									>
								{/if}
							{/each}
							{#if item.ifNoAction}<p class="credit" title="What happens if nobody acts">
									If nothing happens: {item.ifNoAction}
								</p>{/if}
						</div>
					</Fold>
				</div>
			</div>
		{/if}
	</div>
	{#snippet responseFields()}
		<div class="inbox-inputs">
			{#if error}<Notice tone="danger" title={error} />{/if}
			{#if requestingReviewChanges}
				<label
					>Changes for {recipient}<textarea
						bind:this={reviewNote}
						bind:value={reviewFeedback}
						rows="3"
						placeholder="Describe what should change…"
						onkeydown={(event) => {
							if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
								event.preventDefault();
								void decideReview('request_changes');
							}
						}}></textarea></label
				>
			{/if}
			{#if appRequestsFailed}<Notice
					tone="danger"
					title="Could not check whether this step is an app request. If it is, adding the app resumes it; Mark done would not."
				/>{/if}
			{#if approveMandate}
				<p
					class="note"
					title="Exec judges whether each recipient and message fits the mandate. The system enforces the mechanical limits."
				>
					Approving gives Exec bounded sending authority.
				</p>
			{/if}

			{#if passing}
				<textarea
					bind:value={reason}
					{@attach (input) => input.focus()}
					aria-label="Why the owner should decide"
					maxlength={2000}
					placeholder="Why should the owner decide this? They read it with the item."
					disabled={acting}
					onkeydown={(event) => keys(event, passUp)}></textarea>
			{:else if handingTo}
				<textarea
					bind:value={handNote}
					{@attach (input) => input.focus()}
					aria-label={`Note for ${handingTo.display}`}
					maxlength={2000}
					placeholder={`Anything ${handingTo.display} should know? Optional.`}
					disabled={acting}
					onkeydown={(event) => keys(event, handOn)}></textarea>
			{:else if approvalNote}
				<textarea
					bind:value={reason}
					{@attach (input) => input.focus()}
					aria-label={approvalNote === 'changes' ? 'What should change' : 'Why not'}
					maxlength={4000}
					placeholder={approvalNote === 'changes'
						? 'What should change? A new draft comes back here for you.'
						: 'Why not? Optional. Exec is told either way.'}
					disabled={acting}
					onkeydown={(event) => keys(event, sendApprovalNote)}></textarea>
			{:else if record}
				{#if fields.length}
					<div class="fields">
						{#each fields as field (field.id)}
							<label
								><span>{field.label}{field.required ? '' : ' (optional)'}</span>
								{#if field.type === 'choice' || field.type === 'boolean'}<select
										bind:value={fieldValues[field.id]}
										required={field.required}
										disabled={acting}
										><option value="">Choose…</option>{#each optionsFor(field) as option}<option
												value={option}>{option}</option
											>{/each}</select
									>{:else}<input
										type={field.type === 'date'
											? 'date'
											: field.type === 'amount'
												? 'number'
												: 'text'}
										step={field.type === 'amount' ? '0.01' : undefined}
										inputmode={field.type === 'code' ? 'numeric' : undefined}
										autocomplete={field.type === 'code' ? 'one-time-code' : undefined}
										maxlength={field.type === 'code' ? 12 : undefined}
										class:code={field.type === 'code'}
										bind:value={fieldValues[field.id]}
										required={field.required}
										disabled={acting}
									/>{/if}
							</label>
						{/each}
					</div>
				{/if}
				<textarea
					bind:value={reply}
					aria-label={`Reply to ${asker}`}
					placeholder={fields.length ? 'Anything else (optional)' : `Reply to ${asker}…`}
					disabled={acting}
					onkeydown={(event) => keys(event, sendReply)}></textarea>
			{/if}
		</div>
	{/snippet}
	<AttentionFooter
		fields={requestingReviewChanges ||
		error ||
		appRequestsFailed ||
		approveMandate ||
		passing ||
		handingTo ||
		approvalNote ||
		record
			? responseFields
			: undefined}
	>
		{#if passing}<div class="action-row">
				{@render secondaryActions()}
				<button class="btn primary small" disabled={acting || !reason.trim()} onclick={passUp}
					>{acting ? 'Passing on…' : 'Pass to owner'}</button
				>
				<button class="btn small ghost" disabled={acting} onclick={() => (passing = false)}
					>Back</button
				>
				<span class="hint">⌘ ↵</span>
			</div>
		{:else if handingTo}<div class="action-row">
				{@render secondaryActions()}
				<button class="btn primary small" disabled={acting} onclick={handOn}
					>{acting ? 'Handing on…' : `Hand to ${handingTo.display}`}</button
				>
				<button class="btn small ghost" disabled={acting} onclick={() => (handingTo = null)}
					>Back</button
				>
				<span class="hint">⌘ ↵</span>
			</div>
		{:else if approvalNote}<div class="action-row">
				{@render secondaryActions()}
				<button
					class="btn primary small"
					disabled={acting || (approvalNote === 'changes' && !reason.trim())}
					title={approvalNote === 'changes'
						? 'Declines this draft. The company prepares a changed one, which asks you again.'
						: `${decline?.consequence ?? ''} ${decline?.nextState ?? ''}`.trim()}
					onclick={sendApprovalNote}
					>{acting
						? 'Sending…'
						: approvalNote === 'changes'
							? 'Ask for changes'
							: 'Decline'}</button
				>
				<button class="btn small ghost" disabled={acting} onclick={() => (approvalNote = '')}
					>Back</button
				>
				<span class="hint" title="Goes to Exec with your decision">To Exec · ⌘ ↵</span>
			</div>
		{:else if record}<div class="action-row">
				{@render secondaryActions()}
				<button
					class="btn primary small"
					disabled={acting || !replyReady}
					title={replyReady
						? `${record.consequence} ${record.nextState}`
						: 'Write your answer first'}
					onclick={sendReply}>{acting ? 'Sending…' : 'Send'}</button
				>
				{#if discuss}<button
						class="btn small ghost"
						type="button"
						aria-expanded={discussing}
						onclick={() => (discussing = !discussing)}>Discuss</button
					>{/if}
				<span class="hint" title={`Goes to ${recipient} as a message in your conversation`}
					>To {recipient} · ⌘ ↵</span
				>
			</div>
		{:else}
			<div class="action-row">
				{@render secondaryActions()}
				{#if cardSlot.primary || cardSlot.secondary.length}
					{@const primary = cardSlot.primary}
					{#if humanStep && !appRequest}<button
							class="btn small"
							type="button"
							disabled={acting || markingDone || !!cardSlot.busy}
							title={`${humanStep.consequence} ${humanStep.nextState} Undo stays available for a few seconds.`}
							onclick={markDone}>{markingDone ? 'Marking done…' : 'Mark done'}</button
						>{/if}
					{#if requestDeny}<button
							class="btn small"
							disabled={acting || !!cardSlot.busy}
							title={`${requestDeny.consequence} ${requestDeny.nextState}`}
							onclick={() => void answerRequest('denied')}>{requestDeny.label}</button
						>{/if}
					{#each cardSlot.secondary as extra (extra.label)}<button
							class="btn small"
							type="button"
							disabled={!!cardSlot.busy || extra.disabled}
							title={extra.title}
							onclick={() => void cardSlot.invoke(extra)}
							>{cardSlot.busy === extra.label ? (extra.busyLabel ?? extra.label) : extra.label}</button
						>{/each}
					{#if !primary}
						<!-- Equal choices, none recommended over another. -->
					{:else if primary.hold}
						{#key `${item.id}:${primary.label}`}
							<HoldApprove
								small
								label={`Hold to ${primary.label.charAt(0).toLowerCase()}${primary.label.slice(1)}`}
								completeLabel={primary.busyLabel ?? 'Saving…'}
								disabled={!!cardSlot.busy || primary.disabled}
								title={primary.title ?? primary.label}
								onapprove={() => void cardSlot.invoke(primary)}
							/>
						{/key}
					{:else}<button
							class="btn small primary"
							type="button"
							disabled={!!cardSlot.busy || primary.disabled}
							title={primary.title}
							onclick={() => void cardSlot.invoke(primary)}
							>{cardSlot.busy === primary.label
								? (primary.busyLabel ?? primary.label)
								: primary.label}</button
						>{/if}
				{:else if review}
					{#if requestingReviewChanges}
						<button
							class="btn primary small"
							type="button"
							disabled={acting || !reviewFeedback.trim()}
							onclick={() => void decideReview('request_changes')}
							>{acting ? 'Sending…' : 'Send changes'}</button
						>
						<button
							class="btn small ghost"
							type="button"
							disabled={acting}
							onclick={() => (requestingReviewChanges = false)}>Back</button
						>
					{:else}
						<button
							class="btn primary small"
							type="button"
							disabled={acting}
							title={action('accept-review')?.consequence}
							onclick={() => void decideReview('accept')}
							>{acting ? 'Accepting…' : 'Accept outcome'}</button
						>
						<button
							class="btn small"
							type="button"
							disabled={acting}
							onclick={() => void showReviewChanges()}>Request changes</button
						>
					{/if}
				{:else if grant}
					{#key `${item.id}:${attempt}`}
						<HoldApprove
							small
							completeLabel="Saving approval…"
							label={`Hold to ${grant.label.toLowerCase()}`}
							disabled={acting}
							title={`${grant.consequence} ${grant.nextState}`}
							onapprove={() => void approve()}
						/>
					{/key}
					{#if grantAlways}
						{#key `${item.id}:${attempt}:always`}
							<HoldApprove
								small
								completeLabel="Saving approval…"
								armedLabel="press again to always allow"
								label={`Hold to ${grantAlways.label.toLowerCase()}`}
								disabled={acting}
								title={`${grantAlways.consequence} ${grantAlways.nextState} It will stop asking before this tool acts, until you revoke it.`}
								onapprove={() => void approve(true)}
							/>
						{/key}
					{/if}
					{#if decline}<button
							class="btn small"
							disabled={acting}
							title="Not like this: say what should change, and a new draft comes back for you."
							onclick={() => (approvalNote = 'changes')}>Ask for changes</button
						><button
							class="btn small"
							disabled={acting}
							title={`${decline.consequence} ${decline.nextState}`}
							onclick={() => (approvalNote = 'decline')}>{decline.label}</button
						>{/if}
				{:else if noticeFine || noticeFreeze}
					{#if noticeFine}<button
							class="btn small primary"
							disabled={acting}
							title={`${noticeFine.consequence} ${noticeFine.nextState}`}
							onclick={() => void answerNotice('fine')}>{noticeFine.label}</button
						>{/if}
					{#if noticeFreeze}<button
							class="btn small"
							disabled={acting}
							title={`${noticeFreeze.consequence} ${noticeFreeze.nextState}`}
							onclick={() => void answerNotice('freeze')}>{noticeFreeze.label}</button
						>{/if}
				{:else if requestApprove || requestLimits || requestDeny}
					{#if requestApprove}
						{#key `${item.id}:${attempt}`}
							<HoldApprove
								small
								completeLabel="Saving approval…"
								label="Hold to approve"
								disabled={acting}
								title={`${requestApprove.consequence} ${requestApprove.nextState}`}
								onapprove={() => void answerRequest('approved')}
							/>
						{/key}
					{/if}
					{#if requestDeny}<button
							class="btn small"
							disabled={acting}
							title={`${requestDeny.consequence} ${requestDeny.nextState}`}
							onclick={() => void answerRequest('denied')}>{requestDeny.label}</button
						>{/if}
				{:else if approveMandate}
					{#key `${item.id}:${attempt}`}
						<HoldApprove
							small
							completeLabel="Saving approval…"
							label="Hold to approve mandate"
							disabled={acting}
							title={`${approveMandate.consequence} ${approveMandate.nextState}`}
							onapprove={() => void decideMandate('approve')}
						/>
					{/key}
					{#if declineMandate}<button
							class="btn small"
							disabled={acting}
							title="Decline, with an optional note for Exec"
							onclick={() => (approvalNote = 'decline')}>{declineMandate.label}…</button
						>{/if}
				{:else if approvePayee}
					{#key `${item.id}:${attempt}`}
						<HoldApprove
							small
							completeLabel="Saving approval…"
							label="Hold to approve payee"
							disabled={acting}
							title={`${approvePayee.consequence} ${approvePayee.nextState}`}
							onapprove={() => void answerPayee(true)}
						/>
					{/key}
					{#if declinePayee}<button
							class="btn small"
							disabled={acting}
							title="Decline, with an optional note for Exec"
							onclick={() => (approvalNote = 'decline')}>{declinePayee.label}…</button
						>{/if}
				{:else if authorizeOnline}
					{#key `${item.id}:${attempt}`}
						<HoldApprove
							small
							completeLabel="Authorising…"
							label="Hold to authorise"
							disabled={acting}
							title={`${authorizeOnline.consequence} ${authorizeOnline.nextState}`}
							onapprove={() => void decidePublication(true)}
						/>
					{/key}
					{#if declineOnline}<button
							class="btn small"
							disabled={acting}
							title={`${declineOnline.consequence} ${declineOnline.nextState}`}
							onclick={() => void decidePublication(false)}>{declineOnline.label}</button
						>{/if}
				{:else if documentRequest && (item.nativeDocument || onopenDocument)}
					{#if item.nativeDocument}<a class="btn primary small" href={base} title={item.ifNoAction}
							>{item.source.kind === 'document_review'
								? 'Review this version'
								: 'Open and edit together'}</a
						>{:else}<button class="btn primary small" disabled={acting} onclick={openDocument}
							>{acting ? 'Opening…' : 'Open the document'}</button
						>{/if}
				{:else if appRequest && humanStep}
					<!-- Answered in the card above; the footer keeps skip and the menu. -->
				{:else if humanStep}
					{#if desktopSurface}
						<!-- The step happens on the computer in the card above. -->
					{:else if instructionLink}<a
							class="btn primary small"
							href={instructionLink.href}
							target="_blank"
							rel="noreferrer">{instructionLink.label}</a
						>{:else if navigation[0]}<a
							class="btn primary small"
							href={navigation[0].href}
							target={navigation[0].href?.startsWith('/') ? undefined : '_blank'}
							rel="noreferrer"
							data-open-externally={ownBrowser(navigation[0])}>{navigation[0].label}</a
						>{/if}
					<button
						class="btn small"
						class:primary={desktopSurface || (!instructionLink && !navigation[0])}
						disabled={acting || markingDone || item.preparing}
						title={`${humanStep.consequence} ${humanStep.nextState} Undo stays available for a few seconds.`}
						onclick={markDone}>{markingDone ? 'Marking done…' : 'Mark done'}</button
					>
				{:else if open && !open.href && (desktopSurface || outcomeInline)}
					<!-- The outcome or the computer is in the card above. -->
				{:else if open && !open.href}
					<a
						class="btn primary small"
						href={`${base}&${item.category === 'review' ? 'review' : 'computer'}=${encodeURIComponent(item.id)}`}
						title={`${open.consequence} ${open.nextState}`}>{open.label}</a
					>
				{:else if review && !navigation.length}
					<a class="btn primary small" href={`${base}&review=${encodeURIComponent(item.id)}`}
						>Review the outcome</a
					>
				{:else if navigation[0]}
					<a
						class="btn small"
						class:primary={!outcomeSurface}
						href={navigation[0].href}
						target={navigation[0].href?.startsWith('/') ? undefined : '_blank'}
						rel="noreferrer"
						data-open-externally={ownBrowser(navigation[0])}
						title={`${navigation[0].consequence} ${navigation[0].nextState}`}
						>{navigation[0].label}</a
					>
					{#if inlineNavigation > 1 && navigation[1]}<a
							class="btn small"
							href={navigation[1].href}
							target={navigation[1].href?.startsWith('/') ? undefined : '_blank'}
							rel="noreferrer"
							data-open-externally={ownBrowser(navigation[1])}
							title={`${navigation[1].consequence} ${navigation[1].nextState}`}
							>{navigation[1].label}</a
						>{/if}
				{/if}
				{#if discuss && !humanStep}<button
						class="btn small ghost"
						type="button"
						aria-expanded={discussing}
						onclick={() => (discussing = !discussing)}>Discuss</button
					>{/if}
			</div>
		{/if}
	</AttentionFooter>
</article>

<style>
	.account-request {
		display: grid;
		gap: 2px;
		margin: 8px 0 0;
		font-size: var(--t-label);
	}
	.account-request span {
		color: var(--text-tertiary);
	}
	.account-request q {
		color: var(--text-secondary);
	}
	input.code {
		max-width: 12ch;
		font-family: var(--font-mono);
		letter-spacing: 0.2em;
	}
	.inbox-detail {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
		overflow: hidden;
		container: page / inline-size;
	}
	.detail-content.has-preview {
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
	.detail-content.has-preview .head {
		flex: none;
	}
	.detail-preview {
		flex: 1;
		min-height: 0;
		overflow: hidden;
	}
	.detail-preview[hidden],
	.body[hidden] {
		display: none;
	}
	.detail-content {
		flex: 1;
		min-height: 0;
		overflow: auto;
	}
	.head {
		display: flex;
		align-items: flex-start;
		gap: var(--space-3);
		padding: 22px 24px 14px;
		border-bottom: 1px solid var(--border);
	}
	.title {
		flex: 1;
		min-width: 0;
	}
	h1 {
		margin: 0;
		color: var(--ink);
		font-size: var(--t-title);
		font-weight: 600;
		line-height: 1.3;
		letter-spacing: -0.015em;
		text-wrap: balance;
	}
	.meta {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 6px;
		margin: 6px 0 0;
		color: var(--text-tertiary);
		font-size: var(--t-body);
	}
	.meta :global(time) {
		color: inherit;
	}
	.deadline {
		color: var(--intent-authority);
	}
	.continues.held {
		color: var(--text-secondary);
	}
	.body {
		padding: 20px 24px 8px;
	}
	.message {
		display: grid;
		grid-template-columns: 30px minmax(0, 1fr);
		gap: 12px;
		max-width: 760px;
	}
	.avatar {
		display: grid;
		place-items: center;
		width: 30px;
		height: 30px;
		border: 1px solid var(--border-strong);
		border-radius: 50%;
		background: var(--surface-alt);
		color: var(--text-secondary);
		font-size: var(--t-body);
		font-weight: 600;
	}
	.message-head {
		display: flex;
		align-items: baseline;
		gap: 8px;
		margin-bottom: 4px;
		font-size: var(--t-body);
	}
	.message-head strong {
		color: var(--ink);
	}
	.message-head span {
		color: var(--text-tertiary);
	}
	.ask {
		color: var(--ink);
		font-size: var(--t-body);
		line-height: 1.65;
		overflow-wrap: anywhere;
	}
	.ask :global(.md > :first-child),
	.prose :global(.md > :first-child) {
		margin-top: 0;
	}
	.ask :global(.md > :last-child),
	.prose :global(.md > :last-child) {
		margin-bottom: 0;
	}
	.waiting {
		margin: 0;
		color: var(--text-secondary);
	}
	.draft {
		max-height: min(46vh, 420px);
		margin: 12px 0 0;
		overflow: auto;
		padding: 10px 12px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface-alt);
		color: var(--ink);
		font: var(--t-body) / 1.6 var(--font-ui);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
	.files {
		margin-top: 12px;
	}
	.uncertain {
		display: grid;
		gap: 2px;
		margin: 12px 0 0;
		padding: 8px 10px;
		border-left: 2px solid var(--intent-authority);
		background: color-mix(in srgb, var(--intent-authority) 5%, transparent);
		color: var(--text-secondary);
		font-size: var(--t-body);
		line-height: 1.55;
	}
	.uncertain span {
		color: var(--intent-authority);
		font-size: var(--t-label);
		font-weight: 600;
	}
	.folds {
		max-width: 760px;
		margin: 20px 0 0 42px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		overflow: hidden;
	}
	.folds > :global(* + *) {
		border-top: 1px solid var(--border);
	}
	.prose {
		display: grid;
		gap: 12px;
		padding: 14px 16px 16px 38px;
		color: var(--text-secondary);
		font-size: var(--t-body);
		line-height: 1.6;
		overflow-wrap: anywhere;
	}
	.recommended strong {
		display: block;
		margin-bottom: 4px;
		color: var(--intent-feedback);
		font-size: var(--t-label);
	}
	.credit {
		margin: 0;
		color: var(--text-tertiary);
	}
	.credit strong {
		color: var(--ink);
		font-weight: 500;
	}
	.quote span {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.quote blockquote {
		margin: 4px 0 0;
		padding-left: 10px;
		border-left: 2px solid var(--border-strong);
		white-space: pre-wrap;
	}
	.link {
		color: var(--intent-conversation);
		text-decoration: none;
	}
	.link:hover {
		text-decoration: underline;
	}
	.inbox-inputs {
		display: grid;
		gap: 10px;
	}
	.inbox-inputs textarea {
		width: 100%;
		min-height: 64px;
		max-height: 220px;
		field-sizing: content;
		resize: none;
	}
	.fields {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
		gap: 10px;
	}
	.fields label {
		display: grid;
		gap: 4px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.fields :is(input, select) {
		width: 100%;
	}
	.hint {
		order: -1;
		margin-right: auto;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.note {
		margin: 0;
		color: var(--text-secondary);
		font-size: var(--t-body);
	}
	/* Choosing another item settles its detail in place: a short fade and a
	 * 4px rise after the selection lands, instead of swapping the pane. */
	.head,
	.body {
		animation: inbox-detail-in 260ms var(--ease-out) 30ms backwards;
	}
	@keyframes inbox-detail-in {
		from {
			opacity: 0;
			transform: translateY(4px);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.head,
		.body {
			animation: none;
		}
	}
	@container page (max-width: 560px) {
		.head,
		.body {
			padding-inline: 14px;
		}
		.folds {
			margin-left: 0;
		}
		.hint {
			display: none;
		}
	}
</style>
