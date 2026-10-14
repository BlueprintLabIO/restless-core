<script lang="ts">
	/* The Inbox: everything that needs the owner, as one list. Blocking items
	 * first, then what is due soonest, then the oldest ask. No categories to file into; the kind is an
	 * icon with a tooltip. Rows carry their own quick actions, J/K walks the
	 * list on into For you, and what the owner opened or snoozed is kept on the
	 * server, so every device agrees. */
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { onMount, tick, untrack } from 'svelte';
	import CircleCheck from '@lucide/svelte/icons/circle-check';
	import Eye from '@lucide/svelte/icons/eye';
	import GitPullRequest from '@lucide/svelte/icons/git-pull-request';
	import Hand from '@lucide/svelte/icons/hand';
	import MessageCircle from '@lucide/svelte/icons/message-circle';
	import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { Item } from '$lib/ui/page';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import SetAsideList from './SetAside.svelte';
	import SnoozeChoices from './SnoozeChoices.svelte';
	import {
		appRequestHandoffs,
		copyInboxLink,
		inboxItemHref,
		markDoneSoon,
		inboxMarksKey,
		markInbox,
		setAsideInboxItem,
		snoozeInboxItem,
		wakeInboxItem
	} from './inbox-actions';
	import { celebrate } from '$lib/ui/celebrate';
	import ForYou from './ForYou.svelte';
	import MyRequests from './MyRequests.svelte';
	import DirectionToday from './DirectionToday.svelte';
	import { useForYou } from '$lib/model/for-you.svelte';
	import { getInboxMarks, type InboxMarkChange } from '$lib/model/for-you';
	import {
		attentionAsker,
		attentionKindLabel,
		attentionTitle,
		inboxOrder,
		snoozeChoices
	} from '$lib/model/attention-presentation';
	import type { AttentionItem } from '$lib/model/view';

	let {
		companyId,
		items,
		selectedId,
		blocker = '',
		loaded = false,
		failure = null,
		coordination = null,
		onretry,
		onvisible,
		canSetAside = true
	}: {
		companyId: string;
		items: AttentionItem[];
		selectedId?: string;
		blocker?: string;
		loaded?: boolean;
		failure?: Error | null;
		/** The company's coordination is unavailable, so the list may be incomplete. */
		coordination?: Error | null;
		onretry?: () => unknown;
		onvisible?: (items: AttentionItem[]) => void;
		/** Setting an item aside is the owner's: it answers for the whole company queue. */
		canSetAside?: boolean;
	} = $props();

	const client = useQueryClient();
	/* Beside the decisions: mentions, replies, shares and changes to what the owner follows. */
	const forYou = useForYou();
	let forYouList: ForYou | undefined = $state();
	/* One list at a time: decisions, or what happened around you. Remembered per company on this
	 * device; the counts on both tabs keep the other one in view. */
	let tab = $state<'inbox' | 'for-you'>('inbox');
	const tabKey = () => `restless:inbox-tab:${companyId}`;
	$effect(() => {
		try {
			tab =
				page.url.searchParams.get('tab') === 'for-you' ||
				localStorage.getItem(tabKey()) === 'for-you'
					? 'for-you'
					: 'inbox';
		} catch {
			tab = 'inbox';
		}
	});
	function showTab(next: 'inbox' | 'for-you') {
		tab = next;
		try {
			localStorage.setItem(tabKey(), next);
		} catch {
			/* Storage can be unavailable; the tab still switches. */
		}
	}
	let showSnoozed = $state(false);
	/* The Today line's "requests waiting" shows only those requests until it is pressed again. */
	let requestsOnly = $state(false);
	$effect(() => {
		if (requestsOnly && !items.some((item) => item.directionRequest)) requestsOnly = false;
	});
	let completing = $state('');
	let clock = $state(Date.now());

	/* What this person opened and snoozed, on every device. */
	const marksQuery = createQuery(() => ({
		queryKey: inboxMarksKey(companyId),
		queryFn: () => getInboxMarks(companyId),
		enabled: Boolean(companyId),
		staleTime: 10_000,
		refetchInterval: 30_000
	}));
	const marks = $derived(new Map((marksQuery.data ?? []).map((mark) => [mark.item_id, mark])));
	const snoozedUntil = (item: AttentionItem) => {
		const until = marks.get(item.id)?.snoozed_until;
		return until ? Date.parse(until) : 0;
	};
	const mark = (changes: InboxMarkChange[], quiet = false) =>
		markInbox(client, companyId, changes, quiet);

	const snoozedCount = $derived(items.filter((item) => snoozedUntil(item) > clock).length);
	const visible = $derived(
		inboxOrder(
			items.filter(
				(item) =>
					(showSnoozed || snoozedUntil(item) <= clock) && (!requestsOnly || !!item.directionRequest)
			)
		)
	);
	$effect(() => {
		const rows = visible;
		untrack(() => onvisible?.(rows));
	});

	const stamp = (item: AttentionItem) => String(item.briefedAt ?? item.createdAt);
	const href = (id: string) => inboxItemHref(companyId, id);
	const canDecline = (item: AttentionItem) =>
		item.actions.some(
			(action) =>
				action.id === 'record-decision' ||
				action.id === 'complete-human-step' ||
				(action.id === 'decline' && item.category === 'approval')
		);
	const canDiscuss = (item: AttentionItem) =>
		item.actions.some((action) => action.id === 'chat-lead');
	/* An app request resumes when the app is added and allowed, so it has no Mark done here
	 * either; the detail says so and links to the app. */
	const appRequests = appRequestHandoffs(
		() => companyId,
		() => canSetAside
	);
	const appRequestIds = $derived(
		new Set((appRequests.data ?? []).map((request) => request.handoff_id))
	);
	const canComplete = (item: AttentionItem) =>
		item.actions.some((action) => action.id === 'complete-human-step') &&
		!(item.source.kind === 'owner_handoff' && appRequestIds.has(item.source.reference));
	const iconFor = (item: AttentionItem) =>
		item.category === 'approval'
			? CircleCheck
			: item.category === 'review'
				? Eye
				: item.category === 'blocker'
					? TriangleAlert
					: item.category === 'human_step'
						? Hand
						: item.category === 'conversation'
							? MessageCircle
							: GitPullRequest;

	$effect(() => {
		companyId;
		showSnoozed = false;
		requestsOnly = false;
	});
	onMount(() => {
		const timer = setInterval(() => (clock = Date.now()), 30_000);
		return () => clearInterval(timer);
	});
	/* Opening an item reads this brief of it; a later brief reads as new again. Each brief is
	 * sent once: a refused write must not loop through the refetch that follows it. */
	const sentSeen = new Set<string>();
	$effect(() => {
		if (!marksQuery.data) return;
		const item = items.find((row) => row.id === selectedId);
		if (!item || marks.get(item.id)?.seen_stamp === stamp(item)) return;
		const sent = `${companyId} ${item.id} ${stamp(item)}`;
		if (sentSeen.has(sent)) return;
		sentSeen.add(sent);
		void mark([{ item_id: item.id, seen_stamp: stamp(item) }], true);
	});

	function next(after: AttentionItem): string {
		const others = visible.filter((row) => row.id !== after.id);
		const index = visible.findIndex((row) => row.id === after.id);
		const target = others[Math.min(index, others.length - 1)];
		return target ? href(target.id) : `/${companyId}`;
	}
	function snooze(item: AttentionItem, until = snoozeChoices()[0].until) {
		snoozeInboxItem(client, companyId, item.id, until);
		if (selectedId === item.id) void goto(next(item), { noScroll: true });
	}
	const copy = (item: AttentionItem) => copyInboxLink(companyId, item.id);
	/* Mark done resumes the Work, so it waits out a few seconds of Undo first (E does the same). */
	async function done(item: AttentionItem) {
		if (completing || !canComplete(item)) return;
		completing = item.id;
		/* Where the row is now; it leaves the list once the step is recorded. */
		const row = document.activeElement?.closest('[role="listitem"], li, a') ?? null;
		const at = row?.getBoundingClientRect() ?? null;
		try {
			if (await markDoneSoon(client, companyId, item.source.reference)) {
				celebrate(at);
				toast('Marked done. The work resumes.');
			}
		} finally {
			completing = '';
		}
	}
	/* Stepping through the list with J/K replaces the address rather than adding history, so Back
	 * leaves the Inbox instead of walking back through every item passed on the way. A click on a
	 * row is a deliberate open and keeps its history entry. */
	function select(target: AttentionItem) {
		void goto(href(target.id), { keepFocus: true, noScroll: true, replaceState: true }).then(() => {
			const row = document.querySelector<HTMLElement>('.inbox-list [aria-current="true"]');
			row?.focus();
			row?.scrollIntoView({
				block: 'nearest',
				behavior: window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth'
			});
		});
	}

	function keyboard(event: KeyboardEvent) {
		if (
			event.defaultPrevented ||
			(event.target as HTMLElement)?.closest(
				'input, textarea, select, [contenteditable], dialog, #bridge-exrail, .for-you'
			) ||
			document.querySelector('dialog[open]')
		)
			return;
		const item = items.find((row) => row.id === selectedId);
		if ((event.metaKey || event.ctrlKey) && event.key === '.' && item) {
			event.preventDefault();
			void copy(item);
			return;
		}
		if (event.metaKey || event.ctrlKey || event.altKey) return;
		const step = { j: 1, ArrowDown: 1, k: -1, ArrowUp: -1 }[event.key];
		if (step) {
			const index = visible.findIndex((row) => row.id === selectedId);
			/* Past the last decision, the keys walk on into For you. */
			if (step > 0 && (index === visible.length - 1 || !visible.length)) {
				if (forYou?.items.length) {
					event.preventDefault();
					showTab('for-you');
					void tick().then(() => forYouList?.focusFirst());
				}
				return;
			}
			if (!visible.length) return;
			event.preventDefault();
			select(visible[Math.max(0, Math.min(visible.length - 1, index + step))]);
		}
		if (!item) return;
		if (event.key === 'h') {
			event.preventDefault();
			snooze(item);
		}
		if (event.key === 'e' && canComplete(item)) {
			event.preventDefault();
			void done(item);
		}
		if (event.key === 'r') {
			event.preventDefault();
			/* Items without a reply box (approvals, requests, documents) take focus to their first
			 * answer instead of doing nothing. */
			const answer =
				document.querySelector<HTMLElement>('.inbox-composer textarea') ??
				document.querySelector<HTMLElement>(
					'.inbox-composer .bar :is(button, a), .direction-request .answers button, .attention-document button'
				);
			if (answer) answer.focus();
			else toast('Nothing to reply to here. Open the item to answer it.');
		}
		if (event.key === 'c' && canDiscuss(item)) {
			event.preventDefault();
			void goto(`${href(item.id)}&conversation=${encodeURIComponent(item.id)}`);
		}
	}
</script>

<svelte:window onkeydown={keyboard} />

<div class="inbox">
	<header class="inbox-head">
		<div class="inbox-tabs" role="tablist" aria-label="Inbox">
			<button
				type="button"
				role="tab"
				aria-selected={tab === 'inbox'}
				class:active={tab === 'inbox'}
				title="Decisions and steps waiting on you"
				onclick={() => showTab('inbox')}
				>Inbox{#if loaded && visible.length}<span class="tab-count">{visible.length}</span
					>{/if}</button
			>
			{#if forYou}<button
					type="button"
					role="tab"
					aria-selected={tab === 'for-you'}
					class:active={tab === 'for-you'}
					title="Mentions, replies, shares and changes to what you follow"
					onclick={() => showTab('for-you')}
					>For you{#if forYou.unreadCount}<span class="tab-count unread">{forYou.unreadCount}</span
						>{/if}</button
				>{/if}
		</div>
		<span class="spacer"></span>
		<ActionMenu label="Inbox options">
			{#if tab === 'inbox'}<button
					onclick={() => (showSnoozed = !showSnoozed)}
					disabled={!snoozedCount && !showSnoozed}
					>{showSnoozed
						? 'Hide snoozed'
						: `Show snoozed${snoozedCount ? ` (${snoozedCount})` : ''}`}</button
				>{/if}
			<span class="shortcuts" role="note"
				><kbd>J</kbd><kbd>K</kbd> move · <kbd>R</kbd> answer · <kbd>C</kbd> discuss · <kbd>H</kbd>
				snooze · <kbd>E</kbd> done, with Undo · <kbd>?</kbd> all keys</span
			>
		</ActionMenu>
	</header>
	<DirectionToday
		{companyId}
		{requestsOnly}
		onrequests={(only) => {
			requestsOnly = only;
			showTab('inbox');
		}}
	/>

	<div class="inbox-list" role="tabpanel">
		{#if tab === 'for-you' && forYou}
			<MyRequests {companyId} all />
			<ForYou
				bind:this={forYouList}
				source={forYou}
				embedded
				onexit={() => {
					showTab('inbox');
					const last = visible.at(-1);
					if (last) select(last);
				}}
			/>
		{:else}
			{#if failure && !loaded}
				<div class="inbox-failure">
					<FailureNotice error={failure} subject="the Inbox" variant="block" {onretry} />
				</div>
			{:else if coordination && loaded}
				<div class="inbox-failure">
					<FailureNotice error={coordination} {onretry} />
				</div>
			{/if}
			{#if blocker && !items.some( (item) => item.actions.some((action) => action.id === 'open-intelligence-provider') )}
				<a class="setup" href={`/${companyId}/company/provider`}
					><TriangleAlert size={15} strokeWidth={1.8} aria-hidden="true" /><span>{blocker}</span></a
				>
			{/if}
			<div role="list" aria-label="Waiting on you" class="inbox-rows">
				{#each visible as item (item.id)}
					{@const Icon = iconFor(item)}
					{@const snoozedNow = snoozedUntil(item) > clock}
					{@const unreadNow = !!marksQuery.data && marks.get(item.id)?.seen_stamp !== stamp(item)}
					<Item
						role="listitem"
						stacked
						title={attentionTitle(item)}
						label={`${unreadNow ? 'Unread. ' : ''}${attentionKindLabel(item)}: ${attentionTitle(item)}${snoozedNow ? '. Snoozed' : ''}`}
						meta={item.preparing ? 'Being prepared' : attentionAsker(item)}
						href={href(item.id)}
						selected={selectedId === item.id}
						unread={unreadNow}
						dim={snoozedNow}
					>
						{#snippet leading()}<span
								class="kind"
								class:blocker={item.category === 'blocker'}
								title={attentionKindLabel(item)}
								><Icon size={15} strokeWidth={1.8} aria-hidden="true" /></span
							>{/snippet}
						{#snippet trailing()}<RelativeTime value={item.createdAt} />{/snippet}
						{#snippet actions()}
							<ActionMenu label={`Actions for ${attentionTitle(item)}`}>
								<a href={href(item.id)}>Open</a>
								{#if canDiscuss(item)}<a
										href={`${href(item.id)}&conversation=${encodeURIComponent(item.id)}`}
										>Discuss <kbd>C</kbd></a
									>{/if}
								{#if canComplete(item)}<button disabled={!!completing} onclick={() => done(item)}
										>{completing === item.id ? 'Recording…' : 'Mark done'} <kbd>E</kbd></button
									>{/if}
								{#if canDecline(item)}<a href={`${href(item.id)}&decline=1`}>Not doing this…</a
									>{/if}
								{#if snoozedNow}
									<button onclick={() => void wakeInboxItem(client, companyId, item.id)}
										>Wake now</button
									>
								{:else}
									<SnoozeChoices shortcut onpick={(until) => snooze(item, until)} />
								{/if}
								{#if canSetAside}<button
										title="Not needed: off your Inbox, kept under Set aside. Whoever asked stops waiting."
										onclick={() => void setAsideInboxItem(client, companyId, item.id)}
										>Set aside</button
									>{/if}
								<button onclick={() => copy(item)}>Copy link</button>
							</ActionMenu>
						{/snippet}
					</Item>
				{/each}
			</div>
			{#if loaded && !visible.length && !(coordination && !items.length)}
				<p class="empty">
					{requestsOnly
						? 'No requests waiting on you.'
						: items.length
							? 'Everything here is snoozed.'
							: 'No decisions waiting on you.'}
				</p>
			{:else if !loaded && !failure}
				<div class="waiting" aria-label="Loading the Inbox"><i></i><i></i><i></i></div>
			{/if}
			{#if canSetAside}<div class="set-aside-slot">
					<SetAsideList {companyId} version={items.length} />
					<a
						class="decided"
						href={`/${encodeURIComponent(companyId)}/company/activity?show=decisions`}
						title="What you answered lately, and where each stands. Follow up on one from there."
						>Recently decided</a
					>
				</div>{/if}
		{/if}
	</div>
</div>

<style>
	.set-aside-slot {
		display: grid;
		justify-items: start;
		margin: 12px 8px 8px;
	}
	.decided {
		padding: 6px 8px;
		border-radius: var(--radius-control);
		color: var(--text-tertiary);
		font-size: var(--t-label);
		text-decoration: none;
	}
	.decided:hover {
		background: var(--wash-hover);
		color: var(--text-secondary);
	}
	.inbox {
		display: grid;
		grid-template-rows: auto auto minmax(0, 1fr);
		height: 100%;
		min-height: 0;
		container: page / inline-size;
	}
	.inbox-head {
		display: flex;
		align-items: center;
		gap: 8px;
		min-height: 52px;
		padding: 8px 8px 8px 10px;
		border-bottom: 1px solid var(--border);
	}
	.inbox-tabs {
		display: flex;
		gap: 2px;
		min-width: 0;
	}
	.inbox-tabs button {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		height: 30px;
		padding: 0 10px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-secondary);
		font: inherit;
		font-size: var(--t-body);
		font-weight: 500;
		white-space: nowrap;
		cursor: pointer;
		transition:
			background-color var(--motion-state) var(--ease-standard),
			color var(--motion-state) var(--ease-standard);
	}
	.inbox-tabs button:hover {
		background: var(--surface-hover);
		color: var(--ink);
	}
	.inbox-tabs button.active {
		background: var(--surface-alt);
		color: var(--ink);
		font-weight: 600;
	}
	.tab-count {
		min-width: 18px;
		padding: 0 5px;
		border-radius: 9px;
		background: var(--surface-hover);
		color: var(--text-tertiary);
		font-size: var(--t-label);
		font-variant-numeric: tabular-nums;
		line-height: 18px;
		text-align: center;
	}
	.tab-count.unread {
		background: color-mix(in srgb, var(--intent-conversation) 14%, transparent);
		color: var(--intent-conversation);
	}
	.spacer {
		flex: 1;
	}
	.shortcuts {
		display: block;
		max-width: 240px;
		padding: 8px 10px;
		border-top: 1px solid var(--border);
		color: var(--text-tertiary);
		font-size: var(--t-label);
		line-height: 1.7;
	}
	kbd {
		padding: 0 4px;
		border: 1px solid var(--border-strong);
		border-radius: 3px;
		font: var(--t-label) var(--font-mono);
	}
	.inbox-list {
		min-height: 0;
		overflow: auto;
		padding: 4px 0 12px;
	}
	.inbox-list :global(.item-meta) {
		flex: 0 1 auto;
	}
	.kind {
		display: grid;
		color: var(--text-tertiary);
	}
	.kind.blocker {
		color: var(--state-danger);
	}
	.setup {
		display: flex;
		align-items: center;
		gap: 8px;
		margin: 6px 8px;
		padding: 10px 12px;
		border: 1px solid color-mix(in srgb, var(--state-danger) 25%, var(--border));
		border-radius: var(--radius-lg);
		background: color-mix(in srgb, var(--state-danger) 5%, var(--surface-raised));
		color: var(--ink);
		font-size: var(--t-body);
		text-decoration: none;
	}
	.setup :global(svg) {
		flex: none;
		color: var(--state-danger);
	}
	.empty {
		margin: 0;
		padding: 32px 16px;
		color: var(--text-tertiary);
		text-align: center;
	}
	.waiting {
		display: grid;
		gap: 8px;
		padding: 12px;
	}
	.waiting i {
		height: 36px;
		border-radius: var(--radius-control);
		background: var(--surface-alt);
		animation: inbox-waiting var(--motion-working) var(--ease-standard) infinite;
	}
	@keyframes inbox-waiting {
		50% {
			opacity: 0.45;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.waiting i {
			animation: none;
		}
	}
	.inbox-failure {
		display: none;
	}
	@media (max-width: 760px) {
		.inbox-failure {
			display: block;
			padding: 12px;
		}
	}
</style>
