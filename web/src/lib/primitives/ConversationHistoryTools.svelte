<script lang="ts">
	import { tick } from 'svelte';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import ChevronUp from '@lucide/svelte/icons/chevron-up';
	import History from '@lucide/svelte/icons/history';
	import Search from '@lucide/svelte/icons/search';
	import X from '@lucide/svelte/icons/x';
	import type { ThreadMessage } from '$lib/model/view';

	let {
		messages,
		participantName,
		onjump
	}: {
		messages: ThreadMessage[];
		participantName: string;
		onjump: (messageId: string) => void;
	} = $props();

	let open = $state(false);
	let query = $state('');
	let activeMatch = $state(-1);
	let searchInput = $state<HTMLInputElement | undefined>();
	const normalizedQuery = $derived(query.trim().toLocaleLowerCase());
	const matches = $derived(
		normalizedQuery
			? messages.filter((message) =>
					[message.author, message.text, message.contextPath ?? '']
						.join('\n')
						.toLocaleLowerCase()
						.includes(normalizedQuery)
				)
			: []
	);

	async function toggle() {
		open = !open;
		if (!open) return;
		await tick();
		searchInput?.focus();
	}

	function setQuery(value: string) {
		query = value;
		activeMatch = -1;
	}

	function jump(direction: -1 | 1) {
		if (!matches.length) return;
		activeMatch =
			activeMatch < 0
				? direction < 0
					? matches.length - 1
					: 0
				: (activeMatch + direction + matches.length) % matches.length;
		onjump(matches[activeMatch].id);
	}

	function onSearchKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			event.preventDefault();
			jump(event.shiftKey ? -1 : 1);
		} else if (event.key === 'Escape') {
			open = false;
		}
	}
</script>

<div class="history-tools" class:open>
	<button
		type="button"
		class="history-toggle"
		aria-expanded={open}
		aria-label={`Search conversation history with ${participantName}`}
		title="Conversation history"
		onclick={toggle}
	>
		<History size={15} strokeWidth={1.9} aria-hidden="true" />
		<span>History</span>
	</button>

	{#if open}
		<div class="history-panel" role="search" aria-label={`${participantName} conversation history`}>
			<label class="history-search">
				<Search size={14} strokeWidth={2} aria-hidden="true" />
				<input
					bind:this={searchInput}
					value={query}
					oninput={(event) => setQuery(event.currentTarget.value)}
					onkeydown={onSearchKeydown}
					placeholder="Search this conversation"
					aria-label="Search this conversation"
				/>
				{#if query}
					<button
						type="button"
						aria-label="Clear search"
						title="Clear"
						onclick={() => setQuery('')}
					>
						<X size={13} strokeWidth={2} aria-hidden="true" />
					</button>
				{/if}
			</label>
			<div class="history-status">
				<span aria-live="polite">
					{#if normalizedQuery}
						{matches.length
							? activeMatch < 0
								? `${matches.length} matches`
								: `${activeMatch + 1} of ${matches.length}`
							: 'No matches'}
					{:else}
						{messages.length} message{messages.length === 1 ? '' : 's'} in this record
					{/if}
				</span>
				<div class="history-nav">
					<button
						type="button"
						aria-label="Previous match"
						disabled={!matches.length}
						onclick={() => jump(-1)}
					>
						<ChevronUp size={14} strokeWidth={2} aria-hidden="true" />
					</button>
					<button
						type="button"
						aria-label="Next match"
						disabled={!matches.length}
						onclick={() => jump(1)}
					>
						<ChevronDown size={14} strokeWidth={2} aria-hidden="true" />
					</button>
				</div>
			</div>
		</div>
	{/if}
</div>

<style>
	.history-tools {
		position: relative;
		flex: none;
	}

	.history-toggle {
		display: flex;
		align-items: center;
		gap: 5px;
		padding: 5px 7px;
		border: 1px solid transparent;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-tertiary);
		font: 600 var(--t-label) var(--font-ui);
		cursor: pointer;
	}

	.history-toggle:hover,
	.history-toggle:focus-visible,
	.open .history-toggle {
		border-color: var(--border-strong);
		background: color-mix(in srgb, var(--surface-alt) 74%, transparent);
		color: var(--text-secondary);
	}

	.history-toggle:focus-visible,
	.history-panel button:focus-visible,
	.history-search:focus-within {
		outline: 2px solid color-mix(in srgb, var(--intent-conversation) 34%, transparent);
		outline-offset: 1px;
	}

	.history-panel {
		position: absolute;
		z-index: var(--z-raised);
		top: calc(100% + 8px);
		right: 0;
		width: min(310px, calc(100vw - 32px));
		padding: 8px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-pane);
		background: var(--surface-rail);
		box-shadow: var(--shadow-lift);
		animation: bridge-popover-in var(--motion-disclosure) var(--ease-out) both;
	}

	.history-search {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto;
		align-items: center;
		gap: 7px;
		padding: 7px 8px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: var(--surface-alt);
		color: var(--text-tertiary);
	}

	.history-search input {
		min-width: 0;
		padding: 0;
		border: 0;
		outline: 0;
		background: transparent;
		color: var(--ink);
		font: 500 var(--t-body) var(--font-ui);
	}

	.history-search input::placeholder {
		color: var(--text-tertiary);
	}

	.history-search button,
	.history-nav button {
		display: grid;
		place-items: center;
		padding: 0;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-tertiary);
		cursor: pointer;
	}

	.history-search button {
		width: 20px;
		height: 20px;
	}

	.history-status {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		padding: 7px 2px 0 6px;
		color: var(--text-tertiary);
		font: 500 var(--t-label) var(--font-mono);
	}

	.history-nav {
		display: flex;
		align-items: center;
		gap: 2px;
	}

	.history-nav button {
		width: 24px;
		height: 22px;
	}

	.history-nav button:hover:not(:disabled) {
		background: var(--surface-alt);
		color: var(--ink);
	}

	.history-nav button:disabled {
		opacity: 0.32;
		cursor: default;
	}
</style>
