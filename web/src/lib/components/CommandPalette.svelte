<script lang="ts">
	/* The ⌘K palette: jump to a surface, find a person or channel, or act —
	 * keyboard-first, over data that is already loaded (no new fetches).
	 * Bridge panel over the dark scrim: one input, grouped results, arrows to
	 * move, Enter to go, Escape to leave. */

	import { goto } from '$app/navigation';
	import { tick } from 'svelte';
	import MatrixGlyph, { GLYPHS, type GlyphName } from '$lib/primitives/MatrixGlyph.svelte';
	import { EXEC_FALLBACK_NAME } from '$lib/brand/brand';
	import type { ThreadSummary, TeamMember } from '$lib/model/view';

	type Item = {
		key: string;
		group: string;
		label: string;
		sub?: string;
		glyph: GlyphName;
		href?: string;
		act?: () => void;
	};

	let {
		threads,
		team,
		execName = EXEC_FALLBACK_NAME,
		companyId,
		open,
		onclose,
		onconversation
	}: {
		threads: ThreadSummary[];
		team: TeamMember[];
		execName?: string;
		companyId: string;
		open: boolean;
		onclose: () => void;
		onconversation?: () => void;
	} = $props();

	let query = $state('');
	let activeIndex = $state(0);
	let inputEl = $state<HTMLInputElement | undefined>();
	let restoreFocus = $state<HTMLElement | null>(null);

	const allItems = $derived.by((): Item[] => {
		const items: Item[] = [
			{
				key: 's-inbox',
				group: 'Surfaces',
				label: 'Inbox',
				sub: 'what needs your word',
				glyph: 'ring',
				href: `/${companyId}`
			},
			{
				key: 's-chats',
				group: 'Surfaces',
				label: 'Chats',
				sub: 'the conversational surface',
				glyph: 'quote',
				href: `/${companyId}/chats`
			},
			{
				key: 's-ops',
				group: 'Surfaces',
				label: 'Ops',
				sub: 'money, work, connections',
				glyph: 'work',
				href: `/${companyId}/ops`
			},
			{
				key: 's-people',
				group: 'Surfaces',
				label: 'People',
				sub: 'who is here and what they carry',
				glyph: 'people',
				href: `/${companyId}/people`
			},
			{
				key: 's-mission',
				group: 'Surfaces',
				label: 'Mission',
				sub: 'the constitution',
				glyph: 'rules',
				href: `/${companyId}/mission`
			},
			{
				key: 's-library',
				group: 'Surfaces',
				label: 'Library',
				sub: 'versioned records',
				glyph: 'square',
				href: `/${companyId}/library`
			},
			{
				key: 's-tape',
				group: 'Surfaces',
				label: 'Tape',
				sub: 'everything, raw',
				glyph: 'dots',
				href: `/${companyId}/tape`
			}
		];
		if (onconversation) {
			items.push({
				key: 'a-exec',
				group: 'Actions',
				label: `Talk to ${execName}`,
				sub: 'open the executive rail',
				glyph: 'p',
				act: onconversation
			});
		}
		items.push({
			key: 'a-new',
			group: 'Actions',
			label: 'Found a new company',
			sub: 'the door, then the floor',
			glyph: 'plus',
			href: '/onboarding'
		});
		for (const thread of threads) {
			items.push({
				key: `t-${thread.key}`,
				group: thread.kind === 'goal' ? 'Channels' : 'People',
				label: thread.kind === 'goal' ? `#${thread.title}` : thread.title,
				sub: thread.subtitle,
				glyph: thread.kind === 'goal' ? 'work' : thread.kind === 'executive' ? 'p' : 'people',
				href: `/${companyId}/chats?t=${encodeURIComponent(thread.key)}`
			});
		}
		for (const agent of team) {
			items.push({
				key: `p-${agent.id}`,
				group: 'Staff',
				label: agent.name,
				sub: agent.role,
				glyph: 'dots',
				href: `/${companyId}/staff/${agent.id}`
			});
		}
		return items;
	});

	const results = $derived.by(() => {
		const q = query.trim().toLowerCase();
		const matched = q
			? allItems.filter((item) => `${item.label} ${item.sub ?? ''}`.toLowerCase().includes(q))
			: allItems;
		/* group order is stable; cap keeps the panel glanceable */
		const groups: Array<{ name: string; items: Item[] }> = [];
		for (const item of matched) {
			let group = groups.find((g) => g.name === item.group);
			if (!group) {
				group = { name: item.group, items: [] };
				groups.push(group);
			}
			if (group.items.length < 6) group.items.push(item);
		}
		return groups.slice(0, 5);
	});

	const flat = $derived(results.flatMap((group) => group.items));

	$effect(() => {
		if (open) {
			restoreFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
			query = '';
			activeIndex = 0;
			tick().then(() => inputEl?.focus());
		} else {
			restoreFocus?.focus?.();
			restoreFocus = null;
		}
	});

	$effect(() => {
		if (activeIndex > flat.length - 1) activeIndex = Math.max(0, flat.length - 1);
	});

	function choose(item: Item) {
		onclose();
		if (item.act) item.act();
		else if (item.href) goto(item.href as `/${string}`);
	}

	function onKeydown(event: KeyboardEvent) {
		if (event.key === 'ArrowDown') {
			event.preventDefault();
			activeIndex = Math.min(activeIndex + 1, flat.length - 1);
		} else if (event.key === 'ArrowUp') {
			event.preventDefault();
			activeIndex = Math.max(activeIndex - 1, 0);
		} else if (event.key === 'Enter') {
			event.preventDefault();
			const item = flat[activeIndex];
			if (item) choose(item);
		} else if (event.key === 'Escape') {
			event.preventDefault();
			onclose();
		}
	}
</script>

{#if open}
	<div class="pal-wrap">
		<button class="pal-scrim" type="button" aria-label="Close palette" onclick={onclose}></button>
		<div
			class="pal-panel"
			role="dialog"
			tabindex="-1"
			aria-modal="true"
			aria-label="Command palette"
		>
			<div class="pal-inputrow">
				<input
					bind:this={inputEl}
					class="pal-input"
					type="text"
					placeholder="Search or jump…"
					aria-label="Search or jump"
					bind:value={query}
					onkeydown={onKeydown}
				/>
				<span class="pal-kbd mono">esc</span>
			</div>
			<div class="pal-results" role="listbox" aria-label="Results">
				{#each results as group (group.name)}
					<div class="pal-group mono">{group.name}</div>
					{#each group.items as item (item.key)}
						{@const index = flat.indexOf(item)}
						<button
							class="pal-row"
							class:on={index === activeIndex}
							type="button"
							role="option"
							aria-selected={index === activeIndex}
							onmouseenter={() => (activeIndex = index)}
							onclick={() => choose(item)}
						>
							<span class="pal-glyph"><MatrixGlyph rows={GLYPHS[item.glyph]} size={10} /></span>
							<span class="pal-label">{item.label}</span>
							{#if item.sub}<span class="pal-sub">{item.sub}</span>{/if}
						</button>
					{/each}
				{:else}
					<p class="pal-empty">Nothing matches — the tape sees everything, just not that.</p>
				{/each}
			</div>
		</div>
	</div>
{/if}

<style>
	.pal-wrap {
		position: fixed;
		inset: 0;
		z-index: var(--z-palette);
		display: flex;
		justify-content: center;
		align-items: flex-start;
		padding-top: 16vh;
	}
	.pal-scrim {
		position: absolute;
		inset: 0;
		border: 0;
		background: rgba(0, 0, 0, 0.62);
		cursor: default;
	}
	.pal-panel {
		position: relative;
		width: min(560px, calc(100vw - 32px));
		background: var(--surface);
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		box-shadow: var(--bevel-subtle), var(--shadow-lift);
		overflow: hidden;
		animation: pal-in 0.16s ease both;
	}
	@keyframes pal-in {
		from {
			opacity: 0;
			transform: translateY(-6px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.pal-panel {
			animation: none;
		}
	}
	.pal-inputrow {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 12px 14px;
		border-bottom: 1px solid var(--border);
	}
	.pal-input {
		flex: 1;
		border: 0;
		background: transparent;
		color: var(--ink);
		font: inherit;
		font-size: 14.5px;
		outline: none;
	}
	.pal-input::placeholder {
		color: var(--text-tertiary);
	}
	.pal-kbd {
		font-size: 10px;
		color: var(--text-tertiary);
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		padding: 2px 6px;
	}
	.pal-results {
		max-height: 340px;
		overflow-y: auto;
		padding: 6px;
	}
	.pal-group {
		font-size: 10px;
		letter-spacing: 0.12em;
		text-transform: uppercase;
		color: var(--text-tertiary);
		padding: 8px 10px 4px;
	}
	.pal-row {
		display: flex;
		align-items: center;
		gap: 10px;
		width: 100%;
		padding: 8px 10px;
		border: 0;
		border-radius: var(--radius-md);
		background: none;
		font: inherit;
		font-size: 13px;
		color: var(--t1);
		text-align: left;
		cursor: pointer;
	}
	.pal-row.on {
		background: var(--surface-alt);
	}
	.pal-row.on .pal-label {
		color: var(--ink);
	}
	.pal-glyph {
		display: flex;
		color: var(--text-tertiary);
		flex: 0 0 auto;
	}
	.pal-row.on .pal-glyph {
		color: var(--ink);
	}
	.pal-label {
		font-weight: 500;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.pal-sub {
		margin-left: auto;
		flex: 0 1 auto;
		font-size: 11.5px;
		color: var(--text-tertiary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.pal-empty {
		padding: 18px 12px;
		font-size: 12.5px;
		color: var(--text-secondary);
	}
</style>
