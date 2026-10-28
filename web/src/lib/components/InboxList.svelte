<script lang="ts">
	/* The Inbox: everything that needs the owner, as one list. Blocking items
	 * first, then what is due soonest, then the oldest ask. No categories to file into; the kind is an
	 * icon with a tooltip. Rows carry their own quick actions, J/K walks the
	 * list on into For you, and what the owner opened or snoozed is kept on the
	 * server, so every device agrees. */
	import { goto } from '$app/navigation';
	import { askExecAboutItem } from '$lib/model/ask-exec';
	import { pollEvery } from '$lib/model/connection.svelte';
	import { page } from '$app/state';
	import { onMount, tick, untrack } from 'svelte';
	import CircleCheck from '@lucide/svelte/icons/circle-check';
	import Eye from '@lucide/svelte/icons/eye';
	import GitPullRequest from '@lucide/svelte/icons/git-pull-request';
	import Hand from '@lucide/svelte/icons/hand';
	import ListFilter from '@lucide/svelte/icons/list-filter';
	import MessageCircle from '@lucide/svelte/icons/message-circle';
	import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { Item } from '$lib/ui/page';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import SetAsideList from './SetAside.svelte';
	import InboxHistory from './InboxHistory.svelte';
	import SnoozeChoices from './SnoozeChoices.svelte';
	import {
		appRequestHandoffs,
		copyInboxLink,
		inboxItemHref,
		markDoneSoon,
		inboxMarksKey,
		markInbox,
		setAsideInboxItem,
		skipHandoffSoon,
		skipsByHandoff,
		snoozeInboxItem,
		wakeInboxItem
	} from './inbox-actions';
	import { celebrate } from '$lib/ui/celebrate';
	import InboxUpdate from './InboxUpdate.svelte';
	import { inboxFeed } from '$lib/model/inbox-feed';
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
		notice = '',
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
		/** A company-wide state the reader is part of, such as an ownership handover. */
		notice?: string;
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
	const filter = $derived(page.url.searchParams.get('filter') ?? 'open');
	const showOpen = $derived(!canSetAside || !['skipped', 'done'].includes(filter));
	function setFilter(value: string) {
		const url = new URL(page.url);
		if (value === 'open') url.searchParams.delete('filter');
		else url.searchParams.set('filter', value);
		for (const key of ['item', 'review', 'update']) url.searchParams.delete(key);
		void goto(url, { keepFocus: true, noScroll: true });
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
		refetchInterval: pollEvery(60_000)
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
			(showOpen ? items : []).filter(
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
		item.category === 'approval' && item.actions.some((action) => action.id === 'decline');
	const canSkip = (item: AttentionItem) => canSetAside || skipsByHandoff(item);
	const skip = (item: AttentionItem) => {
		const who = attentionAsker(item);
		if (skipsByHandoff(item)) void skipHandoffSoon(client, companyId, item.source.reference, who);
		else void setAsideInboxItem(client, companyId, item.id, who);
	};
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

	const feed = $derived(inboxFeed(visible, !showOpen || requestsOnly ? [] : (forYou?.items ?? [])));
	function keyboard(event: KeyboardEvent) {
		if (
			event.defaultPrevented ||
			(event.target as HTMLElement)?.closest(
				'input, textarea, select, [contenteditable], dialog, #bridge-exrail'
			) ||
			document.querySelector('dialog[open]')
		)
			return;
		const focused = (document.activeElement as HTMLElement)?.closest<HTMLElement>(
			'[data-inbox-key]'
		)?.dataset.inboxKey;
		const row =
			feed.find((row) => row.key === focused) ??
			feed.find((row) => row.kind === 'attention' && row.item.id === selectedId);
		const item = row?.kind === 'attention' ? row.item : null;
		if ((event.metaKey || event.ctrlKey) && event.key === '.' && item) {
			event.preventDefault();
			void copy(item);
			return;
		}
		if (event.metaKey || event.ctrlKey || event.altKey) return;
		const step = { j: 1, ArrowDown: 1, k: -1, ArrowUp: -1 }[event.key];
		if (step && feed.length) {
			event.preventDefault();
			const index = row ? feed.indexOf(row) : -1;
			const target = feed[Math.max(0, Math.min(feed.length - 1, index + step))];
			if (target.kind === 'attention') select(target.item);
			else {
				const wrapper = [...document.querySelectorAll<HTMLElement>('[data-inbox-key]')].find(
					(el) => el.dataset.inboxKey === target.key
				);
				const link = wrapper?.querySelector<HTMLElement>('.item-hit');
				link?.focus();
				link?.scrollIntoView({ block: 'nearest' });
			}
			return;
		}
		if (row?.kind === 'update' && forYou) {
			if (event.key === 'e' || (event.key === 'U' && event.shiftKey)) {
				event.preventDefault();
				void (
					event.key === 'e' ? forYou.read([row.item.key]) : forYou.unread([row.item.key])
				).catch(() => toast('The update was not marked. Try again.', { tone: 'failure' }));
			}
			return;
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
			const answer =
				document.querySelector<HTMLElement>('.inbox-composer textarea') ??
				document.querySelector<HTMLElement>(
					'.inbox-composer .bar :is(button, a), .direction-request .answers button, .attention-document button'
				);
			if (answer) answer.focus();
			else toast('Open the item to answer it.');
		}
		if (event.key === 'c' && canDiscuss(item)) {
			event.preventDefault();
			askExecAboutItem(companyId, item);
		}
	}
</script>

<svelte:window onkeydown={keyboard} />

<div class="inbox">
	<header class="inbox-head">
		<h2>Inbox</h2>
		<span class="spacer"></span>
		{#if canSetAside}
			<ActionMenu label={`Filter Inbox: ${filter === 'open' ? 'Open' : filter === 'skipped' ? 'Skipped' : filter === 'done' ? 'Done' : 'All'}`}>
				{#snippet trigger()}<ListFilter size={16} strokeWidth={1.8} aria-hidden="true" />{/snippet}
				{#each ['open', 'skipped', 'done', 'all'] as value}
					<button aria-pressed={filter === value} onclick={() => setFilter(value)}>{value[0].toUpperCase() + value.slice(1)}</button>
				{/each}
			</ActionMenu>
		{/if}
		<ActionMenu label="Inbox options">
			<button onclick={() => (showSnoozed = !showSnoozed)} disabled={!snoozedCount && !showSnoozed}
				>{showSnoozed
					? 'Hide snoozed'
					: `Show snoozed${snoozedCount ? ` (${snoozedCount})` : ''}`}</button
			>
			{#if forYou}
				<button onclick={() => forYou.setFilter(forYou.filter === 'archived' ? 'all' : 'archived')}>
					{forYou.filter === 'archived' ? 'Show current updates' : 'Show archived updates'}
				</button>
				<button
					disabled={!forYou.unreadCount}
					onclick={() =>
						void forYou
							.read('all')
							.catch(() => toast('Updates could not be marked read.', { tone: 'failure' }))}
					>Mark updates read</button
				>
			{/if}
			<button onclick={() => window.dispatchEvent(new CustomEvent('restless:shortcuts'))}
				>Keyboard shortcuts <kbd>?</kbd></button
			>
		</ActionMenu>
	</header>
	<div>{#if showOpen}<DirectionToday
		{companyId}
		{requestsOnly}
		onrequests={(only) => {
			requestsOnly = only;
		}}
	/>{/if}</div>

	<div class="inbox-list">
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
		{#if notice}
			<a class="setup handover" href={`/${companyId}/company/members`} aria-live="polite"
				><TriangleAlert size={15} strokeWidth={1.8} aria-hidden="true" /><span>{notice}</span></a
			>
		{/if}
		<div role="list" aria-label="Waiting on you" class="inbox-rows">
			{#each feed as row (row.key)}
				<div data-inbox-key={row.key}>
					{#if row.kind === 'update' && forYou}
						<InboxUpdate {companyId} source={forYou} item={row.item} />
					{:else if row.kind === 'attention'}
						{@const item = row.item}
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
									<button onclick={() => askExecAboutItem(companyId, item)}
										>Ask Exec about this <kbd>C</kbd></button
									>
									{#if canComplete(item)}<button disabled={!!completing} onclick={() => done(item)}
											>{completing === item.id ? 'Recording…' : 'Mark done'} <kbd>E</kbd></button
										>{/if}
									{#if canDecline(item)}<a href={`${href(item.id)}&decline=1`}>Decline…</a>{/if}
									{#if snoozedNow}
										<button onclick={() => void wakeInboxItem(client, companyId, item.id)}
											>Wake now</button
										>
									{:else}
										<SnoozeChoices shortcut onpick={(until) => snooze(item, until)} />
									{/if}
									{#if canSkip(item)}<button
											title={`${attentionAsker(item)} carries on without it. You can undo.`}
											onclick={() => skip(item)}>Skip</button
										>{/if}
									<button onclick={() => copy(item)}>Copy link</button>
								</ActionMenu>
							{/snippet}
						</Item>
					{/if}
				</div>
			{/each}
		</div>
		{#if showOpen && loaded && !feed.length && !(coordination && !items.length)}
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
		{#if canSetAside && (filter === 'skipped' || filter === 'all')}
			<SetAsideList {companyId} version={items.length} />
		{/if}
		{#if canSetAside && (filter === 'done' || filter === 'all')}
			<InboxHistory {companyId} version={items.length} />
		{/if}
		{#if showOpen && forYou?.failure}<p class="empty">
				Updates could not be read. <button type="button" onclick={() => forYou?.refresh()}
					>Try again</button
				>
			</p>{/if}
		{#if showOpen && forYou?.hasMore}<button
				type="button"
				class="btn small ghost"
				disabled={forYou.loadingMore}
				onclick={() => forYou?.loadMore()}>Show earlier updates</button
			>{/if}
	</div>
</div>

<style>
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
	h2 {
		margin: 0;
		color: var(--ink);
		font-size: var(--t-head);
		font-weight: 600;
	}

	.spacer {
		flex: 1;
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
	.setup.handover {
		border-color: color-mix(in srgb, var(--intent-authority) 25%, var(--border));
		background: color-mix(in srgb, var(--intent-authority) 5%, var(--surface-raised));
	}
	.setup.handover :global(svg) {
		color: var(--intent-authority);
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
