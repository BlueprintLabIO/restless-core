<script lang="ts">
	/* The Inbox: everything that needs the owner, as one list. Blocking items
	 * first, then the oldest ask. No categories to file into; the kind is an
	 * icon with a tooltip. Rows carry their own quick actions, J/K walks the
	 * list on into For you, and what the owner opened or snoozed is kept on the
	 * server, so every device agrees. */
	import { goto } from '$app/navigation';
	import { onMount, untrack } from 'svelte';
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
	import {
		completeHandoffHumanStep,
		dismissAttention,
		restoreAttention
	} from '$lib/model/attention';
	import SetAsideList from './SetAside.svelte';
	import ForYou from './ForYou.svelte';
	import { useForYou } from '$lib/model/for-you.svelte';
	import {
		getInboxMarks,
		setInboxMarks,
		withMarks,
		type InboxMark,
		type InboxMarkChange
	} from '$lib/model/for-you';
	import { refreshAttention } from '$lib/model/queries.svelte';
	import { failureSentence } from '$lib/model/failure';
	import {
		attentionAsker,
		attentionKindLabel,
		attentionTitle,
		inboxOrder
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
		onvisible
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
	} = $props();

	const client = useQueryClient();
	/* Beside the decisions: mentions, replies, shares and changes to what the owner follows. */
	const forYou = useForYou();
	let forYouList: ForYou | undefined = $state();
	let showSnoozed = $state(false);
	let completing = $state('');
	let clock = $state(Date.now());

	/* What this person opened and snoozed, on every device. */
	const marksKey = () => ['inbox-marks', companyId] as const;
	const marksQuery = createQuery(() => ({
		queryKey: marksKey(),
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
	async function mark(changes: InboxMarkChange[], quiet = false) {
		const key = marksKey();
		client.setQueryData<InboxMark[]>(key, (data) => withMarks(data ?? [], changes));
		try {
			await setInboxMarks(companyId, changes);
		} catch (cause) {
			/* A seen mark that did not stick only leaves a dot; it is tried again next time. */
			if (!quiet) toast(failureSentence(cause, 'That was not kept.'), { tone: 'failure' });
		} finally {
			void client.invalidateQueries({ queryKey: key });
		}
	}

	async function notNeeded(item: AttentionItem) {
		try {
			await dismissAttention(companyId, item.id);
			toast('Set aside', {
				undo: async () => {
					try {
						await restoreAttention(companyId, item.id);
					} catch (cause) {
						toast(failureSentence(cause, 'That was not restored.'), { tone: 'failure' });
					}
					await refreshAttention(client, companyId);
				}
			});
		} catch (cause) {
			toast(failureSentence(cause, 'That was not set aside.'), { tone: 'failure' });
		}
		await refreshAttention(client, companyId);
	}

	const snoozedCount = $derived(items.filter((item) => snoozedUntil(item) > clock).length);
	const visible = $derived(
		inboxOrder(items.filter((item) => showSnoozed || snoozedUntil(item) <= clock))
	);
	$effect(() => {
		const rows = visible;
		untrack(() => onvisible?.(rows));
	});

	const stamp = (item: AttentionItem) => String(item.briefedAt ?? item.createdAt);
	const href = (id: string) => `/${encodeURIComponent(companyId)}?item=${encodeURIComponent(id)}`;
	const canDecline = (item: AttentionItem) =>
		item.actions.some(
			(action) =>
				action.id === 'record-decision' ||
				action.id === 'complete-human-step' ||
				(action.id === 'decline' && item.category === 'approval')
		);
	const canDiscuss = (item: AttentionItem) => item.actions.some((action) => action.id === 'chat-lead');
	const canComplete = (item: AttentionItem) =>
		item.actions.some((action) => action.id === 'complete-human-step');
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
	function snooze(item: AttentionItem) {
		void mark([{ item_id: item.id, snoozed_until: new Date(Date.now() + 3_600_000).toISOString() }]);
		toast('Snoozed for an hour', {
			undo: () => mark([{ item_id: item.id, snoozed_until: null }])
		});
		if (selectedId === item.id) void goto(next(item), { noScroll: true });
	}
	async function copy(item: AttentionItem) {
		try {
			await navigator.clipboard.writeText(new URL(href(item.id), location.origin).href);
			toast('Link copied');
		} catch {
			toast('Could not copy the link', { tone: 'failure' });
		}
	}
	async function done(item: AttentionItem) {
		if (completing || !canComplete(item)) return;
		completing = item.id;
		try {
			await completeHandoffHumanStep(companyId, item.source.reference);
			await refreshAttention(client, companyId);
			toast('Marked done');
		} catch (cause) {
			toast(failureSentence(cause, 'That was not recorded.'), { tone: 'failure' });
		} finally {
			completing = '';
		}
	}
	function select(target: AttentionItem) {
		void goto(href(target.id), { keepFocus: true, noScroll: true }).then(() => {
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
				if (forYouList?.focusFirst()) event.preventDefault();
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
			document.querySelector<HTMLTextAreaElement>('.inbox-composer textarea')?.focus();
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
		<h2>Inbox</h2>
		<span class="inbox-count">{loaded ? visible.length : ''}</span>
		<span class="spacer"></span>
		<ActionMenu label="Inbox options">
			<button onclick={() => (showSnoozed = !showSnoozed)} disabled={!snoozedCount && !showSnoozed}
				>{showSnoozed ? 'Hide snoozed' : `Show snoozed${snoozedCount ? ` (${snoozedCount})` : ''}`}</button
			>
			<span class="shortcuts" role="note"
				><kbd>J</kbd><kbd>K</kbd> move · <kbd>R</kbd> reply · <kbd>C</kbd> discuss · <kbd>H</kbd> snooze
				· <kbd>E</kbd> done</span
			>
		</ActionMenu>
	</header>

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
		{#if blocker && !items.some((item) => item.actions.some((action) => action.id === 'open-intelligence-provider'))}
			<a class="setup" href={`/${companyId}/company/provider`}
				><TriangleAlert size={15} strokeWidth={1.8} aria-hidden="true" /><span>{blocker}</span></a
			>
		{/if}
		{#each visible as item (item.id)}
			{@const Icon = iconFor(item)}
			{@const snoozedNow = snoozedUntil(item) > clock}
			<Item
				title={attentionTitle(item)}
				meta={item.preparing ? 'Being prepared' : attentionAsker(item)}
				href={href(item.id)}
				selected={selectedId === item.id}
				unread={!!marksQuery.data && marks.get(item.id)?.seen_stamp !== stamp(item)}
				dim={snoozedNow}
			>
				{#snippet leading()}<span class="kind" class:blocker={item.category === 'blocker'} title={attentionKindLabel(item)}
						><Icon size={15} strokeWidth={1.8} aria-label={attentionKindLabel(item)} /></span
					>{/snippet}
				{#snippet trailing()}<RelativeTime value={item.createdAt} />{/snippet}
				{#snippet actions()}
					<ActionMenu label={`Actions for ${attentionTitle(item)}`}>
						<a href={href(item.id)}>Open</a>
						{#if canDiscuss(item)}<a href={`${href(item.id)}&conversation=${encodeURIComponent(item.id)}`}
								>Discuss <kbd>C</kbd></a
							>{/if}
						{#if canComplete(item)}<button disabled={!!completing} onclick={() => done(item)}
								>{completing === item.id ? 'Recording…' : 'Mark done'} <kbd>E</kbd></button
							>{/if}
						{#if canDecline(item)}<a href={`${href(item.id)}&decline=1`}>Not doing this…</a>{/if}
						{#if snoozedNow}
							<button onclick={() => void mark([{ item_id: item.id, snoozed_until: null }])}
								>Wake now</button
							>
						{:else}
							<button onclick={() => snooze(item)}>Snooze for an hour <kbd>H</kbd></button>
						{/if}
						<button
							title="Not needed: off your Inbox, kept under Set aside. Whoever asked stops waiting."
							onclick={() => void notNeeded(item)}>Not needed</button
						>
						<button onclick={() => copy(item)}>Copy link</button>
					</ActionMenu>
				{/snippet}
			</Item>
		{/each}
		{#if loaded && !visible.length && !(coordination && !items.length)}
			<p class="empty">{items.length ? 'Everything here is snoozed.' : 'Nothing needs you.'}</p>
		{:else if !loaded && !failure}
			<div class="waiting" aria-label="Loading the Inbox"><i></i><i></i><i></i></div>
		{/if}
		{#if forYou && (forYou.items.length || forYou.filter === 'unread')}
			<div class="for-you-slot">
				<ForYou
					bind:this={forYouList}
					source={forYou}
					onexit={() => {
						const last = visible.at(-1);
						if (last) select(last);
					}}
				/>
			</div>
		{/if}
		<div class="set-aside-slot"><SetAsideList {companyId} version={items.length} /></div>
	</div>
</div>

<style>
	.set-aside-slot {
		margin: 12px 8px 8px;
	}
	.for-you-slot {
		margin-top: 8px;
		padding-top: 4px;
		border-top: 1px solid var(--border);
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
		padding: 8px 8px 8px 16px;
		border-bottom: 1px solid var(--border);
	}
	h2 {
		margin: 0;
		color: var(--ink);
		font-size: var(--t-head);
		font-weight: 600;
	}
	.inbox-count {
		color: var(--text-tertiary);
		font-variant-numeric: tabular-nums;
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
