<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { onMount, untrack } from 'svelte';
	import CircleHelp from '@lucide/svelte/icons/circle-help';
	import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
	import CircleCheck from '@lucide/svelte/icons/circle-check';
	import GitPullRequest from '@lucide/svelte/icons/git-pull-request';
	import MessageCircle from '@lucide/svelte/icons/message-circle';
	import Eye from '@lucide/svelte/icons/eye';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { completeHandoffHumanStep } from '$lib/model/attention';
	import { refreshAttention } from '$lib/model/queries.svelte';
	import { attentionGroup, attentionTitle } from '$lib/model/attention-presentation';
	import type { AttentionItem } from '$lib/model/view';
	let {
		companyId,
		items,
		selectedId,
		blocker = '',
		loaded = false,
		failure = null,
		onretry,
		onvisible
	}: {
		companyId: string;
		items: AttentionItem[];
		selectedId?: string;
		blocker?: string;
		loaded?: boolean;
		failure?: Error | null;
		onretry?: () => unknown;
		onvisible?: (items: AttentionItem[]) => void;
	} = $props();
	let filter = $state('All');
	let sort = $state('priority');
	let seen = $state<Record<string, string>>({});
	let deferred = $state<Record<string, number>>({});
	let showDeferred = $state(false);
	let notice = $state('');
	let undoId = $state('');
	let help = $state(false);
	const client = useQueryClient();
	let completing = $state('');
	let clock = $state(Date.now());
	const groups = ['Fix setup', 'Decide', 'Approve', 'Review'];
	const deferredCount = $derived(items.filter((item) => (deferred[item.id] ?? 0) > clock).length);
	const visible = $derived.by(() => {
		const rows = items.filter(
			(item) =>
				(showDeferred || (deferred[item.id] ?? 0) <= clock) &&
				(filter === 'All' || attentionGroup(item) === filter)
		);
		return sort === 'newest'
			? [...rows].sort((a, b) => +new Date(b.createdAt) - +new Date(a.createdAt))
			: rows;
	});
	const displayed = $derived(
		groups.flatMap((group) => visible.filter((item) => attentionGroup(item) === group))
	);
	$effect(() => {
		const rows = displayed;
		untrack(() => onvisible?.(rows));
	});
	const stamp = (item: AttentionItem) => String(item.briefedAt ?? item.createdAt);
	const href = (id: string) => `/${encodeURIComponent(companyId)}?item=${encodeURIComponent(id)}`;
	function persist() {
		try {
			localStorage.setItem(`restless:attention:${companyId}`, JSON.stringify({ seen, deferred }));
		} catch {
			/* Browsing still works without storage. */
		}
	}
	onMount(() => {
		try {
			const saved = JSON.parse(localStorage.getItem(`restless:attention:${companyId}`) ?? '{}');
			seen = saved.seen ?? {};
			deferred = saved.deferred ?? {};
		} catch {
			/* Ignore an obsolete preference. */
		}
		const timer = setInterval(() => (clock = Date.now()), 30_000);
		return () => clearInterval(timer);
	});
	$effect(() => {
		const item = items.find((row) => row.id === selectedId);
		if (item && seen[item.id] !== stamp(item)) {
			seen = { ...seen, [item.id]: stamp(item) };
			persist();
		}
	});
	function snooze(item: AttentionItem) {
		deferred = { ...deferred, [item.id]: Date.now() + 3_600_000 };
		persist();
		undoId = item.id;
		notice = 'Hidden for 1 hour on this device';
		if (selectedId === item.id)
			void goto(
				visible.find((row) => row.id !== item.id)
					? href(visible.find((row) => row.id !== item.id)!.id)
					: `/${companyId}`,
				{ noScroll: true }
			);
	}
	async function copy(item: AttentionItem) {
		try {
			await navigator.clipboard.writeText(new URL(href(item.id), location.origin).href);
			notice = 'Link copied';
			undoId = '';
		} catch {
			notice = 'Could not copy the link';
		}
	}
	async function done(item: AttentionItem) {
		if (completing || !item.actions.some((action) => action.id === 'complete-human-step')) return;
		completing = item.id;
		try {
			await completeHandoffHumanStep(companyId, item.source.reference);
			await refreshAttention(client, companyId);
			notice = 'Prepared step completed';
			undoId = '';
		} catch {
			notice = 'The completion was not recorded. Try again.';
		} finally {
			completing = '';
		}
	}
	function keyboard(event: KeyboardEvent) {
		if (
			event.defaultPrevented ||
			(event.target as HTMLElement)?.closest(
				'input, textarea, select, [contenteditable], dialog, #bridge-exrail'
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
		if (step && displayed.length) {
			const index = displayed.findIndex((row) => row.id === selectedId);
			const next = displayed[Math.max(0, Math.min(displayed.length - 1, index + step))];
			event.preventDefault();
			void goto(href(next.id), { keepFocus: true, noScroll: true }).then(() =>
				document.querySelector('.attention-item.selected')?.scrollIntoView({
					block: 'nearest',
					behavior: window.matchMedia('(prefers-reduced-motion: reduce)').matches
						? 'auto'
						: 'smooth'
				})
			);
		}
		if (event.key === '?') {
			event.preventDefault();
			help = !help;
		}
		if (event.key === 'Escape') {
			help = false;
		}
		if (
			event.key === 'e' &&
			item &&
			item.actions.some((action) => action.id === 'complete-human-step')
		) {
			event.preventDefault();
			void done(item);
		}
		if (event.key === 'h' && item) {
			event.preventDefault();
			snooze(item);
		}
		if (event.key === 'c' && item && item.actions.some((action) => action.id === 'chat-lead')) {
			event.preventDefault();
			void goto(`${href(item.id)}&conversation=${encodeURIComponent(item.id)}`);
		}
	}
</script>

<svelte:window onkeydown={keyboard} />
<div class="inbox-toolbar">
	<h2>Attention <span>{loaded ? items.length : '—'}</span></h2>
	<button
		class="inbox-help"
		onclick={() => (help = !help)}
		title="Keyboard shortcuts (?)"
		aria-label="Keyboard shortcuts"><CircleHelp size={15} /></button
	>
	<ActionMenu label="Attention options">
		<button onclick={() => (sort = sort === 'priority' ? 'newest' : 'priority')}
			>Sort: {sort === 'priority' ? 'Priority' : 'Newest'}</button
		>
		<button onclick={() => (showDeferred = !showDeferred)}
			>{showDeferred ? 'Hide snoozed' : `Show snoozed (${deferredCount})`}</button
		>
	</ActionMenu>
</div>
<div class="inbox-filters" aria-label="Filter attention">
	{#each ['All', 'Decide', 'Approve', 'Review', 'Fix setup'] as group}<button
			class:active={filter === group}
			disabled={!loaded}
			aria-pressed={filter === group}
			onclick={() => (filter = group)}>{group === 'Fix setup' ? 'Fix' : group}</button
		>{/each}
</div>
{#if help}<div class="inbox-shortcuts">
		<strong>Keyboard shortcuts</strong><span><kbd>J / K</kbd> Next / previous</span><span
			><kbd>C</kbd> Discuss</span
		><span><kbd>H</kbd> Snooze 1 hour</span><span><kbd>⌘ / Ctrl .</kbd> Copy link</span><span
			><kbd>E</kbd> Complete prepared step</span
		><span><kbd>⌘ / Ctrl ↵</kbd> Record decision</span>
	</div>{/if}
{#if notice}<div class="inbox-notice" role="status">
		{notice}{#if undoId}<button
				onclick={() => {
					deferred = { ...deferred, [undoId]: 0 };
					persist();
					notice = '';
				}}>Undo</button
			>{/if}<button aria-label="Dismiss notification" onclick={() => (notice = '')}>×</button>
	</div>{/if}
<div class="attention-index-scroll">
	{#if failure && !loaded}<div class="inbox-failure">
			<FailureNotice error={failure} subject="Attention" variant="block" {onretry} />
		</div>{/if}
	{#if blocker && !items.some( (item) => item.actions.some((action) => action.id === 'open-intelligence-provider') )}<a
			class="inbox-setup"
			href={`/${companyId}/company/provider`}
			><TriangleAlert size={16} /><span>{blocker}</span><span aria-hidden="true">→</span></a
		>{/if}
	{#each groups as group}
		{@const rows = visible.filter((item) => attentionGroup(item) === group)}
		{#if rows.length}<section class="inbox-group" aria-label={group}>
				<div class="inbox-group-heading">{group}<span>{rows.length}</span></div>
				{#each rows as item (item.id)}
					{@const Icon =
						item.category === 'approval'
							? CircleCheck
							: item.category === 'review'
								? Eye
								: item.category === 'blocker'
									? TriangleAlert
									: item.category === 'conversation'
										? MessageCircle
										: GitPullRequest}
					<div
						class="inbox-row"
						role="group"
						class:selected={selectedId === item.id}
						class:unread={seen[item.id] !== stamp(item)}
						oncontextmenu={(event) => {
							event.preventDefault();
							const menu = event.currentTarget.querySelector('details');
							if (menu) {
								menu.open = true;
								menu.querySelector('summary')?.focus();
							}
						}}
					>
						<a
							class="attention-item category-{item.category}"
							class:selected={selectedId === item.id}
							href={href(item.id)}
							aria-current={selectedId === item.id ? 'true' : undefined}
							title={attentionTitle(item)}
						>
							<span class="inbox-kind" aria-label={group}><Icon size={15} strokeWidth={1.8} /></span
							><strong>{attentionTitle(item)}</strong><span class="inbox-time"
								><RelativeTime value={item.createdAt} /></span
							>
						</a>
						<div class="inbox-row-menu">
							<ActionMenu label={`Actions for ${attentionTitle(item)}`}
								><a href={href(item.id)}>Open</a
								>{#if item.actions.some((action) => action.id === 'chat-lead')}<a
										href={`${href(item.id)}&conversation=${encodeURIComponent(item.id)}`}
										>Discuss with Exec <kbd>C</kbd></a
									>{/if}<button onclick={() => snooze(item)}>Snooze 1 hour <kbd>H</kbd></button
								><button onclick={() => copy(item)}>Copy link</button>
								{#if item.actions.some((action) => action.id === 'complete-human-step')}<button
										disabled={Boolean(completing)}
										onclick={() => done(item)}
										>{completing === item.id ? 'Recording…' : 'Complete prepared step'}
										<kbd>E</kbd></button
									>{/if}
							</ActionMenu>
						</div>
					</div>
				{/each}
			</section>{/if}
	{/each}
	{#if loaded && !visible.length}<p class="inbox-empty">
			{items.length ? 'No items in this view.' : 'All clear.'}
		</p>{:else if !loaded && !failure}<div
			class="attention-list-waiting"
			aria-label="Loading attention"
		>
			<i></i><i></i><i></i>
		</div>{/if}
</div>
<div class="inbox-footer">
	{loaded ? `${visible.length} of ${items.length}` : failure ? 'Unavailable' : 'Loading…'}<span
		title="Navigate with J and K">J / K</span
	>
</div>

<style>
	.inbox-failure {
		display: none;
	}
	@media (max-width: 760px) {
		.inbox-failure {
			display: block;
			padding: 12px;
		}
	}
	.inbox-toolbar {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 12px 12px 4px;
		flex: none;
	}
	h2 {
		margin: 0 auto 0 2px;
		font-size: var(--t-body);
		font-weight: 600;
	}
	h2 span {
		margin-left: 5px;
		color: var(--text-tertiary);
		font-weight: 400;
	}
	.inbox-help {
		display: grid;
		place-items: center;
		width: 28px;
		height: 28px;
		padding: 0;
		border: 0;
		background: transparent;
		color: var(--text-tertiary);
		cursor: pointer;
		border-radius: 4px;
	}
	.inbox-filters {
		display: flex;
		gap: 2px;
		padding: 4px 12px 12px;
		border-bottom: 1px solid var(--border);
		flex: none;
	}
	.inbox-filters button {
		flex: 1 1 auto;
		padding: 5px;
		white-space: nowrap;
		border: 0;
		border-radius: 5px;
		background: transparent;
		font: 500 var(--t-label) var(--font-ui);
		color: var(--text-tertiary);
		cursor: pointer;
	}
	.inbox-filters button.active {
		background: var(--surface-alt);
		color: var(--ink);
	}
	.attention-index-scroll {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		padding: 8px;
	}
	.inbox-group-heading {
		display: flex;
		justify-content: space-between;
		padding: 10px 8px 5px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		font-weight: 500;
	}
	.inbox-row {
		position: relative;
		border-radius: 5px;
	}
	.inbox-row:hover {
		background: var(--surface-alt);
	}
	.inbox-row.selected {
		background: var(--surface-alt);
		box-shadow: inset 2px 0 var(--intent-feedback);
	}
	.attention-item {
		display: grid;
		grid-template-columns: 18px minmax(0, 1fr) auto;
		align-items: center;
		gap: 8px;
		min-height: 52px;
		padding: 9px 8px;
		background: transparent;
		border: 0;
		box-shadow: none;
		color: var(--ink);
		text-decoration: none;
	}
	.attention-item strong {
		font-size: var(--t-body);
		font-weight: 500;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.unread .attention-item strong {
		font-weight: 600;
	}
	.unread .inbox-kind::after {
		content: '';
		position: absolute;
		left: 4px;
		top: 8px;
		width: 4px;
		height: 4px;
		border-radius: 50%;
		background: var(--intent-feedback);
	}
	.inbox-kind {
		display: flex;
		color: var(--text-tertiary);
	}
	.category-approval .inbox-kind {
		color: var(--intent-authority);
	}
	.category-blocker .inbox-kind {
		color: var(--state-danger);
	}
	.category-review .inbox-kind {
		color: var(--intent-feedback);
	}
	.inbox-time {
		max-width: 70px;
		white-space: nowrap;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.inbox-row-menu {
		position: absolute;
		right: 4px;
		top: 10px;
		opacity: 0;
		background: var(--surface-alt);
		border-radius: 5px;
	}
	.inbox-row:hover .inbox-row-menu,
	.inbox-row:focus-within .inbox-row-menu {
		opacity: 1;
	}
	.inbox-footer {
		display: flex;
		justify-content: space-between;
		padding: 10px 16px;
		border-top: 1px solid var(--border);
		color: var(--text-tertiary);
		font-size: var(--t-label);
		flex: none;
	}
	.inbox-setup {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 12px 8px;
		font-size: var(--t-body);
		color: var(--intent-authority);
		text-decoration: none;
	}
	.inbox-setup span:first-of-type {
		flex: 1;
	}
	.inbox-shortcuts {
		display: grid;
		gap: 8px;
		padding: 14px;
		border-bottom: 1px solid var(--border);
		font-size: var(--t-label);
	}
	.inbox-shortcuts span {
		display: flex;
		justify-content: space-between;
		gap: 12px;
		color: var(--text-secondary);
	}
	kbd {
		font: inherit;
		color: var(--text-tertiary);
	}
	.inbox-notice {
		display: flex;
		gap: 6px;
		align-items: center;
		padding: 10px 12px;
		font-size: var(--t-label);
		background: var(--surface-alt);
	}
	.inbox-notice button {
		border: 0;
		background: none;
		color: var(--intent-feedback);
		cursor: pointer;
	}
	.inbox-notice button:last-child {
		margin-left: auto;
	}
	.inbox-empty {
		padding: 24px 8px;
		color: var(--text-tertiary);
		font-size: var(--t-body);
		text-align: center;
	}
	:where(a, button):focus-visible {
		outline: 2px solid var(--intent-feedback);
		outline-offset: -2px;
	}
	@media (hover: none) {
		.inbox-row-menu {
			opacity: 1;
		}
		.inbox-time {
			margin-right: 28px;
		}
	}
</style>
