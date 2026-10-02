<script lang="ts" module>
	export type AppSection = 'attention' | 'work' | 'people' | 'company';
</script>

<script lang="ts">
	import type { Snippet } from 'svelte';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import CircleDot from '@lucide/svelte/icons/circle-dot';
	import ClipboardList from '@lucide/svelte/icons/clipboard-list';
	import MessageSquare from '@lucide/svelte/icons/message-square';
	import Network from '@lucide/svelte/icons/network';
	import Users from '@lucide/svelte/icons/users';
	import Wordmark from '../glyph/Wordmark.svelte';

	/* The owner workspace's frame: wordmark, company, the four sections and the Exec conversation.
	 * A page renders any view inside it to show where that surface lives in the product. */
	let {
		company,
		active,
		attention = 0,
		rail,
		children
	}: {
		company: string;
		active: AppSection;
		/** Items waiting in Attention, shown on its tab. */
		attention?: number;
		/** An optional left rail (documents, rooms, people, company settings). */
		rail?: Snippet;
		children: Snippet;
	} = $props();

	const SECTIONS = [
		{ key: 'attention', label: 'Attention', icon: CircleDot },
		{ key: 'work', label: 'Work', icon: ClipboardList },
		{ key: 'people', label: 'People', icon: Users },
		{ key: 'company', label: 'Company', icon: Network }
	] as const;
</script>

<div class="app-frame">
	<header class="app-bar">
		<span class="app-brand"><Wordmark size={14} restless /></span>
		<span class="app-slash" aria-hidden="true">/</span>
		<span class="app-company">{company} <ChevronDown size={13} strokeWidth={2.2} /></span>
		<nav class="app-sections" aria-label="Sections">
			{#each SECTIONS as section (section.key)}
				{@const Icon = section.icon}
				<span class="app-section" class:active={section.key === active} aria-current={section.key === active ? 'page' : undefined}>
					<Icon size={13} strokeWidth={2.2} />
					{section.label}
					{#if section.key === 'attention' && attention > 0}<b>{attention}</b>{/if}
				</span>
			{/each}
		</nav>
		<span class="app-exec"><MessageSquare size={12} strokeWidth={2.2} /> Exec</span>
	</header>
	<div class="app-body" class:with-rail={!!rail}>
		{#if rail}<aside class="app-rail">{@render rail()}</aside>{/if}
		<main class="app-main">{@render children()}</main>
	</div>
</div>

<style>
	.app-frame {
		display: grid;
		grid-template-rows: auto 1fr;
		min-width: 0;
		height: 100%;
		background: var(--bg-app);
		color: var(--ink);
		font: 400 var(--t-body) var(--font-ui);
	}
	.app-bar {
		display: flex;
		align-items: center;
		gap: 10px;
		min-width: 0;
		padding: 8px 10px;
		border-bottom: 1px solid var(--border);
		background: var(--surface-pane);
	}
	.app-brand {
		display: inline-flex;
		color: var(--ink);
	}
	.app-slash {
		color: var(--text-tertiary);
	}
	.app-company {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		font: 500 var(--t-label) var(--font-mono);
		color: var(--text-secondary);
		white-space: nowrap;
	}
	.app-sections {
		display: flex;
		gap: 4px;
		margin-left: 8px;
		min-width: 0;
		overflow: hidden;
	}
	.app-section {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		padding: 4px 9px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--surface);
		font-weight: 560;
		white-space: nowrap;
		box-shadow: var(--bevel-subtle);
	}
	.app-section.active {
		border-color: color-mix(in srgb, var(--intent-conversation) 45%, transparent);
		background: var(--intent-conversation-soft);
		color: var(--intent-conversation);
	}
	.app-section b {
		padding: 0 5px;
		border-radius: 999px;
		background: var(--intent-direction);
		color: #fff;
		font: 600 var(--t-label)/16px var(--font-ui);
	}
	.app-exec {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		margin-left: auto;
		padding: 4px 8px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		font: 500 var(--t-label) var(--font-mono);
		text-transform: uppercase;
		color: var(--intent-conversation);
	}
	.app-body {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: var(--pane-gap, 4px);
		min-height: 0;
		padding: var(--pane-gap, 4px);
	}
	.app-body.with-rail {
		grid-template-columns: minmax(150px, 220px) minmax(0, 1fr);
	}
	.app-rail,
	.app-main {
		min-width: 0;
		min-height: 0;
		overflow: hidden;
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
		background: var(--surface-pane);
	}
	@media (max-width: 720px) {
		.app-sections,
		.app-company,
		.app-slash {
			display: none;
		}
		.app-body.with-rail {
			grid-template-columns: minmax(0, 1fr);
		}
		.app-rail {
			display: none;
		}
	}
</style>
