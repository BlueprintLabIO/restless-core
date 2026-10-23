<script module lang="ts">
	export type Command = {
		id: string;
		group: string;
		label: string;
		/** Secondary text: where this goes or what it is. Searched as well. */
		hint?: string;
		href?: string;
		run?: () => void;
		keywords?: string;
		/** The hint is a keyboard shortcut, so touch screens leave it out. */
		shortcut?: boolean;
	};
	/** Results found elsewhere for what was typed, such as text inside documents. They are already
	 * matched, so they are shown as found rather than filtered again. `unreached` names the sources
	 * that could not be searched ("documents"), so the menu never reads their silence as no match. */
	export type CommandSearchResult = { results: Command[]; unreached: string[] };
	export type CommandSearch = (query: string, signal: AbortSignal) => Promise<CommandSearchResult>;
</script>

<script lang="ts">
	/* One place to jump to anything in the company: what was open lately, surfaces, settings
	 * pages, Work, goals, people, conversations and other companies. It replaces hunting through
	 * the switcher, the settings spine and the Work map when the person already knows what they
	 * want. */
	import { tick } from 'svelte';
	import { goto } from '$app/navigation';
	import Search from '@lucide/svelte/icons/search';
	import CornerDownLeft from '@lucide/svelte/icons/corner-down-left';
	import { composing } from '$lib/primitives/composer-keys';
	import { matches } from '$lib/search/search';

	let {
		open = $bindable(false),
		commands,
		recents = [],
		search = null,
		ask = null
	}: {
		open?: boolean;
		commands: Command[];
		/** What this person opened lately, shown first before anything is typed. */
		recents?: Command[];
		search?: CommandSearch | null;
		/** Hands the typed words to Exec: always offered last while something is typed. */
		ask?: ((query: string) => void) | null;
	} = $props();
	/* What was typed and found nothing is a question; Exec is the one to ask. */
	const ASK_ID = 'ask:exec';

	let dialog: HTMLDialogElement | undefined = $state();
	let input: HTMLInputElement | undefined = $state();
	let list: HTMLElement | undefined = $state();
	let query = $state('');
	let active = $state(0);
	let returnFocus: HTMLElement | null = null;

	const PER_GROUP = 8;
	/* A group's "more" opens the rest of it in place; typing again folds it back. */
	let expanded = $state<string[]>([]);
	/* The exit plays before the dialog closes. */
	let leaving = $state(false);

	/* A short pause after typing, then one search; a newer query cancels the older one. Earlier
	 * results stay until newer ones arrive, so the list does not flicker while typing. */
	let found = $state<Command[]>([]);
	let unreached = $state<string[]>([]);
	let searching = $state(false);
	$effect(() => {
		const text = query.trim();
		if (!open || !search || !text) {
			found = [];
			unreached = [];
			searching = false;
			return;
		}
		const run = search;
		const abort = new AbortController();
		searching = true;
		const timer = setTimeout(() => {
			run(text, abort.signal)
				.then((answer) => {
					if (abort.signal.aborted) return;
					found = answer.results;
					unreached = answer.unreached;
				})
				.catch(() => {
					if (abort.signal.aborted) return;
					found = [];
					unreached = ['the company'];
				})
				.finally(() => {
					if (!abort.signal.aborted) searching = false;
				});
		}, 180);
		return () => {
			clearTimeout(timer);
			abort.abort();
		};
	});

	const results = $derived.by(() => {
		const words = query.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
		const pool = words.length ? commands : [...recents, ...commands];
		const hits = pool.filter((command) =>
			matches(query, [command.label, command.hint, command.group, command.keywords])
		);
		/* Label-prefix matches first; otherwise keep the author's order. */
		const first = words[0];
		const ranked = first
			? hits.toSorted(
					(a, b) =>
						Number(!a.label.toLocaleLowerCase().startsWith(first)) -
						Number(!b.label.toLocaleLowerCase().startsWith(first))
				)
			: hits;
		/* Found results follow the label-prefix matches and lead everything else, so a phrase from
		 * inside a document surfaces that document near the top. */
		const listed = new Set(ranked.map((command) => command.id));
		const prefixed = first
			? ranked.findIndex((command) => !command.label.toLocaleLowerCase().startsWith(first))
			: 0;
		const split = prefixed < 0 ? ranked.length : prefixed;
		const groups = new Map<string, { items: Command[]; total: number }>();
		for (const command of [
			...ranked.slice(0, split),
			...found.filter((command) => !listed.has(command.id)),
			...ranked.slice(split)
		]) {
			const group = groups.get(command.group) ?? { items: [], total: 0 };
			if (group.items.length < PER_GROUP || expanded.includes(command.group))
				group.items.push(command);
			group.total += 1;
			groups.set(command.group, group);
		}
		const sections = [...groups].map(([group, { items, total }]) => ({
			group,
			items,
			more: total - items.length
		}));
		const text = query.trim();
		if (ask && text)
			sections.push({
				group: 'Ask',
				items: [
					{
						id: ASK_ID,
						group: 'Ask',
						label: `Ask Exec about “${text}”`,
						run: () => ask?.(text)
					}
				],
				more: 0
			});
		return sections;
	});
	const flat = $derived(results.flatMap((group) => group.items));
	/* Matches, not counting the standing offer to ask Exec. */
	const matched = $derived(flat.filter((command) => command.id !== ASK_ID).length);
	const unreachedLine = $derived(
		unreached.length
			? `Couldn’t search ${
					unreached.length === 1
						? unreached[0]
						: `${unreached.slice(0, -1).join(', ')} and ${unreached.at(-1)}`
				} right now.`
			: ''
	);
	const emptyLine = $derived(
		searching
			? 'Searching…'
			: unreachedLine
				? `${unreachedLine} Nothing else matched “${query.trim()}”.`
				: `No match for “${query.trim()}”.`
	);

	$effect(() => {
		void query;
		active = 0;
		expanded = [];
	});

	function expand(group: string) {
		const first = results.find((section) => section.group === group)?.items.length ?? 0;
		expanded = [...expanded, group];
		/* The keyboard lands on the first row that was hidden. */
		void tick().then(() => {
			const before = results.slice(
				0,
				results.findIndex((section) => section.group === group)
			);
			active = before.reduce((total, section) => total + section.items.length, 0) + first;
			input?.focus();
			list?.querySelector(`[data-index="${active}"]`)?.scrollIntoView({ block: 'nearest' });
		});
	}

	$effect(() => {
		if (!dialog) return;
		if (open && !dialog.open) {
			returnFocus = document.activeElement as HTMLElement | null;
			query = '';
			active = 0;
			expanded = [];
			dialog.showModal();
			void tick().then(() => input?.focus());
		} else if (!open && dialog.open) {
			leave();
		}
	});

	/* Close with the panel's exit animation rather than cutting it. Under reduced motion the duration
	 * tokens are 1ms, so the same path settles at once; the media check just skips the wait. */
	function leave() {
		if (!dialog || leaving) return;
		const panel = dialog.querySelector<HTMLElement>('.command-panel');
		if (!panel || window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
			dialog.close();
			return;
		}
		leaving = true;
		const finish = () => {
			leaving = false;
			if (dialog?.open) dialog.close();
		};
		panel.addEventListener('animationend', finish, { once: true });
		window.setTimeout(finish, 400);
	}

	/* Focus goes back where it was once the dialog has closed: while it is open the page behind it
	 * is inert and cannot take focus. */
	function close() {
		open = false;
	}

	function choose(command: Command | undefined) {
		if (!command) return;
		close();
		if (command.run) command.run();
		else if (command.href) void goto(command.href);
	}

	function move(delta: number) {
		if (!flat.length) return;
		active = (active + delta + flat.length) % flat.length;
		void tick().then(() =>
			list?.querySelector(`[data-index="${active}"]`)?.scrollIntoView({ block: 'nearest' })
		);
	}

	function onkeydown(event: KeyboardEvent) {
		if (event.key === 'ArrowDown' || (event.ctrlKey && event.key === 'n')) {
			event.preventDefault();
			move(1);
		} else if (event.key === 'ArrowUp' || (event.ctrlKey && event.key === 'p')) {
			event.preventDefault();
			move(-1);
		} else if (event.key === 'Enter') {
			/* Enter that confirms an input method's conversion is not a choice. */
			if (composing(event)) return;
			event.preventDefault();
			choose(flat[active]);
		}
	}
</script>

<!-- Carries its own design tokens: account pages mount it outside the cockpit's token root. -->
<dialog
	bind:this={dialog}
	class="command-menu bridge-tokens"
	data-closing={leaving ? '' : undefined}
	aria-label="Search and jump"
	onclose={() => {
		open = false;
		if (returnFocus?.isConnected) returnFocus.focus();
	}}
	oncancel={(event) => {
		// Escape takes the animated exit instead of the browser's instant close.
		event.preventDefault();
		close();
	}}
	onclick={(event) => {
		if (event.target === dialog) close();
	}}
>
	<div class="command-panel">
		<label class="command-search">
			<Search size={16} strokeWidth={2} aria-hidden="true" />
			<input
				bind:this={input}
				bind:value={query}
				{onkeydown}
				type="text"
				placeholder="Search documents, conversations, people and settings…"
				aria-label="Search"
				role="combobox"
				aria-expanded="true"
				aria-controls="command-results"
				aria-activedescendant={flat.length ? `command-${active}` : undefined}
				autocomplete="off"
				spellcheck="false"
			/>
			<kbd>esc</kbd>
			<button type="button" class="command-cancel" onclick={close}>Cancel</button>
		</label>
		<p class="sr-only" role="status" aria-live="polite">
			{matched ? `${matched} ${matched === 1 ? 'result' : 'results'}` : 'No results'}
			{unreachedLine}
		</p>
		<div class="command-results" id="command-results" role="listbox" bind:this={list}>
			{#if open && query.trim() && !matched && results.length}
				<p class="command-empty">{emptyLine}</p>
			{/if}
			<!-- The menu is in every company page; its results render only while it is open. -->
			{#each open ? results : [] as section (section.group)}
				<div class="command-group" role="group" aria-label={section.group}>
					<div class="command-group-label" aria-hidden="true">{section.group}</div>
					{#each section.items as command (command.id)}
						{@const index = flat.indexOf(command)}
						<button
							type="button"
							id={`command-${index}`}
							class="command-item"
							class:active={index === active}
							role="option"
							aria-selected={index === active}
							data-index={index}
							tabindex="-1"
							onpointermove={() => (active = index)}
							onclick={() => choose(command)}
						>
							<span class="command-label">{command.label}</span>
							{#if command.hint}<span class="command-hint" class:shortcut={command.shortcut}
									>{command.hint}</span
								>{/if}
							<CornerDownLeft class="command-enter" size={13} aria-hidden="true" />
						</button>
					{/each}
					{#if section.more > 0}
						<button
							type="button"
							class="command-more"
							tabindex="-1"
							onclick={() => expand(section.group)}>Show {section.more} more</button
						>
					{/if}
				</div>
			{:else}
				<p class="command-empty">{emptyLine}</p>
			{/each}
			{#if open && matched && unreachedLine && !searching}
				<p class="command-unreached">{unreachedLine}</p>
			{/if}
		</div>
		<footer class="command-foot" aria-hidden="true">
			<span><kbd>↑</kbd><kbd>↓</kbd> move</span>
			<span><kbd>↵</kbd> open</span>
		</footer>
	</div>
</dialog>

<style>
	.command-menu {
		width: min(640px, calc(100vw - 24px));
		max-width: none;
		max-height: min(560px, calc(100dvh - 24vh));
		margin: 14vh auto auto;
		padding: 0;
		border: 0;
		background: transparent;
		overflow: visible;
		color: var(--ink);
	}

	.command-menu::backdrop {
		background: rgba(34, 39, 51, 0.18);
		backdrop-filter: blur(2px);
		animation: command-fade var(--motion-state) var(--ease-standard) both;
	}

	.command-menu[open] .command-panel {
		animation: bridge-popover-in var(--motion-disclosure) var(--ease-spring) both;
	}

	.command-menu[data-closing] .command-panel {
		animation: bridge-popover-out var(--motion-state) var(--ease-standard) both;
	}

	.command-menu[data-closing]::backdrop {
		animation: command-fade-out var(--motion-state) var(--ease-standard) both;
	}

	.command-panel {
		max-height: inherit;
		display: grid;
		grid-template-rows: auto minmax(0, 1fr) auto;
		overflow: hidden;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--glass-strong);
		box-shadow: var(--bevel), var(--shadow-float);
		backdrop-filter: blur(24px) saturate(1.2);
		-webkit-backdrop-filter: blur(24px) saturate(1.2);
	}

	.command-search {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 0 14px;
		border-bottom: 1px solid var(--border);
		color: var(--text-tertiary);
	}

	.command-search input {
		flex: 1;
		min-width: 0;
		height: 50px;
		border: 0;
		outline: none;
		background: transparent;
		color: var(--ink);
		font-size: var(--t-head);
	}

	.command-search input::placeholder {
		color: var(--text-tertiary);
	}

	.command-results {
		overflow: auto;
		padding: 6px;
		overscroll-behavior: contain;
	}

	.command-group + .command-group {
		margin-top: 4px;
	}

	.command-group-label {
		padding: 8px 10px 4px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		font-weight: 500;
	}

	.command-item {
		width: 100%;
		min-height: 38px;
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto;
		align-items: center;
		gap: 10px;
		padding: 7px 10px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		text-align: left;
		cursor: pointer;
	}

	.command-item.active {
		background: var(--accent-soft);
	}

	.command-label {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-weight: 500;
	}

	.command-hint {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--text-tertiary);
	}

	.command-item :global(.command-enter) {
		color: var(--text-tertiary);
		opacity: 0;
		transition: opacity var(--motion-state) var(--ease-standard);
	}

	.command-item.active :global(.command-enter) {
		opacity: 1;
	}

	.command-cancel {
		display: none;
		min-height: 44px;
		padding: 0 4px 0 10px;
		border: 0;
		background: none;
		color: var(--intent-conversation);
		font-weight: 500;
		cursor: pointer;
	}

	@media (hover: none), (pointer: coarse) {
		.command-hint.shortcut {
			display: none;
		}
	}

	.command-empty {
		margin: 0;
		padding: 22px 12px;
		color: var(--text-secondary);
		text-align: center;
	}
	.command-unreached {
		margin: 0;
		padding: 8px 12px 4px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}

	.command-foot {
		display: flex;
		gap: 16px;
		padding: 8px 14px;
		border-top: 1px solid var(--border);
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}

	kbd {
		display: inline-grid;
		min-width: 18px;
		place-items: center;
		margin-right: 3px;
		padding: 0 4px;
		border: 1px solid var(--border-strong);
		border-radius: 3px;
		background: var(--surface);
		color: var(--text-tertiary);
		font: var(--t-label) var(--font-mono);
	}

	@keyframes command-fade {
		from {
			opacity: 0;
		}
	}

	@keyframes command-fade-out {
		to {
			opacity: 0;
		}
	}

	.command-more {
		margin: 0 0 4px;
		padding: 4px 10px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		cursor: pointer;
	}

	.command-more:hover {
		background: var(--wash-hover);
		color: var(--text-secondary);
	}

	@media (max-width: 760px) {
		.command-menu {
			margin-top: 10px;
			max-height: calc(100dvh - 20px);
		}

		.command-foot,
		.command-search kbd {
			display: none;
		}

		.command-cancel {
			display: inline-flex;
			align-items: center;
		}

		.command-item {
			min-height: 44px;
		}
	}
</style>
