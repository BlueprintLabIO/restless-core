<script module lang="ts">
	export interface ArtifactLink {
		id: string;
		title: string;
		href: string;
		updatedAt: string;
		hint?: string;
	}
</script>

<script lang="ts">
	import { onDestroy, onMount, tick, type Snippet } from 'svelte';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import FileText from '@lucide/svelte/icons/file-text';
	import Sheet from '@lucide/svelte/icons/sheet';
	import Plus from '@lucide/svelte/icons/plus';
	import Search from '@lucide/svelte/icons/search';
	import X from '@lucide/svelte/icons/x';
	import ArrowDownWideNarrow from '@lucide/svelte/icons/arrow-down-wide-narrow';
	import RefreshCw from '@lucide/svelte/icons/refresh-cw';

	let {
		kind,
		items,
		selected,
		workHref,
		docsHref,
		sheetsHref,
		loading = false,
		failure = '',
		creating = false,
		createDisabled = false,
		hasMore = false,
		loadingMore = false,
		searching = false,
		filterLocally = true,
		queryByteLimit,
		onquery,
		oncreate,
		onretry,
		onloadmore,
		onopen,
		creation
	}: {
		kind: 'docs' | 'sheets';
		items: ArtifactLink[];
		selected: string;
		workHref: string;
		docsHref: string;
		sheetsHref: string;
		loading?: boolean;
		failure?: string;
		creating?: boolean;
		createDisabled?: boolean;
		hasMore?: boolean;
		loadingMore?: boolean;
		oncreate: () => void;
		searching?: boolean;
		filterLocally?: boolean;
		queryByteLimit?: number;
		onquery?: (value: string) => void;
		onretry?: () => void;
		onloadmore?: () => void;
		onopen?: (item: ArtifactLink) => void | Promise<void>;
		creation?: Snippet;
	} = $props();
	const label = $derived(kind === 'docs' ? 'documents' : 'sheets');
	const singular = $derived(kind === 'docs' ? 'document' : 'sheet');
	let query = $state('');
	let order = $state<'recent' | 'name'>('recent');
	let activeId = $state('');
	let root: HTMLElement,
		input: HTMLInputElement,
		list: HTMLElement,
		createButton: HTMLButtonElement;
	let returnFocus: HTMLElement | null = null;
	let disposed = false;
	const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' });
	const visible = $derived.by(() => {
		const words = query.trim().toLocaleLowerCase().split(/\s+/u).filter(Boolean);
		return items
			.filter(
				(item) =>
					!filterLocally || words.every((word) => item.title.toLocaleLowerCase().includes(word))
			)
			.toSorted(
				(a, b) =>
					(order === 'recent'
						? (Date.parse(b.updatedAt) || 0) - (Date.parse(a.updatedAt) || 0)
						: 0) ||
					collator.compare(a.title, b.title) ||
					a.id.localeCompare(b.id)
			);
	});
	onDestroy(() => {
		disposed = true;
	});
	onMount(() => {
		root.addEventListener('keydown', keydown);
		return () => root.removeEventListener('keydown', keydown);
	});
	$effect(() => {
		onquery?.(query);
	});
	$effect(() => {
		if (!visible.some((item) => item.id === activeId)) {
			activeId = visible.find((item) => item.id === selected)?.id ?? visible[0]?.id ?? '';
		}
	});
	$effect(() => {
		const id = selected;
		visible;
		void tick().then(() => {
			if (disposed || !root?.getClientRects().length) return;
			const focused = list?.contains(document.activeElement) ? document.activeElement : row(id);
			focused?.scrollIntoView({ block: 'nearest' });
		});
	});
	function row(id: string): HTMLAnchorElement | undefined {
		return [...(list?.querySelectorAll<HTMLAnchorElement>('[data-artifact-id]') ?? [])].find(
			(node) => node.dataset.artifactId === id
		);
	}
	function focusRow(index: number) {
		const item = visible[Math.max(0, Math.min(index, visible.length - 1))];
		if (!item) return;
		activeId = item.id;
		void tick().then(() => {
			if (!disposed) {
				row(item.id)?.focus();
				row(item.id)?.scrollIntoView({ block: 'nearest' });
			}
		});
	}
	function focusSearch() {
		returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
		input?.focus();
	}
	function limitQuery() {
		if (!queryByteLimit) return;
		let value = '',
			bytes = 0;
		for (const character of input.value) {
			bytes += new TextEncoder().encode(character).length;
			if (bytes > queryByteLimit) break;
			value += character;
		}
		input.value = value;
		query = value;
	}
	function open(event: MouseEvent, item: ArtifactLink) {
		// Keep real links for new tabs, context menus, copying and assistive technology.
		if (
			!onopen ||
			event.defaultPrevented ||
			event.button !== 0 ||
			event.metaKey ||
			event.ctrlKey ||
			event.altKey ||
			event.shiftKey
		)
			return;
		event.preventDefault();
		void onopen(item);
	}
	function keydown(event: KeyboardEvent) {
		if (
			event.defaultPrevented ||
			event.isComposing ||
			event.metaKey ||
			event.ctrlKey ||
			event.altKey
		)
			return;
		const target = event.target as HTMLElement;
		if (event.key === 'Escape' && creating && !createDisabled) {
			event.preventDefault();
			oncreate();
			void tick().then(() => {
				if (!disposed) createButton?.focus();
			});
			return;
		}
		const editing = target.closest('input, textarea, select, [contenteditable]');
		// This shortcut belongs to navigation, never the document or workbook editor.
		if (event.key === '/' && !editing && !event.shiftKey) {
			event.preventDefault();
			focusSearch();
			return;
		}
		const inSearch = target === input;
		const inRow = Boolean(target.closest('[data-artifact-id]'));
		if (!inSearch && !inRow) return;
		const index = visible.findIndex((item) => item.id === activeId);
		if (
			event.key === 'ArrowDown' ||
			event.key === 'ArrowUp' ||
			(inRow && (event.key === 'Home' || event.key === 'End'))
		) {
			event.preventDefault();
			focusRow(
				event.key === 'Home'
					? 0
					: event.key === 'End'
						? visible.length - 1
						: inSearch
							? event.key === 'ArrowDown'
								? 0
								: visible.length - 1
							: index + (event.key === 'ArrowDown' ? 1 : -1)
			);
		} else if (event.key === 'Enter' && inSearch) {
			event.preventDefault();
			row(activeId || visible[0]?.id)?.click();
		} else if (event.key === 'Escape') {
			event.preventDefault();
			if (query) {
				query = '';
				input.focus();
			} else if (inRow) focusSearch();
			else if (returnFocus?.isConnected && root.contains(returnFocus) && returnFocus !== input)
				returnFocus.focus();
			else (row(selected) ?? row(activeId) ?? row(visible[0]?.id) ?? createButton)?.focus();
		}
	}
</script>

<aside class="artifact-sidebar cockpit-pane" aria-label={`Company ${label}`} bind:this={root}>
	<header class="artifact-head">
		<a class="work-link" href={workHref} title="Back to the Library"
			><ArrowLeft size={14} aria-hidden="true" /><span>Library</span></a
		>
		<button
			bind:this={createButton}
			class="create-control"
			type="button"
			disabled={createDisabled}
			aria-expanded={creating}
			title={creating ? `Cancel new ${singular}` : `Create a ${singular}`}
			onclick={() => {
				query = '';
				oncreate();
			}}
		>
			{#if creating}<X size={14} aria-hidden="true" />{:else}<Plus
					size={14}
					aria-hidden="true"
				/>{/if}
			<span>{creating ? 'Cancel' : 'New'}</span>
		</button>
	</header>
	<nav class="artifact-switcher" aria-label="Library">
		<a
			href={docsHref}
			class:current={kind === 'docs'}
			aria-current={kind === 'docs' ? 'page' : undefined}
			><FileText size={15} aria-hidden="true" />Docs</a
		>
		<a
			href={sheetsHref}
			class:current={kind === 'sheets'}
			aria-current={kind === 'sheets' ? 'page' : undefined}
			><Sheet size={15} aria-hidden="true" />Sheets</a
		>
	</nav>
	{#if creating && creation}<div class="artifact-creation">{@render creation()}</div>{/if}
	<div class="artifact-tools">
		<label class="artifact-search">
			<Search size={14} aria-hidden="true" /><span class="sr-only">Search {label}</span>
			<input
				bind:this={input}
				bind:value={query}
				type="search"
				placeholder={`Search ${label}…`}
				autocomplete="off"
				spellcheck="false"
				aria-label={`Search ${label}`}
				aria-describedby="artifact-keyboard-help"
				oninput={limitQuery}
			/>
			{#if query}<button
					type="button"
					aria-label="Clear search"
					title="Clear search"
					onclick={() => {
						query = '';
						input.focus();
					}}><X size={13} aria-hidden="true" /></button
				>
			{:else}<kbd aria-hidden="true" title="Press / while focus is in this sidebar to search">/</kbd
				>{/if}
		</label>
		<div class="artifact-order">
			<span aria-live="polite"
				>{searching
					? 'Searching…'
					: query.trim()
						? `${visible.length} ${visible.length === 1 ? 'match' : 'matches'}`
						: `${items.length} ${label}`}</span
			>
			<label
				title="Order the loaded list or search results by last update or name. Load more to include older items."
				><ArrowDownWideNarrow size={13} aria-hidden="true" /><span class="sr-only"
					>Sort {label}</span
				>
				<select bind:value={order}
					><option value="recent">Recent</option><option value="name">Name</option></select
				>
			</label>
		</div>
	</div>
	<nav
		class="artifact-list"
		aria-label={kind === 'docs' ? 'Documents' : 'Spreadsheets'}
		aria-busy={loading}
		bind:this={list}
	>
		{#each visible as item (item.id)}
			<a
				class="artifact-row"
				class:selected={selected === item.id}
				href={item.href}
				title={`${item.title}${item.hint ? ` — ${item.hint}` : ''}`}
				aria-current={selected === item.id ? 'page' : undefined}
				data-artifact-id={item.id}
				tabindex={activeId === item.id ? 0 : -1}
				onfocus={() => (activeId = item.id)}
				onclick={(event) => open(event, item)}
			>
				{#if kind === 'docs'}<FileText
						size={15}
						strokeWidth={1.7}
						aria-hidden="true"
					/>{:else}<Sheet size={15} strokeWidth={1.7} aria-hidden="true" />{/if}
				<span>{item.title}</span>
			</a>
		{:else}
			{#if loading}<div class="artifact-loading" role="status" aria-label={`Loading ${label}`}>
					<i></i><i></i><i></i>
				</div>
			{:else if failure}<div class="artifact-empty" role="alert">
					<span>{failure}</span><button type="button" onclick={onretry}
						><RefreshCw size={13} aria-hidden="true" />Try again</button
					>
				</div>
			{:else if query.trim()}<div class="artifact-empty">
					<span>No {label} match “{query.trim()}”.</span><button
						type="button"
						onclick={() => {
							query = '';
							input.focus();
						}}>Clear search</button
					>
				</div>
			{:else}<div class="artifact-empty">
					<span>No {label} yet.</span><button
						type="button"
						disabled={createDisabled}
						onclick={oncreate}><Plus size={13} aria-hidden="true" />New {singular}</button
					>
				</div>{/if}
		{/each}
	</nav>
	{#if hasMore}<button class="load-more" type="button" disabled={loadingMore} onclick={onloadmore}
			>{loadingMore
				? 'Loading…'
				: query.trim()
					? 'More search results'
					: `Load older ${label}`}</button
		>{/if}
	{#if failure && visible.length}<div class="artifact-stale" role="status">
			<span>List could not refresh.</span><button type="button" onclick={onretry}>Retry</button>
		</div>{/if}
	<footer
		id="artifact-keyboard-help"
		title="In this sidebar: / searches, ↑/↓ and Home/End move, Enter opens. Escape clears search, then returns focus. Cmd/Ctrl+K still opens company search."
	>
		<span><kbd>↑</kbd><kbd>↓</kbd> Navigate</span><span><kbd>↵</kbd> Open</span><span
			><kbd>esc</kbd> Back</span
		>
	</footer>
</aside>

<style>
	.artifact-sidebar {
		min-width: 0;
		min-height: 0;
		height: 100%;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		background: var(--surface-rail);
	}
	a,
	a:hover,
	a:focus {
		color: inherit;
		text-decoration: none;
	}
	.artifact-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 12px 12px 9px;
		gap: 8px;
	}
	.work-link {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		min-height: 28px;
		color: var(--text-secondary);
		font-weight: 500;
	}
	.work-link:hover {
		color: var(--ink);
	}
	button {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 5px;
		font: inherit;
		cursor: pointer;
	}
	.create-control {
		min-height: 28px;
		padding: 4px 9px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		box-shadow: var(--control-depth);
		color: var(--ink);
		font-weight: 600;
	}
	.create-control:hover {
		border-color: var(--control-edge-hover);
		background: var(--surface);
	}
	.create-control:active {
		transform: translateY(1px);
		box-shadow: var(--control-depth-pressed);
	}
	button:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.artifact-switcher {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 3px;
		padding: 3px;
		margin: 0 10px 12px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface-alt);
	}
	.artifact-switcher a {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 6px;
		min-height: 30px;
		border-radius: calc(var(--radius-control) - 1px);
		color: var(--text-secondary);
		font-weight: 500;
	}
	.artifact-switcher a:hover {
		background: var(--surface);
		color: var(--ink);
	}
	.artifact-switcher a.current {
		background: var(--surface-raised);
		color: var(--ink);
		box-shadow: 0 1px 3px rgba(31, 41, 64, 0.08);
	}
	.artifact-tools {
		padding: 0 10px 5px;
	}
	.artifact-search {
		display: flex;
		align-items: center;
		gap: 7px;
		min-width: 0;
		padding: 0 8px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface);
		color: var(--text-tertiary);
	}
	.artifact-search:focus-within {
		border-color: var(--accent);
		box-shadow: 0 0 0 2px var(--accent-soft);
	}
	.artifact-search input {
		min-width: 0;
		width: 100%;
		height: 32px;
		padding: 0;
		border: 0;
		outline: 0;
		background: transparent;
		color: var(--ink);
		font: inherit;
	}
	.artifact-search input::-webkit-search-cancel-button {
		display: none;
	}
	.artifact-search input::placeholder {
		color: var(--text-tertiary);
	}
	.artifact-search button {
		border: 0;
		padding: 3px;
		background: transparent;
		color: var(--text-secondary);
	}
	.artifact-order {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		min-height: 32px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.artifact-order label {
		display: flex;
		align-items: center;
		gap: 3px;
	}
	.artifact-order select {
		max-width: 90px;
		padding: 3px 1px;
		border: 0;
		background: transparent;
		color: var(--text-secondary);
		font: inherit;
		cursor: pointer;
	}
	.artifact-list {
		min-height: 0;
		flex: 1;
		overflow: auto;
		padding: 0 7px 8px;
		overscroll-behavior: contain;
	}
	.artifact-row {
		display: flex;
		align-items: center;
		gap: 9px;
		min-height: 35px;
		padding: 7px 9px;
		margin: 1px 0;
		border-radius: var(--radius-control);
		color: var(--text-secondary);
		transition: background-color var(--motion-state) var(--ease-standard);
	}
	.artifact-row :global(svg) {
		flex: none;
		color: var(--text-tertiary);
	}
	.artifact-row span {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.artifact-row:hover {
		background: var(--surface-alt);
		color: var(--ink);
	}
	.artifact-row.selected {
		background: var(--accent-soft);
		color: var(--ink);
		font-weight: 500;
		box-shadow: inset 2px 0 0 var(--accent);
	}
	.artifact-row.selected :global(svg) {
		color: var(--accent);
	}
	a:focus-visible,
	button:focus-visible,
	select:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}
	.artifact-row:focus-visible {
		outline-offset: -2px;
		background: var(--surface-alt);
	}
	.artifact-empty {
		display: grid;
		justify-items: start;
		gap: 10px;
		padding: 18px 9px;
		color: var(--text-secondary);
	}
	.artifact-empty button,
	.load-more {
		padding: 5px 8px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface);
		color: var(--ink);
	}
	.artifact-loading {
		display: grid;
		gap: 8px;
		padding: 5px 9px;
	}
	.artifact-loading i {
		height: 27px;
		border-radius: var(--radius-control);
		background: var(--surface-alt);
	}
	.artifact-loading i:nth-child(2) {
		width: 80%;
	}
	.artifact-loading i:nth-child(3) {
		width: 65%;
	}
	.load-more {
		margin: 0 10px 9px;
	}
	.artifact-stale {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 6px;
		padding: 8px 12px;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.artifact-stale button {
		border: 0;
		background: transparent;
		color: var(--accent);
	}
	footer {
		display: flex;
		justify-content: space-between;
		gap: 6px;
		padding: 10px 12px;
		border-top: 1px solid var(--border);
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	footer span {
		display: inline-flex;
		align-items: center;
		gap: 3px;
		white-space: nowrap;
	}
	kbd {
		font: var(--t-label) var(--font-mono);
		color: var(--text-tertiary);
	}
	@media (max-width: 820px) {
		.artifact-head {
			padding: 12px;
		}
		.create-control,
		.work-link {
			min-height: 36px;
		}
		.artifact-switcher a {
			min-height: 36px;
		}
		.artifact-search input {
			height: 38px;
		}
		.artifact-row {
			min-height: 44px;
		}
		.artifact-empty button,
		.load-more {
			min-height: 38px;
		}
	}
	@media (pointer: coarse) {
		footer {
			display: none;
		}
	}
</style>
