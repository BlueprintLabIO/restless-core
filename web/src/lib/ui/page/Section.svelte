<script lang="ts">
	/* One section of a settings page: a quiet title with its help and actions on
	 * one line, then either plain content or a grouped card of rows. Sections
	 * stack and scroll; a page never hides one behind a tab. */
	import type { Snippet } from 'svelte';
	import InfoTip from '../controls/InfoTip.svelte';

	let {
		title,
		info,
		count,
		id,
		group = true,
		actions,
		children
	}: {
		title?: string;
		info?: string;
		count?: number | string | null;
		/** Anchor for in-page links such as `#spend`. */
		id?: string;
		/** Rows inside a bordered card. Off for free content such as prose or a meter. */
		group?: boolean;
		actions?: Snippet;
		children: Snippet;
	} = $props();
</script>

<section class="section" {id} aria-label={title}>
	{#if title || actions}
		<header class="section-head">
			{#if title}<h2>{title}</h2>{/if}
			{#if count != null && count !== ''}<span class="section-count">{count}</span>{/if}
			{#if info}<InfoTip text={info} />{/if}
			{#if actions}<div class="section-actions">{@render actions()}</div>{/if}
		</header>
	{/if}
	<div class:group class="section-body">{@render children()}</div>
</section>

<style>
	.section {
		min-width: 0;
		scroll-margin-top: 24px;
	}
	.section-head {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		min-height: 32px;
		margin-bottom: 10px;
	}
	h2 {
		margin: 0;
		color: var(--ink);
		font-size: var(--t-body);
		font-weight: 600;
		letter-spacing: -0.005em;
	}
	.section-count {
		color: var(--text-tertiary);
		font-size: var(--t-body);
		font-variant-numeric: tabular-nums;
	}
	.section-actions {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		margin-left: auto;
	}
	.section-body {
		min-width: 0;
	}
	.section-body.group {
		overflow: hidden;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		box-shadow: var(--shadow-soft);
	}
	/* Rows and items divide themselves; the card only frames them. */
	.section-body.group > :global(* + *),
	.section-body.group > :global(form > * + *) {
		border-top: 1px solid var(--border);
	}
</style>
