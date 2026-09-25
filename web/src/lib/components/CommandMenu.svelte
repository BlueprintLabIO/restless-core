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
</script>

<script lang="ts">
	/* One place to jump to anything in the company: surfaces, settings pages,
	 * Work, goals, people and other companies. It replaces hunting through the
	 * switcher, the settings spine and the Work map when the owner already
	 * knows what they want. */
	import { tick } from 'svelte';
	import { goto } from '$app/navigation';
	import Search from '@lucide/svelte/icons/search';
	import CornerDownLeft from '@lucide/svelte/icons/corner-down-left';

	let {
		open = $bindable(false),
		commands
	}: {
		open?: boolean;
		commands: Command[];
	} = $props();

	let dialog: HTMLDialogElement | undefined = $state();
	let input: HTMLInputElement | undefined = $state();
	let list: HTMLElement | undefined = $state();
	let query = $state('');
	let active = $state(0);
	let returnFocus: HTMLElement | null = null;

	const PER_GROUP = 8;

	const results = $derived.by(() => {
		const words = query.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
		const matches = commands.filter((command) => {
			if (!words.length) return true;
			const haystack =
				`${command.label} ${command.hint ?? ''} ${command.group} ${command.keywords ?? ''}`.toLocaleLowerCase();
			return words.every((word) => haystack.includes(word));
		});
		/* Label-prefix matches first; otherwise keep the author's order. */
		const first = words[0];
		const ranked = first
			? matches.toSorted(
					(a, b) =>
						Number(!a.label.toLocaleLowerCase().startsWith(first)) -
						Number(!b.label.toLocaleLowerCase().startsWith(first))
				)
			: matches;
		const groups = new Map<string, Command[]>();
		for (const command of ranked) {
			const group = groups.get(command.group) ?? [];
			if (group.length < PER_GROUP) group.push(command);
			groups.set(command.group, group);
		}
		return [...groups].map(([group, items]) => ({ group, items }));
	});
	const flat = $derived(results.flatMap((group) => group.items));

	$effect(() => {
		void query;
		active = 0;
	});

	$effect(() => {
		if (!dialog) return;
		if (open && !dialog.open) {
			returnFocus = document.activeElement as HTMLElement | null;
			query = '';
			active = 0;
			dialog.showModal();
			void tick().then(() => input?.focus());
		} else if (!open && dialog.open) {
			dialog.close();
		}
	});

	function close() {
		open = false;
		returnFocus?.focus?.();
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
			event.preventDefault();
			choose(flat[active]);
		}
	}
</script>

<dialog
	bind:this={dialog}
	class="command-menu"
	aria-label="Search and jump"
	onclose={() => (open = false)}
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
				placeholder="Jump to a surface, Work, person or setting…"
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
		<div class="command-results" id="command-results" role="listbox" bind:this={list}>
			{#each results as section (section.group)}
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
				</div>
			{:else}
				<p class="command-empty">No match for “{query.trim()}”.</p>
			{/each}
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
