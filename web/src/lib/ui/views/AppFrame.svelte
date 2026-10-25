<script lang="ts" module>
	export type AppSection = 'attention' | 'work' | 'library' | 'people' | 'company';
</script>

<script lang="ts">
	import type { Snippet } from 'svelte';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import SearchIcon from '@lucide/svelte/icons/search';
	import MatrixGlyph, { GLYPHS } from '../glyph/MatrixGlyph.svelte';
	import Wordmark from '../glyph/Wordmark.svelte';

	/* The owner workspace's frame, as the cockpit's top bar draws it: wordmark, company, the
	 * surfaces and the Exec control. A page renders any view inside it to show where that surface
	 * lives. Read-only by default; with `onselect` the surfaces are buttons, and with `exec` and
	 * `onexec` the Exec control opens a right-hand rail, so a page can be walked through. */
	let {
		company,
		active,
		attention = 0,
		sections = ['attention', 'work', 'library', 'people', 'company'],
		onselect,
		execLive = true,
		execOpen = false,
		onexec,
		exec,
		rail,
		children
	}: {
		company: string;
		active: AppSection;
		/** Items waiting in the Inbox, shown on its tab. */
		attention?: number;
		sections?: AppSection[];
		/** Makes the surfaces buttons. */
		onselect?: (section: AppSection) => void;
		/** Exec is awake: the lamp is lit. */
		execLive?: boolean;
		execOpen?: boolean;
		/** Makes the Exec control a button that shows and hides `exec`. */
		onexec?: () => void;
		/** The Exec conversation, beside the surface while `execOpen`. */
		exec?: Snippet;
		/** An optional left rail (documents, rooms, people, company settings). */
		rail?: Snippet;
		children: Snippet;
	} = $props();

	const SECTION = {
		attention: { label: 'Inbox', glyph: GLYPHS.alert },
		work: { label: 'Work', glyph: GLYPHS.briefcase },
		library: { label: 'Library', glyph: GLYPHS.rules },
		people: { label: 'People', glyph: GLYPHS.group },
		company: { label: 'Company', glyph: GLYPHS.key }
	} as const;
	const showExec = $derived(!!exec && execOpen);
</script>

{#snippet sectionBody(key: AppSection)}
	<span class="app-mark" aria-hidden="true"><MatrixGlyph rows={SECTION[key].glyph} size={12} /></span>
	{SECTION[key].label}
	{#if key === 'attention' && attention > 0}<b aria-label="{attention} waiting">{attention}</b>{/if}
{/snippet}

<div class="view-shell">
	<div class="app-frame">
		<header class="app-bar">
			<span class="app-brand"><Wordmark size={16} /></span>
			<span class="app-slash" aria-hidden="true">/</span>
			<span class="app-company">{company} <ChevronDown size={13} strokeWidth={2.2} /></span>
			<nav class="app-sections" aria-label="Sections">
				{#each sections as key (key)}
					{#if onselect}
						<button
							type="button"
							class="app-section"
							class:active={key === active}
							data-surface={key}
							aria-current={key === active ? 'page' : undefined}
							onclick={() => onselect(key)}>{@render sectionBody(key)}</button
						>
					{:else}
						<span class="app-section" class:active={key === active} data-surface={key} aria-current={key === active ? 'page' : undefined}
							>{@render sectionBody(key)}</span
						>
					{/if}
				{/each}
			</nav>
			<span class="app-search" aria-hidden="true"><SearchIcon size={14} strokeWidth={2} /> <kbd>⌘K</kbd></span>
			{#if onexec}
				<button type="button" class="app-exec" class:live={execLive} class:on={showExec} aria-expanded={showExec} onclick={onexec}
					><span class="app-lamp" aria-hidden="true"></span>Exec</button
				>
			{:else}
				<span class="app-exec" class:live={execLive}><span class="app-lamp" aria-hidden="true"></span>Exec</span>
			{/if}
		</header>
		<div class="app-body" class:with-rail={!!rail} class:with-exec={showExec}>
			{#if rail}<aside class="app-rail">{@render rail()}</aside>{/if}
			<main class="app-main">{@render children()}</main>
			{#if showExec}<aside class="app-exec-rail" aria-label="Exec">{@render exec?.()}</aside>{/if}
		</div>
	</div>
</div>

<style>
	/* Breakpoints follow this view's own width, not the window's: the same view sits in a full
	 * workspace pane, a narrow side panel or a phone. */
	.view-shell {
		container-type: inline-size;
		width: 100%;
		height: 100%;
		min-width: 0;
	}
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
		padding: 7px 10px;
		border-bottom: 1px solid var(--border);
		background: var(--glass);
	}
	.app-brand {
		display: inline-flex;
	}
	.app-slash {
		color: var(--text-tertiary);
	}
	.app-company {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		font-weight: 500;
		color: var(--text-secondary);
		white-space: nowrap;
	}
	/* One quiet segmented control, as in the cockpit. */
	.app-sections {
		display: flex;
		gap: 2px;
		min-width: 0;
		margin-left: 8px;
		padding: 3px;
		border-radius: calc(var(--radius-control) + 3px);
		background: color-mix(in srgb, var(--accent-soft) 70%, transparent);
	}
	.app-section {
		--tab-tone: var(--surface-attention);
		position: relative;
		display: inline-flex;
		align-items: center;
		gap: 6px;
		height: 26px;
		padding: 0 10px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-secondary);
		font: 500 var(--t-body) var(--font-ui);
		white-space: nowrap;
		transition:
			color var(--motion-state) var(--ease-standard),
			background-color var(--motion-state) var(--ease-standard),
			box-shadow var(--motion-state) var(--ease-standard);
	}
	.app-section[data-surface='work'] {
		--tab-tone: var(--surface-work);
	}
	.app-section[data-surface='library'] {
		--tab-tone: var(--intent-direction);
	}
	.app-section[data-surface='people'] {
		--tab-tone: var(--surface-people);
	}
	.app-section[data-surface='company'] {
		--tab-tone: var(--surface-authority);
	}
	button.app-section {
		cursor: pointer;
	}
	button.app-section:not(.active):hover {
		background: color-mix(in srgb, var(--highlight) 46%, transparent);
		color: var(--ink);
	}
	.app-section:focus-visible,
	.app-exec:focus-visible {
		outline: 2px solid color-mix(in srgb, var(--intent-conversation) 45%, transparent);
		outline-offset: 1px;
	}
	.app-section.active {
		background: var(--glass-pane-strong);
		box-shadow:
			0 0 0 1px color-mix(in srgb, var(--tab-tone) 16%, var(--control-edge)),
			0 1px 2px rgba(65, 76, 104, 0.08);
		color: var(--ink);
	}
	.app-mark {
		display: inline-flex;
		color: var(--tab-tone);
		opacity: 0.72;
	}
	.app-section.active .app-mark,
	button.app-section:hover .app-mark {
		opacity: 1;
	}
	.app-section b {
		position: absolute;
		top: -6px;
		right: -7px;
		min-width: 16px;
		height: 16px;
		display: grid;
		place-items: center;
		padding: 0 4px;
		border-radius: 999px;
		background: var(--intent-authority);
		box-shadow: 0 0 0 2px var(--surface-pane);
		color: var(--text-inverse);
		font: 600 var(--t-label) var(--font-ui);
	}
	.app-search {
		display: inline-flex;
		align-items: center;
		gap: 7px;
		margin-left: auto;
		color: var(--text-tertiary);
	}
	.app-search kbd {
		padding: 1px 5px;
		border: 1px solid var(--border-strong);
		border-radius: 3px;
		font: var(--t-label) var(--font-mono);
	}
	.app-exec {
		display: inline-flex;
		align-items: center;
		gap: 7px;
		height: 28px;
		padding: 0 11px 0 9px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: color-mix(in srgb, var(--highlight) 56%, transparent);
		box-shadow: var(--control-depth);
		color: var(--text-secondary);
		font: 500 var(--t-body) var(--font-ui);
		white-space: nowrap;
	}
	button.app-exec {
		cursor: pointer;
	}
	button.app-exec:hover {
		border-color: color-mix(in srgb, var(--intent-conversation) 40%, var(--control-edge));
		color: var(--ink);
	}
	.app-exec.on {
		border-color: color-mix(in srgb, var(--intent-conversation) 32%, var(--control-edge));
		background: var(--intent-conversation-soft);
		color: var(--ink);
	}
	.app-lamp {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--status-offline);
	}
	.live .app-lamp {
		background: var(--state-success);
		box-shadow: 0 0 0 3px color-mix(in srgb, var(--state-success) 16%, transparent);
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
	.app-body.with-exec {
		grid-template-columns: minmax(0, 1fr) minmax(260px, 340px);
	}
	.app-body.with-rail.with-exec {
		grid-template-columns: minmax(150px, 220px) minmax(0, 1fr) minmax(260px, 340px);
	}
	.app-rail,
	.app-main,
	.app-exec-rail {
		min-width: 0;
		min-height: 0;
		overflow: hidden;
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
		background: var(--surface-pane);
	}
	@container (max-width: 920px) {
		.app-search {
			display: none;
		}
		.app-sections {
			margin-left: auto;
		}
	}
	@container (max-width: 720px) {
		.app-company,
		.app-slash,
		.app-mark {
			display: none;
		}
		.app-section {
			padding: 0 8px;
		}
		.app-body.with-rail,
		.app-body.with-exec,
		.app-body.with-rail.with-exec {
			grid-template-columns: minmax(0, 1fr);
		}
		.app-rail {
			display: none;
		}
		/* On a narrow pane the conversation takes the pane, as on a phone. */
		.with-exec .app-main {
			display: none;
		}
	}
	/* A phone: the surfaces take their own row, each an equal share of it. */
	@container (max-width: 520px) {
		.app-bar {
			flex-wrap: wrap;
			row-gap: 8px;
		}
		.app-exec {
			margin-left: auto;
		}
		.app-sections {
			order: 3;
			width: 100%;
			margin-left: 0;
		}
		.app-section {
			flex: 1;
			justify-content: center;
			padding: 0 4px;
			font-size: var(--t-label);
		}
	}
</style>
